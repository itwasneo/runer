use clap::Parser;
use log::{error, info};

use crate::engine::extractor::*;
use crate::model::commandline::{Cli, Mode};

use crate::engine::executor::execute_flow;
use crate::engine::state::State;

pub fn parse_cmdline_args() -> Cli {
    Cli::parse()
}

pub fn initialize_logger() {
    env_logger::init();
}

pub fn handle_mod(mode: Mode) {
    match mode {
        Mode::Run(args) => {
            let rune = extract_rune(&args.file.unwrap_or_else(|| ".runer".to_owned()))
                .map_err(|e| error!("{e}"))
                .unwrap();

            analyze_fragments(&rune);

            let state = State::default().from_rune(rune);

            smol::block_on(execute_flow(0, state))
                .map_err(|e| error!("{e}"))
                .unwrap();
            // let duration = start.elapsed();
            // info!("Time elapsed: {:?}", duration);
        }
        Mode::Cli => {
            info!("Mode is 'c' which stands for CLI. <Not Implemented>");
        }
        Mode::Desktop => {
            info!("Mode is 'd' whic stands for Desktop. <Not Implemented>");
        }
    }
}
