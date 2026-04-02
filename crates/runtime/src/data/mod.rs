//! Legacy Abuse data format parsers.
//!
//! This module provides parsers for:
//! - `.spe` container format (SPEC 1.0 directory archives)
//! - Level sections (tile maps, objects, lights, links)
//! - Lisp scripts (startup, object definitions, gameplay logic)
//!
//! All parsers are designed to handle original Abuse data with both strict and
//! lenient compatibility modes where applicable.

pub mod level;
pub mod lisp;
pub mod spe;
