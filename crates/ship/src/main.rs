mod cli;

use clap::{CommandFactory, Parser};
use cli::Cli;

fn main() -> std::io::Result<()> {
    let _ = Cli::parse();
    Cli::command().print_help()?;
    println!();
    Ok(())
}
