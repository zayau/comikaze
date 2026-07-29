//! Generate hand-drawn comic panel frames as SVG.
//!
//! Use [`crate::frame::FrameOptions`] to configure a frame and
//! [`crate::frame::build_frame_svg`] to validate those options and generate the
//! SVG markup.

use crate::geometry::Polygon;
use crate::hand_drawn::hand_drawn_polygon;
use crate::svg::{closed_path_data, is_valid_color, is_valid_stroke_width};
use chacha20::ChaCha12Rng;
use rand::SeedableRng;
use std::fmt;

type FrameRng = ChaCha12Rng;

const EDGE_JITTER: f64 = 1.0;
const EDGE_INTERIOR_POINTS: usize = 4;

/// An error produced while validating or generating a frame.
#[derive(Debug, Clone)]
pub enum FrameError {
    /// The stroke width is zero, negative, infinite, or not a number.
    InvalidStrokeWidth {
        /// The invalid stroke width.
        stroke_width: f64,
    },
    /// The stroke color is not one of the supported SVG color forms.
    InvalidColor {
        /// The invalid color string.
        color: String,
    },
    /// The fill color is not one of the supported SVG color forms.
    InvalidFill {
        /// The invalid fill color string.
        fill: String,
    },
    /// The requested dimensions cannot contain the configured stroke and jitter.
    DimensionsTooSmall {
        /// The requested width in pixels.
        width: u32,
        /// The requested height in pixels.
        height: u32,
        /// The requested stroke width in pixels.
        stroke_width: f64,
        /// The exclusive minimum width and height for this stroke width.
        minimum_size: f64,
    },
}

impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStrokeWidth { stroke_width } => {
                write!(
                    formatter,
                    "stroke width must be a finite number greater than 0, got {stroke_width}"
                )
            }
            Self::InvalidColor { color } => {
                write!(
                    formatter,
                    "`{color}` is not a valid color — use a hex code (#rgb, #rrggbb, #rrggbbaa) or `currentColor`"
                )
            }
            Self::InvalidFill { fill } => {
                write!(
                    formatter,
                    "`{fill}` is not a valid fill color — use a hex code (#rgb, #rrggbb, #rrggbbaa) or `currentColor`"
                )
            }
            Self::DimensionsTooSmall {
                width,
                height,
                stroke_width,
                minimum_size,
            } => {
                write!(
                    formatter,
                    "width and height must each be greater than {minimum_size:.1}px at stroke-width {stroke_width} — got {width}x{height}"
                )
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// Options used to generate a comic panel frame.
///
/// The default frame is 400 by 600 pixels with a three-pixel black stroke and
/// a randomly generated wobble. Set [`seed`](Self::seed) to reproduce a frame.
#[derive(Debug, Clone)]
pub struct FrameOptions {
    /// The SVG width in pixels.
    pub width: u32,
    /// The SVG height in pixels.
    pub height: u32,
    /// The stroke color as `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, or
    /// `currentColor`.
    pub color: String,
    /// An optional interior fill color. `None` leaves the frame transparent.
    pub fill: Option<String>,
    /// The stroke width in pixels.
    pub stroke_width: f64,
    /// An optional seed for reproducible output.
    pub seed: Option<u64>,
}

impl Default for FrameOptions {
    fn default() -> Self {
        Self {
            width: 400,
            height: 600,
            color: "#000000".to_string(),
            fill: None,
            stroke_width: 3.0,
            seed: None,
        }
    }
}

impl FrameOptions {
    /// Validates the options without generating SVG output.
    ///
    /// # Errors
    ///
    /// Returns a [`FrameError`] when the stroke width or color is invalid, or
    /// when the dimensions cannot contain the configured stroke and jitter.
    pub fn validate(&self) -> Result<(), FrameError> {
        validate_frame_dimensions(self.width, self.height, self.stroke_width)?;
        validate_color(&self.color)?;
        if let Some(fill) = &self.fill {
            validate_fill(fill)?;
        }

        Ok(())
    }
}

/// Validates a frame stroke width.
///
/// # Errors
///
/// Returns [`FrameError::InvalidStrokeWidth`] unless `stroke_width` is finite
/// and greater than zero.
pub fn validate_stroke_width(stroke_width: f64) -> Result<(), FrameError> {
    if !is_valid_stroke_width(stroke_width) {
        return Err(FrameError::InvalidStrokeWidth { stroke_width });
    }

    Ok(())
}

/// Validates a supported SVG stroke color.
///
/// Accepted values are `currentColor` and three-, four-, six-, or eight-digit
/// hexadecimal colors.
///
/// # Errors
///
/// Returns [`FrameError::InvalidColor`] when `color` is unsupported.
pub fn validate_color(color: &str) -> Result<(), FrameError> {
    if is_valid_color(color) {
        return Ok(());
    }

    Err(FrameError::InvalidColor {
        color: color.to_string(),
    })
}

/// Validates a supported SVG fill color.
///
/// Accepted values are `currentColor` and three-, four-, six-, or eight-digit
/// hexadecimal colors. Use `None` in [`FrameOptions::fill`] for no fill.
///
/// # Errors
///
/// Returns [`FrameError::InvalidFill`] when `fill` is unsupported.
pub fn validate_fill(fill: &str) -> Result<(), FrameError> {
    if is_valid_color(fill) {
        return Ok(());
    }

    Err(FrameError::InvalidFill {
        fill: fill.to_string(),
    })
}

/// Validates that frame dimensions can contain the stroke and edge jitter.
///
/// # Errors
///
/// Returns [`FrameError::InvalidStrokeWidth`] for an invalid stroke width or
/// [`FrameError::DimensionsTooSmall`] when either dimension is too small.
pub fn validate_frame_dimensions(
    width: u32,
    height: u32,
    stroke_width: f64,
) -> Result<(), FrameError> {
    validate_stroke_width(stroke_width)?;

    let margin = EDGE_JITTER + stroke_width / 2.0;
    let minimum_inner_span = EDGE_JITTER * 2.0;
    let min_size = margin * 2.0 + minimum_inner_span;

    if (width as f64) <= min_size || (height as f64) <= min_size {
        return Err(FrameError::DimensionsTooSmall {
            width,
            height,
            stroke_width,
            minimum_size: min_size,
        });
    }

    Ok(())
}

fn make_rng(seed: Option<u64>) -> FrameRng {
    match seed {
        Some(seed) => FrameRng::seed_from_u64(seed),
        None => FrameRng::from_rng(&mut rand::rng()),
    }
}

fn jagged_rect_path(width: u32, height: u32, stroke_width: f64, rng: &mut FrameRng) -> String {
    let w = width as f64;
    let h = height as f64;
    let margin = EDGE_JITTER + stroke_width / 2.0;

    let rectangle = Polygon::rectangle(margin, margin, w - margin, h - margin);

    let points = hand_drawn_polygon(&rectangle, EDGE_INTERIOR_POINTS, EDGE_JITTER, rng);

    closed_path_data(&points)
}

/// Builds a validated comic panel frame as a complete SVG document.
///
/// Supplying the same seed and options produces the same SVG output.
///
/// # Errors
///
/// Returns a [`FrameError`] when any option is invalid.
///
/// # Examples
///
/// ```
/// use comikaze::frame::{FrameOptions, build_frame_svg};
///
/// let options = FrameOptions {
///     width: 300,
///     height: 400,
///     seed: Some(42),
///     ..FrameOptions::default()
/// };
///
/// let svg = build_frame_svg(&options)?;
/// assert!(svg.starts_with("<svg"));
///
/// # Ok::<(), comikaze::frame::FrameError>(())
/// ```
pub fn build_frame_svg(options: &FrameOptions) -> Result<String, FrameError> {
    options.validate()?;

    let width = options.width;
    let height = options.height;
    let color = options.color.as_str();
    let fill = options.fill.as_deref().unwrap_or("none");
    let stroke_width = options.stroke_width;

    let mut rng = make_rng(options.seed);
    let path_d = jagged_rect_path(width, height, stroke_width, &mut rng);

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <path d="{path_d}" fill="{fill}" stroke="{color}" stroke-width="{stroke_width}" stroke-linejoin="round"/>
</svg>"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngExt;

    fn parse_coordinates(path: &str) -> Vec<f64> {
        path.split_whitespace()
            .filter_map(|token| token.parse::<f64>().ok())
            .collect()
    }

    fn frame_options(seed: Option<u64>) -> FrameOptions {
        FrameOptions {
            width: 300,
            height: 400,
            color: "#000000".to_string(),
            fill: None,
            stroke_width: 3.0,
            seed,
        }
    }

    #[test]
    fn path_starts_and_ends_correctly() {
        let path = jagged_rect_path(300, 400, 3.0, &mut FrameRng::seed_from_u64(42));
        assert!(path.starts_with("M "));
        assert!(path.ends_with(" Z"));
    }

    #[test]
    fn path_has_expected_segment_count() {
        let path = jagged_rect_path(300, 400, 3.0, &mut FrameRng::seed_from_u64(42));
        let line_count = path.matches(" L ").count();
        assert_eq!(line_count, 4 * (EDGE_INTERIOR_POINTS + 1)); // 4 edges × (interior points + 1 corner)
    }

    #[test]
    fn same_seed_produces_identical_output() {
        let a = build_frame_svg(&frame_options(Some(42))).unwrap();
        let b = build_frame_svg(&frame_options(Some(42))).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_produce_different_output() {
        let a = build_frame_svg(&frame_options(Some(1))).unwrap();
        let b = build_frame_svg(&frame_options(Some(2))).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn coordinates_never_go_negative() {
        for seed in 0..50 {
            let mut rng = FrameRng::seed_from_u64(seed);
            let path = jagged_rect_path(300, 400, 3.0, &mut rng);
            for coord in parse_coordinates(&path) {
                assert!(coord >= 0.0, "found negative coordinate in: {path}");
            }
        }
    }

    #[test]
    fn rejects_dimensions_where_opposite_edges_could_overlap() {
        assert!(validate_frame_dimensions(6, 6, 3.0).is_err());
        assert!(validate_frame_dimensions(8, 8, 3.0).is_ok());
    }

    #[test]
    fn accepts_reasonable_dimensions() {
        assert!(validate_frame_dimensions(300, 400, 3.0).is_ok());
    }

    #[test]
    fn rejects_width_too_small_for_stroke() {
        assert!(validate_frame_dimensions(10, 400, 50.0).is_err());
    }

    #[test]
    fn rejects_height_too_small_for_stroke() {
        assert!(validate_frame_dimensions(400, 10, 50.0).is_err());
    }

    #[test]
    fn builder_rejects_non_finite_stroke_width() {
        let options = FrameOptions {
            stroke_width: f64::NAN,
            ..FrameOptions::default()
        };

        assert!(build_frame_svg(&options).is_err());
    }

    #[test]
    fn builder_rejects_dimensions_that_are_too_small() {
        let options = FrameOptions {
            width: 1,
            ..FrameOptions::default()
        };

        assert!(build_frame_svg(&options).is_err());
    }

    #[test]
    fn builder_rejects_unsafe_color_input() {
        let options = FrameOptions {
            color: "\"/><script>alert('unsafe')</script>".to_string(),
            ..FrameOptions::default()
        };

        assert!(build_frame_svg(&options).is_err());
    }

    #[test]
    fn builder_uses_configured_fill() {
        let options = FrameOptions {
            fill: Some("#fff8dc".to_string()),
            ..FrameOptions::default()
        };

        let svg = build_frame_svg(&options).unwrap();

        assert!(svg.contains(r##"fill="#fff8dc""##));
    }

    #[test]
    fn builder_without_fill_remains_transparent() {
        let svg = build_frame_svg(&FrameOptions::default()).unwrap();

        assert!(svg.contains(r#"fill="none""#));
    }

    #[test]
    fn builder_rejects_unsafe_fill_input() {
        let options = FrameOptions {
            fill: Some("\"/><script>alert('unsafe')</script>".to_string()),
            ..FrameOptions::default()
        };

        assert!(build_frame_svg(&options).is_err());
    }

    #[test]
    fn seeded_jitter_sequence_is_stable() {
        let mut rng = make_rng(Some(42));

        let actual = [
            rng.random_range(-EDGE_JITTER..EDGE_JITTER),
            rng.random_range(-EDGE_JITTER..EDGE_JITTER),
            rng.random_range(-EDGE_JITTER..EDGE_JITTER),
            rng.random_range(-EDGE_JITTER..EDGE_JITTER),
        ];

        let expected = [
            0.053114818005547626,
            0.08545041980628776,
            0.27293019828778986,
            -0.18819648353844665,
        ];

        assert_eq!(actual, expected);
    }

    #[test]
    fn invalid_stroke_width_returns_typed_error() {
        let error = validate_stroke_width(f64::NAN).unwrap_err();

        match error {
            FrameError::InvalidStrokeWidth { stroke_width } => {
                assert!(stroke_width.is_nan());
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn invalid_color_returns_typed_error() {
        let error = validate_color("red").unwrap_err();

        match error {
            FrameError::InvalidColor { color } => {
                assert_eq!(color, "red");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn invalid_fill_returns_typed_error() {
        let error = validate_fill("red").unwrap_err();

        match error {
            FrameError::InvalidFill { fill } => {
                assert_eq!(fill, "red");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn invalid_dimensions_return_typed_error() {
        let error = validate_frame_dimensions(6, 6, 3.0).unwrap_err();

        assert!(matches!(
            error,
            FrameError::DimensionsTooSmall {
                width: 6,
                height: 6,
                ..
            }
        ));
    }
}
