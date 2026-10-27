<!--
SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>

SPDX-License-Identifier: CC-BY-4.0
-->

# MIMI Message Content

This repository contains the code for a Rust implementation of the [MIMI message content draft](https://datatracker.ietf.org/doc/draft-ietf-mimi-content/).

The crate implements [draft-ietf-mimi-content-09](https://www.ietf.org/archive/id/draft-ietf-mimi-content-09.html), including its message ID derivation, and the message status report format of [draft-mahy-mimi-message-status-01](https://www.ietf.org/archive/id/draft-mahy-mimi-message-status-01.html). Encoding follows the draft's deterministic encoding and data model rules. Decoding is lenient so that messages from older implementations still round-trip.

## Licensing

All crates in this repository are licensed under the [AGPL 3.0](https://www.gnu.org/licenses/agpl-3.0.html). This README file is licensed under [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/).
