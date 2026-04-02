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

pub mod bind;
pub mod build;
pub mod bundle;
pub mod complete;
pub mod cp;
pub mod depends;
pub mod diff;
pub mod forward;
pub mod gui;
pub mod help;
pub mod mv;
pub mod pip;
pub mod pkg_cache;
pub mod plugins;
pub mod rm;
pub mod search_v2;
pub mod solve;
pub mod status;
pub mod suites;
