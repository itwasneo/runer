mod engine;
mod model;

use anyhow::Result;
use clap::Parser;
use engine::extractor::*;
use log::{error, info};
use std::time::Instant;

use crate::engine::executor::execute_flow;
use crate::engine::state::State;
use crate::model::commandline::*;

fn main() -> Result<(), ()> {
    let start = Instant::now();

    // Parsing Command Line Arguments. Here the application directly exits
    // if it can't parse the arguments properly.
    let args = Cli::parse();

    // Initializing Logger
    env_logger::init();

    match args.mode {
        Mode::Run(args) => {
            let rune = extract_rune(&args.file.unwrap_or_else(|| ".runer".to_owned()))
                .map_err(|e| error!("{e}"))?;

            analyze_fragments(&rune);

            let state = State::default().from_rune(rune);

            smol::block_on(execute_flow(0, state)).map_err(|e| error!("{e}"))?;
        }
        Mode::Cli => info!("Mode is C which stands for CLI. <Not Implemented>"),
        Mode::Desktop => info!("Mode is D which stands for Desktop. <Not Implemented>"),
    }

    let duration = start.elapsed();
    info!("Time elapsed: {:?}", duration);
    Ok(())
}
