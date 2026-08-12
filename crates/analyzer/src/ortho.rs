//! Regular English spelling adjustments at a morpheme boundary.
//!
//! A segmenter that tiles a word with literal substrings can only ever find
//! morphemes whose spelling survives composition intact. English routinely
//! rewrites the stem instead: `believe` + `-able` surfaces as `believable`,
//! `happy` + `-ness` as `happiness`, `run` + `-ing` as `running`. The stem is
//! in the table, the affixes are in the table, and the word still fails to
//! parse because the six characters sitting in the root slot (`believ`) are
//! not the seven the table holds (`believe`).
//!
//! This module runs those rules *backwards*: given the surface string that
//! landed in the root slot, it proposes the underlying forms that could have
//! produced it, for the index to look up.
//!
//! Only rules that are **productive** live here — ones that apply to stems
//! the table has never seen. Irregular, stem-specific alternation
//! (`admit` ~ `admiss`) is not predictable from spelling and is listed per
//! morpheme in `Morpheme::variants` instead.

/// Which rule derived a candidate underlying form. Carried through
/// segmentation so callers can explain the spelling change, and so the
/// scorer can charge for having needed one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrthoRule {
    /// Silent-e deletion before a vowel-initial suffix:
    /// `believe` + `-able` -> `believable`, `use` + `-age` -> `usage`.
    SilentE,
    /// Final-consonant doubling before a vowel-initial suffix:
    /// `run` + `-ing` -> `running`, `big` + `-est` -> `biggest`.
    Undouble,
    /// `y` -> `i` before a suffix that does not itself start with `i`:
    /// `happy` + `-ness` -> `happiness`, `beauty` + `-ful` -> `beautiful`.
    /// (The exclusion is what keeps `carry` + `-ing` as `carrying`.)
    YToI,
}

impl OrthoRule {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrthoRule::SilentE => "silent-e deletion",
            OrthoRule::Undouble => "consonant doubling",
            OrthoRule::YToI => "y->i",
        }
    }
}

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

/// Consonants English actually doubles before a vowel-initial suffix.
///
/// Restricted on purpose: `h`, `j`, `k`, `q`, `v`, `w`, `x`, `y` never double,
/// and admitting `s` would rewrite every `-ss` stem (`class`, `press`) into a
/// nonexistent single-`s` root. `l` is in because British spelling doubles it
/// freely (`travelling`).
const DOUBLEABLE: &[char] = &['b', 'd', 'f', 'g', 'l', 'm', 'n', 'p', 'r', 't', 'z'];

/// Underlying forms that could surface as `surface` in the root slot, given
/// the first character of whatever suffix follows it.
///
/// `next_initial` is `None` when the root ends the word. In that position no
/// affix was attached, so nothing rewrote the stem and no rule applies —
/// without this guard every bare word would also be probed as `word` + `e`,
/// which is how you get `hors` proposed for `horse` in reverse.
///
/// Returns at most a couple of candidates; callers try each against the index
/// and keep the best-attested hit.
pub fn underlying_candidates(surface: &str, next_initial: Option<char>) -> Vec<(String, OrthoRule)> {
    let Some(next) = next_initial else {
        return Vec::new();
    };
    let chars: Vec<char> = surface.chars().collect();
    let Some(&last) = chars.last() else {
        return Vec::new();
    };
    let mut out = Vec::new();

    if is_vowel(next) {
        // `believ` <- `believe`. A stem already ending in `e` did not lose
        // one, and a stem ending in a vowel would not have had a silent `e`
        // to lose (`-ee` aside, which is rare enough to leave alone).
        if !is_vowel(last) {
            out.push((format!("{surface}e"), OrthoRule::SilentE));
        }
        // `runn` <- `run`. Doubling only fires on a stem ending in a single
        // short vowel plus one consonant, so the pair must be preceded by
        // exactly one vowel: `sleepp` (two) could not have arisen this way.
        //
        // This still over-proposes — `fall` is CVCC just like `runn`, so
        // `fal` gets offered too. That is fine and deliberate: the rule only
        // *proposes*, and `fal` is in no morpheme table, so the index throws
        // it away. Tightening further would need vowel-length knowledge the
        // spelling does not carry.
        let n = chars.len();
        if n >= 3 && DOUBLEABLE.contains(&last) && chars[n - 2] == last {
            let single_short_vowel = is_vowel(chars[n - 3])
                && n.checked_sub(4).is_none_or(|i| !is_vowel(chars[i]));
            if single_short_vowel {
                out.push((chars[..n - 1].iter().collect(), OrthoRule::Undouble));
            }
        }
    }

    // `happi` <- `happy`. Blocked before `i` so `carrying` is not analysed as
    // `carry` + `-ing` with the `y` rewritten and then an `i` left over.
    if last == 'i' && next != 'i' && chars.len() >= 2 {
        let mut stem: String = chars[..chars.len() - 1].iter().collect();
        stem.push('y');
        out.push((stem, OrthoRule::YToI));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates(surface: &str, next: Option<char>) -> Vec<String> {
        underlying_candidates(surface, next).into_iter().map(|(f, _)| f).collect()
    }

    #[test]
    fn restores_silent_e_before_a_vowel_initial_suffix() {
        assert!(candidates("believ", Some('a')).contains(&"believe".to_string()));
        assert!(candidates("us", Some('a')).contains(&"use".to_string()));
    }

    #[test]
    fn does_not_restore_silent_e_before_a_consonant_initial_suffix() {
        assert!(!candidates("believ", Some('n')).contains(&"believe".to_string()));
    }

    #[test]
    fn undoubles_only_after_a_single_short_vowel() {
        assert!(candidates("runn", Some('i')).contains(&"run".to_string()));
        assert!(candidates("bigg", Some('e')).contains(&"big".to_string()));
        // Two vowels before the pair: never a doubling site.
        assert!(!candidates("sleepp", Some('i')).contains(&"sleep".to_string()));
        // Undoubling is never proposed before a consonant-initial suffix.
        assert!(!candidates("runn", Some('n')).contains(&"run".to_string()));
    }

    #[test]
    fn never_undoubles_s() {
        assert!(!candidates("class", Some('i')).contains(&"clas".to_string()));
        assert!(!candidates("press", Some('u')).contains(&"pres".to_string()));
    }

    #[test]
    fn rewrites_final_i_back_to_y_except_before_i() {
        assert!(candidates("happi", Some('n')).contains(&"happy".to_string()));
        assert!(candidates("beauti", Some('f')).contains(&"beauty".to_string()));
        assert!(candidates("carri", Some('i')).is_empty());
    }

    /// A word-final root was never suffixed, so it was never rewritten.
    #[test]
    fn proposes_nothing_when_the_root_ends_the_word() {
        assert!(candidates("hors", None).is_empty());
        assert!(candidates("happi", None).is_empty());
    }
}
