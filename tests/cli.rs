use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn comikaze_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_comikaze"))
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
    assert!(output.stderr.is_empty());
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
fn unfinished_command_returns_failure() {
    let output = comikaze_command()
        .arg("balloon")
        .output()
        .expect("failed to run comikaze");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("balloon generation is not implemented yet"));
}

#[test]
fn frame_writes_svg_to_requested_file() {
    let unique_number = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();

    let output_path = std::env::temp_dir().join(format!(
        "comikaze-cli-test-{}-{unique_number}.svg",
        std::process::id()
    ));

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
