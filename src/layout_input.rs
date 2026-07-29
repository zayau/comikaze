use comikaze::geometry::Point;
use comikaze::layout::{
    HandDrawnOptions, LayoutSvgOptions, PageLayout, build_layout_path_data, build_layout_svg,
};
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

struct BuiltLayout {
    layout: PageLayout,
    panel_names: Vec<String>,
    options: LayoutSvgOptions,
}

struct PanelBounds {
    min_x: f64,
    min_y: f64,
    width: f64,
    height: f64,
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
    let built = build_layout_from_json(json)?;

    build_layout_svg(&built.layout, &built.options)
        .map_err(|error| format!("invalid render options: {error}"))
}

pub(crate) fn build_mask_svg_from_json(json: &str) -> Result<String, String> {
    let built = build_layout_from_json(json)?;
    let path_data = build_layout_path_data(&built.layout, &built.options)
        .map_err(|error| format!("invalid render options: {error}"))?;

    let masks = built
        .panel_names
        .iter()
        .zip(path_data)
        .map(|(panel_name, path_data)| {
            format!(
                r##"  <path id="panel-mask-{panel_name}" data-panel-name="{panel_name}" d="{path_data}" fill="#000000" stroke="none"/>"##
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let width = built.layout.width();
    let height = built.layout.height();

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" data-comikaze-output="panel-masks">
{masks}
</svg>"#
    ))
}

pub(crate) fn build_single_mask_svg_from_json(
    json: &str,
    panel_name: &str,
) -> Result<String, String> {
    let built = build_layout_from_json(json)?;

    let Some(panel_index) = built
        .panel_names
        .iter()
        .position(|active_name| active_name == panel_name)
    else {
        return Err(format!(
            "unknown panel `{panel_name}`; available panels: {}",
            built.panel_names.join(", ")
        ));
    };

    let path_data = build_layout_path_data(&built.layout, &built.options)
        .map_err(|error| format!("invalid render options: {error}"))?;

    let panel_path_data = path_data
        .get(panel_index)
        .ok_or_else(|| "internal panel path is missing".to_string())?;

    let bounds = panel_path_bounds(panel_path_data)?;
    let PanelBounds {
        min_x,
        min_y,
        width,
        height,
    } = bounds;

    Ok(format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="{min_x} {min_y} {width} {height}" overflow="hidden" data-comikaze-output="panel-mask" data-panel-name="{panel_name}">
  <path id="panel-mask-{panel_name}" d="{panel_path_data}" fill="#000000" stroke="none"/>
</svg>"##
    ))
}

fn build_layout_from_json(json: &str) -> Result<BuiltLayout, String> {
    let specification = serde_json::from_str::<LayoutSpec>(json)
        .map_err(|error| format!("could not parse JSON: {error}"))?;

    specification.build()
}

impl LayoutSpec {
    fn build(self) -> Result<BuiltLayout, String> {
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

        let panel_names = ordered_panel_names(panel_indices, layout.panel_count())?;

        Ok(BuiltLayout {
            layout,
            panel_names,
            options,
        })
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
        if !is_valid_panel_name(output_name) {
            return Err(format!(
                "cut {cut_number} output panel name `{output_name}` may only contain ASCII letters, digits, `-`, or `_`"
            ));
        }

        if panel_indices.contains_key(output_name) {
            return Err(format!(
                "cut {cut_number} cannot create panel `{output_name}` because that name is already active"
            ));
        }
    }

    Ok(())
}

fn is_valid_panel_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn ordered_panel_names(
    panel_indices: HashMap<String, usize>,
    panel_count: usize,
) -> Result<Vec<String>, String> {
    let mut ordered_names = vec![None; panel_count];

    for (panel_name, panel_index) in panel_indices {
        let Some(slot) = ordered_names.get_mut(panel_index) else {
            return Err("internal panel name index is out of bounds".to_string());
        };

        if slot.replace(panel_name).is_some() {
            return Err("internal panel names share the same index".to_string());
        }
    }

    ordered_names
        .into_iter()
        .map(|panel_name| {
            panel_name.ok_or_else(|| "internal panel is missing its name".to_string())
        })
        .collect()
}

fn panel_path_bounds(path_data: &str) -> Result<PanelBounds, String> {
    let mut tokens = path_data.split_whitespace();
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut point_count = 0_usize;

    while let Some(command) = tokens.next() {
        match command {
            "M" | "L" => {
                let x = parse_path_coordinate(tokens.next(), command)?;
                let y = parse_path_coordinate(tokens.next(), command)?;

                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                point_count += 1;
            }
            "Z" => {
                if tokens.next().is_some() {
                    return Err("internal panel path contains data after `Z`".to_string());
                }
            }
            unsupported => {
                return Err(format!(
                    "internal panel path uses unsupported command `{unsupported}`"
                ));
            }
        }
    }

    if point_count < 3 {
        return Err("internal panel path has fewer than three points".to_string());
    }

    let width = max_x - min_x;
    let height = max_y - min_y;

    if width <= 0.0 || height <= 0.0 {
        return Err("internal panel path has empty bounds".to_string());
    }

    Ok(PanelBounds {
        min_x,
        min_y,
        width,
        height,
    })
}

fn parse_path_coordinate(token: Option<&str>, command: &str) -> Result<f64, String> {
    token
        .ok_or_else(|| format!("internal panel path command `{command}` is missing a coordinate"))?
        .parse::<f64>()
        .map_err(|_| format!("internal panel path command `{command}` has an invalid coordinate"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svg_path_data(svg: &str) -> Vec<&str> {
        svg.split(r#" d=""#)
            .skip(1)
            .map(|remainder| {
                remainder
                    .split('"')
                    .next()
                    .expect("path data should have a closing quote")
            })
            .collect()
    }

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
    fn masks_use_final_panel_names_and_rendered_contours() {
        let json = r#"{
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
                }
            ],
            "render": {
                "stroke_width": 4,
                "gutter": 12,
                "hand_drawn": {
                    "point_spacing": 20,
                    "jitter": 2,
                    "seed": 42
                }
            }
        }"#;

        let layout_svg = build_svg_from_json(json).unwrap();
        let mask_svg = build_mask_svg_from_json(json).unwrap();

        assert!(mask_svg.contains(r#"data-comikaze-output="panel-masks""#));
        assert!(mask_svg.contains(r#"id="panel-mask-top_right""#));
        assert!(mask_svg.contains(r#"id="panel-mask-top_left""#));
        assert!(mask_svg.contains(r#"id="panel-mask-bottom""#));
        assert_eq!(mask_svg.matches(r##"fill="#000000""##).count(), 3);
        assert_eq!(mask_svg.matches(r#"stroke="none""#).count(), 3);
        assert_eq!(svg_path_data(&mask_svg), svg_path_data(&layout_svg));
    }

    #[test]
    fn single_mask_is_cropped_to_selected_panel_bounds() {
        let svg = build_single_mask_svg_from_json(
            r#"{
                "version": 1,
                "page": { "width": 100, "height": 100 },
                "cuts": [{
                    "target": "page",
                    "start": [0, 50],
                    "end": [100, 50],
                    "negative": "top",
                    "positive": "bottom"
                }],
                "render": {
                    "stroke_width": 2,
                    "gutter": 10
                }
            }"#,
            "top",
        )
        .unwrap();

        assert!(svg.contains(r#"width="88" height="38" viewBox="6 6 88 38""#));
        assert!(svg.contains(r#"data-comikaze-output="panel-mask""#));
        assert!(svg.contains(r#"data-panel-name="top""#));
        assert!(svg.contains(r#"id="panel-mask-top""#));
        assert_eq!(svg.matches("<path ").count(), 1);
        assert!(!svg.contains("panel-mask-bottom"));
    }

    #[test]
    fn single_mask_rejects_unknown_panel_name() {
        let error = build_single_mask_svg_from_json(
            r#"{
                "version": 1,
                "page": { "width": 100, "height": 100 },
                "cuts": [{
                    "target": "page",
                    "start": [0, 50],
                    "end": [100, 50],
                    "negative": "top",
                    "positive": "bottom"
                }]
            }"#,
            "missing",
        )
        .unwrap_err();

        assert!(error.contains("unknown panel `missing`"));
        assert!(error.contains("available panels: top, bottom"));
    }

    #[test]
    fn unsafe_panel_names_are_rejected() {
        let error = build_mask_svg_from_json(
            r#"{
                "version": 1,
                "page": { "width": 100, "height": 100 },
                "cuts": [{
                    "target": "page",
                    "start": [0, 50],
                    "end": [100, 50],
                    "negative": "top panel",
                    "positive": "bottom"
                }]
            }"#,
        )
        .unwrap_err();

        assert!(error.contains("output panel name `top panel` may only contain"));
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
