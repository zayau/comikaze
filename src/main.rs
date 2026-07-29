mod cli;
mod layout_input;

use clap::Parser;
use cli::{Cli, Commands};
use comikaze::frame;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Frame {
            width,
            height,
            color,
            fill,
            stroke_width,
            output,
            seed,
        } => {
            let options = frame::FrameOptions {
                width,
                height,
                color,
                fill,
                stroke_width,
                seed,
            };

            let svg = frame::build_frame_svg(&options).map_err(|error| error.to_string())?;

            write_svg(svg, output)
        }
        Commands::Layout {
            input,
            masks,
            mask,
            output,
        } => {
            let json = std::fs::read_to_string(&input)
                .map_err(|error| format!("failed to read {}: {error}", input.display()))?;

            let svg = match mask {
                Some(panel_name) => {
                    layout_input::build_single_mask_svg_from_json(&json, &panel_name)
                }
                None if masks => layout_input::build_mask_svg_from_json(&json),
                None => layout_input::build_svg_from_json(&json),
            }
            .map_err(|error| format!("invalid layout {}: {error}", input.display()))?;

            write_svg(svg, output)
        }
    }
}

fn write_svg(svg: String, output: Option<PathBuf>) -> Result<(), String> {
    match output {
        Some(path) => {
            std::fs::write(&path, svg)
                .map_err(|error| format!("failed to write {}: {error}", path.display()))?;

            println!("wrote {}", path.display());
        }
        None => println!("{svg}"),
    }

    Ok(())
}
