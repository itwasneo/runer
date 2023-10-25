use std::time::Instant;

use clap::Parser;
use log::{error, info};
use runer::build_ui;

use crate::engine::extractor::*;
use crate::model::commandline::{Cli, Mode};
use gtk::glib::ExitCode;
use gtk::prelude::*;
use gtk::Application;
use gtk4 as gtk;

use crate::engine::executor::execute_flow;
use crate::engine::state::State;

pub fn parse_cmdline_args() -> Cli {
    Cli::parse()
}

pub fn initialize_logger() {
    env_logger::init();
}

pub fn handle_mod(mode: Mode) -> ExitCode {
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
            ExitCode::SUCCESS
        }
        Mode::Cli => {
            info!("Mode is C which stands for CLI. <Not Implemented>");
            ExitCode::FAILURE
        }
        Mode::Desktop => {
            let start = Instant::now();
            let application = Application::builder()
                .application_id("com.itwasneo.runer")
                .build();

            application.connect_activate(build_ui);
            info!("Application creation took {:?}", start.elapsed());
            application.run_with_args(&[] as &[&str])
        }
    }
}
