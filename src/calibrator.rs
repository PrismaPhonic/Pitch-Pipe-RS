use num::pow::Pow;

use crate::{
    estimators::{SixtyHzThreeAxisNoiseEstimator, ThreeAxisMaxDistanceEstimator},
    tuner::Tuner,
};

#[derive(Default)]
pub struct StartCalibration;

pub struct NoiseCalibrator {
    noise_estimator: SixtyHzThreeAxisNoiseEstimator,
}

pub struct AmplitudeCalibrator {
    noise_std_dev: f64,
    amplitude_estimator: ThreeAxisMaxDistanceEstimator,
}

impl StartCalibration {
    pub fn new() -> Self {
        Self
    }

    // Returns the first stage of calibration which is noise calibration.
    pub fn first_stage(self) -> NoiseCalibrator {
        NoiseCalibrator {
            noise_estimator: SixtyHzThreeAxisNoiseEstimator::new(0.1),
        }
    }
}

impl NoiseCalibrator {
    // Processes the noise - returns true when completed.
    pub fn process_noise(&mut self, x: f64, y: f64, z: f64) -> bool {
        self.noise_estimator.update(x, y, z)
    }

    // Should be called when process_noise returns true (complete to a satisfactory statstical
    // level) -> transforms into the next calibration stage of amplitude calibration.
    pub fn next(self) -> AmplitudeCalibrator {
        let noise_std_dev = self.noise_estimator.mean_variance();
        AmplitudeCalibrator {
            noise_std_dev,
            amplitude_estimator: ThreeAxisMaxDistanceEstimator::new(noise_std_dev),
        }
    }
}

impl AmplitudeCalibrator {
    // Processes motion data for highest amplitude.
    pub fn process_amplitude(&mut self, x: f64, y: f64, z: f64) {
        self.amplitude_estimator.update(x, y, z);
    }

    // When amplitude calibration is done, this can be called to generate all required tuning
    // settings for tuning a one euro filter.
    pub fn tuning_settings(self, least_precision: f64, worst_lag_secs: f64) -> TuningSettings {
        TuningSettings {
            max_target_precision: least_precision / 3.0,
            max_lag_secs: worst_lag_secs,
            noise_variance: self.noise_std_dev.pow(2),
            max_amplitude: self.amplitude_estimator.max_within_reason(),
            sample_rate: 60.0,
        }
    }

    pub fn tuner(self, least_precision: f64, worst_lag_secs: f64) -> Tuner {
        Tuner::new(self.tuning_settings(least_precision, worst_lag_secs))
    }
}

pub struct TuningSettings {
    pub max_target_precision: f64,
    pub max_lag_secs: f64,
    pub noise_variance: f64,
    pub max_amplitude: f64,
    pub sample_rate: f64,
}
