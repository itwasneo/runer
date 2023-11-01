mod engine;
mod model;
mod start_up;

use log::info;
use std::time::Instant;

fn main() {
    let start = Instant::now();
    let args = start_up::parse_cmdline_args();
    start_up::initialize_logger();
    start_up::handle_mod(args.mode);
    info!("Program executed in {:?}", start.elapsed());
}
