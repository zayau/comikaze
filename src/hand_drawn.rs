use crate::geometry::{Point, Polygon};
use rand::{Rng, RngExt};

pub(crate) fn hand_drawn_edge(
    start: Point,
    end: Point,
    interior_points: usize,
    jitter: f64,
    rng: &mut impl Rng,
) -> Vec<Point> {
    let mut points = Vec::with_capacity(interior_points + 2);
    points.push(start);

    let direction_x = end.x - start.x;
    let direction_y = end.y - start.y;
    let edge_length = direction_x.hypot(direction_y);

    if edge_length == 0.0 {
        points.push(end);
        return points;
    }

    let normal_x = -direction_y / edge_length;
    let normal_y = direction_x / edge_length;

    for index in 1..=interior_points {
        let progress = index as f64 / (interior_points + 1) as f64;

        let base_x = start.x + direction_x * progress;
        let base_y = start.y + direction_y * progress;

        let offset = if jitter == 0.0 {
            0.0
        } else {
            rng.random_range(-jitter..jitter)
        };

        points.push(Point::new(
            base_x + normal_x * offset,
            base_y + normal_y * offset,
        ));
    }

    points.push(end);
    points
}

pub(crate) fn hand_drawn_polygon(
    polygon: &Polygon,
    interior_points: usize,
    jitter: f64,
    rng: &mut impl Rng,
) -> Vec<Point> {
    let vertices = polygon.vertices();
    let mut points = vec![vertices[0]];

    for index in 0..vertices.len() {
        let start = vertices[index];
        let end = vertices[(index + 1) % vertices.len()];

        let edge_points = hand_drawn_edge(start, end, interior_points, jitter, rng);

        // The previous edge already added this edge's starting vertex.
        points.extend(edge_points.into_iter().skip(1));
    }

    points
}

#[cfg(test)]
mod tests {
    use super::*;
    use chacha20::ChaCha12Rng;
    use rand::SeedableRng;

    #[test]
    fn generated_edge_includes_both_endpoints() {
        let start = Point::new(10.0, 20.0);
        let end = Point::new(100.0, 80.0);

        let points = hand_drawn_edge(start, end, 4, 1.0, &mut ChaCha12Rng::seed_from_u64(42));

        assert_eq!(points.len(), 6);
        assert_eq!(points.first(), Some(&start));
        assert_eq!(points.last(), Some(&end));
    }

    #[test]
    fn zero_jitter_produces_evenly_spaced_points() {
        let points = hand_drawn_edge(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            3,
            0.0,
            &mut ChaCha12Rng::seed_from_u64(42),
        );

        assert_eq!(
            points,
            vec![
                Point::new(0.0, 0.0),
                Point::new(25.0, 0.0),
                Point::new(50.0, 0.0),
                Point::new(75.0, 0.0),
                Point::new(100.0, 0.0),
            ]
        );
    }

    #[test]
    fn horizontal_edge_does_not_jitter_horizontally() {
        let points = hand_drawn_edge(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            4,
            10.0,
            &mut ChaCha12Rng::seed_from_u64(42),
        );

        let expected_x_coordinates = [0.0, 20.0, 40.0, 60.0, 80.0, 100.0];

        for (point, expected_x) in points.iter().zip(expected_x_coordinates) {
            assert_eq!(point.x, expected_x);
        }
    }

    #[test]
    fn vertical_edge_does_not_jitter_vertically() {
        let points = hand_drawn_edge(
            Point::new(0.0, 0.0),
            Point::new(0.0, 100.0),
            4,
            10.0,
            &mut ChaCha12Rng::seed_from_u64(42),
        );

        let expected_y_coordinates = [0.0, 20.0, 40.0, 60.0, 80.0, 100.0];

        for (point, expected_y) in points.iter().zip(expected_y_coordinates) {
            assert_eq!(point.y, expected_y);
        }
    }

    #[test]
    fn hand_drawn_polygon_supports_non_rectangular_shapes() {
        let triangle = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(0.0, 100.0),
        ]);

        let points = hand_drawn_polygon(&triangle, 1, 0.0, &mut ChaCha12Rng::seed_from_u64(42));

        assert_eq!(
            points,
            vec![
                Point::new(0.0, 0.0),
                Point::new(50.0, 0.0),
                Point::new(100.0, 0.0),
                Point::new(50.0, 50.0),
                Point::new(0.0, 100.0),
                Point::new(0.0, 50.0),
                Point::new(0.0, 0.0),
            ]
        );
    }
}
