// SPDX-FileCopyrightText: 2026 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{borrow::Cow, collections::BTreeMap};

/// A sum type covering the CBOR values you actually need.
/// Inspired from ciborium::Value but rewritten for minicbor.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(Cow<'static, str>),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Null,
}

impl<C> minicbor::Encode<C> for Value {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Value::Bool(b) => e.bool(*b)?,
            Value::Int(i) => e.i64(*i)?,
            Value::Float(f) => e.f64(*f)?,
            Value::Text(s) => e.str(s)?,
            Value::Bytes(b) => e.bytes(b)?,
            Value::Null => e.null()?,
            Value::Array(arr) => {
                e.array(arr.len() as u64)?;
                for item in arr {
                    item.encode(e, ctx)?;
                }
                return Ok(());
            }
            Value::Map(map) => {
                e.map(map.len() as u64)?;
                for (k, v) in map {
                    e.str(k)?;
                    v.encode(e, ctx)?;
                }
                return Ok(());
            }
        };
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Value {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        use minicbor::data::Type;
        match d.datatype()? {
            Type::Bool => Ok(Value::Bool(d.bool()?)),
            Type::U8 | Type::U16 | Type::U32 | Type::U64 => Ok(Value::Int(d.u64()? as i64)),
            Type::I8 | Type::I16 | Type::I32 | Type::I64 => Ok(Value::Int(d.i64()?)),
            Type::F32 => Ok(Value::Float(d.f32()? as f64)),
            Type::F64 => Ok(Value::Float(d.f64()?)),
            Type::String | Type::StringIndef => Ok(Value::Text(d.str()?.to_string().into())),
            Type::Bytes | Type::BytesIndef => Ok(Value::Bytes(d.bytes()?.to_vec())),
            Type::Null | Type::Undefined => {
                d.skip()?;
                Ok(Value::Null)
            }
            Type::Array | Type::ArrayIndef => {
                let len = d.array()?;
                let mut arr = Vec::with_capacity(len.unwrap_or(0) as usize);
                loop {
                    if d.datatype()? == Type::Break {
                        d.skip()?;
                        break;
                    }
                    arr.push(Value::decode(d, _ctx)?);
                    if len.is_some() && arr.len() == len.unwrap() as usize {
                        break;
                    }
                }
                Ok(Value::Array(arr))
            }
            Type::Map | Type::MapIndef => {
                let len = d.map()?;
                let mut map = BTreeMap::new();
                loop {
                    if d.datatype()? == Type::Break {
                        d.skip()?;
                        break;
                    }
                    let k = d.str()?.to_string();
                    let v = Value::decode(d, _ctx)?;
                    map.insert(k, v);
                    if len.is_some() && map.len() == len.unwrap() as usize {
                        break;
                    }
                }
                Ok(Value::Map(map))
            }
            t => Err(minicbor::decode::Error::type_mismatch(t)),
        }
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

// Signed integers
impl From<i8> for Value {
    fn from(v: i8) -> Self {
        Value::Int(v as i64)
    }
}
impl From<i16> for Value {
    fn from(v: i16) -> Self {
        Value::Int(v as i64)
    }
}
impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v as i64)
    }
}
impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

// Unsigned integers (narrowing: u64 values > i64::MAX will panic in debug,
// wrap in release — add a TryFrom if you need to handle that range)
impl From<u8> for Value {
    fn from(v: u8) -> Self {
        Value::Int(v as i64)
    }
}
impl From<u16> for Value {
    fn from(v: u16) -> Self {
        Value::Int(v as i64)
    }
}
impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::Int(v as i64)
    }
}
impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::Int(v as i64)
    }
}
impl From<usize> for Value {
    fn from(v: usize) -> Self {
        Value::Int(v as i64)
    }
}

// Floats
impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Float(v as f64)
    }
}
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

// Strings
impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(Cow::Owned(v))
    }
}

impl From<&'static str> for Value {
    fn from(v: &'static str) -> Self {
        Value::Text(Cow::Borrowed(v))
    }
}

// Bytes
impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Bytes(v)
    }
}

impl From<&[u8]> for Value {
    fn from(v: &[u8]) -> Self {
        Value::Bytes(v.to_vec())
    }
}

// Option<T> — None becomes CborValue::Null
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(inner) => inner.into(),
            None => Value::Null,
        }
    }
}

// BTreeMap<String, T>
impl<T: Into<Value>> From<BTreeMap<String, T>> for Value {
    fn from(v: BTreeMap<String, T>) -> Self {
        Value::Map(v.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}
