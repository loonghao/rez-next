//! # rez-next-bind
//!
//! Implements `rez bind` functionality: bind system-installed tools as rez packages.
//!
//! This is equivalent to the original rez `rez.bind` module. It discovers system
//! tools (python, cmake, git, etc.), inspects their versions, and writes package.py
//! definitions into a configurable packages path.

mod binder;
mod builtin_binders;
mod detect;

pub use binder::{BindError, BindOptions, BindResult, PackageBinder};
pub use builtin_binders::{BuiltinBinder, get_builtin_binder, list_builtin_binders};
pub use detect::{detect_tool_version, find_tool_executable, extract_version_from_output};
