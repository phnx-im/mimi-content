// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod content_container;
mod message_status;

pub use content_container::{
    Disposition, Error, Result, Expiration, InReplyTo, MimiContent, NestedPart, NestedPartContent};
pub use message_status::{MessageStatus, MessageStatusReport, PerMessageStatus, Timestamp};
