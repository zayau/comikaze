//! Generate hand-drawn comic-style SVG components.
//!
//! Comikaze provides deterministic standalone frame generation through the
//! [`frame`] module and irregular page generation through the [`layout`] module.
//! Final panel contours can also be reused as masks or clipping paths.

#![warn(missing_docs)]

mod hand_drawn;
mod svg;
mod topology;

/// Comic panel frame generation.
pub mod frame;

/// Fundamental two-dimensional geometry types.
pub mod geometry;

/// Page-level comic panel layout generation.
pub mod layout;
