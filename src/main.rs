use clap::Parser;
use comikaze::cli::{Cli, Commands};
use comikaze::frame;

fn main() {
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

            let svg = match frame::build_frame_svg(&options) {
                Ok(svg) => svg,
                Err(message) => {
                    eprintln!("error: {message}");
                    std::process::exit(1);
                }
            };

            match output {
                Some(path) => {
                    if let Err(e) = std::fs::write(&path, svg) {
                        eprintln!("error: failed to write {}: {e}", path.display());
                        std::process::exit(1);
                    }
                    println!("wrote {}", path.display());
                }
                None => println!("{svg}"),
            }
        }
        Commands::Balloon => println!("balloon: not yet implemented"),
        Commands::Caption => println!("caption: not yet implemented"),
    }
}
