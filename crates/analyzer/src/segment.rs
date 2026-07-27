use lexiroot_core::MorphemeRef;

use crate::index::MorphemeIndex;

/// Greedy longest-match segmentation: strip the longest known prefix from
/// the front and longest known suffix from the back, then require the
/// remainder to be an exact root match. Single-pass, no backtracking across
/// alternate affix lengths — if the remainder doesn't match a root, the
/// whole segmentation fails rather than trying a shorter affix.
pub fn segment(word: &str, index: &MorphemeIndex) -> Option<Vec<MorphemeRef>> {
    let word_lc = word.to_lowercase();

    if let Some(root_id) = index.root_id(&word_lc) {
        return Some(vec![MorphemeRef {
            morpheme_id: root_id.clone(),
        }]);
    }

    let prefix_match = index.longest_prefix(&word_lc);
    let after_prefix: String = match &prefix_match {
        Some((_, len)) => word_lc.chars().skip(*len).collect(),
        None => word_lc.clone(),
    };
    if after_prefix.is_empty() {
        return None;
    }

    let suffix_match = index.longest_suffix(&after_prefix);
    let root_candidate: String = match &suffix_match {
        Some((_, len)) => {
            let keep = after_prefix.chars().count() - len;
            after_prefix.chars().take(keep).collect()
        }
        None => after_prefix,
    };
    if root_candidate.is_empty() {
        return None;
    }

    let root_id = index.root_id(&root_candidate)?;

    let mut segments = Vec::with_capacity(3);
    if let Some((id, _)) = prefix_match {
        segments.push(MorphemeRef { morpheme_id: id });
    }
    segments.push(MorphemeRef {
        morpheme_id: root_id.clone(),
    });
    if let Some((id, _)) = suffix_match {
        segments.push(MorphemeRef { morpheme_id: id });
    }
    Some(segments)
}
