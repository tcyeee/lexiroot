# Data Sources

Both sources used by the v0.1 minimal loop are MIT-licensed, chosen
specifically to avoid the Wiktionary/MorphoLex/Etymonline licensing risk
flagged in the README's early review. See
[`docs/superpowers/specs/2026-07-02-v0.1-minimal-loop-design.md`](superpowers/specs/2026-07-02-v0.1-minimal-loop-design.md)
for the full rationale.

Only the specific data file(s) each source's importer needs are vendored,
not the full upstream repositories (the `colingoldberg/morphemes` repo also
contains unrelated Python/NLTK tooling under `code/`, `docs/`, `logs/`,
which is not vendored).

## colingoldberg/morphemes

- Upstream: https://github.com/colingoldberg/morphemes
- License: MIT (LICENSE file present upstream)
- Pinned commit: `846aa473cb27916f2c3acedb52d98f3a2e2a6572` (`master`)
- Vendored to: `data/raw/colingoldberg-morphemes/`
  - `LICENSE` — original MIT license text, copyright Colin Goldberg
  - `README.md` — upstream README
  - `morphemes.json` — the dataset itself (originally at `data/morphemes.json`
    upstream), 2435 entries: prefixes, roots ("embedded"), and suffixes with
    meanings and example words

## WithEnglishWeCan/generated-english-roots-list

- Upstream: https://github.com/WithEnglishWeCan/generated-english-roots-list
- License: MIT, as stated in the repository's README ("Licensed under
  MIT."); there is no separate `LICENSE` file upstream, so none is vendored
  — the license statement is preserved by vendoring the README itself.
- Pinned commit: `bf26e3842137d8f7bbc6e69ef39c05b43e3a22a6` (`master`)
- Vendored to: `data/raw/withenglishwecan-roots/`
  - `README.md` — upstream README (contains the license statement, plus an
    auto-generated human-readable table of the same data)
  - `english.roots.list.build.json` — the dataset itself, 1061 English
    roots with meanings and example words

## Reproducibility

Pinning these commit hashes means re-running `pipeline/importer` against
the vendored files always produces the same normalized rows — the raw
inputs to the pipeline are frozen, matching the README's "Immutable
Releases" goal (same input → byte-identical release SQLite file).
