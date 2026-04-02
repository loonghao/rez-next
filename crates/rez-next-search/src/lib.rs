//! rez-next-search: Package search functionality
//!
//! Implements `rez search` — search across configured repositories for packages
//! matching a name pattern or version range. Mirrors rez's `rez search` CLI and
//! `rez.packages_.iter_packages` / family listing semantics.

pub mod filter;
pub mod result;
pub mod searcher;

pub use filter::{FilterMode, SearchFilter};
pub use result::{SearchResult, SearchResultSet};
pub use searcher::{PackageSearcher, SearchOptions, SearchScope};
