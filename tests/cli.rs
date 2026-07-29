use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn comikaze_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_comikaze"))
}

fn unique_temp_path(label: &str, extension: &str) -> PathBuf {
    let unique_number = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "comikaze-{label}-{}-{unique_number}.{extension}",
        std::process::id()
    ))
}

fn example_layout_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/layout.json")
}

#[test]
fn frame_prints_svg_to_stdout() {
    let output = comikaze_command()
        .args(["frame", "--seed", "42"])
        .output()
        .expect("failed to run comikaze");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.starts_with("<svg"));
    assert!(stdout.contains(r#"width="400""#));
    assert!(stdout.contains(r#"height="600""#));
    assert!(stdout.contains("<path"));
    assert!(stdout.contains(r#"fill="none""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn frame_prints_configured_fill() {
    let output = comikaze_command()
        .args(["frame", "--seed", "42", "--fill", "#fff8dc"])
        .output()
        .expect("failed to run comikaze");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains(r##"fill="#fff8dc""##));
    assert!(output.stderr.is_empty());
}

#[test]
fn invalid_frame_fill_returns_clap_error() {
    let output = comikaze_command()
        .args(["frame", "--fill", "white"])
        .output()
        .expect("failed to run comikaze");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("not a valid fill color"));
}

#[test]
fn invalid_stroke_width_returns_clap_error() {
    let output = comikaze_command()
        .args(["frame", "--stroke-width", "NaN"])
        .output()
        .expect("failed to run comikaze");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("stroke width must be a finite number greater than 0"));
}

#[test]
fn frame_writes_svg_to_requested_file() {
    let output_path = unique_temp_path("frame-output", "svg");

    let output = comikaze_command()
        .args(["frame", "--seed", "42", "--output"])
        .arg(&output_path)
        .output()
        .expect("failed to run comikaze");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let svg = fs::read_to_string(&output_path).expect("output SVG was not created");

    fs::remove_file(&output_path).expect("failed to remove temporary SVG");

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<path"));
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, format!("wrote {}\n", output_path.display()));
}

#[test]
fn layout_prints_svg_from_json_specification() {
    let output = comikaze_command()
        .arg("layout")
        .arg(example_layout_path())
        .output()
        .expect("failed to run comikaze");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.starts_with("<svg"));
    assert!(stdout.contains(r#"viewBox="0 0 600 900""#));
    assert_eq!(stdout.matches("<path ").count(), 6);
    assert!(output.stderr.is_empty());
}

#[test]
fn layout_writes_svg_to_requested_file() {
    let output_path = unique_temp_path("layout-output", "svg");

    let output = comikaze_command()
        .arg("layout")
        .arg(example_layout_path())
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("failed to run comikaze");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let svg = fs::read_to_string(&output_path).expect("output SVG was not created");

    fs::remove_file(&output_path).expect("failed to remove temporary SVG");

    assert!(svg.starts_with("<svg"));
    assert_eq!(svg.matches("<path ").count(), 6);
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, format!("wrote {}\n", output_path.display()));
}

#[test]
fn layout_writes_named_panel_masks() {
    let output_path = unique_temp_path("layout-masks", "svg");

    let output = comikaze_command()
        .arg("layout")
        .arg(example_layout_path())
        .arg("--masks")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("failed to run comikaze");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let svg = fs::read_to_string(&output_path).expect("mask SVG was not created");

    fs::remove_file(&output_path).expect("failed to remove temporary mask SVG");

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains(r#"data-comikaze-output="panel-masks""#));
    assert_eq!(svg.matches("<path ").count(), 6);
    assert!(svg.contains(r#"id="panel-mask-top_right""#));
    assert!(svg.contains(r#"id="panel-mask-middle_center""#));
    assert!(svg.contains(r#"id="panel-mask-bottom""#));
    assert_eq!(svg.matches(r#"stroke="none""#).count(), 6);
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, format!("wrote {}\n", output_path.display()));
}

#[test]
fn layout_writes_one_cropped_panel_mask() {
    let output_path = unique_temp_path("single-layout-mask", "svg");

    let output = comikaze_command()
        .arg("layout")
        .arg(example_layout_path())
        .args(["--mask", "bottom"])
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("failed to run comikaze");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let svg = fs::read_to_string(&output_path).expect("single mask SVG was not created");

    fs::remove_file(&output_path).expect("failed to remove temporary single mask SVG");

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains(r#"data-comikaze-output="panel-mask""#));
    assert!(svg.contains(r#"id="panel-mask-bottom""#));
    assert_eq!(svg.matches("<path ").count(), 1);
    assert!(!svg.contains(r#"viewBox="0 0 600 900""#));
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, format!("wrote {}\n", output_path.display()));
}

#[test]
fn layout_rejects_combined_mask_output_modes() {
    let output = comikaze_command()
        .arg("layout")
        .arg(example_layout_path())
        .args(["--masks", "--mask", "bottom"])
        .output()
        .expect("failed to run comikaze");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be used with"));
}

#[test]
fn layout_reports_context_for_unknown_panel() {
    let input_path = unique_temp_path("unknown-panel", "json");

    fs::write(
        &input_path,
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
    .expect("failed to write temporary layout specification");

    let output = comikaze_command()
        .arg("layout")
        .arg(&input_path)
        .output()
        .expect("failed to run comikaze");

    fs::remove_file(&input_path).expect("failed to remove temporary specification");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cut 1 targets unknown panel `missing`"));
}

#[test]
fn layout_reports_malformed_json() {
    let input_path = unique_temp_path("malformed-layout", "json");

    fs::write(&input_path, r#"{ "version": 1, "page": "#)
        .expect("failed to write temporary layout specification");

    let output = comikaze_command()
        .arg("layout")
        .arg(&input_path)
        .output()
        .expect("failed to run comikaze");

    fs::remove_file(&input_path).expect("failed to remove temporary specification");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("could not parse JSON"));
}
