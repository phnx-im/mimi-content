// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later
pub mod cbor;
pub mod content_container;
mod message_status;
#[cfg(feature = "serde")]
mod serde;
pub(crate) mod util;

pub use content_container::{Disposition, Error, MimiContent, MimiId, NestedPart, Result};
pub use message_status::{MessageStatus, MessageStatusReport, PerMessageStatus, Timestamp};
