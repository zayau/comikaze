//! Generate hand-drawn comic-style SVG components.
//!
//! Comikaze currently provides deterministic comic panel frame generation
//! through the [`frame`] module. Speech balloons and caption boxes are planned.

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
