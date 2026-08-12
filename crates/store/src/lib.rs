use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Result};
use lexiroot_analyzer::AnalyzerDb;
use lexiroot_core::{
    Morpheme, MorphemeKind, MorphemePositions, MorphemeRef, Provenance, WordDecomposition,
    META_SCHEMA_VERSION, RELEASE_SCHEMA_VERSION,
};
use rusqlite::Connection;

/// Refuses a database this build cannot read.
///
/// The release file used to carry its version only in its *name*, so opening
/// a stale one — left behind by a version bump some caller missed — looked
/// exactly like opening the current one, and the queries just answered from
/// old data. The version now lives in the file, and a mismatch stops here.
fn check_schema_version(conn: &Connection) -> Result<()> {
    let found: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [META_SCHEMA_VERSION],
            |row| row.get(0),
        )
        .ok();

    match found {
        Some(v) if v == RELEASE_SCHEMA_VERSION.to_string() => Ok(()),
        Some(v) => Err(anyhow!(
            "release database has schema version {v}, this build reads {RELEASE_SCHEMA_VERSION}; \
             re-export it with `cargo run -p lexiroot-pipeline-exporter`"
        )),
        None => Err(anyhow!(
            "release database carries no schema version (predates the `meta` table); \
             re-export it with `cargo run -p lexiroot-pipeline-exporter`"
        )),
    }
}

/// Loads every row from a release SQLite file into memory and builds an
/// `AnalyzerDb`. `analyzer` itself never touches SQLite (see the design
/// plan) — this is where that boundary lives. Shared by every host that
/// needs a loaded database (`cli`, `web`), so the read path exists once.
pub fn load(path: &Path) -> Result<AnalyzerDb> {
    let conn = Connection::open(path)?;
    check_schema_version(&conn)?;

    let mut morphemes = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT form, positions, meanings, variants, confidence, evidence FROM morphemes",
    )?;
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
        let (form, positions, meanings_json, variants_json, confidence, evidence) = row?;
        let positions = MorphemePositions::parse(&positions)?;
        let meanings: Vec<String> = serde_json::from_str(&meanings_json)?;
        let variants: Vec<String> = serde_json::from_str(&variants_json)?;
        let provenance = Provenance::new(confidence as f32, evidence)?;
        morphemes.push(Morpheme::with_variants(form, positions, meanings, variants, provenance));
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
    let mut stmt = conn.prepare("SELECT word, confidence, evidence FROM word_decompositions")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (word, confidence, evidence) = row?;
        let provenance = Provenance::new(confidence as f32, evidence)?;
        let segments = segments_by_word.remove(&word).unwrap_or_default();
        decompositions.push(WordDecomposition {
            word,
            segments,
            provenance,
        });
    }

    Ok(AnalyzerDb::new(morphemes, decompositions))
}
