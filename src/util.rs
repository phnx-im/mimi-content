/// Implement `minicbor::Encode` and `minicbor::Decode` for an enum that implements
/// `num_enum::IntoPrimitive` and `num_from::FromPrimitive`.
#[macro_export]
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
