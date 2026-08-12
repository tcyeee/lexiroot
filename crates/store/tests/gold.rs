//! Scores the live segmenter against the hand-checked set in
//! `data/gold/segmentations.tsv`. Lives in `store` because it needs a loaded
//! release database, which is `store`'s job — `analyzer` never touches SQLite.

use std::path::{Path, PathBuf};

use lexiroot_core::MorphemeKind;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root should exist relative to CARGO_MANIFEST_DIR")
}

struct Case {
    word: String,
    expected: Vec<(String, MorphemeKind)>,
    is_gap: bool,
}

fn parse_gold(text: &str) -> Vec<Case> {
    let mut cases = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = line.split('\t').filter(|f| !f.trim().is_empty());
        let mut first = fields.next().expect("non-empty line has a first field");
        let is_gap = first.trim() == "GAP";
        if is_gap {
            first = fields.next().expect("GAP line has a word");
        }
        let segments = fields.next().expect("line has a segmentation field");
        let expected = segments
            .split_whitespace()
            .map(|s| {
                let (form, role) = s.split_once(':').expect("segment is form:role");
                (
                    form.to_string(),
                    MorphemeKind::parse(role).expect("role is prefix/root/suffix"),
                )
            })
            .collect();
        cases.push(Case {
            word: first.trim().to_string(),
            expected,
            is_gap,
        });
    }
    cases
}

fn actual(db: &lexiroot_analyzer::AnalyzerDb, word: &str) -> Option<Vec<(String, MorphemeKind)>> {
    let decomposition = db.analyze(word)?;
    Some(
        decomposition
            .segments
            .iter()
            .map(|s| (s.morpheme_id.as_str().to_string(), s.role))
            .collect(),
    )
}

fn render(segments: &[(String, MorphemeKind)]) -> String {
    segments
        .iter()
        .map(|(f, r)| format!("{f}:{}", r.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every non-GAP entry must segment exactly as recorded. Failures print the
/// full diff so the scoring weights can be tuned against real output rather
/// than against whichever three words happened to be tried by hand.
#[test]
fn segmenter_matches_gold_set() {
    let root = workspace_root();
    let db = lexiroot_store::load(&root.join("data/release/lexiroot-v0.1.sqlite"))
        .expect("release database should load");
    let gold = std::fs::read_to_string(root.join("data/gold/segmentations.tsv"))
        .expect("gold set should be readable");

    let mut failures = Vec::new();
    let mut checked = 0;
    for case in parse_gold(&gold).into_iter().filter(|c| !c.is_gap) {
        checked += 1;
        let got = actual(&db, &case.word);
        if got.as_deref() != Some(case.expected.as_slice()) {
            failures.push(format!(
                "  {:16} want {:44} got {}",
                case.word,
                render(&case.expected),
                got.map(|g| render(&g)).unwrap_or_else(|| "<no analysis>".into()),
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{}/{} gold segmentations wrong:\n{}",
        failures.len(),
        checked,
        failures.join("\n")
    );
}

/// GAP entries document what this pass cannot do. If one starts passing, the
/// gap closed and the entry should graduate out of the GAP section.
#[test]
fn documented_gaps_still_fail() {
    let root = workspace_root();
    let db = lexiroot_store::load(&root.join("data/release/lexiroot-v0.1.sqlite"))
        .expect("release database should load");
    let gold = std::fs::read_to_string(root.join("data/gold/segmentations.tsv"))
        .expect("gold set should be readable");

    let unexpectedly_passing: Vec<String> = parse_gold(&gold)
        .into_iter()
        .filter(|c| c.is_gap)
        .filter(|c| actual(&db, &c.word).as_deref() == Some(c.expected.as_slice()))
        .map(|c| c.word)
        .collect();

    assert!(
        unexpectedly_passing.is_empty(),
        "these GAP entries now segment correctly and should be promoted out of the GAP section: {}",
        unexpectedly_passing.join(", ")
    );
}
