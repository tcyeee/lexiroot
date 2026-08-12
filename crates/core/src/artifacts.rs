//! The pipeline's two SQLite artifacts: where each lives and, for the
//! published one, what version it is.
//!
//! Both paths are constants because both are shared between a producer and
//! its consumers — `importer` writes the build database and `exporter` reads
//! it; `exporter` writes the release database and `cli`, `web` and the gold
//! test read it. Spelled out at each end, a path is a constant that can drift
//! silently: the reader keeps opening a file that nobody writes anymore.
//!
//! The release path is also deliberately *unversioned*. Its version used to
//! be in its filename (`lexiroot-v0.1.sqlite`), which duplicated the version
//! across every one of those call sites and left the previous file sitting on
//! disk for anything that missed the bump. The version now travels *inside*
//! the file, so a stale or incompatible database fails at `store::load`
//! instead of silently answering queries.

/// The curated morpheme dataset, relative to the workspace root — the single
/// hand-editable input the whole pipeline reads.
pub const DATASET_PATH: &str = "data/sources/morphemes.json";

/// The importer's output and the exporter's input, relative to the workspace
/// root. Named after the pipeline stage that produces it (`normalize`), not
/// after the fact that it has been processed — everything under `data/build/`
/// has been.
pub const BUILD_DB_PATH: &str = "data/build/normalized.sqlite";

/// The published database, relative to the workspace root. Lives under
/// `data/dist/` rather than `data/release/`: in a Cargo workspace "release"
/// already names a build profile (`target/release/`), and the two read as the
/// same thing at a glance.
pub const RELEASE_DB_PATH: &str = "data/dist/lexiroot.sqlite";

/// Table layout of the release database. Bump when a change would stop
/// `store::load` from reading an older file; `load` refuses anything else.
///
/// v2 dropped the `source` column from both record tables.
pub const RELEASE_SCHEMA_VERSION: u32 = 2;

/// The dataset's own version, bumped by hand when `data/sources/` changes in
/// a way worth publishing. Independent of the schema version and of the
/// workspace's crate version — the data can change without the tables moving.
pub const RELEASE_DATA_VERSION: &str = "0.1.0";

/// `meta` key holding [`RELEASE_SCHEMA_VERSION`].
pub const META_SCHEMA_VERSION: &str = "schema_version";

/// `meta` key holding [`RELEASE_DATA_VERSION`].
pub const META_DATA_VERSION: &str = "data_version";
