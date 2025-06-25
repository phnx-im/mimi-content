// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{
    de::{self, Unexpected},
    Deserialize, Serialize,
};
use serde_bytes::ByteBuf;
use serde_list::{ExternallyTagged, Serde_custom, Serde_list};
use sha2::Digest;
use std::{collections::BTreeMap, io::Cursor};

use crate::{MessageStatus, MessageStatusReport, PerMessageStatus};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported content type")]
    UnsupportedContentType,
    #[error("not UTF-8")]
    NotUtf8,
    #[error("deserialization failed")]
    DeserializationFailed,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Serde_list, PartialEq, Debug, Clone, Default)]
pub struct MimiContent {
    pub salt: ByteBuf, // TODO: Enforce size 16 bytes
    pub replaces: Option<ByteBuf>,
    pub topic_id: ByteBuf,
    pub expires: Option<Expiration>,  // TODO: RFC does not allow null
    pub in_reply_to: Option<ByteBuf>, // TODO: Enforce this is a message id
    pub extensions: BTreeMap<ExtensionName, ciborium::Value>, // TODO: Enforce max sizes
    pub nested_part: NestedPart,
}

#[derive(PartialEq, Eq, Debug, Clone, PartialOrd, Ord)]
pub enum ExtensionName {
    Text(String),
    Number(ciborium::value::Integer),
}

impl ExtensionName {
    pub fn into_value(self) -> ciborium::Value {
        match self {
            ExtensionName::Text(text) => ciborium::Value::Text(text),
            ExtensionName::Number(integer) => ciborium::Value::Integer(integer),
        }
    }
    pub fn from_value(value: ciborium::Value) -> Option<Self> {
        Some(match value {
            ciborium::Value::Text(text) => ExtensionName::Text(text),
            ciborium::Value::Integer(integer) => ExtensionName::Number(integer),
            _ => return None,
        })
    }
}

impl Serialize for ExtensionName {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.clone().into_value().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ExtensionName {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let value = ciborium::Value::deserialize(deserializer)?;

        Self::from_value(value.clone()).ok_or_else(|| {
            de::Error::invalid_type(
                Unexpected::Str(&format!("{:?}", value)),
                &"an extension name",
            )
        })
    }
}

impl MimiContent {
    pub fn message_id(&self, sender: &[u8], room: &[u8]) -> Vec<u8> {
        let mut value = Vec::new();
        value.extend(sender);
        value.extend(room);
        value.extend(self.serialize());
        value.extend(&self.salt);

        let hash = sha2::Sha256::digest(value);
        let mut result = vec![0x01];
        result.extend(&hash[0..31]);
        result
    }
}

impl MimiContent {
    pub fn simple_markdown_message(markdown: String, random_salt: &[u8]) -> Self {
        Self {
            salt: ByteBuf::from(random_salt),
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: None,
            extensions: BTreeMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part: NestedPartContent::SinglePart {
                    content_type: "text/markdown".to_owned(),
                    content: ByteBuf::from(markdown.into_bytes()),
                },
            },
        }
    }

    pub fn simple_delivery_receipt(targets: &[&[u8]], random_salt: &[u8]) -> Self {
        let report = MessageStatusReport {
            statuses: targets
                .iter()
                .map(|target| PerMessageStatus {
                    mimi_id: ByteBuf::from(*target),
                    status: MessageStatus::Delivered,
                })
                .collect(),
        };

        Self {
            salt: ByteBuf::from(random_salt),
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: None,
            extensions: BTreeMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Unspecified,
                language: "".to_owned(),
                part: NestedPartContent::SinglePart {
                    content_type: "application/mimi-message-status".to_owned(),
                    content: ByteBuf::from(report.serialize()),
                },
            },
        }
    }

    pub fn string_rendering(&self) -> Result<String> {
        // For now, we only support SingleParts that contain markdown messages.
        match &self.nested_part.part {
            NestedPartContent::SinglePart {
                content,
                content_type,
            } if content_type == "text/markdown" => {
                let markdown =
                    String::from_utf8(content.clone().into_vec()).map_err(|_| Error::NotUtf8)?;
                Ok(markdown)
            }
            _ => Err(Error::UnsupportedContentType),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        ciborium::ser::into_writer(&self, &mut result).unwrap();
        result
    }

    pub fn deserialize(input: &[u8]) -> Result<Self> {
        ciborium::de::from_reader(Cursor::new(input)).map_err(|_| Error::DeserializationFailed)
    }
}

#[derive(Serde_list, PartialEq, Eq, Debug, Clone)]
pub struct Expiration {
    pub relative: bool,
    pub time: u32,
}

/// Content Hashing Algorithm
///
/// See [Named Information Hash Algorithm Registry].
///
/// [Named Information Hash Algorithm Registry]: https://www.iana.org/assignments/named-information/named-information.xhtml
#[derive(Serde_custom, Debug, Clone, Copy, Eq, PartialEq, Default)]
#[repr(u8)]
#[non_exhaustive]
#[allow(non_camel_case_types)]
pub enum HashAlgorithm {
    #[default]
    Unspecified = 0,
    /// [RFC6920](https://www.rfc-editor.org/rfc/rfc6920.html)
    Sha256 = 1,
    /// [RFC6920](https://www.rfc-editor.org/rfc/rfc6920.html)
    Sha256_128 = 2,
    /// [RFC6920](https://www.rfc-editor.org/rfc/rfc6920.html)
    Sha256_120 = 3,
    /// [RFC6920](https://www.rfc-editor.org/rfc/rfc6920.html)
    Sha256_96 = 4,
    /// [RFC6920](https://www.rfc-editor.org/rfc/rfc6920.html)
    Sha256_64 = 5,
    /// [RFC6920](https://www.rfc-editor.org/rfc/rfc6920.html)
    Sha256_32 = 6,
    /// [FIPS 180-4](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf)
    Sha384 = 7,
    /// [FIPS 180-4](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf)
    Sha512 = 8,
    /// [FIPS 202](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf)
    Sha3_224 = 9,
    /// [FIPS 202](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf)
    Sha3_256 = 10,
    /// [FIPS 202](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf)
    Sha3_384 = 11,
    /// [FIPS 202](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf)
    Sha3_512 = 12,
    /// Custom hash algorithm
    Custom(u8),
}

#[derive(Serde_list, Debug, Clone, PartialEq, Eq, Default)]
pub struct NestedPart {
    pub disposition: Disposition,
    pub language: String, // TODO: Parse as Vec<LanguageTag> ?
    #[externally_tagged]
    pub part: NestedPartContent,
}

#[derive(Serde_custom, Debug, Clone, Copy, Eq, PartialEq, Default)]
#[repr(u8)]
pub enum Disposition {
    #[default]
    Unspecified = 0,
    Render = 1,
    Reaction = 2,
    Profile = 3,
    Inline = 4,
    Icon = 5,
    Attachment = 6,
    Session = 7,
    Preview = 8,
    Custom(u8),
}

#[derive(ExternallyTagged, Debug, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
#[allow(clippy::enum_variant_names)]
pub enum NestedPartContent {
    #[default]
    NullPart = 0,
    SinglePart {
        content_type: String,
        content: ByteBuf,
    } = 1,
    ExternalPart {
        content_type: String,
        url: String,
        expires: u32,
        size: u64,
        enc_alg: EncryptionAlgorithm,
        key: ByteBuf,
        nonce: ByteBuf,
        aad: ByteBuf,
        hash_alg: HashAlgorithm,
        content_hash: ByteBuf,
        description: String,
        filename: String,
    } = 2,
    MultiPart {
        part_semantics: PartSemantics,
        parts: Vec<NestedPart>,
    } = 3,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serde_custom)]
#[repr(u16)]
pub enum EncryptionAlgorithm {
    None = 0,
    /// Reference: [RFC5116](https://www.rfc-editor.org/rfc/rfc5116.html)
    Aes128Gcm = 1,
    /// Reference: [RFC5116](https://www.rfc-editor.org/rfc/rfc5116.html)
    Aes256Gcm = 2,
    /// Reference: [RFC5116](https://www.rfc-editor.org/rfc/rfc5116.html)
    Aes128Ccm = 3,
    /// Reference: [RFC5116](https://www.rfc-editor.org/rfc/rfc5116.html)
    Aes256Ccm = 4,
    /// Reference: [RFC5282](https://www.rfc-editor.org/rfc/rfc5282.html)
    Aes128Gcm8 = 5,
    /// Reference: [RFC5282](https://www.rfc-editor.org/rfc/rfc5282.html)
    Aes256Gcm8 = 6,
    /// Reference: [RFC5282](https://www.rfc-editor.org/rfc/rfc5282.html)
    Aes128Gcm12 = 7,
    /// Reference: [RFC5282](https://www.rfc-editor.org/rfc/rfc5282.html)
    Aes256Gcm12 = 8,
    /// Reference: [RFC5282](https://www.rfc-editor.org/rfc/rfc5282.html)
    Aes128CcmShort = 9,
    /// Reference: [RFC5282](https://www.rfc-editor.org/rfc/rfc5282.html)
    Aes256CcmShort = 10,
    /// Reference: [RFC5282](https://www.rfc-editor.org/rfc/rfc5282.html)
    Aes128CcmShort8 = 11,
    /// Reference: [RFC5282](https://www.rfc-editor.org/rfc/rfc5282.html)
    Aes256CcmShort8 = 12,
    /// Reference: [RFC5282](https://www.rfc-editor.org/rfc/rfc5282.html)
    Aes128CcmShort12 = 13,
    /// Reference: [RFC5282](https://www.rfc-editor.org/rfc/rfc5282.html)
    Aes256CcmShort12 = 14,
    /// Reference: [RFC5297](https://www.rfc-editor.org/rfc/rfc5297.html)
    AesSivCmac256 = 15,
    /// Reference: [RFC5297](https://www.rfc-editor.org/rfc/rfc5297.html)
    AesSivCmac384 = 16,
    /// Reference: [RFC5297](https://www.rfc-editor.org/rfc/rfc5297.html)
    AesSivCmac512 = 17,
    /// Reference: [RFC6655](https://www.rfc-editor.org/rfc/rfc6655.html)
    Aes128Ccm8 = 18,
    /// Reference: [RFC6655](https://www.rfc-editor.org/rfc/rfc6655.html)
    Aes256Ccm8 = 19,
    /// Reference: [RFC7253, Section 3.1](https://www.rfc-editor.org/rfc/rfc7253.html#section-3.1)
    Aes128OcbTaglen128 = 20,
    /// Reference: [RFC7253, Section 3.1](https://www.rfc-editor.org/rfc/rfc7253.html#section-3.1)
    Aes128OcbTaglen96 = 21,
    /// Reference: [RFC7253, Section 3.1](https://www.rfc-editor.org/rfc/rfc7253.html#section-3.1)
    Aes128OcbTaglen64 = 22,
    /// Reference: [RFC7253, Section 3.1](https://www.rfc-editor.org/rfc/rfc7253.html#section-3.1)
    Aes192OcbTaglen128 = 23,
    /// Reference: [RFC7253, Section 3.1](https://www.rfc-editor.org/rfc/rfc7253.html#section-3.1)
    Aes192OcbTaglen96 = 24,
    /// Reference: [RFC7253, Section 3.1](https://www.rfc-editor.org/rfc/rfc7253.html#section-3.1)
    Aes192OcbTaglen64 = 25,
    /// Reference: [RFC7253, Section 3.1](https://www.rfc-editor.org/rfc/rfc7253.html#section-3.1)
    Aes256OcbTaglen128 = 26,
    /// Reference: [RFC7253, Section 3.1](https://www.rfc-editor.org/rfc/rfc7253.html#section-3.1)
    Aes256OcbTaglen96 = 27,
    /// Reference: [RFC7253, Section 3.1](https://www.rfc-editor.org/rfc/rfc7253.html#section-3.1)
    Aes256OcbTaglen64 = 28,
    /// Reference: [RFC8439](https://www.rfc-editor.org/rfc/rfc8439.html)
    Chacha20Poly1305 = 29,
    /// Reference: [RFC8452](https://www.rfc-editor.org/rfc/rfc8452.html)
    Aes128GcmSiv = 30,
    /// Reference: [RFC8452](https://www.rfc-editor.org/rfc/rfc8452.html)
    Aes256GcmSiv = 31,
    /// Reference: [draft-irtf-cfrg-aegis-aead-08](https://datatracker.ietf.org/doc/draft-irtf-cfrg-aegis-aead/08/)
    Aegis128L = 32,
    /// Reference: [draft-irtf-cfrg-aegis-aead-08](https://datatracker.ietf.org/doc/draft-irtf-cfrg-aegis-aead/08/)
    Aegis256 = 33,
    /// Unknown algorithm
    Custom(u16),
}

#[derive(Serde_custom, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PartSemantics {
    ChooseOne = 0,
    SingleUnit = 1,
    ProcessAll = 2,
    Custom(u8),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::hex_decode;

    use super::*;

    fn extensions_alice() -> BTreeMap<ExtensionName, ciborium::Value> {
        let mut extensions = BTreeMap::new();
        extensions.insert(
            ExtensionName::Number(1.into()),
            ciborium::Value::from("mimi://example.com/u/alice-smith"),
        );
        extensions.insert(
            ExtensionName::Number(2.into()),
            ciborium::Value::from("mimi://example.com/r/engineering_team"),
        );

        extensions
    }

    fn extensions_bob() -> BTreeMap<ExtensionName, ciborium::Value> {
        let mut extensions = BTreeMap::new();
        extensions.insert(
            ExtensionName::Number(1.into()),
            ciborium::Value::from("mimi://example.com/u/bob-jones"),
        );
        extensions.insert(
            ExtensionName::Number(2.into()),
            ciborium::Value::from("mimi://example.com/r/engineering_team"),
        );

        extensions
    }

    fn extensions_cathy() -> BTreeMap<ExtensionName, ciborium::Value> {
        let mut extensions = BTreeMap::new();
        extensions.insert(
            ExtensionName::Number(1.into()),
            ciborium::Value::from("mimi://example.com/u/cathy-washington"),
        );
        extensions.insert(
            ExtensionName::Number(2.into()),
            ciborium::Value::from("mimi://example.com/r/engineering_team"),
        );

        extensions
    }

    #[test]
    fn original_message() {
        let value = MimiContent {
            salt: hex::decode("5eed9406c2545547ab6f09f20a18b003")
                .unwrap()
                .into(),
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: None,
            extensions: extensions_alice(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part: NestedPartContent::SinglePart {
                    content_type: "text/markdown;variant=GFM-MIMI".to_owned(),
                    content: ByteBuf::from(
                        b"Hi everyone, we just shipped release 2.0. __Good  work__!",
                    ),
                },
            },
        };

        assert_eq!(
            hex::encode(value.message_id(
                b"mimi://example.com/u/alice-smith",
                b"mimi://example.com/r/engineering_team"
            )),
            "01b0084467273cc43d6f0ebeac13eb84229c4fffe8f6c3594c905f47779e5a79"
        );

        let result = value.serialize();

        // Test deserialization
        let value2 = MimiContent::deserialize(&result).unwrap();
        assert_eq!(value, value2);

        // TODO: Mimi draft is wrong here
        let target = hex_decode(
            r#"
            87                                      # array(7)
               50                                   # bytes(16)
                  5eed9406c2545547ab6f09f20a18b003
               f6                                   # primitive(22)
               40                                   # bytes(0)
               f6                                   # primitive(22)
               f6                                   # primitive(22)
               a2                                   # map(2)
                  01                                # unsigned(1)
                  78 20                             # text(32)
                     6d696d693a2f2f6578616d706c652e63
                     6f6d2f752f616c6963652d736d697468
                     # "mimi://example.com/u/alice-smith"
                  02                                # unsigned(2)
                  78 25                             # text(37)
                     6d696d693a2f2f6578616d706c652e63
                     6f6d2f722f656e67696e656572696e67
                     5f7465616d
                     # "mimi://example.com/r/engineering_team"
               85                                   # array(5)
                  01                                # unsigned(1)
                  60                                # text(0)
                                                    # ""
                  01                                # unsigned(1)
                  78 1e                             # text(30)
                     746578742f6d61726b646f776e3b7661
                     7269616e743d47464d2d4d494d49
                     # "text/markdown;variant=GFM-MIMI"
                  58 39                             # bytes(57)
                     48692065766572796f6e652c20776520
                     6a75737420736869707065642072656c
                     6561736520322e302e205f5f476f6f64
                     2020776f726b5f5f21
                     # "Hi everyone, we just shipped release 2.0. __Good  work__!"


            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn reply() {
        let value = MimiContent {
            salt: hex::decode("11a458c73b8dd2cf404db4b378b8fe4d")
                .unwrap()
                .into(),
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: Some(
                hex::decode("01b0084467273cc43d6f0ebeac13eb84229c4fffe8f6c3594c905f47779e5a79")
                    .unwrap()
                    .into(),
            ),
            extensions: extensions_bob(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part: NestedPartContent::SinglePart {
                    content_type: "text/markdown;variant=GFM-MIMI".to_owned(),
                    content: ByteBuf::from(b"Right on! _Congratulations_ 'all!"),
                },
            },
        };

        let result = value.serialize();

        assert_eq!(
            hex::encode(value.message_id(
                b"mimi://example.com/u/bob-jones",
                b"mimi://example.com/r/engineering_team"
            )),
            "01a419aef4e16d43cfc06c28235ecfbe9faebc740d0148e7ca20b22150930836"
        );

        // Test deserialization
        let value2 = MimiContent::deserialize(&result).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex_decode(
            r#"
            87                                      # array(7)
               50                                   # bytes(16)
                  11a458c73b8dd2cf404db4b378b8fe4d
               f6                                   # primitive(22)
               40                                   # bytes(0)
               f6                                   # primitive(22)
               58 20                                # bytes(32)
                  01b0084467273cc43d6f0ebeac13eb84229c4fffe8f6c3594c905f47779e5a79
               a2                                   # map(2)
                  01                                # unsigned(1)
                  78 1e                             # text(30)
                     6d696d693a2f2f6578616d706c652e63
                     6f6d2f752f626f622d6a6f6e6573
                     # "mimi://example.com/u/bob-jones"
                  02                                # unsigned(2)
                  78 25                             # text(37)
                     6d696d693a2f2f6578616d706c652e63
                     6f6d2f722f656e67696e656572696e67
                     5f7465616d
                     # "mimi://example.com/r/engineering_team"
               85                                   # array(5)
                  01                                # unsigned(1)
                  60                                # text(0)
                                                    # ""
                  01                                # unsigned(1)
                  78 1e                             # text(30)
                     746578742f6d61726b646f776e3b7661
                     7269616e743d47464d2d4d494d49
                     # "text/markdown;variant=GFM-MIMI"
                  58 21                             # bytes(33)
                     5269676874206f6e21205f436f6e6772
                     6174756c6174696f6e735f2027616c6c
                     21
                     # "Right on! _Congratulations_ 'all!"
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn reaction() {
        let value = MimiContent {
            salt: hex::decode("d37bc0e6a8b4f04e9e6382375f587bf6")
                .unwrap()
                .into(),
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: Some(
                hex::decode("01b0084467273cc43d6f0ebeac13eb84229c4fffe8f6c3594c905f47779e5a79")
                    .unwrap()
                    .into(),
            ),
            extensions: extensions_cathy(),
            nested_part: NestedPart {
                disposition: Disposition::Reaction,
                language: "".to_owned(),
                part: NestedPartContent::SinglePart {
                    content_type: "text/plain;charset=utf-8".to_owned(),
                    content: ByteBuf::from("❤"),
                },
            },
        };

        let result = value.serialize();

        assert_eq!(
            hex::encode(value.message_id(
                b"mimi://example.com/u/cathy-washington",
                b"mimi://example.com/r/engineering_team"
            )),
            "01b1a14a88f4480e1336be86987854f838a3ec82944d4533d8d4088578550ed7"
        );

        // Test deserialization
        let value2 = MimiContent::deserialize(&result).unwrap();
        assert_eq!(value, value2);

        // TODO: Draft repo has wrong message ids here
        let target = hex_decode(
            r#"
            87                                      # array(7)
               50                                   # bytes(16)
                  d37bc0e6a8b4f04e9e6382375f587bf6
               f6                                   # primitive(22)
               40                                   # bytes(0)
               f6                                   # primitive(22)
               58 20                                # bytes(32)
               01b0084467273cc43d6f0ebeac13eb84229c4fffe8f6c3594c905f47779e5a79
               a2                                   # map(2)
                  01                                # unsigned(1)
                  78 25                             # text(37)
                     6d696d693a2f2f6578616d706c652e63
                     6f6d2f752f63617468792d7761736869
                     6e67746f6e
                     # "mimi://example.com/u/cathy-washington"
                  02                                # unsigned(2)
                  78 25                             # text(37)
                     6d696d693a2f2f6578616d706c652e63
                     6f6d2f722f656e67696e656572696e67
                     5f7465616d
                     # "mimi://example.com/r/engineering_team"
               85                                   # array(5)
                  02                                # unsigned(2)
                  60                                # text(0)
                                                    # ""
                  01                                # unsigned(1)
                  78 18                             # text(24)
                     746578742f706c61696e3b6368617273
                     65743d7574662d38
                     # "text/plain;charset=utf-8"
                  43                                # bytes(3)
                     e29da4                         # "❤"
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn edit() {
        let value = MimiContent {
            salt: hex::decode("b8c2e6d8800ecf45df39be6c45f4c042")
                .unwrap()
                .into(),
            replaces: Some(
                hex::decode(b"01a419aef4e16d43cfc06c28235ecfbe9faebc740d0148e7ca20b22150930836")
                    .unwrap()
                    .into(),
            ),
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: Some(
                hex::decode("01b0084467273cc43d6f0ebeac13eb84229c4fffe8f6c3594c905f47779e5a79")
                    .unwrap()
                    .into(),
            ),
            extensions: extensions_bob(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part: NestedPartContent::SinglePart {
                    content_type: "text/markdown;variant=GFM-MIMI".to_owned(),
                    content: ByteBuf::from(b"Right on! _Congratulations_ y'all!"),
                },
            },
        };

        let result = value.serialize();

        assert_eq!(
            hex::encode(value.message_id(
                b"mimi://example.com/u/bob-jones",
                b"mimi://example.com/r/engineering_team"
            )),
            "01fdcd2f418e4b16f6ba319800a44c12b3b0730871f29385bdc6d151b15751ad"
        );

        // Test deserialization
        let value2 = MimiContent::deserialize(&result).unwrap();
        assert_eq!(value, value2);

        // TODO: Draft repo has wrong message ids here
        let target = hex_decode(
            r#"
            87                                      # array(7)
               50                                   # bytes(16)
                  b8c2e6d8800ecf45df39be6c45f4c042  # "\xB8\xC2\xE6؀\u000E\xCFE\xDF9\xBElE\xF4\xC0B"
               58 20                                # bytes(32)
                  01a419aef4e16d43cfc06c28235ecfbe9faebc740d0148e7ca20b22150930836
               40                                   # bytes(0)
                                                    # ""
               f6                                   # primitive(22)
               58 20                                # bytes(32)
                  01b0084467273cc43d6f0ebeac13eb84229c4fffe8f6c3594c905f47779e5a79
               a2                                   # map(2)
                  01                                # unsigned(1)
                  78 1e                             # text(30)
                     6d696d693a2f2f6578616d706c652e636f6d2f752f626f622d6a6f6e6573 # "mimi://example.com/u/bob-jones"
                  02                                # unsigned(2)
                  78 25                             # text(37)
                     6d696d693a2f2f6578616d706c652e636f6d2f722f656e67696e656572696e675f7465616d # "mimi://example.com/r/engineering_team"
               85                                   # array(5)
                  01                                # unsigned(1)
                  60                                # text(0)
                                                    # ""
                  01                                # unsigned(1)
                  78 1e                             # text(30)
                     746578742f6d61726b646f776e3b76617269616e743d47464d2d4d494d49 # "text/markdown;variant=GFM-MIMI"
                  58 22                             # bytes(34)
                     5269676874206f6e21205f436f6e67726174756c6174696f6e735f207927616c6c21 # "Right on! _Congratulations_ y'all!"
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn delete() {
        let value = MimiContent {
            salt: hex::decode("0a590d73b2c7761c39168be5ebf7f2e6")
                .unwrap()
                .into(),
            replaces: Some(
                hex::decode(b"01a419aef4e16d43cfc06c28235ecfbe9faebc740d0148e7ca20b22150930836")
                    .unwrap()
                    .into(),
            ),
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: Some(
                hex::decode("01b0084467273cc43d6f0ebeac13eb84229c4fffe8f6c3594c905f47779e5a79")
                    .unwrap()
                    .into(),
            ),
            extensions: extensions_bob(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part: NestedPartContent::NullPart,
            },
        };

        let result = value.serialize();

        assert_eq!(
            hex::encode(value.message_id(
                b"mimi://example.com/u/bob-jones",
                b"mimi://example.com/r/engineering_team"
            )),
            "01b85744b443e9db85de5bb826c04bcd65b625e53d17839dc8a3f21321421088"
        );

        // Test deserialization
        let value2 = MimiContent::deserialize(&result).unwrap();
        assert_eq!(value, value2);

        // TODO: Draft repo has wrong message ids here
        let target = hex_decode(
            r#"
            87                                      # array(7)
                50                                   # bytes(16)
                0a590d73b2c7761c39168be5ebf7f2e6  # "\nY\rs\xB2\xC7v\u001C9\u0016\x8B\xE5\xEB\xF7\xF2\xE6"
                58 20                                # bytes(32)
                01a419aef4e16d43cfc06c28235ecfbe9faebc740d0148e7ca20b22150930836
                40                                   # bytes(0)
                                                    # ""
                f6                                   # primitive(22)
                58 20                                # bytes(32)
                01b0084467273cc43d6f0ebeac13eb84229c4fffe8f6c3594c905f47779e5a79
                a2                                   # map(2)
                01                                # unsigned(1)
                78 1e                             # text(30)
                    6d696d693a2f2f6578616d706c652e636f6d2f752f626f622d6a6f6e6573 # "mimi://example.com/u/bob-jones"
                02                                # unsigned(2)
                78 25                             # text(37)
                    6d696d693a2f2f6578616d706c652e636f6d2f722f656e67696e656572696e675f7465616d # "mimi://example.com/r/engineering_team"
                83                                   # array(3)
                01                                # unsigned(1)
                60                                # text(0)
                                                    # ""
                00                                # unsigned(0)
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn expiring() {
        let value = MimiContent {
            salt: hex::decode("33be993eb39f418f9295afc2ae160d2d")
                .unwrap()
                .into(),
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: Some(Expiration { relative: false, time: 1644390004 }),
            in_reply_to: None,
            extensions: extensions_alice(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part: NestedPartContent::SinglePart {
                    content_type: "text/markdown;variant=GFM-MIMI".to_owned(),
                    content: ByteBuf::from(b"__*VPN GOING DOWN*__ I'm rebooting the VPN in ten minutes unless anyone objects."),
                },
            },
        };

        let result = value.serialize();

        assert_eq!(
            hex::encode(value.message_id(
                b"mimi://example.com/u/alice-smith",
                b"mimi://example.com/r/engineering_team"
            )),
            "0106308e2c03346eba95b24abdfa9fe643aa247debfb7192feae647155316920"
        );

        // Test deserialization
        let value2 = MimiContent::deserialize(&result).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex_decode(
            r#"
            87                                      # array(7)
               50                                   # bytes(16)
                  33be993eb39f418f9295afc2ae160d2d
               f6                                   # primitive(22)
               40                                   # bytes(0)
                                                    # ""
               82                                   # array(2)
                  f4                                # primitive(20)
                  1a 62036674                       # unsigned(1644390004)
               f6                                   # primitive(22)
               a2                                   # map(2)
                  01                                # unsigned(1)
                  78 20                             # text(32)
                     6d696d693a2f2f6578616d706c652e63
                     6f6d2f752f616c6963652d736d697468
                     # "mimi://example.com/u/alice-smith"
                  02                                # unsigned(2)
                  78 25                             # text(37)
                     6d696d693a2f2f6578616d706c652e63
                     6f6d2f722f656e67696e656572696e67
                     5f7465616d
                     # "mimi://example.com/r/engineering_team"
               85                                   # array(5)
                  01                                # unsigned(1)
                  60                                # text(0)
                                                    # ""
                  01                                # unsigned(1)
                  78 1e                             # text(30)
                     746578742f6d61726b646f776e3b7661
                     7269616e743d47464d2d4d494d49
                     # "text/markdown;variant=GFM-MIMI"
                  58 50                             # bytes(80)
                     5f5f2a56504e20474f494e4720444f57
                     4e2a5f5f2049276d207265626f6f7469
                     6e67207468652056504e20696e207465
                     6e206d696e7574657320756e6c657373
                     20616e796f6e65206f626a656374732e
                     # "__*VPN GOING DOWN*__ I'm rebooting the VPN in" +
                     # " ten minutes unless anyone objects."
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn attachments() {
        let value = MimiContent {
            salt: hex::decode("18fac6371e4e53f1aeaf8a013155c166")
                .unwrap()
                .into(),
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: None,
            extensions: extensions_bob(),
            nested_part: NestedPart {
                disposition: Disposition::Attachment,
                language: "en".to_owned(),
                part: NestedPartContent::ExternalPart {
                    content_type: "video/mp4".to_owned(),
                    url: "https://example.com/storage/8ksB4bSrrRE.mp4".to_owned(),
                    expires: 0,
                    size: 708234961,
                    enc_alg: EncryptionAlgorithm::Aes128Gcm,
                    key: hex::decode("21399320958a6f4c745dde670d95e0d8")
                        .unwrap()
                        .into(),
                    nonce: hex::decode("c86cf2c33f21527d1dd76f5b").unwrap().into(),
                    aad: ByteBuf::from(b""),
                    hash_alg: HashAlgorithm::Sha256,
                    content_hash: hex::decode(
                        "9ab17a8cf0890baaae7ee016c7312fcc080ba46498389458ee44f0276e783163",
                    )
                    .unwrap()
                    .into(),
                    description: "2 hours of key signing video".to_owned(),
                    filename: "bigfile.mp4".to_owned(),
                },
            },
        };

        let result = value.serialize();

        assert_eq!(
            hex::encode(value.message_id(
                b"mimi://example.com/u/bob-jones",
                b"mimi://example.com/r/engineering_team"
            )),
            "01ad825f6116adeb437a7b1f95a9d9acbcc708f83f5df505d32af9c2826e8b5f"
        );

        // Test deserialization
        let value2 = MimiContent::deserialize(&result).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex_decode(
            r#"
            87                                      # array(7)
               50                                   # bytes(16)
                  18fac6371e4e53f1aeaf8a013155c166
               f6                                   # primitive(22)
               40                                   # bytes(0)
                                                    # ""
               f6                                   # primitive(22)
               f6                                   # primitive(22)
               a2                                   # map(2)
                  01                                # unsigned(1)
                  78 1e                             # text(30)
                     6d696d693a2f2f6578616d706c652e63
                     6f6d2f752f626f622d6a6f6e6573
                     # "mimi://example.com/u/bob-jones"
                  02                                # unsigned(2)
                  78 25                             # text(37)
                     6d696d693a2f2f6578616d706c652e63
                     6f6d2f722f656e67696e656572696e67
                     5f7465616d
                     # "mimi://example.com/r/engineering_team"
               8f                                   # array(15)
                  06                                # unsigned(6)
                  62                                # text(2)
                     656e                           # "en"
                  02                                # unsigned(2)
                  69                                # text(9)
                     766964656f2f6d7034             # "video/mp4"
                  78 2b                             # text(43)
                     68747470733a2f2f6578616d706c652e
                     636f6d2f73746f726167652f386b7342
                     346253727252452e6d7034
                     # "https://example.com/storage/8ksB4bSrrRE.mp4"
                  00                                # unsigned(0)
                  1a 2a36ced1                       # unsigned(708234961)
                  01                                # unsigned(1)
                  50                                # bytes(16)
                     21399320958a6f4c745dde670d95e0d8
                  4c                                # bytes(12)
                     c86cf2c33f21527d1dd76f5b
                  40                                # bytes(0)
                  01                                # unsigned(1)
                  58 20                             # bytes(32)
                     9ab17a8cf0890baaae7ee016c7312fcc
                     080ba46498389458ee44f0276e783163
                  78 1c                             # text(28)
                     3220686f757273206f66206b65792073
                     69676e696e6720766964656f
                     # "2 hours of key signing video"
                  6b                                # text(11)
                     62696766696c652e6d7034         # "bigfile.mp4"
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn conferencing() {
        let value = MimiContent {
            salt: hex::decode("678ac6cd54de049c3e9665cd212470fa")
                .unwrap()
                .into(),
            replaces: None,
            topic_id: ByteBuf::from(b"Foo 118"),
            expires: None,
            in_reply_to: None,
            extensions: extensions_alice(),
            nested_part: NestedPart {
                disposition: Disposition::Session,
                language: "".to_owned(),
                part: NestedPartContent::ExternalPart {
                    content_type: "".to_owned(),
                    url: "https://example.com/join/12345".to_owned(),
                    expires: 0,
                    size: 0,
                    enc_alg: EncryptionAlgorithm::None,
                    key: ByteBuf::new(),
                    nonce: ByteBuf::new(),
                    aad: ByteBuf::new(),
                    hash_alg: HashAlgorithm::Unspecified,
                    content_hash: ByteBuf::from(b""),
                    description: "Join the Foo 118 conference".to_owned(),
                    filename: "".to_owned(),
                },
            },
        };

        let result = value.serialize();

        assert_eq!(
            hex::encode(value.message_id(
                b"mimi://example.com/u/alice-smith",
                b"mimi://example.com/r/engineering_team"
            )),
            "01d8dab2e22b75dee4f5e52bb181d2d732008a235b80375113803e36b32a5f06"
        );

        // Test deserialization
        let value2 = MimiContent::deserialize(&result).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex_decode(
            r#"
            87                                      # array(7)
               50                                   # bytes(16)
                  678ac6cd54de049c3e9665cd212470fa
               f6                                   # primitive(22)
               47                                   # bytes(7)
                  466f6f20313138                    # "Foo 118"
               f6                                   # primitive(22)
               f6                                   # primitive(22)
               a2                                   # map(2)
                  01                                # unsigned(1)
                  78 20                             # text(32)
                     6d696d693a2f2f6578616d706c652e63
                     6f6d2f752f616c6963652d736d697468
                     # "mimi://example.com/u/alice-smith"
                  02                                # unsigned(2)
                  78 25                             # text(37)
                     6d696d693a2f2f6578616d706c652e63
                     6f6d2f722f656e67696e656572696e67
                     5f7465616d
                     # "mimi://example.com/r/engineering_team"
               8f                                   # array(15)
                  07                                # unsigned(7)
                  60                                # text(0)
                                                    # ""
                  02                                # unsigned(2)
                  60                                # text(0)
                                                    # ""
                  78 1e                             # text(30)
                     68747470733a2f2f6578616d706c652e
                     636f6d2f6a6f696e2f3132333435
                     # "https://example.com/join/12345"
                  00                                # unsigned(0)
                  00                                # unsigned(0)
                  00                                # unsigned(0)
                  40                                # bytes(0)
                                                    # ""
                  40                                # bytes(0)
                                                    # ""
                  40                                # bytes(0)
                                                    # ""
                  00                                # unsigned(0)
                  40                                # bytes(0)
                                                    # ""
                  78 1b                             # text(27)
                     4a6f696e2074686520466f6f20313138
                     20636f6e666572656e6365
                     # "Join the Foo 118 conference"
                  60                                # text(0)
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn multipart() {
        let value = MimiContent {
            salt: hex::decode("261c953e178af653fe3d42641b91d814")
                .unwrap()
                .into(),
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: None,
            extensions: extensions_alice(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part: NestedPartContent::MultiPart {
                    part_semantics: PartSemantics::ChooseOne,
                    parts: vec![
                        NestedPart {
                            disposition: Disposition::Render,
                            language: "".to_owned(),
                            part: NestedPartContent::SinglePart {
                                content_type: "text/markdown;variant=GFM-MIMI".to_owned(),
                                content: ByteBuf::from(b"# Welcome!"),
                            },
                        },
                        NestedPart {
                            disposition: Disposition::Render,
                            language: "".to_owned(),
                            part: NestedPartContent::SinglePart {
                                content_type: "application/vnd.examplevendor-fancy-im-message"
                                    .to_owned(),
                                content: hex::decode("dc861ebaa718fd7c3ca159f71a2001")
                                    .unwrap()
                                    .into(),
                            },
                        },
                    ],
                },
            },
        };

        let result = value.serialize();

        assert_eq!(
            hex::encode(value.message_id(
                b"mimi://example.com/u/alice-smith",
                b"mimi://example.com/r/engineering_team"
            )),
            "015c0469c52da0938c27cfa16702e27735a4729746be5f64bc5838f754828464"
        );

        // Test deserialization
        let value2 = MimiContent::deserialize(&result).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex_decode(
            r##"
            87                                      # array(7)
                50                                   # bytes(16)
                    261c953e178af653fe3d42641b91d814  # "&\u001C\x95>\u0017\x8A\xF6S\xFE=Bd\e\x91\xD8\u0014"
                f6                                   # primitive(22)
                40                                   # bytes(0)
                                                    # ""
                f6                                   # primitive(22)
                f6                                   # primitive(22)
                a2                                   # map(2)
                    01                                # unsigned(1)
                    78 20                             # text(32)
                        6d696d693a2f2f6578616d706c652e636f6d2f752f616c6963652d736d697468 # "mimi://example.com/u/alice-smith"
                    02                                # unsigned(2)
                    78 25                             # text(37)
                        6d696d693a2f2f6578616d706c652e636f6d2f722f656e67696e656572696e675f7465616d # "mimi://example.com/r/engineering_team"
                85                                   # array(5)
                    01                                # unsigned(1)
                    60                                # text(0)
                                                    # ""
                    03                                # unsigned(3)
                    00                                # unsigned(0)
                    82                                # array(2)
                        85                             # array(5)
                        01                          # unsigned(1)
                        60                          # text(0)
                                                    # ""
                        01                          # unsigned(1)
                        78 1e                       # text(30)
                            746578742f6d61726b646f776e3b76617269616e743d47464d2d4d494d49 # "text/markdown;variant=GFM-MIMI"
                        4a                          # bytes(10)
                            232057656c636f6d6521     # "# Welcome!"
                        85                             # array(5)
                        01                          # unsigned(1)
                        60                          # text(0)
                                                    # ""
                        01                          # unsigned(1)
                        78 2e                       # text(46)
                            6170706c69636174696f6e2f766e642e6578616d706c6576656e646f722d66616e63792d696d2d6d657373616765 # "application/vnd.examplevendor-fancy-im-message"
                        4f                          # bytes(15)
                            dc861ebaa718fd7c3ca159f71a2001 # "܆\u001E\xBA\xA7\u0018\xFD|<\xA1Y\xF7\u001A \u0001"
            "##,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }
}
