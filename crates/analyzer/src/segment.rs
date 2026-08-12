use lexiroot_core::{MorphemeId, MorphemeKind, MorphemeRef};

use crate::index::{MorphemeIndex, RootMatch};

/// Structural bounds on the morpheme grammar `prefix* root suffix*`. Three
/// of each covers everything English realistically stacks
/// (`re-con-struct-ion`, `nation-al-ly`) while keeping enumeration cheap.
const MAX_PREFIXES: usize = 3;
const MAX_SUFFIXES: usize = 3;
/// One-character affixes in the source data (`a`, `e`, `i`, `o`, `d`, `s`, …)
/// are overwhelmingly noise: admitting them lets the search shred any word
/// into single letters. Two is the shortest generally productive affix.
///
/// The one exception is `-y` in word-final position, which is genuinely
/// productive (`bio-graph-y`, `philo-soph-y`).
const MIN_AFFIX_LEN: usize = 2;

/// One-character suffixes admitted in word-final position, and nothing else.
///
/// The data lists six (`a d e i s y`). Allowing all of them was measured on a
/// 4000-word sample: coverage rose 3.1 points and the new parses were mostly
/// garbage — `andromache` -> `andr + oma + ch + e`, `barnard` -> `bar + nar +
/// d`, `ballata` -> `bal + lat + a`. Only `-y` is a productive English
/// derivational suffix; `-e` is orthographic, `-a`/`-i`/`-d` are fragments of
/// Greek and Latin stems.
///
/// `-s` is deliberately excluded too: it is real, but it is *inflection*, and
/// this engine has no lemmatization step to own it. Segmenting `apophysis`
/// as `apo + physi + s` is the failure mode.
const FINAL_ONE_CHAR_SUFFIXES: &[&str] = &["y"];
const MIN_ROOT_LEN: usize = 3;
/// Refuse to enumerate pathologically long inputs.
const MAX_WORD_LEN: usize = 40;

/// Scoring weights. The search finds many valid parses; these decide which is
/// *right*. Without them, greedy first-match returns `trans + por + tat + ion`
/// for "transportation" as readily as `trans + port + ation`.
///
/// The weights are separated by an order of magnitude so they act as a
/// priority order rather than a blend: root attestation dominates segment
/// plausibility, which dominates root placement, which dominates the rest.
/// They are tuned against `data/gold/segmentations.tsv`; change them and run
/// `cargo test -p lexiroot-store --test gold`.
mod score {
    /// A form only ever attested as an affix is a much weaker root than a
    /// real one. Large enough that no placement bonus can outweigh it —
    /// otherwise "portable" parses as `port-` + a fabricated root `able`.
    pub const AFFIX_ONLY_ROOT: i32 = 100;
    /// Two-character segments are usually coincidental substring hits.
    pub const SHORT_SEGMENT: i32 = 30;
    /// Charged per prefix beyond the first. Words with two stacked prefixes
    /// exist (`re-con-struct-ion`) but are far rarer than the placement
    /// reward implies, so without this the search buys extra prefixes to
    /// slide the root rightward: `tran + spor + tat + ion` for
    /// "transportation". Must exceed the reward for the characters an extra
    /// prefix typically covers (~3, so ~24) or it does not bite.
    ///
    /// Deliberately not charged for stacked suffixes, which really are
    /// common (`-ion-al-ly`).
    pub const EXTRA_PREFIX: i32 = 26;
    /// A word ending in a bare two- or three-letter root, with no suffix.
    /// That slot is normally a suffix, so this is usually a sign the search
    /// pushed the root too far right (`chrono + logi + cal`).
    pub const FINAL_SHORT_ROOT: i32 = 30;
    /// Reward pushing the root rightward, i.e. accounting for as much of the
    /// word's front as possible with real prefixes. This is what separates
    /// `trans-` + `port` from `trans` + `-port`: both use forms in attested
    /// positions and neither is shorter, but English builds Latinate words
    /// by prefixing a stem, not by suffixing one onto a root.
    pub const ROOT_START: i32 = 8;
    /// Among parses that place the root identically, prefer the longer root
    /// (`in-spect-ion` over `in-spec-tion`).
    pub const ROOT_LEN: i32 = 3;
    /// Mild pressure against splitting further than necessary.
    pub const PER_SEGMENT: i32 = 2;
    /// Charged on top of SHORT_SEGMENT for a word-final one-character suffix.
    /// Six of these exist in the data (`a d e i s y`) and only `-y` and `-s`
    /// are productive, so the parse has to be good enough to survive both
    /// penalties before shedding a single letter is worth it.
    pub const ONE_CHAR_SUFFIX: i32 = 25;

    /// Parses below this are reported as no analysis at all.
    ///
    /// A scored search always finds *something* for a long word, so without
    /// a floor "unhappiness" comes back as `un + hap + pi + ness`, stated as
    /// confidently as a correct answer — worse than admitting no analysis.
    /// Measured against `data/gold/segmentations.tsv`, correct parses bottom
    /// out around -35 (`in-spect-or`, two short segments) while junk parses
    /// of unsupported words sit at -150 and below, so the gap is wide.
    pub const MIN_ACCEPTABLE: i32 = -60;
}

/// One candidate parse and the score that ranked it.
#[derive(Debug, Clone, PartialEq)]
pub struct Segmentation {
    pub segments: Vec<MorphemeRef>,
    pub score: i32,
}

/// Best-scoring segmentation of `word`, or `None` if no parse exists.
pub fn segment(word: &str, index: &MorphemeIndex) -> Option<Vec<MorphemeRef>> {
    segment_ranked(word, index, 1).into_iter().next().map(|s| s.segments)
}

/// Up to `limit` segmentations, best first. Ties break deterministically on
/// segment count then form order, so release databases stay reproducible.
pub fn segment_ranked(word: &str, index: &MorphemeIndex, limit: usize) -> Vec<Segmentation> {
    let chars: Vec<char> = word.to_lowercase().chars().collect();
    if chars.is_empty() || chars.len() > MAX_WORD_LEN || limit == 0 {
        return Vec::new();
    }

    let prefix_chains = affix_chains(&chars, index, Side::Prefix);
    let suffix_chains = affix_chains(&chars, index, Side::Suffix);

    let mut candidates: Vec<Segmentation> = Vec::new();
    for (prefix_cuts, root_start) in &prefix_chains {
        for (suffix_cuts, root_end) in &suffix_chains {
            if *root_end <= *root_start || root_end - root_start < MIN_ROOT_LEN {
                continue;
            }
            let root: String = chars[*root_start..*root_end].iter().collect();
            let Some(root_match) = index.match_root(&root) else {
                continue;
            };
            let candidate = build(&chars, prefix_cuts, (*root_start, &root), suffix_cuts, root_match);
            if candidate.score >= score::MIN_ACCEPTABLE {
                candidates.push(candidate);
            }
        }
    }

    candidates.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.segments.len().cmp(&b.segments.len()))
            .then_with(|| forms_key(a).cmp(&forms_key(b)))
    });
    candidates.truncate(limit);
    candidates
}

fn forms_key(s: &Segmentation) -> Vec<&str> {
    s.segments.iter().map(|r| r.morpheme_id.as_str()).collect()
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Prefix,
    Suffix,
}

/// One affix taken out of the word.
#[derive(Clone, Copy)]
struct Cut {
    start: usize,
    len: usize,
}

/// Every chain of up to `MAX_*` affixes strippable from one end, paired with
/// the boundary of the region left for the root. Always includes the empty
/// chain (no affixes on that side). Cuts are returned left-to-right.
fn affix_chains(chars: &[char], index: &MorphemeIndex, side: Side) -> Vec<(Vec<Cut>, usize)> {
    let n = chars.len();
    let (max_affix_len, max_depth, boundary) = match side {
        Side::Prefix => (index.max_prefix_len(), MAX_PREFIXES, 0),
        Side::Suffix => (index.max_suffix_len(), MAX_SUFFIXES, n),
    };

    let mut all = vec![(Vec::new(), boundary)];
    let mut frontier = all.clone();

    for _ in 0..max_depth {
        let mut next: Vec<(Vec<Cut>, usize)> = Vec::new();
        for (cuts, pos) in &frontier {
            // Leave at least MIN_ROOT_LEN characters for the root.
            let room = match side {
                Side::Prefix => n.saturating_sub(pos + MIN_ROOT_LEN),
                Side::Suffix => pos.saturating_sub(MIN_ROOT_LEN),
            };
            // Only the first suffix stripped is word-final, so only it may be
            // a single character.
            let word_final_suffix = side == Side::Suffix && *pos == n;
            let min_len = if word_final_suffix { 1 } else { MIN_AFFIX_LEN };
            for len in min_len..=max_affix_len.min(room) {
                let (start, end) = match side {
                    Side::Prefix => (*pos, pos + len),
                    Side::Suffix => (pos - len, *pos),
                };
                let form: String = chars[start..end].iter().collect();
                if len == 1 && !FINAL_ONE_CHAR_SUFFIXES.contains(&form.as_str()) {
                    continue;
                }
                let matches = match side {
                    Side::Prefix => index.is_prefix(&form),
                    Side::Suffix => index.is_suffix(&form),
                };
                if !matches {
                    continue;
                }
                let cut = Cut { start, len };
                let next_pos = if side == Side::Prefix { end } else { start };
                let mut cuts = cuts.clone();
                match side {
                    // Prefixes are discovered left-to-right, suffixes
                    // right-to-left; prepend the latter to keep both in
                    // word order.
                    Side::Prefix => cuts.push(cut),
                    Side::Suffix => cuts.insert(0, cut),
                }
                next.push((cuts, next_pos));
            }
        }
        if next.is_empty() {
            break;
        }
        all.extend(next.iter().cloned());
        frontier = next;
    }

    all
}

fn build(
    chars: &[char],
    prefix_cuts: &[Cut],
    (root_start, root): (usize, &str),
    suffix_cuts: &[Cut],
    root_match: RootMatch,
) -> Segmentation {
    let mut segments = Vec::with_capacity(prefix_cuts.len() + suffix_cuts.len() + 1);
    let mut short_segments = 0i32;

    let mut push = |form: &str, role: MorphemeKind, segments: &mut Vec<MorphemeRef>| {
        let len = form.chars().count();
        // A word-final one-character suffix is charged ONE_CHAR_SUFFIX
        // instead; charging both would price it out of every parse.
        if len > 1 && len <= MIN_AFFIX_LEN {
            short_segments += 1;
        }
        segments.push(MorphemeRef {
            morpheme_id: MorphemeId::new(form),
            role,
        });
    };

    for cut in prefix_cuts {
        let form: String = chars[cut.start..cut.start + cut.len].iter().collect();
        push(&form, MorphemeKind::Prefix, &mut segments);
    }
    push(root, MorphemeKind::Root, &mut segments);
    for cut in suffix_cuts {
        let form: String = chars[cut.start..cut.start + cut.len].iter().collect();
        push(&form, MorphemeKind::Suffix, &mut segments);
    }

    let root_len = root.chars().count();
    let extra_prefixes = prefix_cuts.len().saturating_sub(1) as i32;
    let final_short_root = suffix_cuts.is_empty() && root_len <= MIN_ROOT_LEN;
    let one_char_suffix = suffix_cuts.last().is_some_and(|c| c.len == 1);

    let score = score::ROOT_START * root_start as i32 + score::ROOT_LEN * root_len as i32
        - score::PER_SEGMENT * segments.len() as i32
        - score::SHORT_SEGMENT * short_segments
        - score::EXTRA_PREFIX * extra_prefixes
        - if final_short_root { score::FINAL_SHORT_ROOT } else { 0 }
        - if one_char_suffix { score::ONE_CHAR_SUFFIX } else { 0 }
        - match root_match {
            RootMatch::Attested => 0,
            RootMatch::AffixOnly => score::AFFIX_ONLY_ROOT,
        };

    Segmentation { segments, score }
}
