// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{ser::SerializeSeq as _, Serialize};
use serde_bytes::ByteBuf;
use serde_tuple::Serialize_tuple;

#[derive(Debug, Clone)]
pub struct MessageStatusReport {
    timestamp: u64,
    statuses: Vec<PerMessageStatus>,
}

impl Serialize for MessageStatusReport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(2))?;
        state.serialize_element(&ciborium::Value::Tag(
            62,
            Box::new(ciborium::Value::Integer(ciborium::value::Integer::from(
                self.timestamp,
            ))),
        ))?;
        state.serialize_element(&self.statuses)?;
        state.end()
    }
}

#[derive(Serialize_tuple, Debug, Clone)]
pub struct PerMessageStatus {
    message_id: ByteBuf,
    status: MessageStatus,
}

#[derive(Debug, Clone)]
pub enum MessageStatus {
    Unread,
    Delivered,
    Read,
    Expired,
    Deleted,
    Hidden,
    Error,
    Custom(u8),
}

impl Serialize for MessageStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            MessageStatus::Unread => 0,
            MessageStatus::Delivered => 1,
            MessageStatus::Read => 2,
            MessageStatus::Expired => 3,
            MessageStatus::Deleted => 4,
            MessageStatus::Hidden => 5,
            MessageStatus::Error => 6,
            MessageStatus::Custom(u) => *u,
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_message() {
        let value = MessageStatusReport {
            timestamp: 1644284703227,
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

        // Taken from MIMI content format draft
        let target = hex::decode("82d83e1b0000017ed70171fb84825820d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e5150102825820e701beee59f9376282f39092e1041b2ac2e3aad1776570c1a28de244979c71ed028258206b50bfdd71edc83554ae21380080f4a3ba77985da34528a515fac3c38e4998b8008258205c95a4dfddab84348bcc265a479299fbd3a2eecfa3d490985da5113e5480c7f103").unwrap();

        assert_eq!(result, target);
    }
}
