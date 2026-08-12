use std::collections::{HashMap, HashSet};

use lexiroot_core::{Morpheme, MorphemeId, MorphemeKind};

use crate::ortho::{underlying_candidates, OrthoRule};

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

/// A resolved root-slot match: which morpheme it is, how well attested, and
/// what (if anything) had to be undone to spell it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootHit {
    /// The *canonical* morpheme, not the surface substring. `believ` in
    /// `unbelievable` resolves to the morpheme `believe`, so downstream
    /// consumers get an id that actually exists in the morpheme table.
    pub id: MorphemeId,
    pub quality: RootMatch,
    /// `Some` when a spelling rule was reversed to reach `id`, `None` when
    /// the surface matched a listed form (canonical or variant) directly.
    pub rule: Option<OrthoRule>,
}

/// Lookup structures built once from the full `Morpheme` table: position-keyed
/// form sets used by `segment` to enumerate candidate splits.
///
/// The sets hold *surface* forms — every morpheme's own form plus each of its
/// `variants` — because that is what the segmenter has in hand when it slices
/// a word. `canonical` maps the ones that are not themselves morpheme ids back
/// to the morpheme they realize.
pub struct MorphemeIndex {
    by_id: HashMap<MorphemeId, Morpheme>,
    prefixes: HashSet<String>,
    suffixes: HashSet<String>,
    roots: HashSet<String>,
    /// Variant surface -> the morpheme it realizes. Only holds surfaces that
    /// are not already a morpheme id in their own right.
    canonical: HashMap<String, MorphemeId>,
    max_prefix_len: usize,
    max_suffix_len: usize,
}

impl MorphemeIndex {
    pub fn build(morphemes: Vec<Morpheme>) -> Self {
        let mut index = Self {
            by_id: HashMap::with_capacity(morphemes.len()),
            prefixes: HashSet::new(),
            suffixes: HashSet::new(),
            roots: HashSet::new(),
            canonical: HashMap::new(),
            max_prefix_len: 0,
            max_suffix_len: 0,
        };

        // Pass 1: canonical forms. Done before any variant so a morpheme's
        // own form always wins the surface that spells it — `im-` is listed
        // as a variant of `in-` and is also a morpheme in its own right, and
        // it must resolve to itself.
        for m in &morphemes {
            index.record_surface(&m.form.to_lowercase(), m);
        }
        let ids: HashSet<&str> = morphemes.iter().map(|m| m.id.as_str()).collect();

        // Pass 2: variants, in id order so a surface claimed by two morphemes
        // resolves the same way in every build — release databases must be
        // byte-reproducible. A variant occupies the same positions as the
        // morpheme it realizes: `admiss` is a root because `admit` is.
        let mut owners: Vec<&Morpheme> = morphemes.iter().collect();
        owners.sort_unstable_by(|a, b| a.id.cmp(&b.id));
        for m in owners {
            for variant in &m.variants {
                let variant_lc = variant.to_lowercase();
                if ids.contains(variant_lc.as_str()) {
                    continue;
                }
                index.record_surface(&variant_lc, m);
                index.canonical.entry(variant_lc).or_insert_with(|| m.id.clone());
            }
        }

        for m in morphemes {
            index.by_id.insert(m.id.clone(), m);
        }
        index
    }

    /// Files one surface spelling of `owner` into the position-keyed sets.
    fn record_surface(&mut self, surface_lc: &str, owner: &Morpheme) {
        let len = surface_lc.chars().count();
        if owner.positions.contains(MorphemeKind::Prefix) {
            self.max_prefix_len = self.max_prefix_len.max(len);
            self.prefixes.insert(surface_lc.to_string());
        }
        if owner.positions.contains(MorphemeKind::Suffix) {
            self.max_suffix_len = self.max_suffix_len.max(len);
            self.suffixes.insert(surface_lc.to_string());
        }
        if owner.positions.contains(MorphemeKind::Root) {
            self.roots.insert(surface_lc.to_string());
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

    /// The morpheme a surface form realizes. Falls back to treating the
    /// surface as its own id, which is the answer for every form that is not
    /// a listed variant.
    pub fn canonical_id(&self, form_lc: &str) -> MorphemeId {
        let direct = MorphemeId::new(form_lc);
        if self.by_id.contains_key(&direct) {
            return direct;
        }
        self.canonical.get(form_lc).cloned().unwrap_or(direct)
    }

    /// Exact root lookup — used by the `root`/`family` query paths, which
    /// should only ever report forms a source actually called a root.
    pub fn root_id(&self, form_lc: &str) -> Option<MorphemeId> {
        self.roots.contains(form_lc).then(|| self.canonical_id(form_lc))
    }

    /// Root lookup for segmentation.
    ///
    /// `next_initial` is the first character of the suffix that follows the
    /// root, or `None` if the root ends the word; it gates which spelling
    /// rules could have applied (see `ortho`).
    ///
    /// `min_literal_len` is the shortest surface the caller will accept *as
    /// written*. Below it, only a spelling-derived reading is offered. This
    /// is what lets `usable` work: `us` is listed as a root upstream, and
    /// taking that literal match would both shadow the real analysis and hand
    /// the caller a two-letter root it is going to reject anyway.
    ///
    /// Candidates are tried in a fixed precedence — a listed form beats a
    /// spelling-derived one, and an attested root beats an affix-only
    /// stand-in — so the caller gets the single best reading of the slot.
    pub fn match_root(
        &self,
        form_lc: &str,
        next_initial: Option<char>,
        min_literal_len: usize,
    ) -> Option<RootHit> {
        let literal = (form_lc.chars().count() >= min_literal_len)
            .then(|| self.match_listed(form_lc))
            .flatten();

        if literal == Some(RootMatch::Attested) {
            return Some(RootHit {
                id: self.canonical_id(form_lc),
                quality: RootMatch::Attested,
                rule: None,
            });
        }

        let derived = underlying_candidates(form_lc, next_initial);
        let attested = derived
            .iter()
            .find(|(underlying, _)| self.roots.contains(underlying.as_str()));
        if let Some((underlying, rule)) = attested {
            return Some(RootHit {
                id: self.canonical_id(underlying),
                quality: RootMatch::Attested,
                rule: Some(*rule),
            });
        }

        if literal.is_some() {
            return Some(RootHit {
                id: self.canonical_id(form_lc),
                quality: RootMatch::AffixOnly,
                rule: None,
            });
        }

        derived
            .iter()
            .find(|(underlying, _)| self.match_listed(underlying) == Some(RootMatch::AffixOnly))
            .map(|(underlying, rule)| RootHit {
                id: self.canonical_id(underlying),
                quality: RootMatch::AffixOnly,
                rule: Some(*rule),
            })
    }

    /// How a surface form matches the root slot using only listed forms, with
    /// no spelling rules applied.
    fn match_listed(&self, form_lc: &str) -> Option<RootMatch> {
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
