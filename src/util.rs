// SPDX-FileCopyrightText: 2026 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

/// Implement `minicbor::Encode` and `minicbor::Decode` for an enum that implements
/// `num_enum::IntoPrimitive` and `num_from::FromPrimitive`.
macro_rules! impl_encode_decode_num_enum {
    ($ty:ty, $repr:ty) => {
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
    };
}

pub(crate) use impl_encode_decode_num_enum;

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
