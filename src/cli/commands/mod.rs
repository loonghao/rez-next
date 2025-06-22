//! # CLI Commands
//!
//! This module contains implementations of all Rez CLI commands.
//! Each command is implemented in its own module with a consistent interface.

pub mod config;
pub mod context;
pub mod env;
pub mod release;
pub mod test;
pub mod view;

// TODO: Add more commands as they are implemented
pub mod build;
// pub mod search;
pub mod bind;
pub mod cp;
pub mod depends;
pub mod diff;
pub mod help;
pub mod mv;
pub mod pkg_cache;
pub mod plugins;
pub mod rm;
pub mod search_v2;
pub mod solve;
pub mod status;
