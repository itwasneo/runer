mod engine;
mod model;

use anyhow::Result;
use engine::extractor::*;
use log::{error, info};
use std::time::Instant;

use crate::engine::executor::execute_flow;
use crate::engine::state::State;

fn main() -> Result<(), ()> {
    let start = Instant::now();
    env_logger::init();

    let rune = extract_rune(".runer").map_err(|e| error!("{e}"))?;

    analyze_fragments(&rune);

    let state = State::default().from_rune(rune);
    smol::block_on(execute_flow(0, state)).map_err(|e| error!("{e}"))?;

    let duration = start.elapsed();
    info!("Time elapsed: {:?}", duration);
    Ok(())
}
