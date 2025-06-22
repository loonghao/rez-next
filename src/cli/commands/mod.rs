//! # CLI Commands
//!
//! This module contains implementations of all Rez CLI commands.
//! Each command is implemented in its own module with a consistent interface.

pub mod config;
pub mod context;
pub mod view;
pub mod env;
pub mod release;
pub mod test;

// TODO: Add more commands as they are implemented
pub mod build;
// pub mod search;
pub mod search_v2;
pub mod bind;
pub mod depends;
pub mod solve;
pub mod cp;
pub mod mv;
pub mod rm;
pub mod status;
pub mod diff;
pub mod help;
pub mod plugins;
pub mod pkg_cache;
