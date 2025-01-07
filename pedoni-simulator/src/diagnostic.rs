use std::fmt::Debug;

use serde::Serialize;

#[derive(Debug, Default, Clone, Serialize)]
pub struct DiagnositcLog {
    pub model: String,
    pub scenario: String,
    pub total_steps: usize,
    pub preprocess_metrics: PreprocessMetrics,
    pub step_metrics: StepMetricsCollection,
}

impl DiagnositcLog {
    pub fn push(&mut self, step_metrics: StepMetrics) {
        self.step_metrics.push(step_metrics);
        self.total_steps += 1;
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct StepMetricsCollection {
    pub active_ped_count: Vec<i32>,
    pub time_spawn: Vec<f64>,
    pub time_calc_state: Vec<f64>,
    pub time_calc_state_kernel: Vec<Option<f64>>,
}

impl StepMetricsCollection {
    pub fn push(&mut self, metrics: StepMetrics) {
        self.active_ped_count.push(metrics.active_ped_count);
        self.time_spawn.push(metrics.time_spawn);
        self.time_calc_state.push(metrics.time_calc_state);
        self.time_calc_state_kernel
            .push(metrics.time_calc_state_kernel);
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PreprocessMetrics {
    pub time_calc_field: f64,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct StepMetrics {
    pub active_ped_count: i32,
    pub time_spawn: f64,
    pub time_calc_state: f64,
    pub time_calc_state_kernel: Option<f64>,
}
