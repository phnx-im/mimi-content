// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![recursion_limit = "4096"]

extern crate proc_macro;

use proc_macro::TokenStream;
use punctuated::Punctuated;
use quote::quote;
use syn::*;

/// Serialize an enum as a list of the discriminator followed by all fields.
#[proc_macro_derive(ExternallyTagged)]
pub fn derive_externally_tagged(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let enum_name = ast.ident;

    let Data::Enum(data) = ast.data else {
        panic!("Only enums can be externally tagged");
    };

    // Example:
    // Self::Variant2 { .. } => 2,
    let mut field_num_impls = Vec::new();

    // Example:
    // Self::Variant2 { field1, field2 } => {
    //     state.serialize_element(&2);
    //     state.serialize_element(field1);
    //     state.serialize_element(field2);
    // },
    let mut variant_serialize_impls = Vec::new();

    // Example:
    // 2 => Self::Variant2 {
    //     field1: seq.next_element()?
    //         .ok_or_else(|| de::Error::invalid_length(next_index(), &"enum EnumName"))?,
    //     field2: seq.next_element()?
    //         .ok_or_else(|| de::Error::invalid_length(next_index(), &"enum EnumName"))?,
    // }
    let mut variant_deserialize_impls = Vec::new();

    for variant in data.variants {
        let variant_name = variant.ident;

        let fields = match variant.fields {
            Fields::Named(fields_named) => fields_named.named,
            Fields::Unnamed(_fields_unnamed) => panic!("All fields must be named"),
            Fields::Unit => Punctuated::new(), // Handle no fields like empty list of named fields
        };

        // Each enum variant must have an explicit discriminator,
        // like "Variant { field: type } = discriminator"
        let Expr::Lit(ExprLit {
            lit: Lit::Int(variant_discriminant),
            ..
        }) = variant
            .discriminant
            .expect("All enum variants must have an explicit discriminant value")
            .1
        else {
            panic!("Discriminant values must be integers")
        };

        // TODO: Handle u16 and other types by parsing #[repr(inttype)]
        let variant_discriminant = variant_discriminant
            .base10_parse::<u8>()
            .expect("Discriminant must be a valid u8");

        let mut field_names = Vec::new();
        let mut field_serialize_impls = Vec::new();
        let mut field_deserialize_impls = Vec::new();

        field_serialize_impls.push(quote! { state.serialize_element(&#variant_discriminant)?; });

        for field in &fields {
            let field_name = field.ident.as_ref().expect("All fields are named");

            field_names.push(field_name);
            field_serialize_impls.push(quote! { state.serialize_element(#field_name)?; });
            field_deserialize_impls.push(quote! {
                #field_name: seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(next_index(), &"enum #enum_name"))?,
            });
        }

        let num = field_names.len();

        field_num_impls.push(quote! {
            Self::#variant_name { .. } => #num,
        });

        variant_serialize_impls.push(quote! {
            Self::#variant_name { #(#field_names),* } => { #(#field_serialize_impls)* },
        });

        variant_deserialize_impls.push(quote! {
            #variant_discriminant => Self::#variant_name { #(#field_deserialize_impls)* },
        });
    }

    quote! {
        impl ExternallyTagged for #enum_name {
            fn num_fields(&self) -> usize {
                match self {
                    #(#field_num_impls)*
                }
            }

            fn serialize_fields<S: serde::ser::SerializeSeq>(&self, state: &mut S) -> Result<(), S::Error> {
                Ok(match self {
                    #(#variant_serialize_impls)*
                })
            }

            fn deserialize_fields<'a, S: serde::de::SeqAccess<'a>>(seq: &mut S, next_index: &mut impl FnMut() -> usize) -> Result<Self, S::Error> {
                let target_discriminant = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(next_index(), &"enum #enum_name"))?;

                Ok(match target_discriminant {
                    #(#variant_deserialize_impls)*
                    u => {
                        return Err(de::Error::invalid_value(
                            Unexpected::Unsigned(u64::from(u)),
                            &"A valid discriminant for enum #enum_name",
                        ))
                    }
                })
            }
        }
    }
    .into()
}

#[proc_macro_derive(Serde_custom_u8)]
pub fn derive_serde_custom_u8(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let enum_name = ast.ident;

    let Data::Enum(data) = ast.data else {
        panic!();
    };

    // Example:
    // Self::Variant2 => 2,
    // Self::Custom(u) => u,
    let mut variant_serializations = Vec::new();

    // Example:
    // 2 => Self::Variant2,
    // u => Self::Custom(u),
    let mut variant_deserializations = Vec::new();

    let mut found_custom_field = false;
    for variant in data.variants {
        if found_custom_field {
            panic!("There should be no more variants after Custom(u8)");
        }

        let variant_name = variant.ident;

        match variant.fields {
            Fields::Unit => {
                let Expr::Lit(ExprLit {
                    lit: Lit::Int(discriminant),
                    ..
                }) = variant
                    .discriminant
                    .expect("All normal enum variants must have an explicit discriminant value")
                    .1
                else {
                    panic!("Discriminant values must be integers")
                };

                let discriminant = discriminant
                    .base10_parse::<u8>()
                    .expect("Discriminant must be a valid u8");

                variant_serializations.push(quote! {
                    Self::#variant_name => #discriminant,
                });
                variant_deserializations.push(quote! {
                    #discriminant => Self::#variant_name,
                });
            }
            Fields::Unnamed(_fields_unnamed) => {
                if variant_name == "Custom" {
                    variant_serializations.push(quote! {
                        Self::Custom(u) => *u,
                    });
                    variant_deserializations.push(quote! {
                        u => Self::Custom(u),
                    });
                    found_custom_field = true;
                } else {
                    panic!("Enum cannot contain fields except for Custom(u8)");
                }
            }
            Fields::Named(_fields_named) => {
                panic!("Enum cannot contain fields except for Custom(u8)")
            }
        };
    }

    if !found_custom_field {
        panic!("The last variant must be Custom(u8)");
    }

    quote! {
        impl Serialize for #enum_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                match self {
                    #(#variant_serializations)*
                }
                .serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for #enum_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                let value = u8::deserialize(deserializer)?;

                Ok(match value {
                    #(#variant_deserializations)*
                })
            }
        }
    }
    .into()
}

#[proc_macro_derive(Serde_list, attributes(externally_tagged))]
pub fn derive_serde_list(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = ast.ident;

    // Example:
    // num_fields += 1;
    // num_fields += 1;
    // num_fields += ExternallyTagged::num_fields(&self.external);
    let mut field_num_updates = Vec::new();

    // Example:
    // state.serialize_element(&self.field1)?;
    // state.serialize_element(&self.field2)?;
    // ExternallyTagged::serialize_fields(&self.external, &mut state)?;
    let mut field_serializations = Vec::new();

    // Example:
    // field1: seq
    //     .next_element()?
    //     .ok_or_else(|| de::Error::invalid_length(next_index(), &self))?,
    // field2: seq
    //     .next_element()?
    //     .ok_or_else(|| de::Error::invalid_length(next_index(), &self))?,
    // external: ExternallyTagged::deserialize_fields(&mut seq, &mut next_index)?,
    //
    let mut field_deserializations = Vec::new();

    let Data::Struct(data) = ast.data else {
        panic!();
    };

    let Fields::Named(fields) = data.fields else {
        panic!();
    };

    'fields: for field in &fields.named {
        let Some(field_name) = &field.ident else {
            panic!();
        };

        for attr in &field.attrs {
            if attr.path().is_ident("externally_tagged") {
                field_num_updates.push(quote! {
                    // discriminator + fields
                    num_fields += 1 + ExternallyTagged::num_fields(&self.#field_name);
                });

                field_serializations.push(quote! {
                    ExternallyTagged::serialize_fields(&self.#field_name, &mut state)?;
                });

                field_deserializations.push(quote! {
                    #field_name: ExternallyTagged::deserialize_fields(&mut seq, &mut next_index)?,
                });

                continue 'fields;
            }
        }

        field_num_updates.push(quote! {
            num_fields += 1;
        });

        field_serializations.push(quote! {
            state.serialize_element(&self.#field_name)?;
        });

        field_deserializations.push(quote! {
            #field_name: seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(next_index(), &self))?,
        });
    }

    quote! {
        impl serde::Serialize for #struct_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut num_fields = 0;

                #(#field_num_updates)*

                let mut state = serializer.serialize_seq(Some(num_fields))?;

                #(#field_serializations)*

                state.end()
            }
        }

        impl<'de> Deserialize<'de> for #struct_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct NestedPartVisitor;
                impl<'de> Visitor<'de> for NestedPartVisitor {
                    type Value = NestedPart;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("struct #struct_name")
                    }

                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::SeqAccess<'de>,
                    {
                        // Keep track of current array index for better error messages
                        let mut current_index = 0;
                        let mut next_index = || {
                            let old = current_index;
                            current_index += 1;
                            old
                        };

                        let result = NestedPart {
                            #(#field_deserializations)*
                        };

                        assert!(
                            seq.next_element::<u8>()?.is_none(),
                            "parsing finished with data remaining"
                        );

                        Ok(result)
                    }
                }
                deserializer.deserialize_seq(NestedPartVisitor)
            }
        }

    }
    .into()
}
