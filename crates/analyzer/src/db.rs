use std::collections::HashMap;

use lexiroot_core::{Morpheme, MorphemeId, Provenance, SourceId, WordDecomposition};

use crate::index::MorphemeIndex;
use crate::segment::segment_ranked;

pub struct RootInfo<'a> {
    pub morpheme: &'a Morpheme,
    pub related_words: Vec<&'a str>,
}

/// The full in-memory query surface: a `MorphemeIndex` for live algorithmic
/// segmentation plus the precomputed source-listed decomposition table.
/// Owns no SQLite handle — callers (currently `cli`) are responsible for
/// loading rows from the release database and handing them here.
pub struct AnalyzerDb {
    index: MorphemeIndex,
    precomputed: HashMap<String, WordDecomposition>,
}

impl AnalyzerDb {
    pub fn new(morphemes: Vec<Morpheme>, precomputed_words: Vec<WordDecomposition>) -> Self {
        let precomputed = precomputed_words
            .into_iter()
            .map(|d| (d.word.to_lowercase(), d))
            .collect();
        Self {
            index: MorphemeIndex::build(morphemes),
            precomputed,
        }
    }

    pub fn get_morpheme(&self, id: &MorphemeId) -> Option<&Morpheme> {
        self.index.get(id)
    }

    /// The loaded morpheme index, for callers that want to run segmentation
    /// directly (ranked candidates, scoring diagnostics) rather than through
    /// `analyze`'s precomputed-first path.
    pub fn index(&self) -> &MorphemeIndex {
        &self.index
    }

    /// Tier 1: exact hit in the precomputed (source-listed) table.
    /// Tier 2: live algorithmic fallback, tagged `Inferred` with lower
    /// confidence and evidence naming the matched affixes.
    /// Tier 3: no match at either tier -> `None`.
    pub fn analyze(&self, word: &str) -> Option<WordDecomposition> {
        let word_lc = word.to_lowercase();
        if let Some(d) = self.precomputed.get(&word_lc) {
            return Some(d.clone());
        }

        let parse = segment_ranked(word, &self.index, 1).into_iter().next()?;
        let forms: Vec<&str> = parse
            .segments
            .iter()
            .filter_map(|s| self.index.get(&s.morpheme_id))
            .map(|m| m.form.as_str())
            .collect();
        // Naming the rule matters here: the forms below are canonical, so
        // without it the evidence claims a root the word does not visibly
        // contain (`believe` for `unbelievable`) and reads like a bug.
        let spelling = match parse.root_rule {
            Some(rule) => format!(", after reversing {} at the root boundary", rule.as_str()),
            None => String::new(),
        };
        let evidence = format!(
            "inferred by stripping known affixes around a matched root: {}{spelling}",
            forms.join(" + ")
        );
        let provenance = Provenance::new(SourceId::Inferred, 0.5, evidence)
            .expect("0.5 is always a valid confidence value");

        Some(WordDecomposition {
            word: word.to_string(),
            segments: parse.segments,
            provenance,
        })
    }

    /// Looks up a `Morpheme` by exact root form and returns it plus every
    /// precomputed word that references it. Limited to whatever the source
    /// datasets happen to list as examples for that root — not a live
    /// segmentation over arbitrary words.
    pub fn root(&self, text: &str) -> Option<RootInfo<'_>> {
        let text_lc = text.to_lowercase();
        let root_id = self.index.root_id(&text_lc)?;
        let morpheme = self.index.get(&root_id)?;

        let mut related_words: Vec<&str> = self
            .precomputed
            .values()
            .filter(|d| d.segments.iter().any(|s| s.morpheme_id == root_id))
            .map(|d| d.word.as_str())
            .collect();
        related_words.sort_unstable();

        Some(RootInfo {
            morpheme,
            related_words,
        })
    }

    /// Groups precomputed `WordDecomposition`s by shared root morpheme.
    pub fn family(&self, text: &str) -> Vec<&WordDecomposition> {
        let text_lc = text.to_lowercase();
        let Some(root_id) = self.index.root_id(&text_lc) else {
            return Vec::new();
        };

        let mut words: Vec<&WordDecomposition> = self
            .precomputed
            .values()
            .filter(|d| d.segments.iter().any(|s| s.morpheme_id == root_id))
            .collect();
        words.sort_unstable_by(|a, b| a.word.cmp(&b.word));
        words
    }
}
