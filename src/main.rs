mod engine;
mod model;
mod start_up;

use std::time::Instant;

use gtk4::glib::ExitCode;
use log::info;

fn main() -> ExitCode {
    let start = Instant::now();
    let args = start_up::parse_cmdline_args();
    start_up::initialize_logger();
    let exit_code = start_up::handle_mod(args.mode);
    info!("Program executed in {:?}", start.elapsed());
    exit_code
}
