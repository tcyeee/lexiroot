use lexiroot_analyzer::{AnalyzerDb, RootInfo};
use lexiroot_core::{Morpheme, MorphemeKind, WordDecomposition};

fn meanings_line(m: &Morpheme) -> String {
    if m.meanings.is_empty() {
        "(no recorded meaning)".to_string()
    } else {
        m.meanings.join(", ")
    }
}

pub fn format_analysis(db: &AnalyzerDb, decomposition: &WordDecomposition) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Word: {}  (confidence {:.2}, source: {})\n",
        decomposition.word,
        decomposition.provenance.confidence(),
        decomposition.provenance.source.as_str()
    ));

    for kind in [MorphemeKind::Prefix, MorphemeKind::Root, MorphemeKind::Suffix] {
        let label = match kind {
            MorphemeKind::Prefix => "Prefix",
            MorphemeKind::Root => "Root",
            MorphemeKind::Suffix => "Suffix",
        };
        for seg in &decomposition.segments {
            let Some(m) = db.get_morpheme(&seg.morpheme_id) else {
                continue;
            };
            if m.kind != kind {
                continue;
            }
            out.push_str(&format!("\n{label}:\n  {} = {}\n", m.form, meanings_line(m)));
        }
    }

    out.push_str(&format!("\nEvidence: {}\n", decomposition.provenance.evidence));
    out
}

pub fn format_root(text: &str, info: &RootInfo<'_>) -> String {
    let mut out = String::new();
    out.push_str(&format!("{text}\n\nMeaning:\n"));
    for meaning in &info.morpheme.meanings {
        out.push_str(&format!("  {meaning}\n"));
    }

    out.push_str("\nRelated words:\n\n");
    if info.related_words.is_empty() {
        out.push_str("  (none in this pass's dataset)\n");
    } else {
        for word in &info.related_words {
            out.push_str(&format!("{word}\n"));
        }
    }
    out
}

pub fn format_family(text: &str, words: &[&WordDecomposition]) -> String {
    let mut out = String::new();
    out.push_str(&format!("{text}\n"));
    if words.is_empty() {
        out.push_str("  (no known words for this root in this pass's dataset)\n");
        return out;
    }
    for (i, d) in words.iter().enumerate() {
        let branch = if i + 1 == words.len() { "└── " } else { "├── " };
        out.push_str(&format!("{branch}{}\n", d.word));
    }
    out
}
