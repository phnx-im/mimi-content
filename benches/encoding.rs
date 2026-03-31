// SPDX-FileCopyrightText: 2026 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mimi_content::{
    cbor,
    content_container::{Disposition, ExtensionName, MimiContent, NestedPart, PartSemantics},
};

use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::BTreeMap;

fn extensions_alice() -> BTreeMap<ExtensionName, cbor::Value> {
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
}

fn multipart() -> MimiContent {
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

fn criterion_benchmark(c: &mut Criterion) {
    let multipart = multipart();

    c.bench_function("encoding", |b| {
        b.iter(|| {
            let bytes = multipart.serialize().unwrap();
            let _new_content = MimiContent::deserialize(&bytes).unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
