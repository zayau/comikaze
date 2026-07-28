//! Page-level comic panel layout generation.

use crate::geometry::{Line, Point, Polygon};
use crate::svg::{closed_path_data, is_valid_color, is_valid_stroke_width};
use crate::topology::BoundaryGraph;
use std::fmt;

/// An error produced while creating or modifying a page layout.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutError {
    /// One or both page dimensions are zero.
    InvalidDimensions {
        /// Invalid page width.
        width: u32,

        /// Invalid page height.
        height: u32,
    },

    /// The cut points are equal or contain non-finite coordinates.
    InvalidCut {
        /// First cut point.
        start: Point,

        /// Second cut point.
        end: Point,
    },

    /// The requested panel does not exist.
    PanelNotFound {
        /// Requested panel index.
        panel_index: usize,

        /// Number of panels currently in the layout.
        panel_count: usize,
    },

    /// The cut does not divide the selected panel into two polygons.
    CutDoesNotSplitPanel {
        /// Index of the selected panel.
        panel_index: usize,
    },

    /// The SVG stroke width is invalid.
    InvalidStrokeWidth {
        /// Invalid stroke width.
        stroke_width: f64,
    },

    /// The SVG stroke color is unsupported.
    InvalidColor {
        /// Invalid color.
        color: String,
    },

    /// The requested gutter is invalid.
    InvalidGutter {
        /// Invalid gutter width.
        gutter: f64,
    },

    /// The gutter would collapse a panel.
    GutterTooLarge {
        /// Requested gutter width.
        gutter: f64,

        /// Panel that cannot contain the gutter.
        panel_index: usize,
    },

    /// The hand-drawn point spacing is invalid.
    InvalidPointSpacing {
        /// Invalid point spacing.
        point_spacing: f64,
    },

    /// The hand-drawn jitter is invalid.
    InvalidJitter {
        /// Invalid jitter.
        jitter: f64,
    },

    /// A hand-drawn panel boundary could not be
    /// assembled.
    HandDrawnRenderingFailed {
        /// Panel that could not be rendered.
        panel_index: usize,
    },

    /// Point spacing would generate too many
    /// boundary points.
    PointSpacingTooSmall {
        /// Requested point spacing.
        point_spacing: f64,
    },

    /// Jitter would move a boundary beyond its
    /// available inward offset.
    JitterTooLarge {
        /// Requested jitter.
        jitter: f64,

        /// Maximum safe jitter.
        maximum: f64,
    },
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions { width, height } => {
                write!(
                    formatter,
                    "page width and height must be greater than zero, got {width}x{height}"
                )
            }
            Self::InvalidCut { start, end } => {
                write!(
                    formatter,
                    "cut points must be distinct and finite, got ({}, {}) to ({}, {})",
                    start.x, start.y, end.x, end.y
                )
            }
            Self::PanelNotFound {
                panel_index,
                panel_count,
            } => {
                write!(
                    formatter,
                    "panel {panel_index} does not exist; layout contains {panel_count} panels"
                )
            }
            Self::CutDoesNotSplitPanel { panel_index } => {
                write!(
                    formatter,
                    "cut does not divide panel {panel_index} into two panels"
                )
            }
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
            Self::InvalidGutter { gutter } => {
                write!(
                    formatter,
                    "gutter must be a finite non-negative number, got {gutter}"
                )
            }
            Self::GutterTooLarge {
                gutter,
                panel_index,
            } => {
                write!(
                    formatter,
                    "gutter {gutter} is too large for panel {panel_index}"
                )
            }
            Self::InvalidPointSpacing { point_spacing } => {
                write!(
                    formatter,
                    "point spacing must be a finite number \
         greater than 0, got {point_spacing}"
                )
            }
            Self::InvalidJitter { jitter } => {
                write!(
                    formatter,
                    "jitter must be a finite non-negative \
         number, got {jitter}"
                )
            }
            Self::HandDrawnRenderingFailed { panel_index } => {
                write!(
                    formatter,
                    "could not assemble the hand-drawn \
         boundary for panel {panel_index}"
                )
            }
            Self::PointSpacingTooSmall { point_spacing } => {
                write!(
                    formatter,
                    "point spacing {point_spacing} would \
         generate too many boundary points; \
         increase the spacing"
                )
            }
            Self::JitterTooLarge { jitter, maximum } => {
                write!(
                    formatter,
                    "jitter {jitter} is too large for the \
         available panel inset; maximum is \
         {maximum}"
                )
            }
        }
    }
}

impl std::error::Error for LayoutError {}

/// Options controlling coordinated hand-drawn
/// layout boundaries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandDrawnOptions {
    /// Approximate maximum distance between
    /// neighboring points along a boundary.
    pub point_spacing: f64,

    /// Maximum perpendicular displacement from
    /// the exact boundary.
    pub jitter: f64,

    /// Seed used for reproducible boundary noise.
    pub seed: u64,
}

impl Default for HandDrawnOptions {
    fn default() -> Self {
        Self {
            point_spacing: 40.0,
            jitter: 1.0,
            seed: 0,
        }
    }
}

/// Options controlling layout SVG rendering.
#[derive(Debug, Clone)]
pub struct LayoutSvgOptions {
    /// Panel stroke color.
    pub color: String,

    /// Panel stroke width in SVG units.
    pub stroke_width: f64,

    /// Clear whitespace between neighboring
    /// panel strokes.
    ///
    /// Zero disables visible gutter whitespace.
    pub gutter: f64,

    /// Enables coordinated hand-drawn boundaries.
    ///
    /// `None` preserves exact straight polygon
    /// edges.
    pub hand_drawn: Option<HandDrawnOptions>,
}

impl Default for LayoutSvgOptions {
    fn default() -> Self {
        Self {
            color: "#000000".to_string(),
            stroke_width: 3.0,
            gutter: 0.0,
            hand_drawn: None,
        }
    }
}

/// A comic page containing one or more polygonal panels.
#[derive(Debug, Clone)]
pub struct PageLayout {
    width: u32,
    height: u32,
    panels: Vec<Polygon>,
}

impl PageLayout {
    /// Creates a page containing one rectangular panel.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::InvalidDimensions`] when either dimension is zero.
    pub fn new(width: u32, height: u32) -> Result<Self, LayoutError> {
        if width == 0 || height == 0 {
            return Err(LayoutError::InvalidDimensions { width, height });
        }

        let page = Polygon::rectangle(0.0, 0.0, width as f64, height as f64);

        Ok(Self {
            width,
            height,
            panels: vec![page],
        })
    }

    /// Returns the page width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the page height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the number of panels.
    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    /// Returns the ordered vertices of one panel.
    pub fn panel_vertices(&self, panel_index: usize) -> Option<&[Point]> {
        self.panels
            .get(panel_index)
            .map(|polygon| polygon.vertices())
    }

    /// Splits one panel using an infinite line through two points.
    ///
    /// # Errors
    ///
    /// Returns an error when the panel does not exist, the cut is invalid, or
    /// the line does not divide the selected panel.
    pub fn split_panel(
        &mut self,
        panel_index: usize,
        start: Point,
        end: Point,
    ) -> Result<(), LayoutError> {
        let panel_count = self.panels.len();

        let panel = self
            .panels
            .get(panel_index)
            .ok_or(LayoutError::PanelNotFound {
                panel_index,
                panel_count,
            })?;

        let cut_is_finite =
            start.x.is_finite() && start.y.is_finite() && end.x.is_finite() && end.y.is_finite();

        if !cut_is_finite || start == end {
            return Err(LayoutError::InvalidCut { start, end });
        }

        let line = Line::new(start, end);

        let Some((negative, positive)) = panel.split(line) else {
            return Err(LayoutError::CutDoesNotSplitPanel { panel_index });
        };

        self.panels[panel_index] = negative;
        self.panels.insert(panel_index + 1, positive);

        Ok(())
    }
}

/// Builds a page layout as a complete SVG document.
///
/// A positive gutter insets every panel enough to preserve the requested clear
/// whitespace between neighboring strokes.
///
/// # Errors
///
/// Returns an error when a rendering option is invalid or a gutter would
/// collapse a panel.
pub fn build_layout_svg(
    layout: &PageLayout,
    options: &LayoutSvgOptions,
) -> Result<String, LayoutError> {
    if !is_valid_stroke_width(options.stroke_width) {
        return Err(LayoutError::InvalidStrokeWidth {
            stroke_width: options.stroke_width,
        });
    }

    if !is_valid_color(&options.color) {
        return Err(LayoutError::InvalidColor {
            color: options.color.clone(),
        });
    }

    if !options.gutter.is_finite() || options.gutter < 0.0 {
        return Err(LayoutError::InvalidGutter {
            gutter: options.gutter,
        });
    }

    if let Some(hand_drawn) = &options.hand_drawn {
        if !hand_drawn.point_spacing.is_finite() || hand_drawn.point_spacing <= 0.0 {
            return Err(LayoutError::InvalidPointSpacing {
                point_spacing: hand_drawn.point_spacing,
            });
        }

        if !hand_drawn.jitter.is_finite() || hand_drawn.jitter < 0.0 {
            return Err(LayoutError::InvalidJitter {
                jitter: hand_drawn.jitter,
            });
        }
    }

    let width = layout.width;
    let height = layout.height;
    let color = options.color.as_str();
    let stroke_width = options.stroke_width;

    let inset_distance = (options.gutter + options.stroke_width) / 2.0;

    let hand_drawn_state = if let Some(hand_drawn) = &options.hand_drawn {
        if hand_drawn.jitter > inset_distance {
            return Err(LayoutError::JitterTooLarge {
                jitter: hand_drawn.jitter,
                maximum: inset_distance,
            });
        }

        let graph = BoundaryGraph::from_panels(&layout.panels);

        if !graph.supports_point_spacing(hand_drawn.point_spacing) {
            return Err(LayoutError::PointSpacingTooSmall {
                point_spacing: hand_drawn.point_spacing,
            });
        }

        let profiles =
            graph.hand_drawn_profiles(hand_drawn.point_spacing, hand_drawn.jitter, hand_drawn.seed);

        Some((graph, profiles))
    } else {
        None
    };

    let mut paths = Vec::with_capacity(layout.panels.len());

    for (panel_index, panel) in layout.panels.iter().enumerate() {
        let path_data = if let Some((graph, profiles)) = &hand_drawn_state {
            if panel.inset(inset_distance).is_none() {
                return Err(LayoutError::GutterTooLarge {
                    gutter: options.gutter,
                    panel_index,
                });
            }

            let points = graph
                .hand_drawn_panel_points(panel_index, panel, profiles, inset_distance)
                .ok_or(LayoutError::HandDrawnRenderingFailed { panel_index })?;

            closed_path_data(&points)
        } else if options.gutter == 0.0 {
            closed_path_data(panel.vertices())
        } else {
            let inset = panel
                .inset(inset_distance)
                .ok_or(LayoutError::GutterTooLarge {
                    gutter: options.gutter,
                    panel_index,
                })?;

            closed_path_data(inset.vertices())
        };

        paths.push(format!(
        r#"  <path d="{path_data}" fill="none" stroke="{color}" stroke-width="{stroke_width}" stroke-linejoin="round"/>"#
    ));
    }

    let paths = paths.join("\n");

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
{paths}
</svg>"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_points_close(actual: &[Point], expected: &[Point]) {
        assert_eq!(actual.len(), expected.len());

        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual.x - expected.x).abs() < 1e-9 && (actual.y - expected.y).abs() < 1e-9,
                "expected {expected:?}, got {actual:?}"
            );
        }
    }

    #[test]
    fn new_layout_contains_one_page_sized_panel() {
        let layout = PageLayout::new(100, 200).unwrap();

        assert_eq!(layout.width(), 100);
        assert_eq!(layout.height(), 200);
        assert_eq!(layout.panel_count(), 1);

        assert_eq!(
            layout.panel_vertices(0).unwrap(),
            &[
                Point::new(0.0, 0.0),
                Point::new(100.0, 0.0),
                Point::new(100.0, 200.0),
                Point::new(0.0, 200.0),
            ]
        );
    }

    #[test]
    fn horizontal_cut_creates_two_matching_panels() {
        let mut layout = PageLayout::new(100, 100).unwrap();

        layout
            .split_panel(0, Point::new(0.0, 50.0), Point::new(100.0, 50.0))
            .unwrap();

        assert_eq!(layout.panel_count(), 2);

        assert_points_close(
            layout.panel_vertices(0).unwrap(),
            &[
                Point::new(0.0, 0.0),
                Point::new(100.0, 0.0),
                Point::new(100.0, 50.0),
                Point::new(0.0, 50.0),
            ],
        );

        assert_points_close(
            layout.panel_vertices(1).unwrap(),
            &[
                Point::new(100.0, 50.0),
                Point::new(100.0, 100.0),
                Point::new(0.0, 100.0),
                Point::new(0.0, 50.0),
            ],
        );
    }

    #[test]
    fn slanted_cut_creates_matching_slanted_edges() {
        let mut layout = PageLayout::new(100, 100).unwrap();

        layout
            .split_panel(0, Point::new(0.0, 40.0), Point::new(100.0, 60.0))
            .unwrap();

        assert_points_close(
            layout.panel_vertices(0).unwrap(),
            &[
                Point::new(0.0, 0.0),
                Point::new(100.0, 0.0),
                Point::new(100.0, 60.0),
                Point::new(0.0, 40.0),
            ],
        );

        assert_points_close(
            layout.panel_vertices(1).unwrap(),
            &[
                Point::new(100.0, 60.0),
                Point::new(100.0, 100.0),
                Point::new(0.0, 100.0),
                Point::new(0.0, 40.0),
            ],
        );
    }

    #[test]
    fn cuts_can_be_applied_sequentially() {
        let mut layout = PageLayout::new(120, 160).unwrap();

        layout
            .split_panel(0, Point::new(0.0, 60.0), Point::new(120.0, 70.0))
            .unwrap();

        layout
            .split_panel(0, Point::new(60.0, 0.0), Point::new(70.0, 70.0))
            .unwrap();

        assert_eq!(layout.panel_count(), 3);
    }

    #[test]
    fn unsuccessful_cut_does_not_modify_layout() {
        let mut layout = PageLayout::new(100, 100).unwrap();

        let error = layout
            .split_panel(0, Point::new(0.0, -10.0), Point::new(100.0, -10.0))
            .unwrap_err();

        assert!(matches!(
            error,
            LayoutError::CutDoesNotSplitPanel { panel_index: 0 }
        ));

        assert_eq!(layout.panel_count(), 1);
    }

    #[test]
    fn layout_svg_contains_one_path_per_panel() {
        let mut layout = PageLayout::new(100, 100).unwrap();

        layout
            .split_panel(0, Point::new(0.0, 40.0), Point::new(100.0, 60.0))
            .unwrap();

        let svg = build_layout_svg(&layout, &LayoutSvgOptions::default()).unwrap();

        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert_eq!(svg.matches("<path ").count(), 2);
        assert!(svg.contains(r#"viewBox="0 0 100 100""#));
    }

    #[test]
    fn layout_svg_rejects_invalid_stroke_width() {
        let layout = PageLayout::new(100, 100).unwrap();

        let options = LayoutSvgOptions {
            stroke_width: f64::NAN,
            ..LayoutSvgOptions::default()
        };

        assert!(matches!(
            build_layout_svg(&layout, &options),
            Err(LayoutError::InvalidStrokeWidth { stroke_width })
                if stroke_width.is_nan()
        ));
    }

    #[test]
    fn layout_svg_rejects_invalid_color() {
        let layout = PageLayout::new(100, 100).unwrap();

        let options = LayoutSvgOptions {
            color: "red".to_string(),
            ..LayoutSvgOptions::default()
        };

        assert!(matches!(
            build_layout_svg(&layout, &options),
            Err(LayoutError::InvalidColor { color }) if color == "red"
        ));
    }

    #[test]
    fn positive_gutter_insets_rendered_panels() {
        let mut layout = PageLayout::new(100, 100).unwrap();

        layout
            .split_panel(0, Point::new(0.0, 50.0), Point::new(100.0, 50.0))
            .unwrap();

        let options = LayoutSvgOptions {
            stroke_width: 4.0,
            gutter: 12.0,
            ..LayoutSvgOptions::default()
        };

        let svg = build_layout_svg(&layout, &options).unwrap();

        assert_eq!(svg.matches("<path ").count(), 2);
        assert!(!svg.contains(r#"d="M 0 0"#));
        assert!(svg.contains(r#"stroke-width="4""#));
    }

    #[test]
    fn negative_gutter_is_rejected() {
        let layout = PageLayout::new(100, 100).unwrap();

        let options = LayoutSvgOptions {
            gutter: -1.0,
            ..LayoutSvgOptions::default()
        };

        assert!(matches!(
            build_layout_svg(&layout, &options),
            Err(LayoutError::InvalidGutter { gutter })
                if gutter == -1.0
        ));
    }

    #[test]
    fn gutter_that_collapses_panel_is_rejected() {
        let layout = PageLayout::new(100, 100).unwrap();

        let options = LayoutSvgOptions {
            stroke_width: 4.0,
            gutter: 200.0,
            ..LayoutSvgOptions::default()
        };

        assert!(matches!(
            build_layout_svg(&layout, &options),
            Err(LayoutError::GutterTooLarge {
                gutter: 200.0,
                panel_index: 0
            })
        ));
    }

    #[test]
    fn invalid_hand_drawn_point_spacing_is_rejected() {
        let layout = PageLayout::new(100, 100).unwrap();

        let options = LayoutSvgOptions {
            hand_drawn: Some(HandDrawnOptions {
                point_spacing: 0.0,
                ..HandDrawnOptions::default()
            }),
            ..LayoutSvgOptions::default()
        };

        assert!(matches!(
            build_layout_svg(&layout, &options),
            Err(LayoutError::InvalidPointSpacing { point_spacing: 0.0 })
        ));
    }

    #[test]
    fn invalid_hand_drawn_jitter_is_rejected() {
        let layout = PageLayout::new(100, 100).unwrap();

        let options = LayoutSvgOptions {
            hand_drawn: Some(HandDrawnOptions {
                jitter: f64::NAN,
                ..HandDrawnOptions::default()
            }),
            ..LayoutSvgOptions::default()
        };

        assert!(matches!(
            build_layout_svg(&layout, &options),
            Err(LayoutError::InvalidJitter { jitter })
                if jitter.is_nan()
        ));
    }

    #[test]
    fn hand_drawn_layout_is_seeded_and_deterministic() {
        let mut layout = PageLayout::new(100, 100).unwrap();

        layout
            .split_panel(0, Point::new(0.0, 50.0), Point::new(100.0, 50.0))
            .unwrap();

        let options = LayoutSvgOptions {
            stroke_width: 4.0,
            gutter: 12.0,
            hand_drawn: Some(HandDrawnOptions {
                point_spacing: 20.0,
                jitter: 2.0,
                seed: 42,
            }),
            ..LayoutSvgOptions::default()
        };

        let first = build_layout_svg(&layout, &options).unwrap();

        let second = build_layout_svg(&layout, &options).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.matches("<path ").count(), 2);

        let straight = build_layout_svg(
            &layout,
            &LayoutSvgOptions {
                stroke_width: 4.0,
                gutter: 12.0,
                ..LayoutSvgOptions::default()
            },
        )
        .unwrap();

        assert_ne!(first, straight);

        let mut changed_seed = options.clone();

        changed_seed
            .hand_drawn
            .as_mut()
            .expect("hand drawing should be enabled")
            .seed = 43;

        let different = build_layout_svg(&layout, &changed_seed).unwrap();

        assert_ne!(first, different);
    }

    #[test]
    fn extremely_small_point_spacing_is_rejected() {
        let layout = PageLayout::new(100, 100).unwrap();

        let options = LayoutSvgOptions {
            hand_drawn: Some(HandDrawnOptions {
                point_spacing: 0.0001,
                ..HandDrawnOptions::default()
            }),
            ..LayoutSvgOptions::default()
        };

        assert!(matches!(
            build_layout_svg(&layout, &options),
            Err(LayoutError::PointSpacingTooSmall {
                point_spacing: 0.0001
            })
        ));
    }

    #[test]
    fn jitter_larger_than_inset_is_rejected() {
        let layout = PageLayout::new(100, 100).unwrap();

        let options = LayoutSvgOptions {
            stroke_width: 4.0,
            gutter: 12.0,
            hand_drawn: Some(HandDrawnOptions {
                jitter: 9.0,
                ..HandDrawnOptions::default()
            }),
            ..LayoutSvgOptions::default()
        };

        assert!(matches!(
            build_layout_svg(&layout, &options),
            Err(LayoutError::JitterTooLarge {
                jitter: 9.0,
                maximum: 8.0
            })
        ));
    }

    #[test]
    fn collapsed_hand_drawn_panel_returns_gutter_error() {
        let layout = PageLayout::new(100, 100).unwrap();

        let options = LayoutSvgOptions {
            stroke_width: 4.0,
            gutter: 200.0,
            hand_drawn: Some(HandDrawnOptions::default()),
            ..LayoutSvgOptions::default()
        };

        assert!(matches!(
            build_layout_svg(&layout, &options),
            Err(LayoutError::GutterTooLarge {
                gutter: 200.0,
                panel_index: 0
            })
        ));
    }
}
