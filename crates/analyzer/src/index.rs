use std::collections::{HashMap, HashSet};

use lexiroot_core::{Morpheme, MorphemeId, MorphemeKind};

/// A form long enough to plausibly carry lexical content even though no
/// source ever labelled it `root`. See `RootMatch::AffixOnly`.
const MIN_FALLBACK_ROOT_LEN: usize = 4;

/// How a candidate substring matched the root slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootMatch {
    /// The form is attested as a root.
    Attested,
    /// The form is only attested as a prefix or suffix, but is long enough
    /// to stand in as a root. Upstream position labels are noisy (`work` is
    /// recorded suffix-only, `love` prefix-only), so refusing these loses
    /// real words — but a genuine root always outranks one of these, which
    /// is why `segment` scores it with a penalty rather than accepting it
    /// outright.
    AffixOnly,
}

/// Lookup structures built once from the full `Morpheme` table: position-keyed
/// form sets used by `segment` to enumerate candidate splits. Morpheme ids are
/// the lowercased forms themselves, so the sets store forms and ids are
/// derived on demand.
pub struct MorphemeIndex {
    by_id: HashMap<MorphemeId, Morpheme>,
    prefixes: HashSet<String>,
    suffixes: HashSet<String>,
    roots: HashSet<String>,
    max_prefix_len: usize,
    max_suffix_len: usize,
}

impl MorphemeIndex {
    pub fn build(morphemes: Vec<Morpheme>) -> Self {
        let mut by_id = HashMap::with_capacity(morphemes.len());
        let mut prefixes = HashSet::new();
        let mut suffixes = HashSet::new();
        let mut roots = HashSet::new();
        let mut max_prefix_len = 0;
        let mut max_suffix_len = 0;

        for m in morphemes {
            let form_lc = m.form.to_lowercase();
            let len = form_lc.chars().count();
            if m.positions.contains(MorphemeKind::Prefix) {
                max_prefix_len = max_prefix_len.max(len);
                prefixes.insert(form_lc.clone());
            }
            if m.positions.contains(MorphemeKind::Suffix) {
                max_suffix_len = max_suffix_len.max(len);
                suffixes.insert(form_lc.clone());
            }
            if m.positions.contains(MorphemeKind::Root) {
                roots.insert(form_lc);
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

    pub fn max_prefix_len(&self) -> usize {
        self.max_prefix_len
    }

    pub fn max_suffix_len(&self) -> usize {
        self.max_suffix_len
    }

    pub fn is_prefix(&self, form_lc: &str) -> bool {
        self.prefixes.contains(form_lc)
    }

    pub fn is_suffix(&self, form_lc: &str) -> bool {
        self.suffixes.contains(form_lc)
    }

    /// Exact root lookup — used by the `root`/`family` query paths, which
    /// should only ever report forms a source actually called a root.
    pub fn root_id(&self, form_lc: &str) -> Option<MorphemeId> {
        self.roots.contains(form_lc).then(|| MorphemeId::new(form_lc))
    }

    /// Root lookup for segmentation, which also accepts long affix-only forms
    /// as stand-ins. Returns how the match was made so the caller can score it.
    pub fn match_root(&self, form_lc: &str) -> Option<RootMatch> {
        if self.roots.contains(form_lc) {
            return Some(RootMatch::Attested);
        }
        let long_enough = form_lc.chars().count() >= MIN_FALLBACK_ROOT_LEN;
        if long_enough && (self.prefixes.contains(form_lc) || self.suffixes.contains(form_lc)) {
            return Some(RootMatch::AffixOnly);
        }
        None
    }
}
