use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Data, DeriveInput, Expr, Fields, Lit, Variant, parse_macro_input};

/// Derives [`minicbor::Encode`] for a C-like enum.
///
/// The enum must have a `#[repr(u8)]`, `#[repr(u16)]`, or `#[repr(u32)]` attribute. Each variant
/// is encoded as its integer discriminant using the corresponding minicbor encoder method.
///
/// An optional *wildcard* variant — a single-field tuple variant whose field type matches the repr
/// type (e.g. `Unknown(u8)`) — may be included. It is encoded by writing the wrapped value
/// directly.
///
/// # Requirements
///
/// - The type must be an enum.
/// - Must have `#[repr(u8 | u16 | u32)]`.
/// - All non-wildcard variants must be unit variants.
/// - At most one wildcard (tuple) variant is allowed, and its field type must equal the repr type.
/// - Discriminants must be integer literals and must not overflow the repr type.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Encode, Decode)]
/// #[repr(u8)]
/// enum Status {
///     Active  = 1,
///     Pending = 2,
///     Custom(u8),
/// }
/// ```
#[proc_macro_derive(Encode)]
pub fn derive_cbor_encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_encode(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(Decode)]
pub fn derive_cbor_decode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_decode(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

enum ReprInt {
    U8,
    U16,
    U32,
}

impl ReprInt {
    /// The Rust primitive type token (`u8`, `u16`, `u32`).
    fn rust_ty(&self) -> TokenStream2 {
        match self {
            ReprInt::U8 => quote! { u8  },
            ReprInt::U16 => quote! { u16 },
            ReprInt::U32 => quote! { u32 },
        }
    }

    /// The minicbor `Encoder` method name (`u8`, `u16`, `u32`).
    fn encode_method(&self) -> syn::Ident {
        let name = match self {
            ReprInt::U8 => "u8",
            ReprInt::U16 => "u16",
            ReprInt::U32 => "u32",
        };
        syn::Ident::new(name, Span::call_site())
    }

    /// The minicbor `Decoder` method name (`u8`, `u16`, `u32`).
    fn decode_method(&self) -> syn::Ident {
        self.encode_method() // same names on Decoder
    }
}

/// Extracts the integer repr from `#[repr(uN)]`. Errors if missing or unsupported.
fn parse_repr(input: &DeriveInput) -> syn::Result<ReprInt> {
    for attr in &input.attrs {
        if !attr.path().is_ident("repr") {
            continue;
        }
        // repr takes a list of idents: #[repr(u8)], #[repr(C)], etc.
        let mut found = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("u8") {
                found = Some(ReprInt::U8);
            } else if meta.path.is_ident("u16") {
                found = Some(ReprInt::U16);
            } else if meta.path.is_ident("u32") {
                found = Some(ReprInt::U32);
            }
            // Silently skip unrecognised repr args (e.g. `align`, `C`, …).
            Ok(())
        })?;
        if let Some(r) = found {
            return Ok(r);
        }
    }
    Err(syn::Error::new(
        Span::call_site(),
        "CborEncode/CborDecode requires #[repr(u8)], #[repr(u16)], or #[repr(u32)]",
    ))
}

struct VariantInfo {
    ident: syn::Ident,
    discriminant: u32, // wide enough for all supported reprs
    is_wildcard: bool,
}

fn parse_variants(data: &Data, repr: &ReprInt) -> syn::Result<Vec<VariantInfo>> {
    let variants = match data {
        Data::Enum(e) => &e.variants,
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                "CborEncode/CborDecode can only be derived for enums",
            ));
        }
    };

    let max_disc: u32 = match repr {
        ReprInt::U8 => u8::MAX as u32,
        ReprInt::U16 => u16::MAX as u32,
        ReprInt::U32 => u32::MAX,
    };

    let mut next: u32 = 0;
    let mut wildcard_seen = false;
    let mut out = Vec::with_capacity(variants.len());

    for v in variants {
        match &v.fields {
            Fields::Unit => {
                let disc = explicit_discriminant(v)?.unwrap_or(next);
                if disc > max_disc {
                    return Err(syn::Error::new_spanned(
                        v,
                        format!("discriminant {disc} overflows repr type"),
                    ));
                }
                next = disc
                    .checked_add(1)
                    .ok_or_else(|| syn::Error::new_spanned(v, "discriminant overflows u32"))?;
                out.push(VariantInfo {
                    ident: v.ident.clone(),
                    discriminant: disc,
                    is_wildcard: false,
                });
            }
            Fields::Unnamed(f) if f.unnamed.len() == 1 => {
                if wildcard_seen {
                    return Err(syn::Error::new_spanned(
                        v,
                        "CborEncode/CborDecode: only one wildcard (tuple) variant is allowed",
                    ));
                }
                let field = f.unnamed.first().unwrap();
                let repr_ty = repr.rust_ty().to_string();
                if !is_type_ident(&field.ty, &repr_ty) {
                    return Err(syn::Error::new_spanned(
                        &field.ty,
                        format!(
                            "CborEncode/CborDecode: wildcard variant field must match repr type (`{repr_ty}`)"
                        ),
                    ));
                }
                wildcard_seen = true;
                out.push(VariantInfo {
                    ident: v.ident.clone(),
                    discriminant: 0,
                    is_wildcard: true,
                });
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    v,
                    "CborEncode/CborDecode: only unit variants and a single tuple(repr_type) wildcard are supported",
                ));
            }
        }
    }

    Ok(out)
}

fn explicit_discriminant(v: &Variant) -> syn::Result<Option<u32>> {
    let Some((_, expr)) = &v.discriminant else {
        return Ok(None);
    };
    match expr {
        Expr::Lit(el) => match &el.lit {
            Lit::Int(i) => Ok(Some(i.base10_parse::<u32>()?)),
            _ => Err(syn::Error::new_spanned(expr, "expected integer literal")),
        },
        _ => Err(syn::Error::new_spanned(
            expr,
            "CborEncode/CborDecode: only integer literal discriminants are supported",
        )),
    }
}

/// Emits a typed integer literal matching the repr, e.g. `4u8`, `4u16`, `4u32`.
fn typed_int_lit(value: u32, repr: &ReprInt) -> proc_macro2::Literal {
    match repr {
        ReprInt::U8 => proc_macro2::Literal::u8_suffixed(value as u8),
        ReprInt::U16 => proc_macro2::Literal::u16_suffixed(value as u16),
        ReprInt::U32 => proc_macro2::Literal::u32_suffixed(value),
    }
}

fn is_type_ident(ty: &syn::Type, ident: &str) -> bool {
    matches!(ty, syn::Type::Path(tp)
        if tp.qself.is_none() && tp.path.is_ident(ident))
}

fn encode_impl_generics(input: &DeriveInput) -> TokenStream2 {
    let existing = &input.generics.params;
    if existing.is_empty() {
        quote! { <__C__> }
    } else {
        quote! { <__C__, #existing> }
    }
}

fn expand_encode(input: DeriveInput) -> syn::Result<TokenStream2> {
    let repr = parse_repr(&input)?;
    let name = &input.ident;
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();
    let impl_generics = encode_impl_generics(&input);
    let encode_method = repr.encode_method();

    let variants = parse_variants(&input.data, &repr)?;

    let arms = variants.iter().map(|v| {
        let ident = &v.ident;
        if v.is_wildcard {
            quote! { #name::#ident(v) => *v }
        } else {
            let disc_lit = typed_int_lit(v.discriminant, &repr);
            quote! { #name::#ident => #disc_lit }
        }
    });

    Ok(quote! {
        impl #impl_generics minicbor::Encode<__C__> for #name #ty_generics #where_clause {
            fn encode<__W__: minicbor::encode::Write>(
                &self,
                __e__: &mut minicbor::Encoder<__W__>,
                __ctx__: &mut __C__,
            ) -> ::core::result::Result<(), minicbor::encode::Error<__W__::Error>> {
                __e__.#encode_method(match self {
                    #(#arms,)*
                })?;
                ::core::result::Result::Ok(())
            }
        }
    })
}

fn expand_decode(input: DeriveInput) -> syn::Result<TokenStream2> {
    let repr = parse_repr(&input)?;
    let name = &input.ident;
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();
    let existing_params = &input.generics.params;
    let decode_method = repr.decode_method();

    let variants = parse_variants(&input.data, &repr)?;

    let (unit_variants, wildcard_variants): (Vec<_>, Vec<_>) =
        variants.iter().partition(|v| !v.is_wildcard);

    let unit_arms = unit_variants.iter().map(|v| {
        let ident = &v.ident;
        // Emit a typed literal (e.g. `4u16`) — casts are not valid in pattern position.
        let disc_lit = typed_int_lit(v.discriminant, &repr);
        quote! { #disc_lit => #name::#ident }
    });

    let wildcard_arm = if let Some(wc) = wildcard_variants.first() {
        let ident = &wc.ident;
        quote! { v => #name::#ident(v) }
    } else {
        quote! {
            _ => return Err(minicbor::decode::Error::message(
                "unknown discriminant for enum"
            ))
        }
    };

    Ok(quote! {
        impl<'__b__, #existing_params __C__> minicbor::Decode<'__b__, __C__> for #name #ty_generics #where_clause {
            fn decode(
                __d__: &mut minicbor::Decoder<'__b__>,
                __ctx__: &mut __C__,
            ) -> ::core::result::Result<Self, minicbor::decode::Error> {
                ::core::result::Result::Ok(match __d__.#decode_method()? {
                    #(#unit_arms,)*
                    #wildcard_arm
                })
            }
        }
    })
}
