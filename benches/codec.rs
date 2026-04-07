// SPDX-FileCopyrightText: 2026 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::BTreeMap;

fn minicbor_mimi_content() -> mimi_content::MimiContent {
    use mimi_content::content_container::{
        Disposition, ExtensionName, MimiContent, NestedPart, PartSemantics,
    };

    let extensions_alice = || {
        let mut extensions = BTreeMap::new();
        extensions.insert(
            ExtensionName::Number(1),
            "mimi://example.com/u/alice-smith".into(),
        );
        extensions.insert(
            ExtensionName::Number(2),
            "mimi://example.com/r/engineering_team".into(),
        );

        extensions
    };

    MimiContent {
        salt: hex::decode("261c953e178af653fe3d42641b91d814").unwrap(),
        replaces: None,
        topic_id: b"".to_vec(),
        expires: None,
        in_reply_to: None,
        extensions: extensions_alice(),
        nested_part: NestedPart::MultiPart {
            disposition: Disposition::Render,
            language: "".to_owned(),
            part_semantics: PartSemantics::ChooseOne,
            parts: vec![
                NestedPart::SinglePart {
                    disposition: Disposition::Render,
                    language: "".to_owned(),
                    content_type: "text/markdown;variant=GFM-MIMI".to_owned(),
                    content: b"# Welcome!".to_vec(),
                },
                NestedPart::SinglePart {
                    disposition: Disposition::Render,
                    language: "".to_owned(),
                    content_type: "application/vnd.examplevendor-fancy-im-message".to_owned(),
                    content: hex::decode("dc861ebaa718fd7c3ca159f71a2001").unwrap(),
                },
            ],
        },
    }
}

fn ciborium_mimi_content() -> mimi_content_ciborium::MimiContent {
    use mimi_content_ciborium::{
        content_container::{
            Disposition, ExtensionName, MimiContent, NestedPart, NestedPartContent, PartSemantics,
        },
        ByteBuf,
    };

    let extensions_alice = || {
        let mut extensions = BTreeMap::new();
        extensions.insert(
            ExtensionName::Number(1.into()),
            "mimi://example.com/u/alice-smith".into(),
        );
        extensions.insert(
            ExtensionName::Number(2.into()),
            "mimi://example.com/r/engineering_team".into(),
        );

        extensions
    };

    MimiContent {
        salt: ByteBuf::from(hex::decode("261c953e178af653fe3d42641b91d814").unwrap()),
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
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let minicbor_multipart = minicbor_mimi_content();
    let minicbor_multipart_bytes = minicbor_multipart.serialize().unwrap();

    let ciborium_multipart = ciborium_mimi_content();
    let ciborium_multipart_bytes = ciborium_multipart.serialize().unwrap();

    let mut minicbor = c.benchmark_group("minicbor");
    minicbor.bench_function("encode", |b| {
        b.iter(|| {
            black_box(minicbor_multipart.serialize().unwrap());
        });
    });
    minicbor.bench_function("decode", |b| {
        b.iter(|| {
            black_box(mimi_content::MimiContent::deserialize(&minicbor_multipart_bytes).unwrap());
        });
    });
    minicbor.finish();

    let mut ciborium = c.benchmark_group("ciborium");
    ciborium.bench_function("ciborium-encode", |b| {
        b.iter(|| {
            black_box(ciborium_multipart.serialize().unwrap());
        })
    });

    ciborium.bench_function("ciborium-decode", |b| {
        b.iter(|| {
            black_box(
                mimi_content_ciborium::MimiContent::deserialize(&ciborium_multipart_bytes).unwrap(),
            );
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
