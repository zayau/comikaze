//! Fundamental two-dimensional geometry types.

const GEOMETRY_EPSILON: f64 = 1e-9;

/// A point in two-dimensional SVG space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f64,

    /// Vertical coordinate.
    pub y: f64,
}

impl Point {
    /// Creates a point from its horizontal and vertical coordinates.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A point's position relative to a directed line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineSide {
    /// The point has a negative signed distance from the line.
    Negative,

    /// The point lies on the line.
    On,

    /// The point has a positive signed distance from the line.
    Positive,
}

/// An infinite directed line passing through two points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Line {
    start: Point,
    end: Point,
}

impl Line {
    /// Creates a line from two distinct finite points.
    pub(crate) fn new(start: Point, end: Point) -> Self {
        assert!(
            start.x.is_finite() && start.y.is_finite() && end.x.is_finite() && end.y.is_finite(),
            "line points must have finite coordinates"
        );

        assert!(start != end, "a line requires two distinct points");

        Self { start, end }
    }

    /// Calculates a point's signed perpendicular distance from the line.
    pub(crate) fn signed_distance_to(&self, point: Point) -> f64 {
        let direction_x = self.end.x - self.start.x;
        let direction_y = self.end.y - self.start.y;

        let point_x = point.x - self.start.x;
        let point_y = point.y - self.start.y;

        let cross_product = direction_x * point_y - direction_y * point_x;
        let line_length = direction_x.hypot(direction_y);

        cross_product / line_length
    }

    /// Classifies a point as being on either side of the line or on the line.
    pub(crate) fn side_of(&self, point: Point) -> LineSide {
        let distance = self.signed_distance_to(point);

        if distance > GEOMETRY_EPSILON {
            LineSide::Positive
        } else if distance < -GEOMETRY_EPSILON {
            LineSide::Negative
        } else {
            LineSide::On
        }
    }

    /// Calculates where the line crosses a segment.
    fn intersection_with_segment(&self, segment_start: Point, segment_end: Point) -> Point {
        let start_distance = self.signed_distance_to(segment_start);
        let end_distance = self.signed_distance_to(segment_end);

        let progress = start_distance / (start_distance - end_distance);

        Point::new(
            segment_start.x + (segment_end.x - segment_start.x) * progress,
            segment_start.y + (segment_end.y - segment_start.y) * progress,
        )
    }
}

/// An internally generated polygon with ordered boundary vertices.
///
/// The first vertex is not repeated at the end. Closing the polygon is the
/// renderer's responsibility.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Polygon {
    vertices: Vec<Point>,
}

impl Polygon {
    /// Creates a polygon from at least three finite vertices.
    pub(crate) fn new(vertices: Vec<Point>) -> Self {
        assert!(
            vertices.len() >= 3,
            "a polygon requires at least three vertices"
        );

        assert!(
            vertices
                .iter()
                .all(|point| point.x.is_finite() && point.y.is_finite()),
            "polygon vertices must have finite coordinates"
        );

        Self { vertices }
    }

    /// Creates an axis-aligned rectangular polygon.
    pub(crate) fn rectangle(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        assert!(
            left < right && top < bottom,
            "rectangle bounds must have positive width and height"
        );

        Self::new(vec![
            Point::new(left, top),
            Point::new(right, top),
            Point::new(right, bottom),
            Point::new(left, bottom),
        ])
    }

    /// Returns the polygon's ordered boundary vertices.
    pub(crate) fn vertices(&self) -> &[Point] {
        &self.vertices
    }

    /// Splits the polygon into its negative and positive sides.
    pub(crate) fn split(&self, line: Line) -> Option<(Self, Self)> {
        let mut negative_vertices = Vec::new();
        let mut positive_vertices = Vec::new();

        for index in 0..self.vertices.len() {
            let current = self.vertices[index];
            let next = self.vertices[(index + 1) % self.vertices.len()];

            let current_side = line.side_of(current);
            let next_side = line.side_of(next);

            match current_side {
                LineSide::Negative => negative_vertices.push(current),
                LineSide::Positive => positive_vertices.push(current),
                LineSide::On => {
                    negative_vertices.push(current);
                    positive_vertices.push(current);
                }
            }

            let crosses_line = current_side == LineSide::Negative
                && next_side == LineSide::Positive
                || current_side == LineSide::Positive && next_side == LineSide::Negative;

            if crosses_line {
                let intersection = line.intersection_with_segment(current, next);

                negative_vertices.push(intersection);
                positive_vertices.push(intersection);
            }
        }

        if negative_vertices.len() < 3 || positive_vertices.len() < 3 {
            return None;
        }

        Some((Self::new(negative_vertices), Self::new(positive_vertices)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_vertices_follow_its_boundary() {
        let polygon = Polygon::rectangle(10.0, 20.0, 110.0, 220.0);

        assert_eq!(
            polygon.vertices(),
            &[
                Point::new(10.0, 20.0),
                Point::new(110.0, 20.0),
                Point::new(110.0, 220.0),
                Point::new(10.0, 220.0),
            ]
        );
    }

    #[test]
    #[should_panic(expected = "a polygon requires at least three vertices")]
    fn polygon_requires_at_least_three_vertices() {
        Polygon::new(vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)]);
    }

    #[test]
    fn horizontal_line_classifies_points_by_side() {
        let line = Line::new(Point::new(0.0, 50.0), Point::new(100.0, 50.0));

        assert_eq!(line.side_of(Point::new(50.0, 25.0)), LineSide::Negative);

        assert_eq!(line.side_of(Point::new(50.0, 50.0)), LineSide::On);

        assert_eq!(line.side_of(Point::new(50.0, 75.0)), LineSide::Positive);
    }

    #[test]
    fn signed_distance_is_perpendicular_distance() {
        let line = Line::new(Point::new(0.0, 50.0), Point::new(100.0, 50.0));

        assert_eq!(line.signed_distance_to(Point::new(50.0, 25.0)), -25.0);

        assert_eq!(line.signed_distance_to(Point::new(50.0, 75.0)), 25.0);
    }

    #[test]
    fn reversing_line_reverses_its_sides() {
        let forward = Line::new(Point::new(0.0, 50.0), Point::new(100.0, 50.0));

        let reversed = Line::new(Point::new(100.0, 50.0), Point::new(0.0, 50.0));

        let point = Point::new(50.0, 75.0);

        assert_eq!(forward.side_of(point), LineSide::Positive);
        assert_eq!(reversed.side_of(point), LineSide::Negative);
    }

    #[test]
    #[should_panic(expected = "a line requires two distinct points")]
    fn line_requires_distinct_points() {
        Line::new(Point::new(10.0, 20.0), Point::new(10.0, 20.0));
    }
}
