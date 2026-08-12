use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Result};
use lexiroot_analyzer::AnalyzerDb;
use lexiroot_core::{
    Morpheme, MorphemeKind, MorphemePositions, MorphemeRef, Provenance, SourceId, WordDecomposition,
};
use rusqlite::Connection;

/// Loads every row from a release SQLite file into memory and builds an
/// `AnalyzerDb`. `analyzer` itself never touches SQLite (see the design
/// plan) — this is where that boundary lives. Shared by every host that
/// needs a loaded database (`cli`, `web`), so the read path exists once.
pub fn load(path: &Path) -> Result<AnalyzerDb> {
    let conn = Connection::open(path)?;

    let mut morphemes = Vec::new();
    let mut stmt =
        conn.prepare("SELECT form, positions, meanings, source, confidence, evidence FROM morphemes")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, f64>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;
    for row in rows {
        let (form, positions, meanings_json, source, confidence, evidence) = row?;
        let positions = MorphemePositions::parse(&positions)?;
        let source = SourceId::parse(&source).ok_or_else(|| anyhow!("unknown source id '{source}'"))?;
        let meanings: Vec<String> = serde_json::from_str(&meanings_json)?;
        let provenance = Provenance::new(source, confidence as f32, evidence)?;
        morphemes.push(Morpheme::new(form, positions, meanings, provenance));
    }

    let mut segments_by_word: HashMap<String, Vec<MorphemeRef>> = HashMap::new();
    let mut stmt =
        conn.prepare("SELECT word, morpheme_id, role FROM word_decomposition_segments ORDER BY word, position")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (word, morpheme_id, role) = row?;
        let role = MorphemeKind::parse(&role).ok_or_else(|| anyhow!("unknown segment role '{role}'"))?;
        segments_by_word
            .entry(word)
            .or_default()
            .push(MorphemeRef {
                morpheme_id: lexiroot_core::MorphemeId(morpheme_id),
                role,
            });
    }

    let mut decompositions = Vec::new();
    let mut stmt = conn.prepare("SELECT word, source, confidence, evidence FROM word_decompositions")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (word, source, confidence, evidence) = row?;
        let source = SourceId::parse(&source).ok_or_else(|| anyhow!("unknown source id '{source}'"))?;
        let provenance = Provenance::new(source, confidence as f32, evidence)?;
        let segments = segments_by_word.remove(&word).unwrap_or_default();
        decompositions.push(WordDecomposition {
            word,
            segments,
            provenance,
        });
    }

    Ok(AnalyzerDb::new(morphemes, decompositions))
}
