// SPDX-FileCopyrightText: 2026 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use minicbor::{data::Type, decode};
#[cfg(feature = "serde")]
use serde::Serialize;
use std::{borrow::Cow, cmp::Ordering, collections::BTreeMap, num::TryFromIntError};

#[cfg(feature = "serde")]
use crate::serde::ValueSerdeError;
use crate::util::{decode_bytes, decode_text};

/// A sum type covering the CBOR values you actually need.
///
/// Inspired from ciborium::Value but rewritten for minicbor.
///
/// The ordering of the Value is based on the canonical key order (RFC 8949 §4.2.1: keys sorted
/// bytewise on their encodings).
///
/// The value can be decoded via `minicbor::decode` can have a maximum nesting depth of 32.
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Bytes(Vec<u8>),
    Text(Cow<'static, str>),
    Array(Vec<Value>),
    Map(BTreeMap<Value, Value>),
    Bool(bool),
    Null,
    Float(f64),
}

impl Value {
    #[cfg(feature = "serde")]
    pub fn from_serde(v: impl Serialize) -> Result<Self, ValueSerdeError> {
        v.serialize(crate::serde::ValueSerializer)
    }

    /// Returns `true` if the depth <= `max_depth`, otherwise `false`.
    ///
    /// The depth is counted recursively starting at 0 incremented by 1 for maps and arrays. Scalars
    /// are not counted.
    pub(crate) fn within_depth(&self, max_depth: usize) -> bool {
        let mut max = 0;
        let mut stack = vec![(self, 1)];
        while let Some((value, depth)) = stack.pop() {
            match value {
                Value::Array(items) => {
                    max = max.max(depth);
                    stack.extend(items.iter().map(|v| (v, depth + 1)));
                }
                Value::Map(items) => {
                    max = max.max(depth);
                    stack.extend(
                        items
                            .iter()
                            .flat_map(|(k, v)| [(k, depth + 1), (v, depth + 1)]),
                    );
                }
                _ => {}
            }
            if max > max_depth {
                return false;
            }
        }
        true
    }

    fn rank(&self) -> u8 {
        match self {
            Value::Int(i) if *i >= 0 => 0,
            Value::Int(_) => 1,
            Value::Bytes(..) => 2,
            Value::Text(..) => 3,
            Value::Array(..) => 4,
            Value::Map(..) => 5,
            Value::Bool(..) => 6,
            Value::Null => 7,
            Value::Float(..) => 8,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        }
    }
}

impl Eq for Value {}

// Implements the ordering of CBOR values as specified in
// <https://www.rfc-editor.org/rfc/rfc8949.html#core-det>.
impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) if *a >= 0 && *b >= 0 => a.cmp(b),
            (Value::Int(a), Value::Int(b)) if *a < 0 && *b < 0 => a.cmp(b).reverse(),
            (Value::Int(a), Value::Int(b)) => a.cmp(b).reverse(),
            (Value::Bytes(a), Value::Bytes(b)) => a.len().cmp(&b.len()).then(a.cmp(b)),
            (Value::Text(a), Value::Text(b)) => a.len().cmp(&b.len()).then(a.cmp(b)),
            (Value::Array(a), Value::Array(b)) => a.len().cmp(&b.len()).then(a.cmp(b)),
            (Value::Map(a), Value::Map(b)) => a.len().cmp(&b.len()).then(a.cmp(b)),
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Float(a), Value::Float(b)) => a.to_bits().cmp(&b.to_bits()),
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
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
                    k.encode(e, ctx)?;
                    v.encode(e, ctx)?;
                }
                return Ok(());
            }
        };
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Value {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, decode::Error> {
        decode_value(d, 0)
    }
}

/// Maximum nesting depth of a decoded [`Value`].
///
/// Decoding recurses once per level, and a debug build stack should have enough capacity to handle
/// it easily.
const MAX_NESTING: usize = 32;

fn decode_value<'b>(
    d: &mut minicbor::Decoder<'b>,
    depth: usize,
) -> Result<Value, minicbor::decode::Error> {
    if depth >= MAX_NESTING {
        return Err(decode::Error::message("too many levels of nesting"));
    }
    match d.datatype()? {
        Type::Bool => Ok(Value::Bool(d.bool()?)),
        Type::U8 | Type::U16 | Type::U32 | Type::U64 => Ok(Value::Int(
            d.u64()?
                .try_into()
                .map_err(|_| decode::Error::message("u64 out of range"))?,
        )),
        Type::I8 | Type::I16 | Type::I32 | Type::I64 => Ok(Value::Int(d.i64()?)),
        Type::F32 => Ok(Value::Float(d.f32()?.into())),
        Type::F64 => Ok(Value::Float(d.f64()?)),
        Type::String | Type::StringIndef => Ok(Value::Text(decode_text(d)?.into())),
        Type::Bytes | Type::BytesIndef => Ok(Value::Bytes(decode_bytes(d)?)),
        Type::Null | Type::Undefined => {
            d.skip()?;
            Ok(Value::Null)
        }
        Type::Array | Type::ArrayIndef => {
            let len = d
                .array()?
                .map(usize::try_from)
                .transpose()
                .map_err(|_| decode::Error::message("array length usize overflow"))?;
            let remaining = d.input().len().saturating_sub(d.position());
            let cap = len.unwrap_or(0).min(remaining);
            let mut arr = Vec::with_capacity(cap);
            loop {
                match len {
                    Some(len) if arr.len() == len => break,
                    None if d.datatype()? == Type::Break => {
                        d.skip()?;
                        break;
                    }
                    _ => {}
                }
                arr.push(decode_value(d, depth + 1)?);
            }
            Ok(Value::Array(arr))
        }
        Type::Map | Type::MapIndef => {
            let len = d
                .map()?
                .map(usize::try_from)
                .transpose()
                .map_err(|_| decode::Error::message("map length usize overflow"))?;
            let mut map = BTreeMap::new();
            loop {
                match len {
                    Some(len) if map.len() == len => break,
                    None if d.datatype()? == Type::Break => {
                        d.skip()?;
                        break;
                    }
                    _ => {}
                }
                let k = decode_value(d, depth + 1)?;
                let v = decode_value(d, depth + 1)?;
                if map.insert(k, v).is_some() {
                    return Err(decode::Error::message("duplicate key"));
                }
            }
            Ok(Value::Map(map))
        }
        t => Err(decode::Error::type_mismatch(t)),
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
        Value::Int(v.into())
    }
}
impl From<i16> for Value {
    fn from(v: i16) -> Self {
        Value::Int(v.into())
    }
}
impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v.into())
    }
}
impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<u8> for Value {
    fn from(v: u8) -> Self {
        Value::Int(v.into())
    }
}
impl From<u16> for Value {
    fn from(v: u16) -> Self {
        Value::Int(v.into())
    }
}
impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::Int(v.into())
    }
}
impl TryFrom<u64> for Value {
    type Error = TryFromIntError;

    fn try_from(v: u64) -> Result<Self, Self::Error> {
        Ok(Value::Int(v.try_into()?))
    }
}
impl TryFrom<usize> for Value {
    type Error = TryFromIntError;

    fn try_from(v: usize) -> Result<Self, Self::Error> {
        Ok(Value::Int(v.try_into()?))
    }
}

// Floats
impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Float(v.into())
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

// BTreeMap<K, V>
impl<K: Into<Value>, V: Into<Value>> From<BTreeMap<K, V>> for Value {
    fn from(v: BTreeMap<K, V>) -> Self {
        Value::Map(v.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(bytes: &[u8]) -> Value {
        minicbor::decode(bytes).unwrap()
    }

    #[test]
    fn definite_length_roundtrip() {
        let value = Value::Array(vec![
            Value::Text("example".into()),
            Value::Bytes(vec![1, 2, 3]),
            Value::Map(BTreeMap::from([("key".to_owned().into(), Value::Int(-7))])),
        ]);

        let mut buf = Vec::new();
        minicbor::encode(&value, &mut buf).unwrap();
        assert_eq!(decode(&buf), value);
    }

    #[test]
    fn indefinite_length_text() {
        let mut e = minicbor::Encoder::new(Vec::new());
        e.begin_str().unwrap();
        e.str("exa").unwrap();
        e.str("mple").unwrap();
        e.end().unwrap();

        assert_eq!(decode(&e.into_writer()), Value::Text("example".into()));
    }

    #[test]
    fn indefinite_length_bytes() {
        let mut e = minicbor::Encoder::new(Vec::new());
        e.begin_bytes().unwrap();
        e.bytes(&[1, 2]).unwrap();
        e.bytes(&[3]).unwrap();
        e.end().unwrap();

        assert_eq!(decode(&e.into_writer()), Value::Bytes(vec![1, 2, 3]));
    }

    #[test]
    fn indefinite_length_map_key() {
        let mut e = minicbor::Encoder::new(Vec::new());
        e.map(1).unwrap();
        e.begin_str().unwrap();
        e.str("ke").unwrap();
        e.str("y").unwrap();
        e.end().unwrap();
        e.u8(7).unwrap();

        let expected = Value::Map(BTreeMap::from([("key".to_owned().into(), Value::Int(7))]));
        assert_eq!(decode(&e.into_writer()), expected);
    }

    #[test]
    fn rejects_unsupported_type() {
        let mut buf = Vec::new();
        minicbor::encode(minicbor::data::IanaTag::DateTime.tag(), &mut buf).unwrap();
        assert!(minicbor::decode::<Value>(&buf).is_err());
    }

    fn try_decode(bytes: &[u8]) -> Result<Value, minicbor::decode::Error> {
        minicbor::decode(bytes)
    }

    fn roundtrip(value: &Value) -> Value {
        let mut buf = Vec::new();
        minicbor::encode(value, &mut buf).unwrap();
        decode(&buf)
    }

    #[test]
    fn empty_array_roundtrip() {
        let value = Value::Array(Vec::new());
        assert_eq!(decode(&[0x80]), value);
        assert_eq!(roundtrip(&value), value);
    }

    #[test]
    fn empty_map_roundtrip() {
        let value = Value::Map(BTreeMap::new());
        assert_eq!(decode(&[0xa0]), value);
        assert_eq!(roundtrip(&value), value);
    }

    #[test]
    fn empty_collections_nested() {
        let value = Value::Array(vec![Value::Array(Vec::new()), Value::Int(1)]);
        assert_eq!(roundtrip(&value), value);

        let value = Value::Map(BTreeMap::from([(
            "key".to_owned().into(),
            Value::Array(Vec::new()),
        )]));
        assert_eq!(roundtrip(&value), value);
    }

    #[test]
    fn definite_length_array_rejects_break() {
        // Declares three elements, but a break follows the second. A break is only valid inside an
        // indefinite-length item, and must not truncate the array.
        assert!(try_decode(&[0x83, 0x01, 0x01, 0xff]).is_err());
    }

    #[test]
    fn definite_length_map_enforces_entry_count() {
        // Declares two entries, both keyed "k". The entries must be counted independently of the
        // map's length, and the duplicate key rejected.
        let err = try_decode(&[0xa2, 0x61, 0x6b, 0x01, 0x61, 0x6b, 0x02]).unwrap_err();
        assert!(
            !err.is_end_of_input(),
            "read past the declared entries: {err}"
        );

        // The map must not swallow the item that follows it.
        let err = try_decode(&[0x82, 0xa2, 0x61, 0x6b, 0x01, 0x61, 0x6b, 0x02, 0x03]).unwrap_err();
        assert!(
            !err.is_end_of_input(),
            "read past the declared entries: {err}"
        );
    }

    /// A declared length is attacker-controlled and unrelated to how much data follows, so it must
    /// never drive an allocation on its own.
    #[test]
    fn rejects_oversized_declared_array_length() {
        // u64::MAX elements; unclamped this panics with "capacity overflow"
        assert!(try_decode(&[0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]).is_err());
        // 2^58 elements, still enough to overflow `len * size_of::<Value>()`
        assert!(try_decode(&[0x9b, 0x04, 0, 0, 0, 0, 0, 0, 0]).is_err());
        // 1e9 elements, which unclamped allocates 1e9 * size_of::<Value>() up front
        assert!(try_decode(&[0x9a, 0x3b, 0x9a, 0xca, 0x00]).is_err());
    }

    #[test]
    fn rejects_oversized_declared_map_length() {
        assert!(try_decode(&[0xbb, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]).is_err());
        assert!(try_decode(&[0xba, 0x3b, 0x9a, 0xca, 0x00]).is_err());
    }

    /// The clamp must not penalise well-formed input.
    #[test]
    fn decodes_a_large_well_formed_array() {
        let value = Value::Array((0..1000).map(Value::Int).collect());
        assert_eq!(roundtrip(&value), value);
    }

    /// One array header per level, with a null innermost, is `MAX_NESTING` levels deep.
    #[test]
    fn accepts_input_at_the_nesting_limit() {
        let mut input = vec![0x81; MAX_NESTING - 1];
        input.push(0xf6);
        assert!(try_decode(&input).is_ok());

        let mut input: Vec<u8> = (0..MAX_NESTING - 1).flat_map(|_| [0xa1, 0x00]).collect();
        input.push(0xf6);
        assert!(try_decode(&input).is_ok());
    }

    /// One level past the limit must be an error rather than deeper recursion, so that the
    /// decoder rejects nesting instead of overflowing the stack on it.
    #[test]
    fn rejects_input_past_the_nesting_limit() {
        let mut input = vec![0x81; MAX_NESTING];
        input.push(0xf6);
        let err = try_decode(&input).unwrap_err();
        assert!(err.is_message(), "{err}");

        let mut input: Vec<u8> = (0..MAX_NESTING).flat_map(|_| [0xa1, 0x00]).collect();
        input.push(0xf6);
        let err = try_decode(&input).unwrap_err();
        assert!(err.is_message(), "{err}");

        // 100k levels, which without the limit overflows the stack and aborts the process
        let err = try_decode(&vec![0x81; 100_000]).unwrap_err();
        assert!(err.is_message(), "{err}");

        // the same, with every level also declaring 1e9 elements
        let input: Vec<u8> = (0..100_000)
            .flat_map(|_| [0x9a, 0x3b, 0x9a, 0xca, 0x00])
            .collect();
        let err = try_decode(&input).unwrap_err();
        assert!(err.is_message(), "{err}");
    }
}
