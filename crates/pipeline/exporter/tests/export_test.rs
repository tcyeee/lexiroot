use lexiroot_pipeline_exporter::export;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

fn build_processed_fixture(path: &std::path::Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "
        CREATE TABLE morphemes (
            id TEXT PRIMARY KEY, form TEXT NOT NULL, kind TEXT NOT NULL,
            meanings TEXT NOT NULL, source TEXT NOT NULL, confidence REAL NOT NULL, evidence TEXT NOT NULL
        );
        CREATE TABLE word_decompositions (
            word TEXT PRIMARY KEY, source TEXT NOT NULL, confidence REAL NOT NULL, evidence TEXT NOT NULL
        );
        CREATE TABLE word_decomposition_segments (
            word TEXT NOT NULL, position INTEGER NOT NULL, morpheme_id TEXT NOT NULL, PRIMARY KEY (word, position)
        );
        ",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO morphemes VALUES ('prefix:in','in','prefix','[\"into\"]','colingoldberg_morphemes',0.95,'fixture')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO morphemes VALUES ('root:spect','spect','root','[\"look\",\"see\"]','colingoldberg_morphemes',0.95,'fixture')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO word_decompositions VALUES ('inspect','colingoldberg_morphemes',0.95,'fixture')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO word_decomposition_segments VALUES ('inspect', 0, 'prefix:in')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO word_decomposition_segments VALUES ('inspect', 1, 'root:spect')",
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
    let input = dir.path().join("processed.sqlite");
    build_processed_fixture(&input);

    let output_a = dir.path().join("release-a.sqlite");
    let output_b = dir.path().join("release-b.sqlite");
    export(&input, &output_a).unwrap();
    export(&input, &output_b).unwrap();

    assert_eq!(sha256_of(&output_a), sha256_of(&output_b));
}

#[test]
fn export_preserves_row_contents() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("processed.sqlite");
    build_processed_fixture(&input);

    let output = dir.path().join("release.sqlite");
    export(&input, &output).unwrap();

    let conn = Connection::open(&output).unwrap();
    let morpheme_count: i64 = conn.query_row("SELECT COUNT(*) FROM morphemes", [], |r| r.get(0)).unwrap();
    assert_eq!(morpheme_count, 2);
    let word: String = conn
        .query_row("SELECT word FROM word_decompositions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(word, "inspect");
    let segment_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM word_decomposition_segments WHERE word = 'inspect'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(segment_count, 2);
}
