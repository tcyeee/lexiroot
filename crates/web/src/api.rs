//! JSON shaping for the three analyzer entry points. Mirrors what `cli`
//! prints as text, so the page and the CLI can be compared directly.

use lexiroot_analyzer::AnalyzerDb;
use lexiroot_core::{Morpheme, WordDecomposition};
use serde_json::{json, Value};

fn morpheme_json(m: &Morpheme) -> Value {
    json!({
        "id": m.id.as_str(),
        "form": m.form,
        "kind": m.kind.as_str(),
        "meanings": m.meanings,
    })
}

fn decomposition_json(db: &AnalyzerDb, d: &WordDecomposition) -> Value {
    let segments: Vec<Value> = d
        .segments
        .iter()
        .filter_map(|s| db.get_morpheme(&s.morpheme_id))
        .map(morpheme_json)
        .collect();

    json!({
        "word": d.word,
        "segments": segments,
        "confidence": d.provenance.confidence(),
        "source": d.provenance.source.as_str(),
        "evidence": d.provenance.evidence,
    })
}

pub fn analyze(db: &AnalyzerDb, word: &str) -> Option<Value> {
    let decomposition = db.analyze(word)?;
    Some(decomposition_json(db, &decomposition))
}

pub fn root(db: &AnalyzerDb, text: &str) -> Option<Value> {
    let info = db.root(text)?;
    Some(json!({
        "morpheme": morpheme_json(info.morpheme),
        "confidence": info.morpheme.provenance.confidence(),
        "source": info.morpheme.provenance.source.as_str(),
        "evidence": info.morpheme.provenance.evidence,
        "related_words": info.related_words,
    }))
}

pub fn family(db: &AnalyzerDb, text: &str) -> Value {
    let words: Vec<Value> = db
        .family(text)
        .into_iter()
        .map(|d| decomposition_json(db, d))
        .collect();
    json!({ "root": text, "words": words })
}
