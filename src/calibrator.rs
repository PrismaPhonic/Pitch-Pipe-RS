use num::pow::Pow;

use crate::estimators::{SixtyHzThreeAxisNoiseEstimator, ThreeAxisMaxDistanceEstimator};

pub enum CalibrationState {
    WaitToStart,
    EstimateNoise,
    NoiseEstimateComplete,
    EstimateAmplitude,
    AmplitudeEstimateComplete,
    EstimateParameters,
    ParametersEstimateComplete,
    Tuning,
    Tuned,
}

pub struct Calibrator {
    least_precision: f64,
    worst_lag_secs: f64,

    state: CalibrationState,

    noise_estimator: SixtyHzThreeAxisNoiseEstimator,
    noise_std_dev: Option<f64>,

    amplitude_estimator: Option<ThreeAxisMaxDistanceEstimator>,
    amplitude: Option<f64>,
}

// TODO: Rather than have a single Calibrator we could use builder pattern to have the first
// Estimator generator the next one when completed and so on until we get a tuning configuration at
// the end.

impl Calibrator {
    pub fn new(least_precision: f64, worst_lag_secs: f64) -> Self {
        Self {
            least_precision,
            worst_lag_secs,

            state: CalibrationState::WaitToStart,
            noise_estimator: SixtyHzThreeAxisNoiseEstimator::new(0.1),
            noise_std_dev: None,

            amplitude_estimator: None,
            amplitude: None,
        }
    }

    // Sets the calibration state to EstimateNoise - should be called once before streaming updates
    // to process_noise()
    pub fn prepare_noise(&mut self) {
        self.state = CalibrationState::EstimateNoise;
    }

    pub fn process_noise(&mut self, x: f64, y: f64, z: f64) -> bool {
        let complete = self.noise_estimator.update(x, y, z);
        if complete {
            let noise_std_dev = self.noise_estimator.mean_variance();
            self.amplitude_estimator = Some(ThreeAxisMaxDistanceEstimator::new(noise_std_dev));

            self.state = CalibrationState::NoiseEstimateComplete;
        }

        complete
    }

    /// Sets the calibration state to EstimateAmplitude if noise has finished being estimated. If
    /// not then we don't have the correct data to proceed and false is returned to notify the
    /// caller that they need to first process noise.
    pub fn prepare_amplitude(&mut self) -> bool {
        if self.noise_std_dev.is_none() {
            return false;
        }

        self.state = CalibrationState::EstimateAmplitude;
        true
    }

    // Processes motion data for highest amplitude.
    // returns true if successful or false if we do not have an amplitude estimator yet. We may not
    // have one yet if we have not processed noise yet.
    pub fn process_amplitude(&mut self, x: f64, y: f64, z: f64) -> bool {
        if let Some(amp_est) = &mut self.amplitude_estimator {
            amp_est.update(x, y, z);
            true
        } else {
            false
        }
    }

    // Processes motion data for highest amplitude without checking to see if we have an amplitude
    // estimator yet. Will produce undefined behavior if we do not have an amplitude estimator yet
    // which may happen if we have not processed noise yet.
    pub fn process_amplitude_unchecked(&mut self, x: f64, y: f64, z: f64) {
        unsafe {
            self.amplitude_estimator
                .as_mut()
                .unwrap_unchecked()
                .update(x, y, z);
        }
    }

    // Finalizes amplitude processing. Returns false if we don't have an amplitude estimator yet,
    // which would occur if we have not performed the noise processing yet.
    pub fn finalize_amplitude_processing(&mut self) -> bool {
        if let Some(estimator) = &self.amplitude_estimator {
            self.amplitude = Some(estimator.max_within_reason());
            self.state = CalibrationState::AmplitudeEstimateComplete;

            true
        } else {
            false
        }
    }

    // Finalizes amplitude processing.
    // unwraps the amplitidue estimator without checking - undefined behavior if we don't have an
    // amplitude estimator. Use with caution.
    pub fn finalize_amplitude_processing_unchecked(&mut self) {
        self.amplitude = Some(unsafe {
            self.amplitude_estimator
                .as_ref()
                .unwrap_unchecked()
                .max_within_reason()
        });
        self.state = CalibrationState::AmplitudeEstimateComplete;
    }

    // Proceed with generating tuning settings - should only be called if we have finished
    // processing both noise and amplitude.
    //
    // Return None if we have not completed both noise detection and amplitude processing.
    pub fn tuning_settings(&self) -> Option<TuningSettings> {
        match (self.noise_std_dev, self.amplitude) {
            (Some(noise_std_dev), Some(amplitude)) => Some(TuningSettings {
                max_target_precision: self.least_precision / 3.0,
                max_lag_secs: self.worst_lag_secs,
                noise_variance: noise_std_dev.pow(2),
                max_amplitude: amplitude,
                sample_rate: 60.0,
            }),
            _ => None,
        }
    }
}

pub struct TuningSettings {
    max_target_precision: f64,
    max_lag_secs: f64,
    noise_variance: f64,
    max_amplitude: f64,
    sample_rate: f64,
}
