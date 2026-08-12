use lexiroot_pipeline_exporter::export;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

fn build_normalized_fixture(path: &std::path::Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "
        CREATE TABLE morphemes (
            id TEXT PRIMARY KEY, form TEXT NOT NULL, positions TEXT NOT NULL,
            meanings TEXT NOT NULL, variants TEXT NOT NULL,
            confidence REAL NOT NULL, evidence TEXT NOT NULL
        );
        CREATE TABLE word_decompositions (
            word TEXT PRIMARY KEY, confidence REAL NOT NULL, evidence TEXT NOT NULL
        );
        CREATE TABLE word_decomposition_segments (
            word TEXT NOT NULL, position INTEGER NOT NULL, morpheme_id TEXT NOT NULL,
            role TEXT NOT NULL, PRIMARY KEY (word, position)
        );
        ",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO morphemes VALUES ('in','in','prefix','[\"into\"]','[\"im\",\"il\"]',0.95,'fixture')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO morphemes VALUES ('spect','spect','root','[\"look\",\"see\"]','[]',0.95,'fixture')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO word_decompositions VALUES ('inspect',0.95,'fixture')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO word_decomposition_segments VALUES ('inspect', 0, 'in', 'prefix')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO word_decomposition_segments VALUES ('inspect', 1, 'spect', 'root')",
        [],
    )
    .unwrap();
}

fn sha256_of(path: &std::path::Path) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

#[test]
fn export_is_byte_identical_across_runs() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("normalized.sqlite");
    build_normalized_fixture(&input);

    let output_a = dir.path().join("release-a.sqlite");
    let output_b = dir.path().join("release-b.sqlite");
    export(&input, &output_a).unwrap();
    export(&input, &output_b).unwrap();

    assert_eq!(sha256_of(&output_a), sha256_of(&output_b));
}

#[test]
fn export_preserves_row_contents() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("normalized.sqlite");
    build_normalized_fixture(&input);

    let output = dir.path().join("release.sqlite");
    export(&input, &output).unwrap();

    let conn = Connection::open(&output).unwrap();
    let morpheme_count: i64 = conn.query_row("SELECT COUNT(*) FROM morphemes", [], |r| r.get(0)).unwrap();
    assert_eq!(morpheme_count, 2);
    // Allomorphs have to survive the copy: a release database that drops them
    // silently loses every word whose stem is spelled irregularly.
    let variants: String = conn
        .query_row("SELECT variants FROM morphemes WHERE id = 'in'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(variants, r#"["im","il"]"#);
    let word: String = conn
        .query_row("SELECT word FROM word_decompositions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(word, "inspect");
    // The release has to be able to identify itself: its path carries no
    // version, so an unstamped file is one `store::load` will refuse.
    let schema_version: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [lexiroot_core::META_SCHEMA_VERSION],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(schema_version, lexiroot_core::RELEASE_SCHEMA_VERSION.to_string());
    let segment_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM word_decomposition_segments WHERE word = 'inspect'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(segment_count, 2);
}
