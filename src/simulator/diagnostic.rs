use std::fmt::Debug;

use serde::Serialize;

#[derive(Debug, Default, Clone, Serialize)]
pub struct DiagnositcLog {
    pub model: String,
    pub scenario: String,
    pub total_steps: usize,
    pub preprocess_metrics: PreprocessMetrics,
    pub step_metrics: Vec<StepMetrics>,
}

impl DiagnositcLog {
    pub fn push(&mut self, step_metrics: StepMetrics) {
        self.step_metrics.push(step_metrics);
        self.total_steps += 1;
    }
}

// pub struct Diagnostic {
//     pub history: Vec<StepMetrics>,
//     pub history_size: usize,
//     pub history_cursor: usize,
//     pub time_setup_field: i32,
//     pub time_calc_state: AggregatedMeetrics,
//     pub time_calc_state_kernel: AggregatedMeetrics,
//     pub active_ped_count: i32,
// }

// impl Debug for Diagnostic {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Diagnostic")
//             .field("active_ped_count", &self.active_ped_count)
//             .field("time_calc_state", &self.time_calc_state)
//             .field("time_calc_state_kernel", &self.time_calc_state_kernel)
//             .finish()
//     }
// }

// impl Default for Diagnostic {
//     fn default() -> Self {
//         let history_size = 60;

//         Diagnostic {
//             history: vec![StepMetrics::default(); history_size],
//             history_size,
//             history_cursor: 0,
//             time_setup_field: 0,
//             time_calc_state: AggregatedMeetrics::default(),
//             time_calc_state_kernel: AggregatedMeetrics::default(),
//             active_ped_count: 0,
//         }
//     }
// }

// impl Diagnostic {
//     pub fn push(&mut self, metrics: StepMetrics) {
//         self.history[self.history_cursor] = metrics;
//         self.history_cursor += 1;

//         if self.history_cursor >= self.history_size {
//             self.time_calc_state.init();
//             self.time_calc_state_kernel.init();

//             for metrics in self.history.iter() {
//                 self.time_calc_state.add(metrics.time_calc_state);
//                 self.time_calc_state_kernel
//                     .add(metrics.time_calc_state_kernel);
//             }

//             self.time_calc_state.finish(self.history_size);
//             self.time_calc_state_kernel.finish(self.history_size);

//             self.active_ped_count = self.history[self.history_size - 1].active_ped_count;
//             self.history_cursor = 0;
//         }
//     }

//     pub fn last(&self) -> &StepMetrics {
//         &self.history[self.history_cursor]
//     }
// }

#[derive(Debug, Default, Clone, Serialize)]
pub struct PreprocessMetrics {
    pub time_calc_field: f64,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct StepMetrics {
    pub active_ped_count: i32,
    pub time_calc_state: f64,
    pub time_calc_state_kernel: Option<f64>,
    pub time_apply_state: f64,
}

// #[derive(Default, Clone)]
// pub struct AggregatedMeetrics {
//     pub average: f64,
//     pub deviation: f64,
//     pub min: f64,
//     pub max: f64,
// }

// impl Debug for AggregatedMeetrics {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_fmt(format_args!(
//             "{:7.4} (+/- {:.4}, max: {:.4}, min: {:.4})",
//             self.average, self.deviation, self.max, self.min
//         ))
//     }
// }

// impl AggregatedMeetrics {
//     pub fn init(&mut self) {
//         *self = AggregatedMeetrics {
//             min: f64::MAX,
//             ..Default::default()
//         };
//     }

//     pub fn add(&mut self, value: f64) {
//         self.average += value;
//         self.deviation += value.powi(2);
//         self.min = self.min.min(value);
//         self.max = self.max.max(value);
//     }

//     pub fn finish(&mut self, size: usize) {
//         self.average /= size as f64;
//         self.deviation /= size as f64;
//         self.deviation -= self.average.powi(2);
//         self.deviation = self.deviation.sqrt();
//     }
// }
