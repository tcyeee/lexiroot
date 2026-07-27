use std::collections::HashMap;

use lexiroot_core::{Morpheme, MorphemeId, MorphemeKind};

/// Lookup structures built once from the full `Morpheme` table: exact-form
/// maps for prefixes/suffixes/roots (case-insensitive), used by `segment`
/// to do a greedy longest-match strip from each end of a word.
pub struct MorphemeIndex {
    by_id: HashMap<MorphemeId, Morpheme>,
    prefixes: HashMap<String, MorphemeId>,
    suffixes: HashMap<String, MorphemeId>,
    roots: HashMap<String, MorphemeId>,
    max_prefix_len: usize,
    max_suffix_len: usize,
}

impl MorphemeIndex {
    pub fn build(morphemes: Vec<Morpheme>) -> Self {
        let mut by_id = HashMap::with_capacity(morphemes.len());
        let mut prefixes = HashMap::new();
        let mut suffixes = HashMap::new();
        let mut roots = HashMap::new();
        let mut max_prefix_len = 0;
        let mut max_suffix_len = 0;

        for m in morphemes {
            let form_lc = m.form.to_lowercase();
            match m.kind {
                MorphemeKind::Prefix => {
                    max_prefix_len = max_prefix_len.max(form_lc.chars().count());
                    prefixes.insert(form_lc, m.id.clone());
                }
                MorphemeKind::Suffix => {
                    max_suffix_len = max_suffix_len.max(form_lc.chars().count());
                    suffixes.insert(form_lc, m.id.clone());
                }
                MorphemeKind::Root => {
                    roots.insert(form_lc, m.id.clone());
                }
            }
            by_id.insert(m.id.clone(), m);
        }

        Self {
            by_id,
            prefixes,
            suffixes,
            roots,
            max_prefix_len,
            max_suffix_len,
        }
    }

    pub fn get(&self, id: &MorphemeId) -> Option<&Morpheme> {
        self.by_id.get(id)
    }

    pub fn root_id(&self, form_lc: &str) -> Option<&MorphemeId> {
        self.roots.get(form_lc)
    }

    /// Longest known prefix that is a strict prefix of `word_lc` (leaves at
    /// least one character remaining for a root).
    pub fn longest_prefix(&self, word_lc: &str) -> Option<(MorphemeId, usize)> {
        let char_count = word_lc.chars().count();
        let max_len = self.max_prefix_len.min(char_count.saturating_sub(1));
        for len in (1..=max_len).rev() {
            let candidate: String = word_lc.chars().take(len).collect();
            if let Some(id) = self.prefixes.get(&candidate) {
                return Some((id.clone(), candidate.chars().count()));
            }
        }
        None
    }

    /// Longest known suffix that is a strict suffix of `word_lc`.
    pub fn longest_suffix(&self, word_lc: &str) -> Option<(MorphemeId, usize)> {
        let char_count = word_lc.chars().count();
        let max_len = self.max_suffix_len.min(char_count.saturating_sub(1));
        for len in (1..=max_len).rev() {
            let start = char_count - len;
            let candidate: String = word_lc.chars().skip(start).collect();
            if let Some(id) = self.suffixes.get(&candidate) {
                return Some((id.clone(), candidate.chars().count()));
            }
        }
        None
    }
}
