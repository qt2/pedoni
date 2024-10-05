#[derive(Debug)]
pub struct Diagnostic {
    pub history: Vec<DiagnosticMetrics>,
    pub history_size: usize,
    pub history_cursor: usize,
    pub time_setup_field: i32,
}

impl Default for Diagnostic {
    fn default() -> Self {
        let history_size = 60;

        Diagnostic {
            history: vec![DiagnosticMetrics::default(); history_size],
            history_size,
            history_cursor: 0,
            time_setup_field: 0,
        }
    }
}

impl Diagnostic {
    pub fn push(&mut self, metrics: DiagnosticMetrics) {
        // let prev_cursor = self.history_cursor;
        // let prev = &self.history[prev_cursor];
        // metrics = DiagnosticMetrics {
        //     time_calc_state_cumsum: metrics.time_calc_state_cumsum + prev.time_calc_state_cumsum,
        //     time_apply_state_cumsum: metrics.time_apply_state_cumsum + prev.time_apply_state_cumsum,
        //     ..metrics
        // };

        self.history_cursor = (self.history_cursor + 1) % self.history_size;
        self.history[self.history_cursor] = metrics;
    }

    pub fn last(&self) -> &DiagnosticMetrics {
        &self.history[self.history_cursor]
    }
}

#[derive(Debug, Default, Clone)]
pub struct DiagnosticMetrics {
    pub active_ped_count: i32,
    pub time_calc_state: f64,
    pub time_apply_state: f64,
}
