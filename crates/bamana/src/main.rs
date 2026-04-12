mod cli;

use anyhow::Result;
use bamana_core::{IdentifyRequest, IdentifyService};
use clap::Parser;
use cli::{Cli, Commands, OutputFormat};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Identify(args) => {
            let service = IdentifyService;
            let report = service.identify(IdentifyRequest { path: args.input })?;

            match args.format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                OutputFormat::Text => {
                    println!("path: {}", report.path.display());
                    println!("status: {}", report.status);
                    println!("kind: {}", report.kind);
                    println!("message: {}", report.message);
                }
            }
        }
    }

    Ok(())
}
