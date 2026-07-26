mod cli;

use clap::Parser;
use cli::{Cli, Commands};
use comikaze::frame;
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
            stroke_width,
            output,
            seed,
        } => {
            let options = frame::FrameOptions {
                width,
                height,
                color,
                stroke_width,
                seed,
            };

            let svg = frame::build_frame_svg(&options).map_err(|error| error.to_string())?;

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
        Commands::Balloon => Err("balloon generation is not implemented yet".to_string()),
        Commands::Caption => Err("caption generation is not implemented yet".to_string()),
    }
}
