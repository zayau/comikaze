use clap::{Parser, Subcommand};
use comikaze::frame;
use std::path::PathBuf;

/// comikaze — generate comic SVG assets: panels, balloons, captions
#[derive(Parser)]
#[command(name = "comikaze", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate a comic panel frame
    Frame {
        /// Width in pixels
        #[arg(long, default_value_t = 400, value_parser = positive_u32, allow_hyphen_values = true)]
        width: u32,

        /// Height in pixels
        #[arg(long, default_value_t = 600, value_parser = positive_u32, allow_hyphen_values = true)]
        height: u32,

        /// Stroke color (#rgb, #rgba, #rrggbb, #rrggbbaa, or currentColor)
        #[arg(long, default_value = "#000000", value_parser = parse_color)]
        color: String,

        /// Interior fill color; omit for a transparent frame
        #[arg(long, value_parser = parse_fill)]
        fill: Option<String>,

        /// Stroke width in pixels
        #[arg(long, default_value_t = 3.0, value_parser = positive_f64, allow_hyphen_values = true)]
        stroke_width: f64,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Random seed for reproducible results
        #[arg(long)]
        seed: Option<u64>,
    },
    /// Generate a complete page layout from a JSON specification
    Layout {
        /// JSON layout specification
        input: PathBuf,

        /// Export filled panel masks instead of frame outlines
        #[arg(long, conflicts_with = "mask")]
        masks: bool,

        /// Export one tightly cropped panel mask
        #[arg(long, value_name = "PANEL", conflicts_with = "masks")]
        mask: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate a speech/thought balloon
    Balloon,
    /// Generate a caption box
    Caption,
}

fn positive_f64(s: &str) -> Result<f64, String> {
    let value: f64 = s
        .parse()
        .map_err(|_| format!("`{s}` is not a valid number"))?;

    frame::validate_stroke_width(value).map_err(|error| error.to_string())?;

    Ok(value)
}

fn positive_u32(s: &str) -> Result<u32, String> {
    let value: u32 = s
        .parse()
        .map_err(|_| format!("`{s}` is not a valid whole number"))?;

    if value == 0 {
        return Err("must be greater than 0".to_string());
    }

    Ok(value)
}

fn parse_color(s: &str) -> Result<String, String> {
    frame::validate_color(s).map_err(|error| error.to_string())?;

    Ok(s.to_string())
}

fn parse_fill(s: &str) -> Result<String, String> {
    frame::validate_fill(s).map_err(|error| error.to_string())?;

    Ok(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_f64_accepts_valid_values() {
        assert_eq!(positive_f64("3"), Ok(3.0));
        assert_eq!(positive_f64("3.5"), Ok(3.5));
    }

    #[test]
    fn positive_f64_rejects_invalid_values() {
        assert!(positive_f64("0").is_err());
        assert!(positive_f64("-5").is_err());
        assert!(positive_f64("NaN").is_err());
        assert!(positive_f64("inf").is_err());
        assert!(positive_f64("-inf").is_err());
    }

    #[test]
    fn positive_u32_accepts_valid_values() {
        assert_eq!(positive_u32("1"), Ok(1));
        assert_eq!(positive_u32("400"), Ok(400));
    }

    #[test]
    fn positive_u32_rejects_zero() {
        assert!(positive_u32("0").is_err());
    }

    #[test]
    fn positive_u32_rejects_non_numeric() {
        assert!(positive_u32("banana").is_err());
    }

    #[test]
    fn parse_color_accepts_hex_formats() {
        assert!(parse_color("#000").is_ok());
        assert!(parse_color("#000000").is_ok());
        assert!(parse_color("#000000ff").is_ok());
    }

    #[test]
    fn parse_color_accepts_current_color() {
        assert!(parse_color("currentColor").is_ok());
    }

    #[test]
    fn parse_color_rejects_named_colors() {
        assert!(parse_color("red").is_err());
    }

    #[test]
    fn parse_color_rejects_malformed_hex() {
        assert!(parse_color("#gggggg").is_err()); // bad characters
        assert!(parse_color("#12345").is_err()); // wrong length
    }

    #[test]
    fn parse_fill_accepts_supported_colors() {
        assert_eq!(parse_fill("#fff8dc"), Ok("#fff8dc".to_string()));
        assert_eq!(parse_fill("currentColor"), Ok("currentColor".to_string()));
    }

    #[test]
    fn parse_fill_rejects_unsupported_colors() {
        assert!(parse_fill("white").is_err());
        assert!(parse_fill("none").is_err());
    }
}
