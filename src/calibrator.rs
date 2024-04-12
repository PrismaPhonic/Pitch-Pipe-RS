pub enum CalibrationState {
    WaitToStart,
    EstimateNoise,
    EstimateAmplitude,
    EstimateParameters,
    Tuned,
}

pub struct Calibrator {
    least_precision: f64,
    worst_lag_secs: f64,

    // Placeholder.
    filter: u32,

    state: CalibrationState,
}

impl Calibrator {
    pub fn new(least_precision: f64, worst_lag_secs: f64, filter: u32) -> Self {
        Self {
            least_precision,
            worst_lag_secs,
            filter,

            state: CalibrationState::WaitToStart,
        }
    }
}
