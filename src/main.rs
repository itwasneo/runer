mod custom_button;
mod engine;
mod model;

use clap::Parser;
use engine::extractor::*;
use glib::clone;
use gtk::{glib, Application, ApplicationWindow, Box};
use gtk::{prelude::*, Button};
use gtk4 as gtk;
use log::{error, info};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use crate::engine::executor::execute_flow;
use crate::engine::state::State;
use crate::model::commandline::*;

use self::custom_button::CustomButton;

fn main() -> glib::ExitCode {
    let start = Instant::now();

    // Parsing Command Line Arguments. Here the application directly exits
    // if it can't parse the arguments properly.
    let args = Cli::parse();

    // Initializing Logger
    env_logger::init();

    match args.mode {
        Mode::Run(args) => {
            let rune = extract_rune(&args.file.unwrap_or_else(|| ".runer".to_owned()))
                .map_err(|e| error!("{e}"))
                .unwrap();

            analyze_fragments(&rune);

            let state = State::default().from_rune(rune);

            smol::block_on(execute_flow(0, state))
                .map_err(|e| error!("{e}"))
                .unwrap();
        }
        Mode::Cli => info!("Mode is C which stands for CLI. <Not Implemented>"),
        Mode::Desktop => info!("Mode is D which stands for Desktop. <Not Implemented>"),
    }

    let application = Application::builder()
        .application_id("com.itwasneo.runer")
        .build();

    application.connect_activate(build_ui);

    let duration = start.elapsed();
    info!("Time elapsed: {:?}", duration);

    application.run()
}

fn build_ui(application: &Application) {
    let window = ApplicationWindow::builder()
        .application(application)
        .title("runer")
        .default_height(150)
        .default_width(200)
        .build();

    let button_increase = Button::builder()
        .label("Increase")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let button_decrease = Button::builder()
        .label("Decrease")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let my_button = CustomButton::with_label("Press me!");
    my_button.set_margin_top(12);
    my_button.set_margin_bottom(12);
    my_button.set_margin_start(12);
    my_button.set_margin_end(12);

    // Reference-countet object with inner-mutability
    let number = Rc::new(Cell::new(0));

    button_increase.connect_clicked(clone!(@weak number, @strong button_decrease => move |_| {
        number.set(number.get() + 1);
        button_decrease.set_label(&number.get().to_string());
    }));
    button_decrease.connect_clicked(clone!(@strong button_increase => move |_| {
        number.set(number.get() - 1);
        button_increase.set_label(&number.get().to_string());
    }));

    my_button.connect_clicked(clone!(@strong window => move |_| {
        window.set_title(Some("Custom Button Clicked"));
    }));

    let gtk_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    gtk_box.append(&button_increase);
    gtk_box.append(&button_decrease);
    gtk_box.append(&my_button);

    window.set_child(Some(&gtk_box));

    // Presents the window to the user.
    window.present();
}
