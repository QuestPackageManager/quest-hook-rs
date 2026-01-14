//! [crate_name = "quest_build_helper"]
//! A build helper crate for Quest mods using quest-hook-rs
//! This crate provides utilities for setting up C++ builds with common
//! includes and defines used in Quest modding. It also includes functions for
//! restoring QPM dependencies.

/// CC build helper functions
pub mod cc;

/// Linker helper functions
pub mod linker;

/// QPM helper functions
pub mod qpm;
