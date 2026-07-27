use crate::geometry::Point;

pub(crate) fn is_valid_stroke_width(stroke_width: f64) -> bool {
    stroke_width.is_finite() && stroke_width > 0.0
}

pub(crate) fn is_valid_color(color: &str) -> bool {
    color == "currentColor" || is_valid_hex_color(color)
}

fn is_valid_hex_color(color: &str) -> bool {
    let Some(hex_digits) = color.strip_prefix('#') else {
        return false;
    };

    matches!(hex_digits.len(), 3 | 4 | 6 | 8)
        && hex_digits
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

/// Converts points into closed SVG path data.
pub(crate) fn closed_path_data(points: &[Point]) -> String {
    let Some(first) = points.first() else {
        return String::new();
    };

    let mut data = format!("M {} {}", first.x, first.y);

    for point in &points[1..] {
        data.push_str(&format!(" L {} {}", point.x, point.y));
    }

    data.push_str(" Z");
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_point_list_produces_empty_path_data() {
        assert_eq!(closed_path_data(&[]), "");
    }

    #[test]
    fn points_are_converted_to_closed_path_data() {
        let points = [
            Point::new(10.0, 20.0),
            Point::new(30.0, 40.0),
            Point::new(50.0, 60.0),
        ];

        assert_eq!(closed_path_data(&points), "M 10 20 L 30 40 L 50 60 Z");
    }
}
