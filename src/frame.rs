use chacha20::ChaCha12Rng;
use rand::{RngExt, SeedableRng};

type FrameRng = ChaCha12Rng;

const EDGE_JITTER: f64 = 1.0;
const EDGE_INTERIOR_POINTS: usize = 4;

#[derive(Debug, Clone)]
pub struct FrameOptions {
    pub width: u32,
    pub height: u32,
    pub color: String,
    pub stroke_width: f64,
    pub seed: Option<u64>,
}

impl Default for FrameOptions {
    fn default() -> Self {
        Self {
            width: 400,
            height: 600,
            color: "#000000".to_string(),
            stroke_width: 3.0,
            seed: None,
        }
    }
}

impl FrameOptions {
    pub fn validate(&self) -> Result<(), String> {
        validate_frame_dimensions(self.width, self.height, self.stroke_width)?;
        validate_color(&self.color)?;

        Ok(())
    }
}

pub fn validate_stroke_width(stroke_width: f64) -> Result<(), String> {
    if !stroke_width.is_finite() || stroke_width <= 0.0 {
        return Err(format!(
            "stroke width must be a finite number greater than 0, got {stroke_width}"
        ));
    }

    Ok(())
}

pub fn validate_color(color: &str) -> Result<(), String> {
    if color == "currentColor" || is_valid_hex_color(color) {
        return Ok(());
    }

    Err(format!(
        "`{color}` is not a valid color — use a hex code (#rgb, #rrggbb, #rrggbbaa) or `currentColor`"
    ))
}

fn is_valid_hex_color(color: &str) -> bool {
    let Some(hex_digits) = color.strip_prefix('#') else {
        return false;
    };

    let valid_length = matches!(hex_digits.len(), 3 | 4 | 6 | 8);
    let valid_characters = hex_digits
        .chars()
        .all(|character| character.is_ascii_hexdigit());

    valid_length && valid_characters
}

pub fn validate_frame_dimensions(width: u32, height: u32, stroke_width: f64) -> Result<(), String> {
    validate_stroke_width(stroke_width)?;

    let margin = EDGE_JITTER + stroke_width / 2.0;
    let minimum_inner_span = EDGE_JITTER * 2.0;
    let min_size = margin * 2.0 + minimum_inner_span;

    if (width as f64) <= min_size || (height as f64) <= min_size {
        return Err(format!(
            "width and height must each be greater than {min_size:.1}px at stroke-width {stroke_width} — got {width}x{height}"
        ));
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

    let corners = [
        (margin, margin),         // top-left
        (w - margin, margin),     // top-right
        (w - margin, h - margin), // bottom-right
        (margin, h - margin),     // bottom-left
    ];

    let mut d = format!("M {} {}", corners[0].0, corners[0].1);

    for i in 0..4 {
        let start = corners[i];
        let end = corners[(i + 1) % 4];
        d.push_str(&wobbly_edge(
            start,
            end,
            EDGE_INTERIOR_POINTS,
            EDGE_JITTER,
            rng,
        ));
    }

    d.push_str(" Z");
    d
}

fn wobbly_edge(
    start: (f64, f64),
    end: (f64, f64),
    interior_points: usize,
    jitter: f64,
    rng: &mut FrameRng,
) -> String {
    let mut out = String::new();

    let direction_x = end.0 - start.0;
    let direction_y = end.1 - start.1;
    let edge_length = direction_x.hypot(direction_y);

    if edge_length == 0.0 {
        out.push_str(&format!(" L {} {}", end.0, end.1));
        return out;
    }

    let normal_x = -direction_y / edge_length;
    let normal_y = direction_x / edge_length;

    for index in 1..=interior_points {
        let progress = index as f64 / (interior_points + 1) as f64;

        let base_x = start.0 + direction_x * progress;
        let base_y = start.1 + direction_y * progress;
        let offset = rng.random_range(-jitter..jitter);

        let x = base_x + normal_x * offset;
        let y = base_y + normal_y * offset;

        out.push_str(&format!(" L {x} {y}"));
    }

    out.push_str(&format!(" L {} {}", end.0, end.1));
    out
}

pub fn build_frame_svg(options: &FrameOptions) -> Result<String, String> {
    options.validate()?;

    let width = options.width;
    let height = options.height;
    let color = options.color.as_str();
    let stroke_width = options.stroke_width;

    let mut rng = make_rng(options.seed);
    let path_d = jagged_rect_path(width, height, stroke_width, &mut rng);

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <path d="{path_d}" fill="none" stroke="{color}" stroke-width="{stroke_width}" stroke-linejoin="round"/>
</svg>"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn wobbly_edge_produces_expected_segment_count() {
        let edge = wobbly_edge(
            (0.0, 0.0),
            (100.0, 0.0),
            4,
            3.0,
            &mut FrameRng::seed_from_u64(42),
        );
        assert_eq!(edge.matches(" L ").count(), 5); // 4 interior points + final corner
    }

    #[test]
    fn horizontal_edge_does_not_jitter_along_its_direction() {
        let edge = wobbly_edge(
            (0.0, 0.0),
            (100.0, 0.0),
            4,
            10.0,
            &mut FrameRng::seed_from_u64(42),
        );

        let coordinates = parse_coordinates(&edge);
        let expected_x_coordinates = [20.0, 40.0, 60.0, 80.0, 100.0];

        for (point, expected_x) in coordinates.chunks_exact(2).zip(expected_x_coordinates) {
            assert!(
                (point[0] - expected_x).abs() < f64::EPSILON,
                "expected x={expected_x}, found x={}",
                point[0]
            );
        }
    }

    #[test]
    fn vertical_edge_does_not_jitter_along_its_direction() {
        let edge = wobbly_edge(
            (0.0, 0.0),
            (0.0, 100.0),
            4,
            10.0,
            &mut FrameRng::seed_from_u64(42),
        );

        let coordinates = parse_coordinates(&edge);
        let expected_y_coordinates = [20.0, 40.0, 60.0, 80.0, 100.0];

        for (point, expected_y) in coordinates.chunks_exact(2).zip(expected_y_coordinates) {
            assert!(
                (point[1] - expected_y).abs() < f64::EPSILON,
                "expected y={expected_y}, found y={}",
                point[1]
            );
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
}
