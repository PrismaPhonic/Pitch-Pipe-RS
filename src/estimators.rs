/// Can be used to aggregate variance data, using the Welford algorithm:
/// https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance
///
/// It also stores an active ci95 value, otherwise known as the 95% confidence interval.
pub struct RunningStatistics {
    count: u64,
    mean: f64,
    m2: f64,
    sample_variance: f64,
    max: f64,
    ci95: f64,
}

impl Default for RunningStatistics {
    fn default() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
            sample_variance: 0.0,
            ci95: 0.0,
            max: f64::MIN,
        }
    }
}

impl RunningStatistics {
    pub fn update(&mut self, val: f64) {
        self.count += 1;
        let delta = val - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = val - self.mean;
        self.m2 += delta * delta2;

        self.max = val.max(self.max);

        self.sample_variance = self.m2 / (self.count - 1) as f64;
        self.ci95 = 1.96 * (self.sample_variance / self.count as f64).sqrt();
    }
}

pub struct MaxDistanceEstimator {
    previous: f64,
    // From the JS codebase:
    // This is used to track the top speeds. We will take the minimum top speed, assuming others
    // are outliers due to noise or system tracking errors.
    //
    // It seems like this has nothing to do with speed or velocity, but keeping the naming
    // the same.
    speeds: [f64; 5],
}

impl MaxDistanceEstimator {
    pub fn new(first_sample: f64) -> Self {
        Self {
            previous: first_sample,
            speeds: [0.0; 5],
        }
    }

    pub fn update(&mut self, sample: f64, stddev: f64) {
        let delta = (self.previous - sample).abs();

        if delta > (3.0 * stddev) {
            // Unwrap is safe - the array will never be empty.
            let min = self
                .speeds
                .iter_mut()
                .min_by(|a, b| a.total_cmp(b))
                .unwrap();

            if delta > *min {
                *min = delta;
            }
        }

        self.previous = sample;
    }

    /// Renaming this to max_within_reason. The JS codebase this was ported from calls this
    /// velocity, but that doesn't really make sense. This is used for any sensor data smoothing,
    /// and what sensors actually measure velocity? If anything we would be checking acceleration.
    /// At any rate, this is the lowest of the 5 maximum values - so we should just clearly call it
    /// that.
    pub fn max_within_reason(&self) -> f64 {
        *self.speeds.iter().min_by(|a, b| a.total_cmp(b)).unwrap()
    }
}
