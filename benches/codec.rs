// SPDX-FileCopyrightText: 2026 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use criterion::{criterion_group, criterion_main, Criterion};
use std::{collections::BTreeMap, hint::black_box, io::Cursor};

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

fn large_mimi_content() -> mimi_content::MimiContent {
    use mimi_content::content_container::{Disposition, MimiContent, NestedPart, PartSemantics};

    let parts = (0..13_000)
        .map(|i| NestedPart::SinglePart {
            disposition: Disposition::Render,
            language: "".to_owned(),
            content_type: "text/markdown;variant=GFM-MIMI".to_owned(),
            content: format!("Message number {i}: hello world!").into_bytes(),
        })
        .collect();

    MimiContent {
        salt: hex::decode("261c953e178af653fe3d42641b91d814").unwrap(),
        replaces: None,
        topic_id: b"".to_vec(),
        expires: None,
        in_reply_to: None,
        extensions: Default::default(),
        nested_part: NestedPart::MultiPart {
            disposition: Disposition::Render,
            language: "".to_owned(),
            part_semantics: PartSemantics::ChooseOne,
            parts,
        },
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let content = minicbor_mimi_content();
    let minicbor_content_bytes = content.serialize().unwrap();

    let mut ciborium_multipart_bytes = Vec::new();
    ciborium::into_writer(&content, &mut ciborium_multipart_bytes).unwrap();

    let mut encode = c.benchmark_group("encode");
    encode.bench_function("minicbor-encode", |b| {
        b.iter(|| {
            black_box(content.serialize().unwrap());
        });
    });
    encode.bench_function("minicbor-serde-encode", |b| {
        b.iter(|| {
            black_box(minicbor_serde::to_vec(&content).unwrap());
        });
    });
    encode.bench_function("ciborium-encode", |b| {
        b.iter(|| {
            let mut ciborium_bytes = Vec::new();
            ciborium::into_writer(&content, &mut ciborium_bytes).unwrap();
            black_box(ciborium_bytes);
        })
    });
    encode.finish();

    let large_content = large_mimi_content();
    let large_minicbor_bytes = large_content.serialize().unwrap();
    let mut large_ciborium_bytes = Vec::new();
    ciborium::into_writer(&large_content, &mut large_ciborium_bytes).unwrap();

    let mut encode_large = c.benchmark_group("encode-large");
    encode_large.bench_function("minicbor-encode", |b| {
        b.iter(|| black_box(large_content.serialize().unwrap()));
    });
    encode_large.bench_function("minicbor-serde-encode", |b| {
        b.iter(|| black_box(minicbor_serde::to_vec(&large_content).unwrap()));
    });
    encode_large.bench_function("ciborium-encode", |b| {
        b.iter(|| {
            let mut bytes = Vec::new();
            ciborium::into_writer(&large_content, &mut bytes).unwrap();
            black_box(bytes);
        });
    });
    encode_large.finish();

    let mut decode_large = c.benchmark_group("decode-large");
    decode_large.bench_function("minicbor-decode", |b| {
        b.iter(|| black_box(mimi_content::MimiContent::deserialize(&large_minicbor_bytes).unwrap()));
    });
    decode_large.bench_function("minicbor-serde-decode", |b| {
        b.iter(|| {
            black_box(
                minicbor_serde::from_slice::<mimi_content::MimiContent>(&large_minicbor_bytes)
                    .unwrap(),
            )
        });
    });
    decode_large.bench_function("ciborium-decode", |b| {
        b.iter(|| {
            black_box(
                ciborium::from_reader::<mimi_content::MimiContent, _>(Cursor::new(
                    &large_ciborium_bytes,
                ))
                .unwrap(),
            )
        });
    });
    decode_large.finish();

    let mut decode = c.benchmark_group("decode");
    decode.bench_function("minicbor-decode", |b| {
        b.iter(|| {
            black_box(mimi_content::MimiContent::deserialize(&minicbor_content_bytes).unwrap());
        });
    });
    decode.bench_function("minicbor-serde-decode", |b| {
        b.iter(|| {
            black_box(
                minicbor_serde::from_slice::<mimi_content::MimiContent>(&minicbor_content_bytes)
                    .unwrap(),
            );
        });
    });
    decode.bench_function("ciborium-decode", |b| {
        b.iter(|| {
            black_box(
                ciborium::from_reader::<mimi_content::MimiContent, _>(Cursor::new(
                    &ciborium_multipart_bytes,
                ))
                .unwrap(),
            );
        })
    });
    decode.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
