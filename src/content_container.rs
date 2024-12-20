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
    expires: u32,
    in_reply_to: Option<InReplyTo>,
    last_seen: Vec<ByteBuf>,
    extensions: HashMap<String, ByteBuf>, // TODO: Enforce max sizes
    nested_part: NestedPart,
    // TODO: Wrapper struct for MessageDerivedValues, like messageId, roomUrl, hubAcceptedTimestamp?
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
    part_index: u16,  // TODO: Why is this needed?
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url(String);

impl Serialize for Url {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ciborium::Value::Tag(32, Box::new(ciborium::Value::Text(self.0.clone())))
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Url {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = ciborium::Value::deserialize(deserializer)?;
        if let ciborium::Value::Tag(32, v) = value {
            if let ciborium::Value::Text(url) = *v {
                Ok(Self(url))
            } else {
                Err(de::Error::invalid_type(
                    de::Unexpected::StructVariant,
                    &"Url must be a string",
                ))
            }
        } else {
            Err(de::Error::invalid_type(
                de::Unexpected::StructVariant,
                &"Url must have tag 32",
            ))
        }
    }
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
        url: Url,
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

    #[test]
    fn original_message() {
        let value = MimiContent {
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: 0,
            in_reply_to: None,
            last_seen: vec![],
            extensions: HashMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part_index: 0,
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

        // Taken from MIMI content format draft
        let target = hex::decode("87f64000f680a08601600001781b746578742f6d61726b646f776e3b636861727365743d7574662d38583848692065766572796f6e652c207765206a75737420736869707065642072656c6561736520322e302e205f5f476f6f6420776f726b5f5f21").unwrap();

        assert_eq!(result, target);
    }

    #[test]
    fn reply() {
        let value = MimiContent {
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: 0,
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
                part_index: 0,
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
        let target = hex::decode("87f64000835820d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e515010158206b44053cb68e3f0cdd219da8d7104afc2ae5ffff782154524cef093de39345a5815820d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e51501a08601600001781b746578742f6d61726b646f776e3b636861727365743d7574662d3858215269676874206f6e21205f436f6e67726174756c6174696f6e735f2027616c6c21").unwrap();

        assert_eq!(result, target);
    }

    #[test]
    fn reaction() {
        let value = MimiContent {
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: 0,
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
                part_index: 0,
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
        let target = hex::decode("87f64000835820d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e515010158206b44053cb68e3f0cdd219da8d7104afc2ae5ffff782154524cef093de39345a5815820e701beee59f9376282f39092e1041b2ac2e3aad1776570c1a28de244979c71eda086026000017818746578742f706c61696e3b636861727365743d7574662d3843e299a5").unwrap();

        assert_eq!(result, target);
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
            expires: 0,
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
                part_index: 0,
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
        let target = hex::decode("875820e701beee59f9376282f39092e1041b2ac2e3aad1776570c1a28de244979c71ed4000835820d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e515010158206b44053cb68e3f0cdd219da8d7104afc2ae5ffff782154524cef093de39345a58258204dcab7711a77ea1dd025a6a1a7fe01ab3b0d690f82417663cb752dfcc37779a158206b50bfdd71edc83554ae21380080f4a3ba77985da34528a515fac3c38e4998b8a08601600001781b746578742f6d61726b646f776e3b636861727365743d7574662d3858225269676874206f6e21205f436f6e67726174756c6174696f6e735f207927616c6c21").unwrap();

        assert_eq!(result, target);
    }

    #[test]
    fn delete() {
        let value = MimiContent {
            replaces: Some(
                hex::decode(b"e701beee59f9376282f39092e1041b2ac2e3aad1776570c1a28de244979c71ed")
                    .unwrap()
                    .into(),
            ),
            topic_id: ByteBuf::from(b""),
            expires: 0,
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
                "89d3472622a40d6ceeb27c42490fdc64c0e9c20c598f9d7c8e81640dae8db0fb", // The content draft has a wrong id here, it doesn't end on 0f
            )
            .unwrap()
            .into()],
            extensions: HashMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Reaction, // The draft writes Render, but uses the number for Reaction
                language: "".to_owned(),
                part_index: 0,
                part: NestedPartContent::NullPart,
            },
        };

        let mut result = Vec::new();
        ciborium::ser::into_writer(&value, &mut result).unwrap();

        // Test deserialization
        let value2 = ciborium::de::from_reader(Cursor::new(result.clone())).unwrap();
        assert_eq!(value, value2);

        // Taken from MIMI content format draft
        let target = hex::decode("875820e701beee59f9376282f39092e1041b2ac2e3aad1776570c1a28de244979c71ed4000835820d3c14744d1791d02548232c23d35efa97668174ba385af066011e43bd7e515010158206b44053cb68e3f0cdd219da8d7104afc2ae5ffff782154524cef093de39345a581582089d3472622a40d6ceeb27c42490fdc64c0e9c20c598f9d7c8e81640dae8db0fba08402600000").unwrap();

        assert_eq!(result, target);
    }

    #[test]
    fn expiring() {
        let value = MimiContent {
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: 1644390004,
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
                part_index: 0,
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
        let target = hex::decode("87f6401a62036674f68158201a771ca1d84f8fda4184a1e02a549e201bf434c6bfcf1237fa45463c6861853ba08601600001781b746578742f6d61726b646f776e3b636861727365743d7574662d3858505f5f2a56504e20474f494e4720444f574e2a5f5f0a49276d207265626f6f74696e67207468652056504e20696e2074656e206d696e7574657320756e6c65737320616e796f6e65206f626a656374732e").unwrap();

        assert_eq!(result, target);
    }

    #[test]
    fn attachments() {
        let value = MimiContent {
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: 0,
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
                part_index: 0,
                part: NestedPartContent::ExternalPart {
                    content_type: "video/mp4".to_owned(),
                    url: Url("https://example.com/storage/8ksB4bSrrRE.mp4".to_owned()),
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
        let target = hex::decode("87f64000f68158205c95a4dfddab84348bcc265a479299fbd3a2eecfa3d490985da5113e5480c7f1a0900662656e000269766964656f2f6d7034d820782b68747470733a2f2f6578616d706c652e636f6d2f73746f726167652f386b7342346253727252452e6d7034001a2a36ced1015021399320958a6f4c745dde670d95e0d84cc86cf2c33f21527d1dd76f5b400158209ab17a8cf0890baaae7ee016c7312fcc080ba46498389458ee44f0276e783163781c3220686f757273206f66206b6579207369676e696e6720766964656f6b62696766696c652e6d7034").unwrap();
        dbg!(hex::encode(&result));
        dbg!(hex::encode(&target));

        assert_eq!(result, target);
    }

    #[test]
    fn conferencing() {
        let value = MimiContent {
            replaces: None,
            topic_id: ByteBuf::from(b"Foo 118"),
            expires: 0,
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
                part_index: 0,
                part: NestedPartContent::ExternalPart {
                    content_type: "".to_owned(),
                    url: Url("https://example.com/join/12345".to_owned()),
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
        let target = hex::decode("87f647466f6f2031313800f6815820b267614d43e7676d28ef5b15e8676f23679fe365c78849d83e2ba0ae8196ec4ea0900760000260d820781e68747470733a2f2f6578616d706c652e636f6d2f6a6f696e2f31323334350000004040400040781b4a6f696e2074686520466f6f2031313820636f6e666572656e636560").unwrap();

        assert_eq!(result, target);
    }

    #[test]
    fn multipart() {
        let value = MimiContent {
            replaces: None,
            topic_id: ByteBuf::from(b""),
            expires: 0,
            in_reply_to: None,
            last_seen: vec![],
            extensions: HashMap::new(),
            nested_part: NestedPart {
                disposition: Disposition::Render,
                language: "".to_owned(),
                part_index: 0,
                part: NestedPartContent::MultiPart {
                    part_semantics: PartSemantics::ChooseOne,
                    parts: vec![
                        NestedPart {
                            disposition: Disposition::Render,
                            language: "".to_owned(),
                            part_index: 1,
                            part: NestedPartContent::SinglePart {
                                content_type: "text/markdown;variant=GFM".to_owned(),
                                content: ByteBuf::from(b"# Welcome!"),
                            },
                        },
                        NestedPart {
                            disposition: Disposition::Render,
                            language: "".to_owned(),
                            part_index: 2, // Mimi content format draft has a wrong comment here
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
        let target = hex::decode("87f64000f680a08601600003008286016001017819746578742f6d61726b646f776e3b76617269616e743d47464d4a232057656c636f6d65218601600201782e6170706c69636174696f6e2f766e642e6578616d706c6576656e646f722d66616e63792d696d2d6d657373616765581e646338363165626161373138666437633363613135396637316132303031").unwrap();

        assert_eq!(result, target);
    }
}
