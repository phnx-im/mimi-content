// SPDX-FileCopyrightText: 2026 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::BTreeMap;

use ::serde::{
    de::{self, Visitor},
    ser::SerializeSeq,
    Deserialize, Serialize, Serializer,
};

use crate::{
    cbor::Value,
    content_container::{
        Disposition, EncryptionAlgorithm, Expiration, ExtensionName, HashAlgorithm, MimiContent,
        NestedPart, PartSemantics,
    },
};

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::Bool(v) => v.serialize(serializer),
            Value::Int(v) => v.serialize(serializer),
            Value::Float(v) => v.serialize(serializer),
            Value::Text(v) => v.serialize(serializer),
            Value::Bytes(v) => v.serialize(serializer),
            Value::Array(v) => v.serialize(serializer),
            Value::Map(v) => v.serialize(serializer),
            Value::Null => serializer.serialize_unit(),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("any CBOR value")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
                Ok(Value::Bool(v))
            }
            fn visit_i8<E: de::Error>(self, v: i8) -> Result<Value, E> {
                Ok(Value::Int(v as i64))
            }
            fn visit_i16<E: de::Error>(self, v: i16) -> Result<Value, E> {
                Ok(Value::Int(v as i64))
            }
            fn visit_i32<E: de::Error>(self, v: i32) -> Result<Value, E> {
                Ok(Value::Int(v as i64))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
                Ok(Value::Int(v))
            }
            fn visit_u8<E: de::Error>(self, v: u8) -> Result<Value, E> {
                Ok(Value::Int(v as i64))
            }
            fn visit_u16<E: de::Error>(self, v: u16) -> Result<Value, E> {
                Ok(Value::Int(v as i64))
            }
            fn visit_u32<E: de::Error>(self, v: u32) -> Result<Value, E> {
                Ok(Value::Int(v as i64))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
                Ok(Value::Int(v as i64))
            }
            fn visit_f32<E: de::Error>(self, v: f32) -> Result<Value, E> {
                Ok(Value::Float(v as f64))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
                Ok(Value::Float(v))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
                Ok(Value::Text(v.to_string().into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Value, E> {
                Ok(Value::Text(v.into()))
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Value, E> {
                Ok(Value::Bytes(v.to_vec()))
            }
            fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Value, E> {
                Ok(Value::Bytes(v))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }
            fn visit_none<E: de::Error>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }
            fn visit_some<D: ::serde::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Value, D::Error> {
                Value::deserialize(deserializer)
            }
            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
                let mut arr = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(v) = seq.next_element()? {
                    arr.push(v);
                }
                Ok(Value::Array(arr))
            }
            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
                let mut result = BTreeMap::new();
                while let Some((k, v)) = map.next_entry()? {
                    result.insert(k, v);
                }
                Ok(Value::Map(result))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl Serialize for ExtensionName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ExtensionName::Text(s) => serializer.serialize_str(s),
            ExtensionName::Number(n) => serializer.serialize_u64(*n),
        }
    }
}

impl<'de> Deserialize<'de> for ExtensionName {
    fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ExtensionNameVisitor;

        impl<'de> Visitor<'de> for ExtensionNameVisitor {
            type Value = ExtensionName;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a string or unsigned integer")
            }

            fn visit_u8<E: de::Error>(self, v: u8) -> Result<ExtensionName, E> {
                Ok(ExtensionName::Number(v as u64))
            }
            fn visit_u16<E: de::Error>(self, v: u16) -> Result<ExtensionName, E> {
                Ok(ExtensionName::Number(v as u64))
            }
            fn visit_u32<E: de::Error>(self, v: u32) -> Result<ExtensionName, E> {
                Ok(ExtensionName::Number(v as u64))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<ExtensionName, E> {
                Ok(ExtensionName::Number(v))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<ExtensionName, E> {
                Ok(ExtensionName::Text(v.to_owned()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<ExtensionName, E> {
                Ok(ExtensionName::Text(v))
            }
        }

        deserializer.deserialize_any(ExtensionNameVisitor)
    }
}

impl Serialize for MimiContent {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(7))?;
        seq.serialize_element(serde_bytes::Bytes::new(&self.salt))?;
        seq.serialize_element(&self.replaces.as_deref().map(serde_bytes::Bytes::new))?;
        seq.serialize_element(serde_bytes::Bytes::new(&self.topic_id))?;
        seq.serialize_element(&self.expires)?;
        seq.serialize_element(&self.in_reply_to.as_deref().map(serde_bytes::Bytes::new))?;
        seq.serialize_element(&self.extensions)?;
        seq.serialize_element(&self.nested_part)?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for MimiContent {
    fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MimiContentVisitor;

        impl<'de> Visitor<'de> for MimiContentVisitor {
            type Value = MimiContent;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a 7-element MIMI content array")
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<MimiContent, A::Error> {
                let salt: serde_bytes::ByteBuf = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let replaces: Option<serde_bytes::ByteBuf> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let topic_id: serde_bytes::ByteBuf = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let expires: Option<Expiration> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                let in_reply_to: Option<serde_bytes::ByteBuf> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let extensions: BTreeMap<ExtensionName, Value> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(5, &self))?;
                let nested_part: NestedPart = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(6, &self))?;
                Ok(MimiContent {
                    salt: salt.into_vec(),
                    replaces: replaces.map(|b| b.into_vec()),
                    topic_id: topic_id.into_vec(),
                    expires,
                    in_reply_to: in_reply_to.map(|b| b.into_vec()),
                    extensions,
                    nested_part,
                })
            }
        }

        deserializer.deserialize_seq(MimiContentVisitor)
    }
}

impl Serialize for Expiration {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.relative)?;
        seq.serialize_element(&self.time)?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Expiration {
    fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ExpirationVisitor;

        impl<'de> Visitor<'de> for ExpirationVisitor {
            type Value = Expiration;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a 2-element array [bool, u32]")
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Expiration, A::Error> {
                let relative = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let time = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(Expiration { relative, time })
            }
        }

        deserializer.deserialize_seq(ExpirationVisitor)
    }
}

impl Serialize for NestedPart {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            NestedPart::NullPart {
                disposition,
                language,
            } => {
                let mut seq = serializer.serialize_seq(Some(3))?;
                seq.serialize_element(disposition)?;
                seq.serialize_element(language)?;
                seq.serialize_element(&0u8)?;
                seq.end()
            }
            NestedPart::SinglePart {
                disposition,
                language,
                content_type,
                content,
            } => {
                let mut seq = serializer.serialize_seq(Some(5))?;
                seq.serialize_element(disposition)?;
                seq.serialize_element(language)?;
                seq.serialize_element(&1u8)?;
                seq.serialize_element(content_type)?;
                seq.serialize_element(serde_bytes::Bytes::new(content))?;
                seq.end()
            }
            NestedPart::ExternalPart {
                disposition,
                language,
                content_type,
                url,
                expires,
                size,
                enc_alg,
                key,
                nonce,
                aad,
                hash_alg,
                content_hash,
                description,
                filename,
            } => {
                let mut seq = serializer.serialize_seq(Some(15))?;
                seq.serialize_element(disposition)?;
                seq.serialize_element(language)?;
                seq.serialize_element(&2u8)?;
                seq.serialize_element(content_type)?;
                seq.serialize_element(url)?;
                seq.serialize_element(expires)?;
                seq.serialize_element(size)?;
                seq.serialize_element(enc_alg)?;
                seq.serialize_element(serde_bytes::Bytes::new(key))?;
                seq.serialize_element(serde_bytes::Bytes::new(nonce))?;
                seq.serialize_element(serde_bytes::Bytes::new(aad))?;
                seq.serialize_element(hash_alg)?;
                seq.serialize_element(serde_bytes::Bytes::new(content_hash))?;
                seq.serialize_element(description)?;
                seq.serialize_element(filename)?;
                seq.end()
            }
            NestedPart::MultiPart {
                disposition,
                language,
                part_semantics,
                parts,
            } => {
                let mut seq = serializer.serialize_seq(Some(5))?;
                seq.serialize_element(disposition)?;
                seq.serialize_element(language)?;
                seq.serialize_element(&3u8)?;
                seq.serialize_element(part_semantics)?;
                seq.serialize_element(parts)?;
                seq.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for NestedPart {
    fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct NestedPartVisitor;

        impl<'de> Visitor<'de> for NestedPartVisitor {
            type Value = NestedPart;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a MIMI NestedPart array")
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<NestedPart, A::Error> {
                let disposition: Disposition = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let language: String = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let discriminant: u8 = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                match discriminant {
                    0 => Ok(NestedPart::NullPart {
                        disposition,
                        language,
                    }),
                    1 => {
                        let content_type: String = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                        let content: serde_bytes::ByteBuf = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                        Ok(NestedPart::SinglePart {
                            disposition,
                            language,
                            content_type,
                            content: content.into_vec(),
                        })
                    }
                    2 => {
                        let content_type: String = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                        let url: String = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                        let expires: u32 = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(5, &self))?;
                        let size: u64 = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(6, &self))?;
                        let enc_alg: EncryptionAlgorithm = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(7, &self))?;
                        let key: serde_bytes::ByteBuf = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(8, &self))?;
                        let nonce: serde_bytes::ByteBuf = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(9, &self))?;
                        let aad: serde_bytes::ByteBuf = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(10, &self))?;
                        let hash_alg: HashAlgorithm = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(11, &self))?;
                        let content_hash: serde_bytes::ByteBuf = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(12, &self))?;
                        let description: String = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(13, &self))?;
                        let filename: String = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(14, &self))?;
                        Ok(NestedPart::ExternalPart {
                            disposition,
                            language,
                            content_type,
                            url,
                            expires,
                            size,
                            enc_alg,
                            key: key.into_vec(),
                            nonce: nonce.into_vec(),
                            aad: aad.into_vec(),
                            hash_alg,
                            content_hash: content_hash.into_vec(),
                            description,
                            filename,
                        })
                    }
                    3 => {
                        let part_semantics: PartSemantics = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                        let parts: Vec<NestedPart> = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                        Ok(NestedPart::MultiPart {
                            disposition,
                            language,
                            part_semantics,
                            parts,
                        })
                    }
                    _ => Err(de::Error::custom(format!(
                        "invalid discriminant {discriminant} for NestedPart"
                    ))),
                }
            }
        }

        deserializer.deserialize_seq(NestedPartVisitor)
    }
}

macro_rules! impl_serde_num_enum {
    ($ty:ty, $repr:ty) => {
        impl ::serde::Serialize for $ty {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                let repr: $repr = (*self).into();
                repr.serialize(serializer)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                let value: $repr = Deserialize::deserialize(deserializer)?;
                Ok(Self::from(value))
            }
        }
    };
}

impl_serde_num_enum!(HashAlgorithm, u8);
impl_serde_num_enum!(EncryptionAlgorithm, u16);
impl_serde_num_enum!(Disposition, u8);
impl_serde_num_enum!(PartSemantics, u8);
