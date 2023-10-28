use std::time::Instant;

use clap::Parser;
use log::{error, info};
use runer::build_ui;

use crate::engine::extractor::*;
use crate::model::commandline::{Cli, Mode};
use gdk4::Display;
use gtk::glib::ExitCode;
use gtk::Application;
use gtk::{prelude::*, CssProvider};
use gtk4 as gtk;

use crate::engine::executor::execute_flow;
use crate::engine::state::State;

pub fn parse_cmdline_args() -> Cli {
    Cli::parse()
}

pub fn initialize_logger(mode: Mode) {
    if mode != Mode::Desktop {
        env_logger::init();
    }
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

            application.connect_startup(|_| load_css());
            application.connect_activate(build_ui);
            info!("Application creation took {:?}", start.elapsed());
            application.run_with_args(&[] as &[&str])
        }
    }
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_data(include_str!("style.css"));

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
