mod args;
mod visualizer;

use std::fs::{self, File};

use args::Args;
use clap::Parser;
use log::{info, warn};
use pedoni_simulator::{diagnostic::DiagnositcLog, scenario::Scenario, Simulator};

use crate::visualizer::{Visualizer, VisualizerOptions};

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_module("pedoni", log::LevelFilter::Info)
        .init();

    if cfg!(debug_assertions) {
        warn!("Debug build");
    }

    let args = Args::parse();
    let scenario: Scenario = toml::from_str(&fs::read_to_string(&args.scenario)?)?;

    std::fs::create_dir("output").ok();

    let animation_path = chrono::Local::now()
        .format("output/%Y-%m-%d_%H-%M-%S.gif")
        .to_string();
    let visualizer = Visualizer::new(&animation_path, &scenario, VisualizerOptions::default());

    let log_path = chrono::Local::now()
        .format("logs/%Y-%m-%d_%H-%M-%S_log.json")
        .to_string();
    let mut log_file = File::create(&log_path)?;
    let mut diagnostic_log = DiagnositcLog::default();

    let mut simulator = Simulator::new(args.to_simulator_options(), scenario);

    for step in 0..args.max_steps.unwrap_or(1000) {
        let step_metrics = simulator.tick();

        if step % 10 == 0 {
            visualizer.render(&simulator);
        }
        if simulator.step % 100 == 0 {
            info!(
                "Step: {:6}, Active pedestrians: {:6}",
                simulator.step, step_metrics.active_ped_count
            );
        }

        diagnostic_log.push(step_metrics);
    }

    info!("Exported animated GIF file: {}", animation_path);

    serde_json::to_writer(&mut log_file, &diagnostic_log)?;
    info!("Exported log file: {}", log_path);

    Ok(())
}
