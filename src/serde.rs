// SPDX-FileCopyrightText: 2026 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{borrow::Cow, collections::BTreeMap, fmt};

use ::serde::{
    de::{self, DeserializeSeed, Visitor},
    ser::SerializeSeq,
    Deserialize, Serialize, Serializer,
};

use crate::{
    cbor::{Value, MAX_NESTING},
    content_container::{
        Disposition, EncryptionAlgorithm, Expiration, ExtensionName, HashAlgorithm, MimiContent,
        MimiId, NestedPart, PartSemantics,
    },
};

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::Bool(v) => v.serialize(serializer),
            Value::Int(v) => v.serialize(serializer),
            Value::Float(v) => v.serialize(serializer),
            Value::Text(v) => v.serialize(serializer),
            Value::Bytes(v) => serializer.serialize_bytes(v),
            Value::Array(v) => v.serialize(serializer),
            Value::Map(v) => v.serialize(serializer),
            Value::Null => serializer.serialize_none(),
        }
    }
}

pub(crate) struct ValueSerializer {
    depth: usize,
}

impl ValueSerializer {
    pub(crate) fn root() -> Self {
        Self { depth: 0 }
    }

    /// A serializer for a value nested `depth` levels below the root.
    fn nested(depth: usize) -> Result<Self, ValueSerdeError> {
        if depth >= MAX_NESTING {
            return Err(ValueSerdeError::new("too many levels of nesting"));
        }
        Ok(Self { depth })
    }
}

/// An error that can be returned when serializing a serde value to [`Value`].
#[derive(Debug)]
pub struct ValueSerdeError {
    msg: Cow<'static, str>,
}

impl ValueSerdeError {
    fn new(msg: impl Into<Cow<'static, str>>) -> Self {
        Self { msg: msg.into() }
    }
}

impl fmt::Display for ValueSerdeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for ValueSerdeError {}

impl serde::ser::Error for ValueSerdeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        ValueSerdeError::new(msg.to_string())
    }
}

impl Serializer for ValueSerializer {
    type Ok = Value;
    type Error = ValueSerdeError;

    type SerializeSeq = ValueSeqSerializer;
    type SerializeTuple = ValueSeqSerializer;
    type SerializeTupleStruct = ValueSeqSerializer;
    type SerializeTupleVariant = ValueTupleVariantSerializer;
    type SerializeMap = ValueMapSerializer;
    type SerializeStruct = ValueMapSerializer;
    type SerializeStructVariant = ValueStructVariantSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int(v.into()))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int(v.into()))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int(v.into()))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int(v.into()))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int(v.into()))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int(v.into()))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        let v = v
            .try_into()
            .map_err(|_| ValueSerdeError::new("u64 out of range"))?;
        Ok(Value::Int(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Float(v.into()))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Float(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int(u32::from(v).into()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Text(v.to_owned().into()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Bytes(v.to_owned()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Text(variant.into()))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Ok(Value::Map(BTreeMap::from([(
            Value::Text(variant.into()),
            value.serialize(ValueSerializer::nested(self.depth + 1)?)?,
        )])))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ValueSeqSerializer {
            depth: self.depth,
            items: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(ValueSeqSerializer {
            depth: self.depth,
            items: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(ValueSeqSerializer {
            depth: self.depth,
            items: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(ValueTupleVariantSerializer {
            depth: self.depth + 1,
            variant,
            items: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(ValueMapSerializer {
            depth: self.depth,
            items: BTreeMap::new(),
            next_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(ValueMapSerializer {
            depth: self.depth,
            items: BTreeMap::new(),
            next_key: None,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(ValueStructVariantSerializer {
            depth: self.depth + 1,
            variant,
            items: BTreeMap::new(),
        })
    }
}

pub(crate) struct ValueSeqSerializer {
    depth: usize,
    items: Vec<Value>,
}

impl ValueSeqSerializer {
    fn push<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), ValueSerdeError> {
        self.items
            .push(value.serialize(ValueSerializer::nested(self.depth + 1)?)?);
        Ok(())
    }
}

/// The sequence-like `serde` traits differ only in the name of their element method.
macro_rules! impl_serialize_seq {
    ($trait:ident :: $method:ident) => {
        impl serde::ser::$trait for ValueSeqSerializer {
            type Ok = Value;
            type Error = ValueSerdeError;

            fn $method<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
                self.push(value)
            }

            fn end(self) -> Result<Self::Ok, Self::Error> {
                Ok(Value::Array(self.items))
            }
        }
    };
}

impl_serialize_seq!(SerializeSeq::serialize_element);
impl_serialize_seq!(SerializeTuple::serialize_element);
impl_serialize_seq!(SerializeTupleStruct::serialize_field);

pub(crate) struct ValueMapSerializer {
    depth: usize,
    items: BTreeMap<Value, Value>,
    next_key: Option<Value>,
}

impl serde::ser::SerializeMap for ValueMapSerializer {
    type Ok = Value;
    type Error = ValueSerdeError;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.next_key = Some(key.serialize(ValueSerializer::nested(self.depth + 1)?)?);
        Ok(())
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        let key = self
            .next_key
            .take()
            .ok_or_else(|| ValueSerdeError::new("serialize_value before serialize_key"))?;
        insert_unique(
            &mut self.items,
            key,
            value.serialize(ValueSerializer::nested(self.depth + 1)?)?,
        )
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if self.next_key.is_some() {
            return Err(ValueSerdeError::new("missing value for key"));
        }
        Ok(Value::Map(self.items))
    }
}

impl serde::ser::SerializeStruct for ValueMapSerializer {
    type Ok = Value;
    type Error = ValueSerdeError;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        insert_unique(
            &mut self.items,
            Value::Text(key.into()),
            value.serialize(ValueSerializer::nested(self.depth + 1)?)?,
        )
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.items))
    }
}

pub(crate) struct ValueTupleVariantSerializer {
    depth: usize,
    variant: &'static str,
    items: Vec<Value>,
}

impl serde::ser::SerializeTupleVariant for ValueTupleVariantSerializer {
    type Ok = Value;
    type Error = ValueSerdeError;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.items
            .push(value.serialize(ValueSerializer::nested(self.depth + 1)?)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(wrap_variant(self.variant, Value::Array(self.items)))
    }
}

pub(crate) struct ValueStructVariantSerializer {
    depth: usize,
    variant: &'static str,
    items: BTreeMap<Value, Value>,
}

impl serde::ser::SerializeStructVariant for ValueStructVariantSerializer {
    type Ok = Value;
    type Error = ValueSerdeError;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        insert_unique(
            &mut self.items,
            Value::Text(key.into()),
            value.serialize(ValueSerializer::nested(self.depth + 1)?)?,
        )
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(wrap_variant(self.variant, Value::Map(self.items)))
    }
}

fn insert_unique(
    items: &mut BTreeMap<Value, Value>,
    key: Value,
    value: Value,
) -> Result<(), ValueSerdeError> {
    if items.insert(key, value).is_some() {
        return Err(ValueSerdeError::new("duplicate key"));
    }
    Ok(())
}

fn wrap_variant(variant: &'static str, value: Value) -> Value {
    Value::Map(BTreeMap::from([(Value::Text(variant.into()), value)]))
}

struct ValueSeed {
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for ValueSeed {
    type Value = Value;

    fn deserialize<D: ::serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        struct ValueVisitor {
            depth: usize,
        }

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any CBOR value")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
                Ok(Value::Bool(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
                Ok(Value::Int(v))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
                Ok(Value::Int(
                    v.try_into()
                        .map_err(|_| de::Error::custom("u64 out of range"))?,
                ))
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
                ValueSeed {
                    depth: self.depth + 1,
                }
                .deserialize(deserializer)
            }
            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
                let cap = seq
                    .size_hint()
                    .unwrap_or(0)
                    .min(4096 / std::mem::size_of::<Value>());
                let mut arr = Vec::with_capacity(cap);
                while let Some(v) = seq.next_element_seed(ValueSeed {
                    depth: self.depth + 1,
                })? {
                    arr.push(v);
                }
                Ok(Value::Array(arr))
            }
            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
                let mut result = BTreeMap::new();
                while let Some((k, v)) = map.next_entry_seed(
                    ValueSeed {
                        depth: self.depth + 1,
                    },
                    ValueSeed {
                        depth: self.depth + 1,
                    },
                )? {
                    if result.insert(k, v).is_some() {
                        return Err(de::Error::custom("duplicate key"));
                    }
                }
                Ok(Value::Map(result))
            }
        }

        if self.depth >= MAX_NESTING {
            return Err(de::Error::custom("too many levels of nesting"));
        }

        deserializer.deserialize_any(ValueVisitor { depth: self.depth })
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ValueSeed { depth: 0 }.deserialize(deserializer)
    }
}

impl Serialize for ExtensionName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ExtensionName::Text(s) => serializer.serialize_str(s),
            ExtensionName::Number(n) => serializer.serialize_i64(*n),
        }
    }
}

impl<'de> Deserialize<'de> for ExtensionName {
    fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ExtensionNameVisitor;

        impl<'de> Visitor<'de> for ExtensionNameVisitor {
            type Value = ExtensionName;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string or integer")
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<ExtensionName, E> {
                Ok(ExtensionName::Number(v))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<ExtensionName, E> {
                let number = i64::try_from(v)
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Unsigned(v), &self))?;
                Ok(ExtensionName::Number(number))
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

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a 7-element MIMI content array")
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<MimiContent, A::Error> {
                let salt: serde_bytes::ByteArray<16> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let replaces: Option<serde_bytes::ByteArray<32>> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let topic_id: serde_bytes::ByteBuf = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let expires: Option<Expiration> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                let in_reply_to: Option<serde_bytes::ByteArray<32>> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let extensions: BTreeMap<ExtensionName, Value> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(5, &self))?;
                let nested_part: NestedPart = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(6, &self))?;
                Ok(MimiContent {
                    salt: *salt,
                    replaces: replaces.map(|b| MimiId::from(*b)),
                    topic_id: topic_id.into_vec(),
                    expires,
                    in_reply_to: in_reply_to.map(|b| MimiId::from(*b)),
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

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ::serde::ser::SerializeMap as _;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum Enum {
        Unit,
        Newtype(u32),
        Tuple(u32, u32),
        Struct { x: u32 },
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Struct {
        a: u32,
        #[serde(with = "serde_bytes")]
        b: Vec<u8>,
        c: Option<String>,
    }

    /// `T` -> [`Value`] -> CBOR -> `T`
    fn round_trip<T: Serialize + for<'de> Deserialize<'de>>(v: &T) -> T {
        let bytes = minicbor_serde::to_vec(Value::from_serde(v).unwrap()).unwrap();
        minicbor_serde::from_slice(&bytes).unwrap()
    }

    fn text(s: &'static str) -> Value {
        Value::Text(s.into())
    }

    #[test]
    fn scalars() {
        assert_eq!(Value::from_serde(true).unwrap(), Value::Bool(true));
        assert_eq!(Value::from_serde(-1i8).unwrap(), Value::Int(-1));
        assert_eq!(Value::from_serde(7u32).unwrap(), Value::Int(7));
        assert_eq!(Value::from_serde(1.5f32).unwrap(), Value::Float(1.5));
        assert_eq!(
            Value::from_serde('x').unwrap(),
            Value::Int(u32::from('x').into())
        );
        assert_eq!(Value::from_serde("abc").unwrap(), text("abc"));
    }

    #[test]
    fn u64_beyond_i64_is_rejected() {
        assert_eq!(
            Value::from_serde(i64::MAX as u64).unwrap(),
            Value::Int(i64::MAX)
        );
        assert!((i64::MAX as u64 + 1)
            .serialize(ValueSerializer::root())
            .is_err());
    }

    #[test]
    fn bytes_need_serde_bytes() {
        let bytes = serde_bytes::ByteBuf::from(vec![1u8, 2]);
        assert_eq!(Value::from_serde(&bytes).unwrap(), Value::Bytes(vec![1, 2]));

        let plain = vec![1u8, 2];
        assert_eq!(
            Value::from_serde(&plain).unwrap(),
            Value::Array(vec![Value::Int(1), Value::Int(2)])
        );
    }

    #[test]
    fn options_and_units_collapse_to_null() {
        assert_eq!(Value::from_serde(Some(3u32)).unwrap(), Value::Int(3));
        assert_eq!(Value::from_serde(None::<u32>).unwrap(), Value::Null);
        assert_eq!(Value::from_serde(Some(None::<u32>)).unwrap(), Value::Null);
        assert_eq!(Value::from_serde(()).unwrap(), Value::Null);
    }

    #[test]
    fn sequences_become_arrays() {
        assert_eq!(
            Value::from_serde(vec![1u32, 2]).unwrap(),
            Value::Array(vec![Value::Int(1), Value::Int(2)])
        );
        assert_eq!(
            Value::from_serde((1u32, "a")).unwrap(),
            Value::Array(vec![Value::Int(1), text("a")])
        );
    }

    #[test]
    fn map_pairs_each_key_with_its_value() {
        let map = BTreeMap::from([("k1", 1u32), ("k2", 2)]);
        assert_eq!(
            Value::from_serde(&map).unwrap(),
            Value::Map(BTreeMap::from([
                (text("k1"), Value::Int(1)),
                (text("k2"), Value::Int(2)),
            ]))
        );
    }

    #[test]
    fn map_rejects_duplicate_key() {
        let mut map = ValueSerializer::root().serialize_map(None).unwrap();
        map.serialize_entry("k", &1u32).unwrap();
        assert!(map.serialize_entry("k", &2u32).is_err());
    }

    #[test]
    fn map_rejects_value_without_key() {
        let mut map = ValueSerializer::root().serialize_map(None).unwrap();
        assert!(map.serialize_value(&1u32).is_err());
    }

    #[test]
    fn structs_become_text_keyed_maps() {
        assert_eq!(
            Value::from_serde(&Struct {
                a: 1,
                b: vec![7, 8],
                c: None,
            })
            .unwrap(),
            Value::Map(BTreeMap::from([
                (text("a"), Value::Int(1)),
                (text("b"), Value::Bytes(vec![7, 8])),
                (text("c"), Value::Null),
            ]))
        );
    }

    #[test]
    fn variants_are_externally_tagged() {
        assert_eq!(Value::from_serde(&Enum::Unit).unwrap(), text("Unit"));
        assert_eq!(
            Value::from_serde(Enum::Newtype(9)).unwrap(),
            Value::Map(BTreeMap::from([(text("Newtype"), Value::Int(9))]))
        );
        assert_eq!(
            Value::from_serde(Enum::Tuple(1, 2)).unwrap(),
            Value::Map(BTreeMap::from([(
                text("Tuple"),
                Value::Array(vec![Value::Int(1), Value::Int(2)])
            )]))
        );
        assert_eq!(
            Value::from_serde(&Enum::Struct { x: 5 }).unwrap(),
            Value::Map(BTreeMap::from([(
                text("Struct"),
                Value::Map(BTreeMap::from([(text("x"), Value::Int(5))]))
            )]))
        );
    }

    #[test]
    fn values_round_trip_through_cbor() {
        for value in [
            Enum::Unit,
            Enum::Newtype(9),
            Enum::Tuple(1, 2),
            Enum::Struct { x: 5 },
        ] {
            assert_eq!(round_trip(&value), value);
        }

        let value = Struct {
            a: 1,
            b: vec![7, 8],
            c: Some("x".to_owned()),
        };
        assert_eq!(round_trip(&value), value);
        assert_eq!(round_trip(&'x'), 'x');
    }

    #[test]
    fn serde_encoding_matches_minicbor() {
        let value = Value::Map(BTreeMap::from([
            (text("bytes"), Value::Bytes(vec![1, 2, 3])),
            (text("null"), Value::Null),
            (
                text("nested"),
                Value::Array(vec![Value::Int(-7), Value::Float(1.5), Value::Bool(true)]),
            ),
        ]));

        let bytes = minicbor_serde::to_vec(&value).unwrap();
        assert_eq!(minicbor::decode::<Value>(&bytes).unwrap(), value);

        let mut expected = Vec::new();
        minicbor::encode(&value, &mut expected).unwrap();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn oversized_declared_length_is_rejected() {
        let input = [0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
        assert!(minicbor_serde::from_slice::<Value>(&input).is_err());

        let input = [0x9a, 0x3b, 0x9a, 0xca, 0x00];
        assert!(minicbor_serde::from_slice::<Value>(&input).is_err());
    }

    struct Deep(usize);

    impl Serialize for Deep {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            if self.0 == 0 {
                return s.serialize_unit();
            }
            let mut seq = s.serialize_seq(Some(1))?;
            seq.serialize_element(&Deep(self.0 - 1))?;
            seq.end()
        }
    }

    fn deserialize_err(input: &[u8]) -> String {
        minicbor_serde::from_slice::<Value>(input)
            .expect_err("expected the nesting limit to reject this")
            .to_string()
    }

    #[test]
    fn deserialize_accepts_input_at_the_nesting_limit() {
        let mut input = vec![0x81; MAX_NESTING - 1];
        input.push(0xf6);
        assert!(minicbor_serde::from_slice::<Value>(&input).is_ok());

        let mut input: Vec<u8> = (0..MAX_NESTING - 1).flat_map(|_| [0xa1, 0x00]).collect();
        input.push(0xf6);
        assert!(minicbor_serde::from_slice::<Value>(&input).is_ok());
    }

    #[test]
    fn deserialize_rejects_input_past_the_nesting_limit() {
        let mut input = vec![0x81; MAX_NESTING];
        input.push(0xf6);
        assert!(deserialize_err(&input).contains("too many levels of nesting"));

        let mut input: Vec<u8> = (0..MAX_NESTING).flat_map(|_| [0xa1, 0x00]).collect();
        input.push(0xf6);
        assert!(deserialize_err(&input).contains("too many levels of nesting"));

        // 100k levels, which without the limit overflows the stack and aborts the process
        assert!(deserialize_err(&vec![0x81; 100_000]).contains("too many levels of nesting"));
        assert!(ciborium::from_reader::<Value, _>(&vec![0x81; 100_000][..]).is_err());
    }

    #[test]
    fn deserialize_rejects_duplicate_keys() {
        // {1: 1, 1: 2}
        assert!(deserialize_err(&[0xa2, 0x01, 0x01, 0x01, 0x02]).contains("duplicate key"));
    }

    #[test]
    fn from_serde_enforces_the_nesting_limit() {
        assert!(Value::from_serde(Deep(MAX_NESTING - 1)).is_ok());

        let err = Value::from_serde(Deep(MAX_NESTING)).unwrap_err();
        assert!(
            err.to_string().contains("too many levels of nesting"),
            "{err}"
        );

        let err = Value::from_serde(Deep(100_000)).unwrap_err();
        assert!(
            err.to_string().contains("too many levels of nesting"),
            "{err}"
        );
    }

    #[test]
    fn deepest_value_from_serde_survives_a_cbor_round_trip() {
        let value = Value::from_serde(Deep(MAX_NESTING - 1)).unwrap();
        let bytes = minicbor_serde::to_vec(&value).unwrap();
        assert_eq!(minicbor::decode::<Value>(&bytes).unwrap(), value);
    }

    #[test]
    fn variant_wrappers_count_towards_the_limit() {
        #[derive(Serialize)]
        enum Wrap<T> {
            Newtype(T),
            Tuple(T, u8),
            Struct { x: T },
        }

        // `{"Newtype": v}` puts the payload one level down
        assert!(Value::from_serde(Wrap::Newtype(Deep(MAX_NESTING - 2))).is_ok());
        assert!(Value::from_serde(Wrap::Newtype(Deep(MAX_NESTING - 1))).is_err());

        // `{"Tuple": [v, _]}` and `{"Struct": {"x": v}}` put it two levels down
        assert!(Value::from_serde(Wrap::Tuple(Deep(MAX_NESTING - 3), 0)).is_ok());
        assert!(Value::from_serde(Wrap::Tuple(Deep(MAX_NESTING - 2), 0)).is_err());

        assert!(Value::from_serde(Wrap::Struct {
            x: Deep(MAX_NESTING - 3)
        })
        .is_ok());
        assert!(Value::from_serde(Wrap::Struct {
            x: Deep(MAX_NESTING - 2)
        })
        .is_err());
    }
}
