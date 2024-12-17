// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{
    de::{self},
    Deserialize, Serialize,
};
use serde_bytes::ByteBuf;
use serde_list::{Serde_custom_u8, Serde_list};

#[derive(Serde_list, Debug, Clone, PartialEq, Eq)]
pub struct MessageStatusReport {
    timestamp: Timestamp,
    statuses: Vec<PerMessageStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Timestamp(u64);

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
    message_id: ByteBuf,
    status: MessageStatus,
}

#[derive(Serde_custom_u8, Debug, Clone, Copy, PartialEq, Eq)]
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
    use std::io::Cursor;

    use super::*;

    #[test]
    fn statuses() {
        let value = MessageStatusReport {
            timestamp: Timestamp(1644284703227),
            statuses: vec![
                PerMessageStatus {
                    message_id: hex::decode(
                        "d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e51501",
                    )
                    .unwrap()
                    .into(),
                    status: MessageStatus::Read,
                },
                PerMessageStatus {
                    message_id: hex::decode(
                        "e701beee59f9376282f39092e1041b2ac2e3aad1776570c1a28de244979c71ed",
                    )
                    .unwrap()
                    .into(),
                    status: MessageStatus::Read,
                },
                PerMessageStatus {
                    message_id: hex::decode(
                        "6b50bfdd71edc83554ae21380080f4a3ba77985da34528a515fac3c38e4998b8",
                    )
                    .unwrap()
                    .into(),
                    status: MessageStatus::Unread,
                },
                PerMessageStatus {
                    message_id: hex::decode(
                        "5c95a4dfddab84348bcc265a479299fbd3a2eecfa3d490985da5113e5480c7f1",
                    )
                    .unwrap()
                    .into(),
                    status: MessageStatus::Expired,
                },
            ],
        };

        let mut result = Vec::new();
        ciborium::ser::into_writer(&value, &mut result).unwrap();

        // Test deserialization
        let value2 = ciborium::de::from_reader(Cursor::new(result.clone())).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex::decode("82d83e1b0000017ed70171fb84825820d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e5150102825820e701beee59f9376282f39092e1041b2ac2e3aad1776570c1a28de244979c71ed028258206b50bfdd71edc83554ae21380080f4a3ba77985da34528a515fac3c38e4998b8008258205c95a4dfddab84348bcc265a479299fbd3a2eecfa3d490985da5113e5480c7f103").unwrap();

        assert_eq!(result, target);
    }
}
