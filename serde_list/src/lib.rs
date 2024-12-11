// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![no_std]

pub use serde_list_macros::*;

pub trait ExternallyTagged {
    fn discriminant(&self) -> u8;
    fn num_fields(&self) -> usize;
    fn serialize_fields<S: serde::ser::SerializeSeq>(&self, state: &mut S) -> Result<(), S::Error>;
}
