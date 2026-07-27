use std::collections::BTreeMap;

use anyhow::Result;
use lexiroot_core::MorphemeKind;
use serde::Deserialize;

use crate::parsed::ParsedMorpheme;

#[derive(Deserialize)]
struct RawEntry {
    #[serde(default)]
    meanings: Vec<String>,
    #[serde(default)]
    examples: Vec<String>,
}

/// Many example strings carry a trailing grade-level tag, e.g.
/// `"abdicate - hs"` or `"admission - 6"`. Strip it if present: split on the
/// last `" - "` and keep the prefix only when the trailing token is short
/// and alphanumeric (a real grade tag), so words that legitimately contain
/// `" - "` are left untouched.
fn strip_grade_tag(example: &str) -> &str {
    if let Some(idx) = example.rfind(" - ") {
        let (word, rest) = example.split_at(idx);
        let tag = &rest[3..];
        if !tag.is_empty() && tag.len() <= 4 && tag.chars().all(|c| c.is_ascii_alphanumeric()) {
            return word;
        }
    }
    example
}

/// Parses WithEnglishWeCan/generated-english-roots-list's
/// `english.roots.list.build.json`. Every entry is a root — the map key
/// is the root form itself. Some `meanings` entries carry a parenthetical
/// "(prefix)" etymological note; that's not a structural kind marker, so
/// everything here is still classified as `MorphemeKind::Root`.
pub fn parse(json: &str) -> Result<Vec<ParsedMorpheme>> {
    let raw: BTreeMap<String, RawEntry> = serde_json::from_str(json)?;
    let mut out = Vec::with_capacity(raw.len());

    for (form, entry) in raw {
        if form.is_empty() {
            continue;
        }
        let examples = entry.examples.iter().map(|e| strip_grade_tag(e).to_string()).collect();
        out.push(ParsedMorpheme {
            evidence: format!("withenglishwecan/generated-english-roots-list entry '{form}'"),
            form,
            kind: MorphemeKind::Root,
            meanings: entry.meanings,
            examples,
        });
    }

    Ok(out)
}
