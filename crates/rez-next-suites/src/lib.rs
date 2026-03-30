//! # Rez Next Suites
//!
//! Suite management for Rez Next.
//!
//! A Suite is a collection of resolved contexts that expose a combined set of tools.
//! Tools from multiple contexts can be aliased and exposed together, making suites
//! useful for defining project-specific toolsets.
//!
//! ## Equivalent to original `rez.suite.Suite`
//!
//! ```python
//! # Original rez
//! from rez.suite import Suite
//! s = Suite()
//! s.add_context("maya", ResolvedContext(["maya-2023", "python-3.9"]))
//! s.add_context("nuke", ResolvedContext(["nuke-13", "python-3.9"]))
//! s.save("/path/to/my_suite")
//!
//! # rez-next (identical API via Python bindings)
//! from rez_next.suite import Suite
//! s = Suite()
//! s.add_context("maya", ResolvedContext(["maya-2023", "python-3.9"]))
//! ```

pub mod suite;
pub mod suite_context;
pub mod suite_tool;
pub mod suite_manager;
pub mod error;

pub use suite::{Suite, SuiteStatus};
pub use suite_context::SuiteContext;
pub use suite_tool::{SuiteTool, ToolConflictMode};
pub use suite_manager::SuiteManager;
pub use error::SuiteError;
