<!--
SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>

SPDX-License-Identifier: CC-BY-4.0
-->

# MIMI Message Content

This repository contains the code for a Rust implementation of the [MIMI message content draft](https://datatracker.ietf.org/doc/draft-ietf-mimi-content/).

The crate implements the message container and the message ID derivation of [draft-ietf-mimi-content-09](https://www.ietf.org/archive/id/draft-ietf-mimi-content-09.html), and the message status report format of [draft-mahy-mimi-message-status-01](https://www.ietf.org/archive/id/draft-mahy-mimi-message-status-01.html).

Decoding is lenient. Salts of any length, indefinite-length items, single-part multiparts, floats of any width and extension names outside the draft's data model are accepted, so that messages from older or non-conforming implementations still decode. Re-serializing decoded content reproduces its bytes, except for input that was not in shortest-form or definite-length encoding.

Encoding is deterministic in the sense of RFC 8949 section 4.2.1 for everything the draft's examples use: shortest-form integers and lengths, definite lengths, and extension keys sorted bytewise on their encoding. Floats are always encoded as 64-bit values, which keeps re-serialization byte-stable but is not the shortest form. The data model rules on extension names are enforced by `MimiContent::with_extension`, not by the encoder, and the rules on nested map keys and on nesting depth beyond `with_extension` are not enforced.

## Licensing

All crates in this repository are licensed under the [AGPL 3.0](https://www.gnu.org/licenses/agpl-3.0.html). This README file is licensed under [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/).
