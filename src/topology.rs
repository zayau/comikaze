use crate::geometry::{GEOMETRY_EPSILON, Point, Polygon};
use crate::hand_drawn::hand_drawn_edge;
use chacha20::ChaCha12Rng;
use rand::SeedableRng;
use std::cmp::Ordering;

const MAX_PROFILE_SECTIONS: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PanelEdgeUse {
    pub(crate) panel_index: usize,
    pub(crate) edge_index: usize,
    pub(crate) start_progress: f64,
    pub(crate) end_progress: f64,
    pub(crate) forward: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BoundarySegment {
    pub(crate) start: Point,
    pub(crate) end: Point,
    pub(crate) panel_uses: Vec<PanelEdgeUse>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BoundaryGraph {
    pub(crate) segments: Vec<BoundarySegment>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BoundaryProfile {
    pub(crate) points: Vec<Point>,
}

impl BoundaryProfile {
    fn point_at_progress(&self, progress: f64) -> Option<Point> {
        if !progress.is_finite()
            || !(-GEOMETRY_EPSILON..=1.0 + GEOMETRY_EPSILON).contains(&progress)
        {
            return None;
        }

        let last_index = self.points.len().checked_sub(1)?;

        if last_index == 0 {
            return None;
        }

        let progress = progress.clamp(0.0, 1.0);

        let scaled_index = progress * last_index as f64;

        let lower_index = scaled_index.floor() as usize;

        let upper_index = (lower_index + 1).min(last_index);

        let local_progress = scaled_index - lower_index as f64;

        let lower = self.points[lower_index];
        let upper = self.points[upper_index];

        Some(Point::new(
            lower.x + (upper.x - lower.x) * local_progress,
            lower.y + (upper.y - lower.y) * local_progress,
        ))
    }

    fn points_between(&self, start_progress: f64, end_progress: f64) -> Option<Vec<Point>> {
        if !start_progress.is_finite()
            || !end_progress.is_finite()
            || start_progress < -GEOMETRY_EPSILON
            || end_progress > 1.0 + GEOMETRY_EPSILON
            || start_progress >= end_progress
        {
            return None;
        }

        let start_progress = start_progress.clamp(0.0, 1.0);

        let end_progress = end_progress.clamp(0.0, 1.0);

        let last_index = self.points.len().checked_sub(1)?;

        if last_index == 0 {
            return None;
        }

        let mut points = vec![self.point_at_progress(start_progress)?];

        for index in 1..last_index {
            let point_progress = index as f64 / last_index as f64;

            if point_progress > start_progress + GEOMETRY_EPSILON
                && point_progress < end_progress - GEOMETRY_EPSILON
            {
                points.push(self.points[index]);
            }
        }

        points.push(self.point_at_progress(end_progress)?);

        Some(points)
    }

    pub(crate) fn inset_points_for(
        &self,
        panel: &Polygon,
        panel_use: PanelEdgeUse,
        visible_start_progress: f64,
        visible_end_progress: f64,
        distance: f64,
    ) -> Option<Vec<Point>> {
        if !distance.is_finite()
            || distance < 0.0
            || !visible_start_progress.is_finite()
            || !visible_end_progress.is_finite()
            || visible_start_progress > visible_end_progress
        {
            return None;
        }

        let clipped_start = panel_use.start_progress.max(visible_start_progress);

        let clipped_end = panel_use.end_progress.min(visible_end_progress);

        if clipped_end - clipped_start <= GEOMETRY_EPSILON {
            return Some(Vec::new());
        }

        let use_length = panel_use.end_progress - panel_use.start_progress;

        if use_length <= GEOMETRY_EPSILON {
            return None;
        }

        let local_start = (clipped_start - panel_use.start_progress) / use_length;

        let local_end = (clipped_end - panel_use.start_progress) / use_length;

        let (profile_start, profile_end) = if panel_use.forward {
            (local_start, local_end)
        } else {
            (1.0 - local_end, 1.0 - local_start)
        };

        let mut points = self.points_between(profile_start, profile_end)?;

        if !panel_use.forward {
            points.reverse();
        }

        let (normal_x, normal_y) = panel.edge_inward_unit_normal(panel_use.edge_index)?;

        for point in &mut points {
            point.x += normal_x * distance;
            point.y += normal_y * distance;
        }

        let edge_start = panel.vertices()[panel_use.edge_index];

        let next_vertex = (panel_use.edge_index + 1) % panel.vertices().len();

        let edge_end = panel.vertices()[next_vertex];

        let direction_x = edge_end.x - edge_start.x;
        let direction_y = edge_end.y - edge_start.y;

        let exact_start = Point::new(
            edge_start.x + direction_x * clipped_start + normal_x * distance,
            edge_start.y + direction_y * clipped_start + normal_y * distance,
        );

        let exact_end = Point::new(
            edge_start.x + direction_x * clipped_end + normal_x * distance,
            edge_start.y + direction_y * clipped_end + normal_y * distance,
        );

        let first = points.first_mut()?;
        *first = exact_start;

        let last = points.last_mut()?;
        *last = exact_end;

        Some(points)
    }
}

fn points_are_close(first: Point, second: Point) -> bool {
    (first.x - second.x).abs() <= GEOMETRY_EPSILON && (first.y - second.y).abs() <= GEOMETRY_EPSILON
}

fn push_if_distinct(points: &mut Vec<Point>, candidate: Point) {
    let duplicates_previous = points
        .last()
        .is_some_and(|previous| points_are_close(*previous, candidate));

    if !duplicates_previous {
        points.push(candidate);
    }
}

fn collect_unique_vertices(panels: &[Polygon]) -> Vec<Point> {
    let mut vertices = Vec::new();

    for panel in panels {
        for vertex in panel.vertices() {
            let already_exists = vertices
                .iter()
                .any(|existing| points_are_close(*existing, *vertex));

            if !already_exists {
                vertices.push(*vertex);
            }
        }
    }

    vertices
}

fn segment_progress(segment_start: Point, segment_end: Point, point: Point) -> Option<f64> {
    let direction_x = segment_end.x - segment_start.x;
    let direction_y = segment_end.y - segment_start.y;

    let length_squared = direction_x * direction_x + direction_y * direction_y;

    if length_squared <= GEOMETRY_EPSILON * GEOMETRY_EPSILON {
        return None;
    }

    let point_x = point.x - segment_start.x;
    let point_y = point.y - segment_start.y;

    let cross_product = direction_x * point_y - direction_y * point_x;

    let segment_length = length_squared.sqrt();
    let perpendicular_distance = cross_product / segment_length;

    if perpendicular_distance.abs() > GEOMETRY_EPSILON {
        return None;
    }

    let progress = (point_x * direction_x + point_y * direction_y) / length_squared;

    if !(-GEOMETRY_EPSILON..=1.0 + GEOMETRY_EPSILON).contains(&progress) {
        return None;
    }

    Some(progress.clamp(0.0, 1.0))
}

fn compare_points(first: Point, second: Point) -> Ordering {
    first
        .x
        .total_cmp(&second.x)
        .then_with(|| first.y.total_cmp(&second.y))
}

fn canonical_segment(start: Point, end: Point) -> (Point, Point, bool) {
    if compare_points(start, end) == Ordering::Less {
        (start, end, true)
    } else {
        (end, start, false)
    }
}

impl BoundaryGraph {
    pub(crate) fn from_panels(panels: &[Polygon]) -> Self {
        let known_vertices = collect_unique_vertices(panels);

        let mut segments = Vec::<BoundarySegment>::new();

        for (panel_index, panel) in panels.iter().enumerate() {
            let vertices = panel.vertices();

            for edge_index in 0..vertices.len() {
                let edge_start = vertices[edge_index];
                let edge_end = vertices[(edge_index + 1) % vertices.len()];

                let mut split_points = vec![(0.0, edge_start), (1.0, edge_end)];

                for candidate in &known_vertices {
                    let Some(progress) = segment_progress(edge_start, edge_end, *candidate) else {
                        continue;
                    };

                    if progress > GEOMETRY_EPSILON && progress < 1.0 - GEOMETRY_EPSILON {
                        split_points.push((progress, *candidate));
                    }
                }

                split_points.sort_by(|first, second| first.0.total_cmp(&second.0));

                split_points.dedup_by(|first, second| points_are_close(first.1, second.1));

                for pair in split_points.windows(2) {
                    let original_start = pair[0].1;
                    let original_end = pair[1].1;

                    if points_are_close(original_start, original_end) {
                        continue;
                    }

                    let (canonical_start, canonical_end, forward) =
                        canonical_segment(original_start, original_end);

                    let panel_use = PanelEdgeUse {
                        panel_index,
                        edge_index,
                        start_progress: pair[0].0,
                        end_progress: pair[1].0,
                        forward,
                    };

                    let existing_segment = segments.iter_mut().find(|segment| {
                        points_are_close(segment.start, canonical_start)
                            && points_are_close(segment.end, canonical_end)
                    });

                    if let Some(segment) = existing_segment {
                        let panel_already_recorded = segment
                            .panel_uses
                            .iter()
                            .any(|existing_use| existing_use.panel_index == panel_index);

                        if !panel_already_recorded {
                            segment.panel_uses.push(panel_use);
                        }
                    } else {
                        segments.push(BoundarySegment {
                            start: canonical_start,
                            end: canonical_end,
                            panel_uses: vec![panel_use],
                        });
                    }
                }
            }
        }

        for segment in &mut segments {
            segment
                .panel_uses
                .sort_by_key(|panel_use| (panel_use.panel_index, panel_use.edge_index));
        }

        segments.sort_by(|first, second| {
            compare_points(first.start, second.start)
                .then_with(|| compare_points(first.end, second.end))
        });

        Self { segments }
    }

    pub(crate) fn supports_point_spacing(&self, point_spacing: f64) -> bool {
        if !point_spacing.is_finite() || point_spacing <= 0.0 {
            return false;
        }

        self.segments.iter().all(|segment| {
            let direction_x = segment.end.x - segment.start.x;
            let direction_y = segment.end.y - segment.start.y;

            let edge_length = direction_x.hypot(direction_y);

            let section_count = (edge_length / point_spacing).ceil().max(1.0);

            section_count <= MAX_PROFILE_SECTIONS as f64
        })
    }

    pub(crate) fn hand_drawn_profiles(
        &self,
        point_spacing: f64,
        jitter: f64,
        seed: u64,
    ) -> Vec<BoundaryProfile> {
        assert!(point_spacing.is_finite() && point_spacing > 0.0);

        assert!(jitter.is_finite() && jitter >= 0.0);

        let mut rng = ChaCha12Rng::seed_from_u64(seed);

        let mut profiles = Vec::with_capacity(self.segments.len());

        for segment in &self.segments {
            let direction_x = segment.end.x - segment.start.x;
            let direction_y = segment.end.y - segment.start.y;

            let edge_length = direction_x.hypot(direction_y);

            let section_count = (edge_length / point_spacing).ceil().max(1.0) as usize;

            let interior_points = section_count.saturating_sub(1);

            let points = hand_drawn_edge(
                segment.start,
                segment.end,
                interior_points,
                jitter,
                &mut rng,
            );

            profiles.push(BoundaryProfile { points });
        }

        profiles
    }

    pub(crate) fn hand_drawn_panel_points(
        &self,
        panel_index: usize,
        panel: &Polygon,
        profiles: &[BoundaryProfile],
        inset_distance: f64,
    ) -> Option<Vec<Point>> {
        if profiles.len() != self.segments.len() {
            return None;
        }

        let inset = panel.inset(inset_distance)?;

        let mut panel_points = Vec::new();

        for edge_index in 0..panel.vertices().len() {
            let next_vertex = (edge_index + 1) % panel.vertices().len();

            let visible_start =
                panel.edge_projection_progress(edge_index, inset.vertices()[edge_index])?;

            let visible_end =
                panel.edge_projection_progress(edge_index, inset.vertices()[next_vertex])?;

            if visible_start > visible_end + GEOMETRY_EPSILON {
                return None;
            }

            push_if_distinct(&mut panel_points, inset.vertices()[edge_index]);

            let mut edge_parts = Vec::new();

            for (segment, profile) in self.segments.iter().zip(profiles.iter()) {
                let panel_use = segment
                    .panel_uses
                    .iter()
                    .find(|panel_use| {
                        panel_use.panel_index == panel_index && panel_use.edge_index == edge_index
                    })
                    .copied();

                if let Some(panel_use) = panel_use {
                    edge_parts.push((panel_use, profile));
                }
            }

            edge_parts.sort_by(|(first_use, _), (second_use, _)| {
                first_use
                    .start_progress
                    .total_cmp(&second_use.start_progress)
            });

            if edge_parts.is_empty() {
                return None;
            }

            let mut expected_progress = 0.0;

            for (panel_use, _) in &edge_parts {
                if (panel_use.start_progress - expected_progress).abs() > GEOMETRY_EPSILON {
                    return None;
                }

                expected_progress = panel_use.end_progress;
            }

            if (expected_progress - 1.0).abs() > GEOMETRY_EPSILON {
                return None;
            }

            for (panel_use, profile) in edge_parts {
                let part_points = profile.inset_points_for(
                    panel,
                    panel_use,
                    visible_start,
                    visible_end,
                    inset_distance,
                )?;

                for point in part_points {
                    push_if_distinct(&mut panel_points, point);
                }
            }
        }

        if panel_points.len() > 1 && points_are_close(panel_points[0], *panel_points.last()?) {
            panel_points.pop();
        }

        if panel_points.len() < 3 {
            return None;
        }

        Some(panel_points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_edge_records_both_panels() {
        let top = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 50.0),
            Point::new(0.0, 50.0),
        ]);

        let bottom = Polygon::new(vec![
            Point::new(100.0, 50.0),
            Point::new(100.0, 100.0),
            Point::new(0.0, 100.0),
            Point::new(0.0, 50.0),
        ]);

        let graph = BoundaryGraph::from_panels(&[top, bottom]);

        let shared_segments = graph
            .segments
            .iter()
            .filter(|segment| segment.panel_uses.len() == 2)
            .collect::<Vec<_>>();

        assert_eq!(shared_segments.len(), 1);

        let shared = shared_segments[0];

        assert_eq!(shared.start, Point::new(0.0, 50.0));
        assert_eq!(shared.end, Point::new(100.0, 50.0));

        let top_use = shared.panel_uses[0];
        let bottom_use = shared.panel_uses[1];

        assert_eq!(top_use.panel_index, 0);
        assert_eq!(top_use.edge_index, 2);
        assert_eq!(top_use.start_progress, 0.0);
        assert_eq!(top_use.end_progress, 1.0);

        assert_eq!(bottom_use.panel_index, 1);
        assert_eq!(bottom_use.edge_index, 3);
        assert_eq!(bottom_use.start_progress, 0.0);
        assert_eq!(bottom_use.end_progress, 1.0);

        assert_ne!(top_use.forward, bottom_use.forward);
    }

    #[test]
    fn long_edge_is_split_at_t_junction() {
        let top_left = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 50.0),
            Point::new(0.0, 50.0),
        ]);

        let top_right = Polygon::new(vec![
            Point::new(50.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 50.0),
            Point::new(50.0, 50.0),
        ]);

        let bottom = Polygon::new(vec![
            Point::new(100.0, 50.0),
            Point::new(100.0, 100.0),
            Point::new(0.0, 100.0),
            Point::new(0.0, 50.0),
        ]);

        let graph = BoundaryGraph::from_panels(&[top_left, top_right, bottom]);

        let horizontal_shared = graph
            .segments
            .iter()
            .filter(|segment| {
                segment.panel_uses.len() == 2
                    && (segment.start.y - 50.0).abs() <= GEOMETRY_EPSILON
                    && (segment.end.y - 50.0).abs() <= GEOMETRY_EPSILON
            })
            .collect::<Vec<_>>();

        assert_eq!(horizontal_shared.len(), 2);

        assert_eq!(horizontal_shared[0].start, Point::new(0.0, 50.0));
        assert_eq!(horizontal_shared[0].end, Point::new(50.0, 50.0));

        assert_eq!(horizontal_shared[1].start, Point::new(50.0, 50.0));
        assert_eq!(horizontal_shared[1].end, Point::new(100.0, 50.0));

        assert!(
            graph
                .segments
                .iter()
                .all(|segment| { segment.panel_uses.len() <= 2 })
        );

        let left_bottom_use = horizontal_shared[0]
            .panel_uses
            .iter()
            .find(|panel_use| panel_use.panel_index == 2)
            .expect("left segment should be used by bottom panel");

        let right_bottom_use = horizontal_shared[1]
            .panel_uses
            .iter()
            .find(|panel_use| panel_use.panel_index == 2)
            .expect("right segment should be used by bottom panel");

        assert_eq!(left_bottom_use.edge_index, 3);
        assert_eq!(left_bottom_use.start_progress, 0.0);
        assert_eq!(left_bottom_use.end_progress, 0.5);
        assert!(left_bottom_use.forward);

        assert_eq!(right_bottom_use.edge_index, 3);
        assert_eq!(right_bottom_use.start_progress, 0.5);
        assert_eq!(right_bottom_use.end_progress, 1.0);
        assert!(right_bottom_use.forward);
    }

    #[test]
    fn point_spacing_controls_profile_density() {
        let panel = Polygon::rectangle(0.0, 0.0, 100.0, 50.0);

        let graph = BoundaryGraph::from_panels(&[panel]);

        let profiles = graph.hand_drawn_profiles(30.0, 0.0, 42);

        assert_eq!(profiles.len(), graph.segments.len());

        let (_, top_profile) = graph
            .segments
            .iter()
            .zip(profiles.iter())
            .find(|(segment, _)| {
                segment.start == Point::new(0.0, 0.0) && segment.end == Point::new(100.0, 0.0)
            })
            .expect("top boundary should exist");

        assert_eq!(
            top_profile.points.as_slice(),
            &[
                Point::new(0.0, 0.0),
                Point::new(25.0, 0.0),
                Point::new(50.0, 0.0),
                Point::new(75.0, 0.0),
                Point::new(100.0, 0.0),
            ]
        );
    }

    #[test]
    fn neighboring_panels_reuse_reversed_profile() {
        let top = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 50.0),
            Point::new(0.0, 50.0),
        ]);

        let bottom = Polygon::new(vec![
            Point::new(100.0, 50.0),
            Point::new(100.0, 100.0),
            Point::new(0.0, 100.0),
            Point::new(0.0, 50.0),
        ]);

        let panels = vec![top, bottom];

        let graph = BoundaryGraph::from_panels(&panels);

        let profiles = graph.hand_drawn_profiles(20.0, 2.0, 42);

        let (shared, shared_profile) = graph
            .segments
            .iter()
            .zip(profiles.iter())
            .find(|(segment, _)| segment.panel_uses.len() == 2)
            .expect("shared boundary should exist");

        let top_points = shared_profile
            .inset_points_for(&panels[0], shared.panel_uses[0], 0.0, 1.0, 0.0)
            .expect("top profile should render");

        let bottom_points = shared_profile
            .inset_points_for(&panels[1], shared.panel_uses[1], 0.0, 1.0, 0.0)
            .expect("bottom profile should render");

        assert_eq!(top_points.first(), Some(&Point::new(100.0, 50.0)));

        assert_eq!(bottom_points.first(), Some(&Point::new(0.0, 50.0)));

        let mut reversed_bottom = bottom_points.clone();
        reversed_bottom.reverse();

        assert_eq!(top_points, reversed_bottom);
    }

    #[test]
    fn same_seed_produces_same_boundary_profiles() {
        let panel = Polygon::rectangle(0.0, 0.0, 100.0, 50.0);

        let graph = BoundaryGraph::from_panels(&[panel]);

        let first = graph.hand_drawn_profiles(20.0, 2.0, 42);

        let second = graph.hand_drawn_profiles(20.0, 2.0, 42);

        let different_seed = graph.hand_drawn_profiles(20.0, 2.0, 43);

        assert_eq!(first, second);
        assert_ne!(first, different_seed);
    }

    #[test]
    fn shared_profile_offsets_toward_each_panel() {
        let top = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 50.0),
            Point::new(0.0, 50.0),
        ]);

        let bottom = Polygon::new(vec![
            Point::new(100.0, 50.0),
            Point::new(100.0, 100.0),
            Point::new(0.0, 100.0),
            Point::new(0.0, 50.0),
        ]);

        let graph = BoundaryGraph::from_panels(&[top.clone(), bottom.clone()]);

        let profiles = graph.hand_drawn_profiles(20.0, 2.0, 42);

        let (shared, profile) = graph
            .segments
            .iter()
            .zip(profiles.iter())
            .find(|(segment, _)| segment.panel_uses.len() == 2)
            .expect("shared boundary should exist");

        let top_use = *shared
            .panel_uses
            .iter()
            .find(|panel_use| panel_use.panel_index == 0)
            .expect("top panel should use boundary");

        let bottom_use = *shared
            .panel_uses
            .iter()
            .find(|panel_use| panel_use.panel_index == 1)
            .expect("bottom panel should use boundary");

        let inset_distance = 8.0;

        let top_points = profile
            .inset_points_for(&top, top_use, 0.0, 1.0, inset_distance)
            .expect("top profile should inset");

        let bottom_points = profile
            .inset_points_for(&bottom, bottom_use, 0.0, 1.0, inset_distance)
            .expect("bottom profile should inset");

        for (top_point, bottom_point) in top_points.iter().rev().zip(bottom_points.iter()) {
            assert!((top_point.x - bottom_point.x).abs() <= GEOMETRY_EPSILON);

            assert!(
                (bottom_point.y - top_point.y - 2.0 * inset_distance).abs() <= GEOMETRY_EPSILON
            );
        }
    }

    #[test]
    fn profile_is_trimmed_to_inset_corners() {
        let panel = Polygon::rectangle(0.0, 0.0, 100.0, 50.0);

        let graph = BoundaryGraph::from_panels(std::slice::from_ref(&panel));

        let profiles = graph.hand_drawn_profiles(25.0, 0.0, 42);

        let (top_segment, top_profile) = graph
            .segments
            .iter()
            .zip(profiles.iter())
            .find(|(segment, _)| {
                segment.start == Point::new(0.0, 0.0) && segment.end == Point::new(100.0, 0.0)
            })
            .expect("top boundary should exist");

        let panel_use = top_segment.panel_uses[0];

        let inset = panel.inset(10.0).expect("panel should inset");

        let visible_start = panel
            .edge_projection_progress(panel_use.edge_index, inset.vertices()[panel_use.edge_index])
            .expect("inset start should project");

        let next_vertex = (panel_use.edge_index + 1) % inset.vertices().len();

        let visible_end = panel
            .edge_projection_progress(panel_use.edge_index, inset.vertices()[next_vertex])
            .expect("inset end should project");

        let points = top_profile
            .inset_points_for(&panel, panel_use, visible_start, visible_end, 10.0)
            .expect("profile should be trimmed");

        assert_eq!(
            points,
            vec![
                Point::new(10.0, 10.0),
                Point::new(25.0, 10.0),
                Point::new(50.0, 10.0),
                Point::new(75.0, 10.0),
                Point::new(90.0, 10.0),
            ]
        );
    }

    #[test]
    fn panel_profiles_form_complete_inset_boundary() {
        let panel = Polygon::rectangle(0.0, 0.0, 100.0, 50.0);

        let graph = BoundaryGraph::from_panels(std::slice::from_ref(&panel));

        let profiles = graph.hand_drawn_profiles(25.0, 0.0, 42);

        let points = graph
            .hand_drawn_panel_points(0, &panel, &profiles, 10.0)
            .expect("panel points should assemble");

        assert_eq!(
            points,
            vec![
                Point::new(10.0, 10.0),
                Point::new(25.0, 10.0),
                Point::new(50.0, 10.0),
                Point::new(75.0, 10.0),
                Point::new(90.0, 10.0),
                Point::new(90.0, 25.0),
                Point::new(90.0, 40.0),
                Point::new(75.0, 40.0),
                Point::new(50.0, 40.0),
                Point::new(25.0, 40.0),
                Point::new(10.0, 40.0),
                Point::new(10.0, 25.0),
            ]
        );
    }

    #[test]
    fn t_junction_segments_join_without_duplicate_point() {
        let top_left = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 50.0),
            Point::new(0.0, 50.0),
        ]);

        let top_right = Polygon::new(vec![
            Point::new(50.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 50.0),
            Point::new(50.0, 50.0),
        ]);

        let bottom = Polygon::new(vec![
            Point::new(100.0, 50.0),
            Point::new(100.0, 100.0),
            Point::new(0.0, 100.0),
            Point::new(0.0, 50.0),
        ]);

        let panels = vec![top_left, top_right, bottom.clone()];

        let graph = BoundaryGraph::from_panels(&panels);

        let profiles = graph.hand_drawn_profiles(25.0, 0.0, 42);

        let points = graph
            .hand_drawn_panel_points(2, &bottom, &profiles, 8.0)
            .expect("bottom panel should assemble");

        let mut top_edge_points = points
            .iter()
            .filter(|point| (point.y - 58.0).abs() <= GEOMETRY_EPSILON)
            .copied()
            .collect::<Vec<_>>();

        let first_left_point = top_edge_points
            .iter()
            .position(|point| (point.x - 8.0).abs() <= GEOMETRY_EPSILON)
            .expect("left inset corner should exist");

        top_edge_points.rotate_left(first_left_point);

        assert_eq!(
            top_edge_points,
            vec![
                Point::new(8.0, 58.0),
                Point::new(25.0, 58.0),
                Point::new(50.0, 58.0),
                Point::new(75.0, 58.0),
                Point::new(92.0, 58.0),
            ]
        );
    }
}
