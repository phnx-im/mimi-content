// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{
    de::{self, Unexpected},
    Deserialize, Serialize,
};
use serde_bytes::ByteBuf;
use serde_list::{ExternallyTagged, Serde_custom_u8, Serde_list};
use std::collections::HashMap;

#[derive(Serde_list, PartialEq, Eq, Debug, Clone)]
pub struct MimiContent {
    replaces: Option<ByteBuf>,
    topic_id: ByteBuf,
    expires: Option<Expiration>, // TODO: RFC does not allow null
    in_reply_to: Option<InReplyTo>,
    last_seen: Vec<ByteBuf>,
    extensions: HashMap<String, ByteBuf>, // TODO: Enforce max sizes
    nested_part: NestedPart,
    // TODO: Wrapper struct for MessageDerivedValues, like messageId, roomUrl, hubAcceptedTimestamp?
}

#[derive(Serde_list, PartialEq, Eq, Debug, Clone)]
pub struct Expiration {
    relative: bool,
    time: u32,
}

#[derive(Serde_list, PartialEq, Eq, Debug, Clone)]
pub struct InReplyTo {
    message: ByteBuf,
    hash_alg: u8, // TODO: Enum
    hash: ByteBuf,
}

#[derive(Serde_list, Debug, Clone, PartialEq, Eq)]
pub struct NestedPart {
    disposition: Disposition,
    language: String, // TODO: Parse as Vec<LanguageTag> ?
    #[externally_tagged]
    part: NestedPartContent,
}

#[derive(Serde_custom_u8, Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum Disposition {
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

#[derive(ExternallyTagged, Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum NestedPartContent {
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
        enc_alg: u16,
        key: ByteBuf,
        nonce: ByteBuf,
        aad: ByteBuf,
        hash_alg: u8,
        content_hash: ByteBuf,
        description: String,
        filename: String,
    } = 2,
    MultiPart {
        part_semantics: PartSemantics,
        parts: Vec<NestedPart>,
    } = 3,
}

#[derive(Serde_custom_u8, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PartSemantics {
    ChooseOne = 0,
    SingleUnit = 1,
    ProcessAll = 2,
    Custom(u8),
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

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

    #[test]
    fn original_message() {
        let value = MimiContent {
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: None,
            last_seen: vec![],
            extensions: HashMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part: NestedPartContent::SinglePart {
                    content_type: "text/markdown;charset=utf-8".to_owned(), // Mistake in content format draft: It says variant=GFM here
                    content: ByteBuf::from(
                        b"Hi everyone, we just shipped release 2.0. __Good work__!",
                    ),
                },
            },
        };

        let mut result = Vec::new();
        ciborium::ser::into_writer(&value, &mut result).unwrap();

        // Test deserialization
        let value2 = ciborium::de::from_reader(Cursor::new(result.clone())).unwrap();
        assert_eq!(value, value2);

        // TODO: Mimi draft is wrong here
        let target = hex_decode(
            r#"
            87                                      # array(7)
               f6                                   # primitive(22)
               40                                   # bytes(0)
                                                    # ""
               f6                                   # primitive(22)
               f6                                   # primitive(22)
               80                                   # array(0)
               a0                                   # map(0)
               85                                   # array(6)
                  01                                # unsigned(1)
                  60                                # text(0)
                                                    # ""
                  01                                # unsigned(1)
                  78 1b                             # text(27)
                     746578742f6d61726b646f776e3b636861727365743d7574662d38
                     # "text/markdown;charset=utf-8"
                  58 38                             # bytes(56)
                     48692065766572796f6e652c207765206a757374207368697070656420
                     72656c6561736520322e302e205f5f476f6f6420776f726b5f5f21
                     # "Hi everyone, we just shipped release 2.0. __Good work__!"
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn reply() {
        let value = MimiContent {
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: Some(InReplyTo {
                message: hex::decode(
                    "d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e51501",
                )
                .unwrap()
                .into(),
                hash_alg: 1,
                hash: hex::decode(
                    "6b44053cb68e3f0cdd219da8d7104afc2ae5ffff782154524cef093de39345a5",
                )
                .unwrap()
                .into(),
            }),
            last_seen: vec![hex::decode(
                "d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e51501",
            )
            .unwrap()
            .into()],
            extensions: HashMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part: NestedPartContent::SinglePart {
                    content_type: "text/markdown;charset=utf-8".to_owned(), // Mistake in content format draft: It says variant=GFM here
                    content: ByteBuf::from(b"Right on! _Congratulations_ 'all!"),
                },
            },
        };

        let mut result = Vec::new();
        ciborium::ser::into_writer(&value, &mut result).unwrap();

        // Test deserialization
        let value2 = ciborium::de::from_reader(Cursor::new(result.clone())).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex_decode(
            r#"
            87                                      # array(7)
               f6                                   # primitive(22)
               40                                   # bytes(0)
                                                    # ""
               f6                                   # primitive(22)
               83                                   # array(3)
                  58 20                             # bytes(32)
                     d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e51501
                  01                                # unsigned(1)
                  58 20                             # bytes(32)
                     6b44053cb68e3f0cdd219da8d7104afc2ae5ffff782154524cef093de39345a5
               81                                   # array(1)
                  58 20                             # bytes(32)
                     d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e51501
               a0                                   # map(0)
               85                                   # array(6)
                  01                                # unsigned(1)
                  60                                # text(0)
                                                    # ""
                  01                                # unsigned(1)
                  78 1b                             # text(27)
                     746578742f6d61726b646f776e3b636861727365743d7574662d38
                     # "text/markdown;charset=utf-8"
                  58 21                             # bytes(33)
                     5269676874206f6e21205f436f6e67726174756c6174696f6e735f2027616c6c21
                     # "Right on! _Congratulations_ 'all!"
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn reaction() {
        let value = MimiContent {
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: Some(InReplyTo {
                message: hex::decode(
                    "d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e51501",
                )
                .unwrap()
                .into(),
                hash_alg: 1,
                hash: hex::decode(
                    "6b44053cb68e3f0cdd219da8d7104afc2ae5ffff782154524cef093de39345a5",
                )
                .unwrap()
                .into(),
            }),
            last_seen: vec![hex::decode(
                "e701beee59f9376282f39092e1041b2ac2e3aad1776570c1a28de244979c71ed",
            )
            .unwrap()
            .into()],
            extensions: HashMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Reaction,
                language: "".to_owned(),
                part: NestedPartContent::SinglePart {
                    content_type: "text/plain;charset=utf-8".to_owned(),
                    content: ByteBuf::from("♥"),
                },
            },
        };

        let mut result = Vec::new();
        ciborium::ser::into_writer(&value, &mut result).unwrap();

        // Test deserialization
        let value2 = ciborium::de::from_reader(Cursor::new(result.clone())).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex_decode(
            r#"
            87                                      # array(7)
               f6                                   # primitive(22)
               40                                   # bytes(0)
                                                    # ""
               f6                                   # primitive(22)
               83                                   # array(3)
                  58 20                             # bytes(32)
                     d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e51501
                  01                                # unsigned(1)
                  58 20                             # bytes(32)
                     6b44053cb68e3f0cdd219da8d7104afc2ae5ffff782154524cef093de39345a5
               81                                   # array(1)
                  58 20                             # bytes(32)
                     e701beee59f9376282f39092e1041b2ac2e3aad1776570c1a28de244979c71ed
               a0                                   # map(0)
               85                                   # array(6)
                  02                                # unsigned(2)
                  60                                # text(0)
                                                    # ""
                  01                                # unsigned(1)
                  78 18                             # text(24)
                     746578742f706c61696e3b636861727365743d7574662d38
                     # "text/plain;charset=utf-8"
                  43                                # bytes(3)
                     e299a5                         # "♥"
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn edit() {
        let value = MimiContent {
            replaces: Some(
                hex::decode(b"e701beee59f9376282f39092e1041b2ac2e3aad1776570c1a28de244979c71ed")
                    .unwrap()
                    .into(),
            ),
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: Some(InReplyTo {
                message: hex::decode(
                    "d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e51501",
                )
                .unwrap()
                .into(),
                hash_alg: 1,
                hash: hex::decode(
                    "6b44053cb68e3f0cdd219da8d7104afc2ae5ffff782154524cef093de39345a5",
                )
                .unwrap()
                .into(),
            }),
            last_seen: vec![
                hex::decode("4dcab7711a77ea1dd025a6a1a7fe01ab3b0d690f82417663cb752dfcc37779a1")
                    .unwrap()
                    .into(),
                hex::decode("6b50bfdd71edc83554ae21380080f4a3ba77985da34528a515fac3c38e4998b8")
                    .unwrap()
                    .into(),
            ],
            extensions: HashMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part: NestedPartContent::SinglePart {
                    content_type: "text/markdown;charset=utf-8".to_owned(), // Mistake in content format draft: It says variant=GFM here
                    content: ByteBuf::from(b"Right on! _Congratulations_ y'all!"),
                },
            },
        };

        let mut result = Vec::new();
        ciborium::ser::into_writer(&value, &mut result).unwrap();

        // Test deserialization
        let value2 = ciborium::de::from_reader(Cursor::new(result.clone())).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex_decode(
            r#"
            87                                      # array(7)
               58 20                                # bytes(32)
                  e701beee59f9376282f39092e1041b2ac2e3aad1776570c1a28de244979c71ed
               40                                   # bytes(0)
                                                    # ""
               f6                                   # primitive(22)
               83                                   # array(3)
                  58 20                             # bytes(32)
                     d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e51501
                  01                                # unsigned(1)
                  58 20                             # bytes(32)
                     6b44053cb68e3f0cdd219da8d7104afc2ae5ffff782154524cef093de39345a5
               82                                   # array(2)
                  58 20                             # bytes(32)
                     4dcab7711a77ea1dd025a6a1a7fe01ab3b0d690f82417663cb752dfcc37779a1
                  58 20                             # bytes(32)
                     6b50bfdd71edc83554ae21380080f4a3ba77985da34528a515fac3c38e4998b8
               a0                                   # map(0)
               85                                   # array(6)
                  01                                # unsigned(1)
                  60                                # text(0)
                                                    # ""
                  01                                # unsigned(1)
                  78 1b                             # text(27)
                     746578742f6d61726b646f776e3b636861727365743d7574662d38
                     # "text/markdown;charset=utf-8"
                  58 22                             # bytes(34)
                     5269676874206f6e21205f436f6e67726174756c6174696f6e735f
                     207927616c6c21
                     # "Right on! _Congratulations_ y'all!"
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn delete() {
        let value = MimiContent {
            replaces: Some(
                hex::decode(b"4dcab7711a77ea1dd025a6a1a7fe01ab3b0d690f82417663cb752dfcc37779a1") // The content draft has a wrong id here
                    .unwrap()
                    .into(),
            ),
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: Some(InReplyTo {
                message: hex::decode(
                    "d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e51501",
                )
                .unwrap()
                .into(),
                hash_alg: 1,
                hash: hex::decode(
                    "6b44053cb68e3f0cdd219da8d7104afc2ae5ffff782154524cef093de39345a5",
                )
                .unwrap()
                .into(),
            }),
            last_seen: vec![hex::decode(
                "89d3472622a40d6ceeb27c42490fdc64c0e9c20c598f9d7c8e81640dae8db0fb",
            )
            .unwrap()
            .into()],
            extensions: HashMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Reaction, // The draft writes Render, but uses the number for Reaction
                language: "".to_owned(),
                part: NestedPartContent::NullPart,
            },
        };

        let mut result = Vec::new();
        ciborium::ser::into_writer(&value, &mut result).unwrap();

        // Test deserialization
        let value2 = ciborium::de::from_reader(Cursor::new(result.clone())).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex_decode(
            r#"
            87                                      # array(7)
               58 20                                # bytes(32)
                  4dcab7711a77ea1dd025a6a1a7fe01ab3b0d690f82417663cb752dfcc37779a1
               40                                   # bytes(0)
                                                    # ""
               f6                                   # primitive(22)
               83                                   # array(3)
                  58 20                             # bytes(32)
                     d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e51501
                  01                                # unsigned(1)
                  58 20                             # bytes(32)
                     6b44053cb68e3f0cdd219da8d7104afc2ae5ffff782154524cef093de39345a5
               81                                   # array(1)
                  58 20                             # bytes(32)
                     89d3472622a40d6ceeb27c42490fdc64c0e9c20c598f9d7c8e81640dae8db0fb
               a0                                   # map(0)
               83                                   # array(4)
                  02                                # unsigned(2)
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
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: Some(Expiration { relative: false, time: 1644390004 }),
            in_reply_to: None,
            last_seen: vec![hex::decode(
                "1a771ca1d84f8fda4184a1e02a549e201bf434c6bfcf1237fa45463c6861853b",
            )
            .unwrap()
            .into()],
            extensions: HashMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part: NestedPartContent::SinglePart {
                    content_type: "text/markdown;charset=utf-8".to_owned(), // Mistake in content format draft: It says variant=GFM here
                    content: ByteBuf::from(b"__*VPN GOING DOWN*__\nI'm rebooting the VPN in ten minutes unless anyone objects."),
                },
            },
        };

        let mut result = Vec::new();
        ciborium::ser::into_writer(&value, &mut result).unwrap();

        // Test deserialization
        let value2 = ciborium::de::from_reader(Cursor::new(result.clone())).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex_decode(
            r#"
            87                                      # array(7)
               f6                                   # primitive(22)
               40                                   # bytes(0)
                                                    # ""
               82                                   # array(2)
                  f4                                # primitive(20)
                  1a 62036674                       # unsigned(1644390004)
               f6                                   # primitive(22)
               81                                   # array(1)
                  58 20                             # bytes(32)
                     1a771ca1d84f8fda4184a1e02a549e201bf434c6bfcf1237fa45463c6861853b
               a0                                   # map(0)
               85                                   # array(6)
                  01                                # unsigned(1)
                  60                                # text(0)
                                                    # ""
                  01                                # unsigned(1)
                  78 1b                             # text(27)
                     746578742f6d61726b646f776e3b636861727365743d7574662d38
                     # "text/markdown;charset=utf-8"
                  58 50                             # bytes(80)
                     5f5f2a56504e20474f494e4720444f574e2a5f5f0a49276d207265
                     626f6f74696e67207468652056504e20696e2074656e206d696e75
                     74657320756e6c65737320616e796f6e65206f626a656374732e
                     # "__*VPN GOING DOWN*__\nI'm rebooting the VPN in ten
                     #  minutes unless anyone objects."
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn attachments() {
        let value = MimiContent {
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: None,
            last_seen: vec![hex::decode(
                "5c95a4dfddab84348bcc265a479299fbd3a2eecfa3d490985da5113e5480c7f1",
            )
            .unwrap()
            .into()],
            extensions: HashMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Attachment,
                language: "en".to_owned(),
                part: NestedPartContent::ExternalPart {
                    content_type: "video/mp4".to_owned(),
                    url: "https://example.com/storage/8ksB4bSrrRE.mp4".to_owned(),
                    expires: 0,
                    size: 708234961,
                    enc_alg: 1,
                    key: hex::decode("21399320958a6f4c745dde670d95e0d8")
                        .unwrap()
                        .into(),
                    nonce: hex::decode("c86cf2c33f21527d1dd76f5b").unwrap().into(),
                    aad: ByteBuf::from(b""),
                    hash_alg: 1,
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

        let mut result = Vec::new();
        ciborium::ser::into_writer(&value, &mut result).unwrap();

        // Test deserialization
        let value2 = ciborium::de::from_reader(Cursor::new(result.clone())).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex_decode(
            r#"
            87                                      # array(7)
               f6                                   # primitive(22)
               40                                   # bytes(0)
                                                    # ""
               f6                                   # primitive(22)
               f6                                   # primitive(22)
               81                                   # array(1)
                  58 20                             # bytes(32)
                     5c95a4dfddab84348bcc265a479299fbd3a2eecfa3d490985da5113e5480c7f1
               a0                                   # map(0)
               8f                                   # array(16)
                  06                                # unsigned(6)
                  62                                # text(2)
                     656e                           # "en"
                  # TODO 00                                # unsigned(0)
                  02                                # unsigned(2)
                  69                                # text(9)
                     766964656f2f6d7034             # "video/mp4"
                  78 2b                             # text(43)
                     68747470733a2f2f6578616d706c652e636f6d2f73746f72616
                     7652f386b7342346253727252452e6d7034
                     # "https://example.com/storage/8ksB4bSrrRE.mp4"
                  00                                # unsigned(0)
                  1a 2a36ced1                       # unsigned(708234961)
                  01                                # unsigned(1)
                  50                                # bytes(16)
                     21399320958a6f4c745dde670d95e0d8
                  4c                                # bytes(12)
                     c86cf2c33f21527d1dd76f5b
                  40                                # bytes(0)
                                                    # ""
                  01                                # unsigned(1)
                  58 20                             # bytes(32)
                     9ab17a8cf0890baaae7ee016c7312fcc080ba46498389458ee44f0276e783163
                  78 1c                             # text(28)
                     3220686f757273206f66206b6579207369676e696e6720766964656f
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
            replaces: None,
            topic_id: ByteBuf::from(b"Foo 118"),
            expires: None,
            in_reply_to: None,
            last_seen: vec![hex::decode(
                "b267614d43e7676d28ef5b15e8676f23679fe365c78849d83e2ba0ae8196ec4e",
            )
            .unwrap()
            .into()],
            extensions: HashMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Session,
                language: "".to_owned(),
                part: NestedPartContent::ExternalPart {
                    content_type: "".to_owned(),
                    url: "https://example.com/join/12345".to_owned(),
                    expires: 0,
                    size: 0,
                    enc_alg: 0,
                    key: ByteBuf::new(),
                    nonce: ByteBuf::new(),
                    aad: ByteBuf::new(),
                    hash_alg: 0,
                    content_hash: ByteBuf::from(b""),
                    description: "Join the Foo 118 conference".to_owned(),
                    filename: "".to_owned(),
                },
            },
        };

        let mut result = Vec::new();
        ciborium::ser::into_writer(&value, &mut result).unwrap();

        // Test deserialization
        let value2 = ciborium::de::from_reader(Cursor::new(result.clone())).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex_decode(
            r#"
            87                                      # array(7)
               f6                                   # primitive(22)
               47                                   # bytes(7)
                  466f6f20313138                    # "Foo 118"
               f6                                   # primitive(22)
               f6                                   # primitive(22)
               81                                   # array(1)
                  58 20                             # bytes(32)
                     b267614d43e7676d28ef5b15e8676f23679fe365c78849d83e2ba0ae8196ec4e
               a0                                   # map(0)
               8f                                   # array(16)
                  07                                # unsigned(7)
                  60                                # text(0)
                                                    # ""
                  # TODO: RFC HAS EXPLICIT PARTINDEX: 00                                # unsigned(0)
                  02                                # unsigned(2)
                  60                                # text(0)
                                                    # ""
                  78 1e                             # text(30)
                     68747470733a2f2f6578616d706c652e636f6d2f6a6f696e2f3132333435
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
                     4a6f696e2074686520466f6f2031313820636f6e666572656e6365
                     # "Join the Foo 118 conference"
                  60                                # text(0)
                                                    # ""
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }

    #[test]
    fn multipart() {
        let value = MimiContent {
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: None,
            in_reply_to: None,
            last_seen: vec![],
            extensions: HashMap::new(),
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
                                content_type: "text/markdown;variant=GFM".to_owned(),
                                content: ByteBuf::from(b"# Welcome!"),
                            },
                        },
                        NestedPart {
                            disposition: Disposition::Render,
                            language: "".to_owned(),
                            part: NestedPartContent::SinglePart {
                                content_type: "application/vnd.examplevendor-fancy-im-message"
                                    .to_owned(),
                                content: ByteBuf::from(b"dc861ebaa718fd7c3ca159f71a2001"),
                            },
                        },
                    ],
                },
            },
        };

        let mut result = Vec::new();
        ciborium::ser::into_writer(&value, &mut result).unwrap();

        // Test deserialization
        let value2 = ciborium::de::from_reader(Cursor::new(result.clone())).unwrap();
        assert_eq!(value, value2);

        // There is no target in the spec
        let target = hex_decode(
            r#"
           87f640f6f680a0850160030082850160017819746578742f6d61726b646f776e3b76617269616e743d47464d4a232057656c636f6d652185016001782e6170706c69636174696f6e2f766e642e6578616d706c6576656e646f722d66616e63792d696d2d6d657373616765581e646338363165626161373138666437633363613135396637316132303031
            "#,
        );

        assert_eq!(hex::encode(result), hex::encode(target));
    }
}
