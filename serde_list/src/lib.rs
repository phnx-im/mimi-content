// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![no_std]

pub use serde_list_macros::*;

pub trait ExternallyTagged: Sized {
    fn num_fields(&self) -> usize;
    fn serialize_fields<S: serde::ser::SerializeSeq>(&self, state: &mut S) -> Result<(), S::Error>;
    fn deserialize_fields<'a, S: serde::de::SeqAccess<'a>>(
        seq: &mut S,
        next_index: &mut impl FnMut() -> usize,
    ) -> Result<Self, S::Error>;
}
