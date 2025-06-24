// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod content_container;
mod message_status;

pub use content_container::{
    Disposition, Error, Expiration, MimiContent, NestedPart, NestedPartContent, Result,
};
pub use message_status::{MessageStatus, MessageStatusReport, PerMessageStatus, Timestamp};
pub use serde_bytes::ByteBuf;

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
