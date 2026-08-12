# lexiroot-stems

LexiRoot's own hand-curated **free-stem** list.

## Why this exists

Both upstream datasets (`colingoldberg-morphemes`, `withenglishwecan-roots`)
are Greek/Latin **bound-root** dictionaries. They are good at `spect`, `port`,
`struct` — forms that never stand alone as words — and they contain essentially
none of the native Germanic core of English.

The consequence was total, not partial: `believe`, `help`, `friend`,
`understand`, `break` and `speak` were all absent from the morpheme table, so
no word built on them could be segmented at all. `unbelievable` failed even
though `un-` and `-able` were both present, because the root slot between them
resolved to `believ`, which matched nothing.

This file supplies those stems.

## Format

A JSON object keyed by the stem's canonical spelling:

```json
"believe": {
  "positions": ["root"],
  "meanings": ["accept as true", "have faith in"],
  "examples": ["believable", "unbelievable", "believer"]
}
```

| field | required | meaning |
|---|---|---|
| `positions` | yes | any of `prefix`, `root`, `suffix`. Free stems are `root`. |
| `meanings` | yes | short glosses, most central first |
| `examples` | no | words built on the stem; the importer precomputes decompositions for these |
| `variants` | no | **irregular** surface allomorphs — see below |

## What belongs in `variants`, and what does not

`variants` is for alternations **no general rule predicts**, so they have to be
listed one by one:

- Latin stem alternation — `admit` ~ `admiss`, `receive` ~ `recept`
- prefix assimilation — `in-` ~ `im-`, `il-`, `ir-`

Regular English spelling adjustments at a morpheme boundary are **not** listed
here:

| change | example | handled by |
|---|---|---|
| silent-e deletion | `believe` + `-able` → `believable` | `analyzer::ortho::SilentE` |
| consonant doubling | `run` + `-ing` → `running` | `analyzer::ortho::Undouble` |
| `y` → `i` | `happy` + `-ness` → `happiness` | `analyzer::ortho::YToI` |

Those are productive: they apply to every stem, including ones this file has
never heard of. Enumerating them would mean three or four dead rows per stem
and would still miss anything new. The segmenter derives them instead.

So: **if a rule can predict it, leave it out.**

## Curation rules

- Canonical spelling as the key — the lemma, not an inflected form.
- Minimum three characters. The segmenter's `MIN_ROOT_LEN` is 3, and shorter
  forms shred words into noise.
- No form that is already a productive affix in the other sources (`ship`,
  `ward`, `less`, `hood`, `ment`, `dom`). Adding those as roots makes the
  search prefer them over the real root.
- Every addition is checked against `data/gold/segmentations.tsv` — a stem
  that breaks an existing correct segmentation does not go in.
