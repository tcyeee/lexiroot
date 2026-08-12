use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection};

const SCHEMA: &str = "
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

/// Reads `input_path` (processed.sqlite) and writes a deterministic release
/// SQLite file to `output_path`: fresh file, rows re-inserted in a stable
/// `ORDER BY`, one transaction, then `VACUUM` for a canonical page layout.
/// Given the same logical input, this always produces byte-identical
/// output — the README's "Immutable Releases" reproducibility promise.
pub fn export(input_path: &Path, output_path: &Path) -> Result<()> {
    let input = Connection::open(input_path)?;

    if output_path.exists() {
        std::fs::remove_file(output_path)?;
    }
    let mut output = Connection::open(output_path)?;
    output.execute_batch(SCHEMA)?;

    let tx = output.transaction()?;
    {
        let mut select_morphemes =
            input.prepare("SELECT id, form, positions, meanings, source, confidence, evidence FROM morphemes ORDER BY id")?;
        let mut insert_morpheme = tx.prepare(
            "INSERT INTO morphemes (id, form, positions, meanings, source, confidence, evidence) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        let rows = select_morphemes.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, f64>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        for row in rows {
            let (id, form, positions, meanings, source, confidence, evidence) = row?;
            insert_morpheme.execute(params![id, form, positions, meanings, source, confidence, evidence])?;
        }

        let mut select_words =
            input.prepare("SELECT word, source, confidence, evidence FROM word_decompositions ORDER BY word")?;
        let mut insert_word = tx.prepare(
            "INSERT INTO word_decompositions (word, source, confidence, evidence) VALUES (?1, ?2, ?3, ?4)",
        )?;
        let rows = select_words.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (word, source, confidence, evidence) = row?;
            insert_word.execute(params![word, source, confidence, evidence])?;
        }

        let mut select_segments = input
            .prepare("SELECT word, position, morpheme_id, role FROM word_decomposition_segments ORDER BY word, position")?;
        let mut insert_segment = tx.prepare(
            "INSERT INTO word_decomposition_segments (word, position, morpheme_id, role) VALUES (?1, ?2, ?3, ?4)",
        )?;
        let rows = select_segments.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (word, position, morpheme_id, role) = row?;
            insert_segment.execute(params![word, position, morpheme_id, role])?;
        }
    }
    tx.commit()?;
    output.execute_batch("VACUUM;")?;

    Ok(())
}
