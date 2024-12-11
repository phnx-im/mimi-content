// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![recursion_limit = "4096"]

extern crate proc_macro;

use proc_macro::TokenStream;
use punctuated::Punctuated;
use quote::quote;
use syn::*;

#[proc_macro_derive(ExternallyTagged)]
pub fn derive_externally_tagged(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let enum_name = ast.ident;

    let Data::Enum(data) = ast.data else {
        panic!();
    };

    let mut valid = false;
    for attr in &ast.attrs {
        if attr.path().is_ident("repr") {
            if attr.parse_args::<Ident>().unwrap() == "u8" {
                valid = true;
            }
        }
    }
    if !valid {
        panic!("ExternallyTagged requires #[repr(u8)]")
    }

    let mut field_num_impls = Vec::new();
    let mut field_serialize_impls = Vec::new();

    for variant in data.variants {
        let variant_name = variant.ident;

        let fields = match variant.fields {
            Fields::Named(fields_named) => fields_named.named,
            Fields::Unnamed(_fields_unnamed) => panic!(),
            Fields::Unit => Punctuated::new(),
        };

        let mut names = Vec::new();
        let mut serialized = Vec::new();

        for field in &fields {
            let Some(field_name) = &field.ident else {
                panic!();
            };

            names.push(field_name);
            serialized.push(quote! { state.serialize_element(#field_name)?; });
        }

        let num = names.len();

        field_num_impls.push(quote! {
            Self::#variant_name { .. } => { #num },
        });

        field_serialize_impls.push(quote! {
            Self::#variant_name { #(#names),* } => { #(#serialized)* },
        });
    }

    quote! {
        impl ExternallyTagged for #enum_name {
            // https://doc.rust-lang.org/reference/items/enumerations.html?search=#pointer-casting
            fn discriminant(&self) -> u8 {
                // This is safe if the enum has repr(u8)
                let pointer = self as *const Self as *const u8;
                unsafe { *pointer }
            }

            fn num_fields(&self) -> usize {
                match self {
                    #(#field_num_impls)*
                }
            }

            fn serialize_fields<S: serde::ser::SerializeSeq>(&self, state: &mut S) -> Result<(), S::Error> {
                Ok(match self {
                    #(#field_serialize_impls)*
                })
            }
        }
    }
    .into()
}

#[proc_macro_derive(Serialize_custom_u8)]
pub fn derive_serialize_custom_u8(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let enum_name = ast.ident;

    let Data::Enum(data) = ast.data else {
        panic!();
    };

    let mut valid = false;
    for attr in &ast.attrs {
        if attr.path().is_ident("repr") {
            if attr.parse_args::<Ident>().unwrap() == "u8" {
                valid = true;
            }
        }
    }
    if !valid {
        panic!("Serialize_custom_u8 requires #[repr(u8)]")
    }

    let mut must_be_last = false;
    for variant in data.variants {
        if must_be_last {
            panic!("There should be no more variants after Custom(u8)");
        }

        let variant_name = variant.ident;

        match variant.fields {
            Fields::Named(_fields_named) => {
                panic!("Enum cannot contain fields except for Custom(u8)")
            }
            Fields::Unnamed(_fields_unnamed) => {
                if variant_name == "Custom" {
                    must_be_last = true;
                } else {
                    panic!("Enum cannot contain fields except for Custom(u8)");
                }
            }
            Fields::Unit => {}
        };
    }

    if !must_be_last {
        panic!("The last variant must be Custom(u8)");
    }

    quote! {
        // https://doc.rust-lang.org/reference/items/enumerations.html?search=#pointer-casting
        impl #enum_name {
            fn discriminant(&self) -> u8 {
                // This is safe if the enum has repr(u8)
                let pointer = self as *const Self as *const u8;
                unsafe { *pointer }
            }
        }

        impl Serialize for #enum_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                match self {
                    Self::Custom(custom) => *custom,
                    known => known.discriminant(),
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

                // This assumes that Custom is the last variant of the enum
                let variant = if value < Self::Custom(0).discriminant() {
                    // The value corresponds to the discriminant of the enum
                    let result = unsafe { *(&value as *const u8 as *const Self) };
                    assert_eq!(result.discriminant(), value);

                    result
                } else {
                    Self::Custom(value)
                };

                Ok(variant)
            }
        }
    }
    .into()
}

#[proc_macro_derive(Serialize_list, attributes(externally_tagged))]
pub fn derive_serialize_list(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = ast.ident;

    let mut field_num_updates = Vec::new();
    let mut field_serializations = Vec::new();

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
                    num_fields += ExternallyTagged::num_fields(&self.#field_name);
                });

                field_serializations.push(quote! {
                    state.serialize_element(&ExternallyTagged::discriminant(&self.#field_name))?;
                    ExternallyTagged::serialize_fields(&self.#field_name, &mut state)?;

                });
                continue 'fields;
            }
        }
        field_serializations.push(quote! {
            state.serialize_element(&self.#field_name)?;
        });
    }

    let num_fields = field_serializations.len();

    quote! {
        impl serde::Serialize for #struct_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut num_fields = #num_fields;
                #(#field_num_updates)*

                let mut state = serializer.serialize_seq(Some(num_fields))?;

                #(#field_serializations)*

                state.end()
            }
        }
    }
    .into()
}
