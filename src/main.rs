mod app;
mod format;

use anyhow::{Result, bail};
use app::{Startup, export_text_file, parse_new_dimensions, run_interactive, startup_from_path};
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "terminal_animator")]
#[command(about = "Mouse-first terminal art editor", version)]
struct Cli {
    #[arg(long, value_name = "WIDTHxHEIGHT")]
    new: Option<String>,

    #[arg(long, num_args = 3, value_names = ["FORMAT", "INPUT", "OUTPUT"])]
    export: Option<Vec<String>>,

    path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(export) = cli.export {
        let [format, input, output]: [String; 3] = export
            .try_into()
            .expect("clap enforces exactly three export args");

        if format != "text" {
            bail!("unsupported export format {format:?}; expected \"text\"");
        }

        export_text_file(&PathBuf::from(input), &PathBuf::from(output))?;
        return Ok(());
    }

    let startup = match (cli.new, cli.path) {
        (Some(dimensions), Some(path)) => {
            let (width, height) = parse_new_dimensions(&dimensions)?;
            Startup::New {
                width,
                height,
                path,
            }
        }
        (Some(_), None) => {
            bail!("--new requires a target .tanim.toml path");
        }
        (None, Some(path)) => startup_from_path(path),
        (None, None) => Startup::Welcome,
    };

    run_interactive(startup)
}
