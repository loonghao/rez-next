//! rez-next-search: Package search functionality
//!
//! Implements `rez search` — search across configured repositories for packages
//! matching a name pattern or version range. Mirrors rez's `rez search` CLI and
//! `rez.packages_.iter_packages` / family listing semantics.

pub mod searcher;
pub mod filter;
pub mod result;

pub use searcher::{PackageSearcher, SearchOptions, SearchScope};
pub use filter::{SearchFilter, FilterMode};
pub use result::{SearchResult, SearchResultSet};
