use std::path::Path;

use anyhow::Result;
use lexiroot_core::{Morpheme, WordDecomposition};
use rusqlite::Connection;

pub const SCHEMA: &str = "
CREATE TABLE morphemes (
    id TEXT PRIMARY KEY,
    form TEXT NOT NULL,
    positions TEXT NOT NULL,
    meanings TEXT NOT NULL,
    source TEXT NOT NULL,
    confidence REAL NOT NULL,
    evidence TEXT NOT NULL
);
CREATE TABLE word_decompositions (
    word TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    confidence REAL NOT NULL,
    evidence TEXT NOT NULL
);
CREATE TABLE word_decomposition_segments (
    word TEXT NOT NULL REFERENCES word_decompositions(word),
    position INTEGER NOT NULL,
    morpheme_id TEXT NOT NULL REFERENCES morphemes(id),
    role TEXT NOT NULL CHECK (role IN ('prefix','root','suffix')),
    PRIMARY KEY (word, position)
);
";

/// Writes a fresh `processed.sqlite`: morphemes and word decompositions in
/// the deterministic (already sorted) order the caller provides, inside a
/// single transaction.
pub fn write_processed_db(path: &Path, morphemes: &[Morpheme], decompositions: &[WordDecomposition]) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let mut conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA)?;

    let tx = conn.transaction()?;
    {
        let mut insert_morpheme = tx.prepare(
            "INSERT INTO morphemes (id, form, positions, meanings, source, confidence, evidence) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for m in morphemes {
            insert_morpheme.execute(rusqlite::params![
                m.id.as_str(),
                m.form,
                m.positions.to_storage_string(),
                serde_json::to_string(&m.meanings)?,
                m.provenance.source.as_str(),
                m.provenance.confidence(),
                m.provenance.evidence,
            ])?;
        }

        let mut insert_word = tx.prepare(
            "INSERT INTO word_decompositions (word, source, confidence, evidence) VALUES (?1, ?2, ?3, ?4)",
        )?;
        let mut insert_segment = tx.prepare(
            "INSERT INTO word_decomposition_segments (word, position, morpheme_id, role) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for d in decompositions {
            insert_word.execute(rusqlite::params![
                d.word,
                d.provenance.source.as_str(),
                d.provenance.confidence(),
                d.provenance.evidence,
            ])?;
            for (position, seg) in d.segments.iter().enumerate() {
                insert_segment.execute(rusqlite::params![
                    d.word,
                    position as i64,
                    seg.morpheme_id.as_str(),
                    seg.role.as_str(),
                ])?;
            }
        }
    }
    tx.commit()?;

    Ok(())
}
