use std::collections::BTreeMap;

use anyhow::Result;
use lexiroot_core::MorphemeKind;
use serde::{Deserialize, Deserializer};

use crate::parsed::ParsedMorpheme;

#[derive(Deserialize)]
struct RawForm {
    form: String,
    loc: String,
}

/// A handful of upstream entries have `"examples": ""` instead of an array
/// (e.g. the `capill-` entry) — treat that as no examples rather than
/// failing the whole parse.
fn examples_or_empty<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Flexible {
        List(Vec<String>),
        Other(serde::de::IgnoredAny),
    }
    Ok(match Flexible::deserialize(deserializer)? {
        Flexible::List(v) => v,
        Flexible::Other(_) => Vec::new(),
    })
}

#[derive(Deserialize)]
struct RawEntry {
    forms: Vec<RawForm>,
    #[serde(default)]
    meaning: Vec<String>,
    #[serde(default, deserialize_with = "examples_or_empty")]
    examples: Vec<String>,
}

fn map_kind(loc: &str) -> Option<MorphemeKind> {
    match loc {
        "prefix" => Some(MorphemeKind::Prefix),
        "suffix" => Some(MorphemeKind::Suffix),
        "embedded" => Some(MorphemeKind::Root),
        _ => None,
    }
}

/// Parses colingoldberg/morphemes' `morphemes.json`. Entries are processed
/// in key-sorted order so that when a single entry's `forms[]` mixes what
/// are really distinct morphemes (a real quirk of this source, e.g. `"-ably"`
/// bundling a suffix form with an unrelated `apt` root), output is
/// deterministic regardless of the source JSON's object key order.
pub fn parse(json: &str) -> Result<Vec<ParsedMorpheme>> {
    let raw: BTreeMap<String, RawEntry> = serde_json::from_str(json)?;
    let mut out = Vec::new();

    for (key, entry) in raw {
        for form in &entry.forms {
            let Some(kind) = map_kind(&form.loc) else {
                continue;
            };
            if form.form.is_empty() {
                continue;
            }
            out.push(ParsedMorpheme {
                form: form.form.clone(),
                kind,
                meanings: entry.meaning.clone(),
                evidence: format!("colingoldberg/morphemes entry '{key}'"),
                examples: entry.examples.clone(),
            });
        }
    }

    Ok(out)
}
