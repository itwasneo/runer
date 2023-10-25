use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(version, author, about)]
pub struct Cli {
    #[command(subcommand, long_about)]
    pub mode: Mode,
}

#[derive(Debug, Subcommand)]
pub enum Mode {
    /// (alias <r>) Runs the given .runer file or the .runer files in the current directory
    #[command(alias = "r")]
    Run(RunArgs),

    /// (alias <c>) Starts runer-cli
    #[command(alias = "c")]
    Cli,

    /// (alias <d>) Starts runer-desktop
    #[command(alias = "d")]
    Desktop,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// .runer file to run
    #[arg(short, long)]
    pub file: Option<String>,
}
