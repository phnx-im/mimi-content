// SPDX-FileCopyrightText: 2026 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

/// Define an open enum: one over a primitive code space, such as an IANA registry, where the last
/// variant, written as `Custom(_)`, carries every value the other variants do not name, so that
/// code points added after this crate was built still round-trip.
///
/// Generates the conversions to and from `$repr` as well as `minicbor::Encode` and
/// `minicbor::Decode` implementations.
macro_rules! open_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $ty:ident: $repr:ident {
            $($variants:tt)*
        }
    ) => {
        $crate::util::open_enum!(@collect [$(#[$meta])* $vis enum $ty: $repr] [] $($variants)*);
    };

    // The variants are collected one by one, because a repetition matching them would not know
    // where the last one, the catch-all, starts: both begin with an identifier.
    (
        @collect [$($head:tt)*] [$($collected:tt)*]
        $(#[$variant_meta:meta])* $variant:ident = $value:literal,
        $($rest:tt)*
    ) => {
        $crate::util::open_enum!(
            @collect [$($head)*] [$($collected)* ($(#[$variant_meta])* $variant = $value)]
            $($rest)*
        );
    };

    // The catch-all variant ends the list and emits the definition.
    (
        @collect
        [$(#[$meta:meta])* $vis:vis enum $ty:ident: $repr:ident]
        [$(($(#[$variant_meta:meta])* $variant:ident = $value:literal))*]
        $(#[$catch_all_meta:meta])* $catch_all:ident(_),
    ) => {
        $(#[$meta])*
        #[repr($repr)]
        $vis enum $ty {
            $($(#[$variant_meta])* $variant = $value,)*
            $(#[$catch_all_meta])* $catch_all($repr),
        }

        impl From<$ty> for $repr {
            fn from(value: $ty) -> Self {
                match value {
                    $($ty::$variant => $value,)*
                    $ty::$catch_all(value) => value,
                }
            }
        }

        impl From<$repr> for $ty {
            fn from(value: $repr) -> Self {
                match value {
                    $($value => $ty::$variant,)*
                    value => $ty::$catch_all(value),
                }
            }
        }

        impl<C> ::minicbor::Encode<C> for $ty {
            fn encode<W: ::minicbor::encode::Write>(
                &self,
                e: &mut ::minicbor::Encoder<W>,
                _ctx: &mut C,
            ) -> Result<(), ::minicbor::encode::Error<W::Error>> {
                let repr: $repr = (*self).into();
                e.encode(repr)?;
                Ok(())
            }
        }

        impl<C> ::minicbor::Decode<'_, C> for $ty {
            fn decode(
                d: &mut ::minicbor::Decoder<'_>,
                _ctx: &mut C,
            ) -> Result<Self, ::minicbor::decode::Error> {
                let repr: $repr = d.decode()?;
                Ok(Self::from(repr))
            }
        }

        #[cfg(feature = "serde")]
        impl ::serde::Serialize for $ty {
            fn serialize<S: ::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                let repr: $repr = (*self).into();
                ::serde::Serialize::serialize(&repr, s)
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> ::serde::Deserialize<'de> for $ty {
            fn deserialize<D: ::serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let repr: $repr = ::serde::Deserialize::deserialize(d)?;
                Ok(Self::from(repr))
            }
        }
    };

    (@collect $head:tt $collected:tt $($rest:tt)*) => {
        compile_error!(
            "expected variants of the form `Variant = value,` followed by a `Custom(_),` catch-all"
        );
    };
}

pub(crate) use open_enum;

/// Decode a CBOR text string, concatenating the chunks of an indefinite-length string.
pub(crate) fn decode_text(
    d: &mut minicbor::Decoder<'_>,
) -> Result<String, minicbor::decode::Error> {
    d.str_iter()?.collect()
}

/// Decode a CBOR byte string, concatenating the chunks of an indefinite-length string.
pub(crate) fn decode_bytes(
    d: &mut minicbor::Decoder<'_>,
) -> Result<Vec<u8>, minicbor::decode::Error> {
    let mut bytes = Vec::new();
    for chunk in d.bytes_iter()? {
        bytes.extend_from_slice(chunk?);
    }
    Ok(bytes)
}

#[cfg(test)]
pub(crate) fn hex_decode(input: &str) -> Vec<u8> {
    let raw: String = input
        .lines()
        .map(|line| line.split('#').next().unwrap_or(line).replace(' ', ""))
        .collect();

    hex::decode(raw).unwrap()
}
