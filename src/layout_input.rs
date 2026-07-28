use comikaze::geometry::Point;
use comikaze::layout::{HandDrawnOptions, LayoutSvgOptions, PageLayout, build_layout_svg};
use serde::Deserialize;
use std::collections::HashMap;

const SUPPORTED_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutSpec {
    version: u32,
    page: PageSpec,
    #[serde(default)]
    cuts: Vec<CutSpec>,
    #[serde(default)]
    render: RenderSpec,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageSpec {
    width: u32,
    height: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CutSpec {
    target: String,
    start: [f64; 2],
    end: [f64; 2],
    negative: String,
    positive: String,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RenderSpec {
    color: String,
    stroke_width: f64,
    gutter: f64,
    hand_drawn: Option<HandDrawnSpec>,
}

impl Default for RenderSpec {
    fn default() -> Self {
        let defaults = LayoutSvgOptions::default();

        Self {
            color: defaults.color,
            stroke_width: defaults.stroke_width,
            gutter: defaults.gutter,
            hand_drawn: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct HandDrawnSpec {
    point_spacing: f64,
    jitter: f64,
    seed: u64,
}

impl Default for HandDrawnSpec {
    fn default() -> Self {
        let defaults = HandDrawnOptions::default();

        Self {
            point_spacing: defaults.point_spacing,
            jitter: defaults.jitter,
            seed: defaults.seed,
        }
    }
}

pub(crate) fn build_svg_from_json(json: &str) -> Result<String, String> {
    let specification = serde_json::from_str::<LayoutSpec>(json)
        .map_err(|error| format!("could not parse JSON: {error}"))?;

    specification.build_svg()
}

impl LayoutSpec {
    fn build_svg(self) -> Result<String, String> {
        if self.version != SUPPORTED_VERSION {
            return Err(format!(
                "unsupported specification version {}; expected {SUPPORTED_VERSION}",
                self.version
            ));
        }

        let mut layout = PageLayout::new(self.page.width, self.page.height)
            .map_err(|error| format!("invalid page: {error}"))?;

        let mut panel_indices = HashMap::from([("page".to_string(), 0_usize)]);

        for (cut_index, cut) in self.cuts.into_iter().enumerate() {
            apply_named_cut(&mut layout, &mut panel_indices, cut_index + 1, cut)?;
        }

        let options = LayoutSvgOptions {
            color: self.render.color,
            stroke_width: self.render.stroke_width,
            gutter: self.render.gutter,
            hand_drawn: self.render.hand_drawn.map(HandDrawnSpec::into_options),
        };

        build_layout_svg(&layout, &options)
            .map_err(|error| format!("invalid render options: {error}"))
    }
}

impl HandDrawnSpec {
    fn into_options(self) -> HandDrawnOptions {
        HandDrawnOptions {
            point_spacing: self.point_spacing,
            jitter: self.jitter,
            seed: self.seed,
        }
    }
}

fn apply_named_cut(
    layout: &mut PageLayout,
    panel_indices: &mut HashMap<String, usize>,
    cut_number: usize,
    cut: CutSpec,
) -> Result<(), String> {
    let Some(&target_index) = panel_indices.get(&cut.target) else {
        return Err(format!(
            "cut {cut_number} targets unknown panel `{}`",
            cut.target
        ));
    };

    validate_output_names(panel_indices, cut_number, &cut)?;

    layout
        .split_panel(
            target_index,
            Point::new(cut.start[0], cut.start[1]),
            Point::new(cut.end[0], cut.end[1]),
        )
        .map_err(|error| {
            format!(
                "cut {cut_number} targeting panel `{}` failed: {error}",
                cut.target
            )
        })?;

    panel_indices.remove(&cut.target);

    for panel_index in panel_indices.values_mut() {
        if *panel_index > target_index {
            *panel_index += 1;
        }
    }

    panel_indices.insert(cut.negative, target_index);
    panel_indices.insert(cut.positive, target_index + 1);

    Ok(())
}

fn validate_output_names(
    panel_indices: &HashMap<String, usize>,
    cut_number: usize,
    cut: &CutSpec,
) -> Result<(), String> {
    if cut.negative.trim().is_empty() || cut.positive.trim().is_empty() {
        return Err(format!(
            "cut {cut_number} output panel names must not be empty"
        ));
    }

    if cut.negative == cut.positive {
        return Err(format!(
            "cut {cut_number} uses duplicate output panel name `{}`",
            cut.negative
        ));
    }

    for output_name in [&cut.negative, &cut.positive] {
        if panel_indices.contains_key(output_name) {
            return Err(format!(
                "cut {cut_number} cannot create panel `{output_name}` because that name is already active"
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_specification_builds_svg() {
        let svg = build_svg_from_json(
            r##"{
                "version": 1,
                "page": { "width": 100, "height": 200 },
                "render": { "color": "#123456" }
            }"##,
        )
        .unwrap();

        assert!(svg.contains(r#"viewBox="0 0 100 200""#));
        assert!(svg.contains(r##"stroke="#123456""##));
        assert_eq!(svg.matches("<path ").count(), 1);
    }

    #[test]
    fn named_cuts_can_target_previous_outputs() {
        let svg = build_svg_from_json(
            r#"{
                "version": 1,
                "page": { "width": 100, "height": 100 },
                "cuts": [
                    {
                        "target": "page",
                        "start": [0, 50],
                        "end": [100, 50],
                        "negative": "top",
                        "positive": "bottom"
                    },
                    {
                        "target": "top",
                        "start": [50, 0],
                        "end": [50, 50],
                        "negative": "top_right",
                        "positive": "top_left"
                    },
                    {
                        "target": "bottom",
                        "start": [0, 75],
                        "end": [100, 75],
                        "negative": "bottom_top",
                        "positive": "bottom_bottom"
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(svg.matches("<path ").count(), 4);
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let error = build_svg_from_json(
            r#"{
                "version": 2,
                "page": { "width": 100, "height": 100 }
            }"#,
        )
        .unwrap_err();

        assert!(error.contains("unsupported specification version 2"));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let error = build_svg_from_json(
            r#"{
                "version": 1,
                "page": {
                    "width": 100,
                    "height": 100,
                    "widht": 100
                }
            }"#,
        )
        .unwrap_err();

        assert!(error.contains("unknown field `widht`"));
    }

    #[test]
    fn unknown_target_is_rejected_with_cut_number() {
        let error = build_svg_from_json(
            r#"{
                "version": 1,
                "page": { "width": 100, "height": 100 },
                "cuts": [{
                    "target": "missing",
                    "start": [0, 50],
                    "end": [100, 50],
                    "negative": "top",
                    "positive": "bottom"
                }]
            }"#,
        )
        .unwrap_err();

        assert!(error.contains("cut 1 targets unknown panel `missing`"));
    }

    #[test]
    fn duplicate_output_names_are_rejected() {
        let error = build_svg_from_json(
            r#"{
                "version": 1,
                "page": { "width": 100, "height": 100 },
                "cuts": [{
                    "target": "page",
                    "start": [0, 50],
                    "end": [100, 50],
                    "negative": "same",
                    "positive": "same"
                }]
            }"#,
        )
        .unwrap_err();

        assert!(error.contains("duplicate output panel name `same`"));
    }

    #[test]
    fn invalid_cut_reports_its_number_and_target() {
        let error = build_svg_from_json(
            r#"{
                "version": 1,
                "page": { "width": 100, "height": 100 },
                "cuts": [{
                    "target": "page",
                    "start": [50, 50],
                    "end": [50, 50],
                    "negative": "first",
                    "positive": "second"
                }]
            }"#,
        )
        .unwrap_err();

        assert!(error.contains("cut 1 targeting panel `page` failed"));
        assert!(error.contains("cut points must be distinct and finite"));
    }

    #[test]
    fn library_render_validation_is_preserved() {
        let error = build_svg_from_json(
            r#"{
                "version": 1,
                "page": { "width": 100, "height": 100 },
                "render": {
                    "hand_drawn": {
                        "jitter": -1
                    }
                }
            }"#,
        )
        .unwrap_err();

        assert!(error.contains("jitter must be a finite non-negative number"));
    }
}
