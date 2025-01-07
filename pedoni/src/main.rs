mod args;
pub mod renderer;

use std::{
    fs::{self, File},
    path::PathBuf,
    sync::{atomic::AtomicBool, Mutex},
    thread,
    time::{Duration, Instant},
};

use args::Args;
use clap::Parser;
use log::{info, warn};
use once_cell::sync::Lazy;
use pedoni_simulator::{
    diagnostic::DiagnositcLog, models::Pedestrian, scenario::Scenario, Simulator,
};

use crate::renderer::Renderer;

static SIMULATOR_STATE: Lazy<Mutex<SimulatorState>> =
    Lazy::new(|| Mutex::new(SimulatorState::default()));
static CONTROL_STATE: Mutex<ControlState> = Mutex::new(ControlState {
    paused: true,
    playback_speed: 4.0,
});
static SIG_INT: AtomicBool = AtomicBool::new(false);

pub const DELTA_TIME: f32 = 0.1;

#[derive(Default)]
pub struct SimulatorState {
    pub pedestrians: Vec<Pedestrian>,
    pub scenario: Scenario,
    pub diagnostic_log: DiagnositcLog,
}

#[derive(Clone)]
pub struct ControlState {
    pub paused: bool,
    pub playback_speed: f32,
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_module("pedoni", log::LevelFilter::Info)
        .init();

    if cfg!(debug_assertions) {
        warn!("Debug build");
    }

    let args = Args::parse();
    CONTROL_STATE.lock().unwrap().playback_speed = args.speed;

    let scenario: Scenario = toml::from_str(&fs::read_to_string(&args.scenario)?)?;
    let field_size = scenario.field.size;
    SIMULATOR_STATE.lock().unwrap().scenario = scenario.clone();

    let mut simulator = Simulator::new(args.to_simulator_options(), scenario);

    thread::spawn(move || loop {
        let start = Instant::now();
        let state = CONTROL_STATE.lock().unwrap().clone();

        if !state.paused {
            let step_metrics = simulator.tick();
            if simulator.step % 100 == 0 {
                info!(
                    "Step: {:6}, Active pedestrians: {:6}",
                    simulator.step, step_metrics.active_ped_count
                );
            }

            let mut state = SIMULATOR_STATE.lock().unwrap();
            state.pedestrians = simulator.list_pedestrians();
            state.diagnostic_log.push(step_metrics);
        }

        let step_time = Instant::now() - start;
        let min_interval = Duration::from_secs_f32(DELTA_TIME / state.playback_speed);
        if step_time < min_interval {
            thread::sleep(min_interval - step_time);
        }
    });

    if args.headless {
        info!("Run as headless mode");
        ctrlc::set_handler(|| SIG_INT.store(true, std::sync::atomic::Ordering::SeqCst))?;

        CONTROL_STATE.lock().unwrap().paused = false;

        loop {
            if SIG_INT.load(std::sync::atomic::Ordering::SeqCst) {
                let current_time = chrono::Local::now();
                fs::create_dir("logs").ok();
                let log_path: PathBuf = [
                    "logs",
                    &current_time.format("%Y-%m-%d_%H%M%S_log.json").to_string(),
                ]
                .iter()
                .collect();
                let mut log_file = File::create(&log_path)?;
                let state = SIMULATOR_STATE.lock().unwrap();

                serde_json::to_writer(&mut log_file, &state.diagnostic_log)?;
                info!("Exported log file: {}", log_path.display());

                break;
            }

            thread::sleep(Duration::from_millis(100));
        }
    } else {
        pittore::run("Pedoni", Renderer::new(field_size));
    }

    Ok(())
}
