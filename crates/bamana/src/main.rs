mod cli;
mod commands;
mod output;

use clap::Parser;
use std::process::ExitCode;

use cli::{Cli, Commands};

fn main() -> ExitCode {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Identify(args) => {
            let response = commands::identify::run(args);
            output::emit_json(&response, cli.json_pretty, response.ok)
        }
        Commands::Verify(args) => {
            let response = commands::verify::run(args);
            output::emit_json(&response, cli.json_pretty, response.ok)
        }
        Commands::CheckEof(args) => {
            let response = commands::check_eof::run(args);
            output::emit_json(&response, cli.json_pretty, response.ok)
        }
        Commands::Header(args) => {
            let response = commands::header::run(args);
            output::emit_json(&response, cli.json_pretty, response.ok)
        }
    }
}
