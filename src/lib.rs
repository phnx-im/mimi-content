// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod content_container;
mod message_status;

use std::{borrow::Cow, collections::BTreeMap};

pub use content_container::{Error, Result};
pub use message_status::{MessageStatus, MessageStatusReport, PerMessageStatus, Timestamp};

#[cfg(test)]
fn hex_decode(input: &str) -> Vec<u8> {
    let raw = input
        .lines()
        .map(|l| {
            if let Some(index) = l.find('#') {
                &l[..index]
            } else {
                l
            }
            .replace(' ', "")
        })
        .collect::<String>();

    hex::decode(raw).unwrap()
}

/// A sum type covering the CBOR values you actually need.
/// Inspired from ciborium::Value but rewritten for minicbor.
#[derive(Debug, Clone, PartialEq)]
pub enum CborValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(Cow<'static, str>),
    Bytes(Vec<u8>),
    Array(Vec<CborValue>),
    Map(BTreeMap<String, CborValue>),
    Null,
}

impl<C> minicbor::Encode<C> for CborValue {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            CborValue::Bool(b) => e.bool(*b)?,
            CborValue::Int(i) => e.i64(*i)?,
            CborValue::Float(f) => e.f64(*f)?,
            CborValue::Text(s) => e.str(s)?,
            CborValue::Bytes(b) => e.bytes(b)?,
            CborValue::Null => e.null()?,
            CborValue::Array(arr) => {
                e.array(arr.len() as u64)?;
                for item in arr {
                    item.encode(e, ctx)?;
                }
                return Ok(());
            }
            CborValue::Map(map) => {
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

impl<'b, C> minicbor::Decode<'b, C> for CborValue {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        use minicbor::data::Type;
        match d.datatype()? {
            Type::Bool => Ok(CborValue::Bool(d.bool()?)),
            Type::U8 | Type::U16 | Type::U32 | Type::U64 => Ok(CborValue::Int(d.u64()? as i64)),
            Type::I8 | Type::I16 | Type::I32 | Type::I64 => Ok(CborValue::Int(d.i64()?)),
            Type::F32 => Ok(CborValue::Float(d.f32()? as f64)),
            Type::F64 => Ok(CborValue::Float(d.f64()?)),
            Type::String | Type::StringIndef => Ok(CborValue::Text(d.str()?.to_string().into())),
            Type::Bytes | Type::BytesIndef => Ok(CborValue::Bytes(d.bytes()?.to_vec())),
            Type::Null | Type::Undefined => {
                d.skip()?;
                Ok(CborValue::Null)
            }
            Type::Array | Type::ArrayIndef => {
                let len = d.array()?;
                let mut arr = Vec::with_capacity(len.unwrap_or(0) as usize);
                loop {
                    if d.datatype()? == Type::Break {
                        d.skip()?;
                        break;
                    }
                    arr.push(CborValue::decode(d, ctx)?);
                    if len.is_some() && arr.len() == len.unwrap() as usize {
                        break;
                    }
                }
                Ok(CborValue::Array(arr))
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
                    let v = CborValue::decode(d, ctx)?;
                    map.insert(k, v);
                    if len.is_some() && map.len() == len.unwrap() as usize {
                        break;
                    }
                }
                Ok(CborValue::Map(map))
            }
            t => Err(minicbor::decode::Error::type_mismatch(t)),
        }
    }
}

impl From<bool> for CborValue {
    fn from(v: bool) -> Self {
        CborValue::Bool(v)
    }
}

// Signed integers
impl From<i8> for CborValue {
    fn from(v: i8) -> Self {
        CborValue::Int(v as i64)
    }
}
impl From<i16> for CborValue {
    fn from(v: i16) -> Self {
        CborValue::Int(v as i64)
    }
}
impl From<i32> for CborValue {
    fn from(v: i32) -> Self {
        CborValue::Int(v as i64)
    }
}
impl From<i64> for CborValue {
    fn from(v: i64) -> Self {
        CborValue::Int(v)
    }
}

// Unsigned integers (narrowing: u64 values > i64::MAX will panic in debug,
// wrap in release — add a TryFrom if you need to handle that range)
impl From<u8> for CborValue {
    fn from(v: u8) -> Self {
        CborValue::Int(v as i64)
    }
}
impl From<u16> for CborValue {
    fn from(v: u16) -> Self {
        CborValue::Int(v as i64)
    }
}
impl From<u32> for CborValue {
    fn from(v: u32) -> Self {
        CborValue::Int(v as i64)
    }
}
impl From<u64> for CborValue {
    fn from(v: u64) -> Self {
        CborValue::Int(v as i64)
    }
}
impl From<usize> for CborValue {
    fn from(v: usize) -> Self {
        CborValue::Int(v as i64)
    }
}

// Floats
impl From<f32> for CborValue {
    fn from(v: f32) -> Self {
        CborValue::Float(v as f64)
    }
}
impl From<f64> for CborValue {
    fn from(v: f64) -> Self {
        CborValue::Float(v)
    }
}

// Strings
impl From<String> for CborValue {
    fn from(v: String) -> Self {
        CborValue::Text(Cow::Owned(v))
    }
}

impl From<&'static str> for CborValue {
    fn from(v: &'static str) -> Self {
        CborValue::Text(Cow::Borrowed(v))
    }
}

// Bytes
impl From<Vec<u8>> for CborValue {
    fn from(v: Vec<u8>) -> Self {
        CborValue::Bytes(v)
    }
}

impl From<&[u8]> for CborValue {
    fn from(v: &[u8]) -> Self {
        CborValue::Bytes(v.to_vec())
    }
}

// Option<T> — None becomes CborValue::Null
impl<T: Into<CborValue>> From<Option<T>> for CborValue {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(inner) => inner.into(),
            None => CborValue::Null,
        }
    }
}

// BTreeMap<String, T>
impl<T: Into<CborValue>> From<BTreeMap<String, T>> for CborValue {
    fn from(v: BTreeMap<String, T>) -> Self {
        CborValue::Map(v.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}
