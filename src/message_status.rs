// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::Cursor;

use serde::{
    de::{self},
    Deserialize, Serialize,
};
use serde_bytes::ByteBuf;
use serde_list::{Serde_custom, Serde_list};

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageStatusReport {
    pub statuses: Vec<PerMessageStatus>,
}

impl MessageStatusReport {
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        ciborium::ser::into_writer(&self.statuses, &mut result)
            .map_err(|_| Error::SerializationFailed)?;
        Ok(result)
    }

    pub fn deserialize(input: &[u8]) -> Result<Self> {
        Ok(Self {
            statuses: ciborium::de::from_reader(Cursor::new(input))
                .map_err(|_| Error::DeserializationFailed)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timestamp(pub u64);

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ciborium::Value::Tag(
            62,
            Box::new(ciborium::Value::Integer(ciborium::value::Integer::from(
                self.0,
            ))),
        )
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = ciborium::Value::deserialize(deserializer)?;
        if let ciborium::Value::Tag(62, v) = value {
            if let ciborium::Value::Integer(timestamp) = *v {
                Ok(Timestamp(u64::try_from(timestamp).map_err(|_| {
                    de::Error::invalid_value(
                        de::Unexpected::Other(&i128::from(timestamp).to_string()),
                        &"timestamp must fit in u64",
                    )
                })?))
            } else {
                Err(de::Error::invalid_type(
                    de::Unexpected::StructVariant,
                    &"Timestamp must be an integer",
                ))
            }
        } else {
            Err(de::Error::invalid_type(
                de::Unexpected::StructVariant,
                &"Timestamp must have tag 62",
            ))
        }
    }
}

#[derive(Serde_list, Debug, Clone, PartialEq, Eq)]
pub struct PerMessageStatus {
    pub mimi_id: ByteBuf,
    pub status: MessageStatus,
}

#[derive(Serde_custom, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageStatus {
    Unread = 0,
    Delivered = 1,
    Read = 2,
    Expired = 3,
    Deleted = 4,
    Hidden = 5,
    Error = 6,
    Custom(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses() {
        let value = MessageStatusReport {
            statuses: vec![
                PerMessageStatus {
                    mimi_id: hex::decode(
                        b"010714238126772e253118df3cd18fa69f90841d7df1f6f0cddab1f0dc0c9a26",
                    )
                    .unwrap()
                    .into(),
                    status: MessageStatus::Read,
                },
                PerMessageStatus {
                    mimi_id: hex::decode(
                        b"01efab9eca8374d3618a16b39c658689fd90d07fe666a846178cb4965c94a8bf",
                    )
                    .unwrap()
                    .into(),
                    status: MessageStatus::Read,
                },
                PerMessageStatus {
                    mimi_id: hex::decode(
                        b"0103d50d4980c0a7a0990f65534ebd4f0fa36b1f4680d6e080c19ea4a95def7b",
                    )
                    .unwrap()
                    .into(),
                    status: MessageStatus::Unread,
                },
                PerMessageStatus {
                    mimi_id: hex::decode(
                        b"0114e486b39d705e15e3000b57290de479affbda4ec2c1b17cc25c214229ed7d",
                    )
                    .unwrap()
                    .into(),
                    status: MessageStatus::Expired,
                },
            ],
        };

        let result = value.serialize().unwrap();

        // Test deserialization
        let value2 = MessageStatusReport::deserialize(&result).unwrap();
        assert_eq!(value, value2);

        // TODO: Draft has wrong message ids
        // Taken from MIMI content format draft
        let target = crate::hex_decode(
            r#"
            84                                      # array(4)
               82                                   # array(2)
                  58 20                             # bytes(32)
                     010714238126772e253118df3cd18fa69f90841d7df1f6f0cddab1f0dc0c9a26 # "\u0001\a\u0014#\x81&w.%1\u0018\xDF<я\xA6\x9F\x90\x84\u001D}\xF1\xF6\xF0\xCDڱ\xF0\xDC\f\x9A&"
                  02                                # unsigned(2)
               82                                   # array(2)
                  58 20                             # bytes(32)
                     01efab9eca8374d3618a16b39c658689fd90d07fe666a846178cb4965c94a8bf # "\u0001\uFADEʃt\xD3a\x8A\u0016\xB3\x9Ce\x86\x89\xFD\x90\xD0\u007F\xE6f\xA8F\u0017\x8C\xB4\x96\\\x94\xA8\xBF"
                  02                                # unsigned(2)
               82                                   # array(2)
                  58 20                             # bytes(32)
                     0103d50d4980c0a7a0990f65534ebd4f0fa36b1f4680d6e080c19ea4a95def7b # "\u0001\u0003\xD5\rI\x80\xC0\xA7\xA0\x99\u000FeSN\xBDO\u000F\xA3k\u001FF\x80\xD6\xE0\x80\xC1\x9E\xA4\xA9]\xEF{"
                  00                                # unsigned(0)
               82                                   # array(2)
                  58 20                             # bytes(32)
                     0114e486b39d705e15e3000b57290de479affbda4ec2c1b17cc25c214229ed7d # "\u0001\u0014䆳\x9Dp^\u0015\xE3\u0000\vW)\r\xE4y\xAF\xFB\xDAN\xC2\xC1\xB1|\xC2\\!B)\xED}"
                  03                                # unsigned(3)
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }
}
