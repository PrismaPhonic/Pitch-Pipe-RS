use circular_buffer::CircularBuffer;
use num::{complex::ComplexFloat, pow::Pow, Complex};

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

#[derive(Default)]
pub struct MaxDistanceEstimator {
    previous: Option<f64>,
    // From the JS codebase:
    // This is used to track the top speeds. We will take the minimum top speed, assuming others
    // are outliers due to noise or system tracking errors.
    //
    // It seems like this has nothing to do with speed or velocity, but keeping the naming
    // the same.
    speeds: [f64; 5],
}

impl MaxDistanceEstimator {
    pub fn new() -> Self {
        Self {
            previous: None,
            speeds: [0.0; 5],
        }
    }

    pub fn update(&mut self, sample: f64, stddev: f64) {
        if let Some(previous) = self.previous {
            let delta = (previous - sample).abs();

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
        }
        self.previous = Some(sample);
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

pub struct ThreeAxisMaxDistanceEstimator {
    noise_std_dev: f64,
    x: MaxDistanceEstimator,
    y: MaxDistanceEstimator,
    z: MaxDistanceEstimator,
}

impl ThreeAxisMaxDistanceEstimator {
    pub fn new(noise_std_dev: f64) -> Self {
        Self {
            noise_std_dev,
            x: MaxDistanceEstimator::new(),
            y: MaxDistanceEstimator::new(),
            z: MaxDistanceEstimator::new(),
        }
    }

    pub fn update(&mut self, x: f64, y: f64, z: f64) {
        self.x.update(x, self.noise_std_dev);
        self.y.update(y, self.noise_std_dev);
        self.z.update(z, self.noise_std_dev);
    }

    pub fn max_within_reason(&self) -> f64 {
        self.x
            .max_within_reason()
            .max(self.y.max_within_reason())
            .max(self.z.max_within_reason())
    }
}

/// Estimates power spectral density on the monitor_hz frequency
/// in order to estimate Gaussian white noise variance in
/// an input device signal. When using, ensure the user is
/// idle. Slow movements are fine, but jerks and abrupt
/// stops may inflate the estimate.
///
/// Note 1, sample_hz / 2 is the Nyquist frequency, the highest
/// frequency we can monitor. To simply things, let monitor_hz
/// represent a countdown offset from the Nyquist frequency in
/// 0, 1, 2, etc.
///
/// Note 2, for illustrative purposes, this object is written to
/// monitor one frequency, but can easily be rewritten to
/// efficiently monitor multiple frequencies.
pub struct NoiseEstimator<const N: usize> {
    // Sample frequency as an integer. Should be an integer and ideally an even number.
    sample_hz: u64,
    // To efficiently allocate an internal circular buffer on the stack
    // we make the construction of the NoiseEstimator take a generic
    // of the circular buffer size. This is usually the number of samples in one second.
    samples: CircularBuffer<N, Complex<f64>>,
    power: f64,
    count: u64,

    x0: Complex<f64>,
    x1: Complex<f64>,
    x2: Complex<f64>,

    w0: Complex<f64>,
    w1: Complex<f64>,
    w2: Complex<f64>,

    w: f64,
}

impl<const N: usize> NoiseEstimator<N> {
    pub fn new(monitor_hz: usize) -> Self {
        use std::f64::consts::PI;

        let monitor_hz = (N / 2) - monitor_hz;

        // A buffer to store one seconds worth of samples
        let mut samples = CircularBuffer::<N, Complex<f64>>::new();
        samples.fill(Complex::new(0.0, 0.0));

        // x1 represents the frequency we want to monitor, but
        // for a Hanning window, we need its neighbors as well.
        let x0 = Complex::new(0.0, 0.0);
        let x1 = Complex::new(0.0, 0.0);
        let x2 = Complex::new(0.0, 0.0);

        let w0 = Complex::new(0.0, -2.0 * PI * (monitor_hz as f64 - 1.0) / N as f64).exp();
        let w1 = Complex::new(0.0, -2.0 * PI * monitor_hz as f64 / N as f64).exp();
        let w2 = Complex::new(0.0, -2.0 * PI * (monitor_hz as f64 + 1.0) / N as f64).exp();

        let mut w = 0.0;

        for hz in 0..N {
            let tmp = 2.0 * PI * hz as f64 / (N as f64 - 1.0);
            let win = 0.5 - 0.5 * tmp.cos();
            w += win.pow(2);
        }

        Self {
            sample_hz: N as u64,
            samples,
            power: 0.0,
            count: 0,
            x0,
            x1,
            x2,
            w0,
            w1,
            w2,
            w,
        }
    }

    pub fn update(&mut self, sample: f64) {
        let sample = Complex::new(sample, 0.0);

        self.x0 = self.w0 * (self.x0 + sample - unsafe { self.samples.get(0).unwrap_unchecked() });
        self.x1 = self.w1 * (self.x1 + sample - unsafe { self.samples.get(0).unwrap_unchecked() });
        self.x2 = self.w2 * (self.x2 + sample - unsafe { self.samples.get(0).unwrap_unchecked() });

        self.samples.push_back(sample);
        self.count += 1;

        if self.count >= self.sample_hz {
            let tmp = (Complex::new(0.5, 0.0) * self.x1)
                - (Complex::new(0.25, 0.0) * self.x0)
                - (Complex::new(0.25, 0.0) * self.x2);

            self.power += tmp.abs().pow(2);
        }
    }

    pub fn variance(&self) -> Option<f64> {
        // If we haven't gone through one round of the circular buffer, then we can't determine
        // variance yet.
        if self.count <= self.sample_hz {
            return None;
        }

        let n = self.count - self.sample_hz;

        Some(self.power / (n as f64 * self.w))
    }
}

/// Estimates noise in signal across three axis. N in this case should be the frequency and
/// allocates a circular ring buffer at compile time so we can stack allocate the ring buffer.
///
/// It maps to frequency because each ring buffer has 1 seconds worth of samples.
#[derive(Default)]
pub struct ThreeAxisNoiseEstimator<const N: usize> {
    // TODO: See if we can have these not be in Vecs. Right now they are heap allocated which kind
    // of defeats the point of the circular buffers being stack allocated.
    //
    // Consider turning on generic_const_exprs and depending on nightly.
    // We could also require it as one more generic and leverage the caller passing the value in,
    // but this seems really clunky.
    x: Vec<NoiseEstimator<N>>,
    y: Vec<NoiseEstimator<N>>,
    z: Vec<NoiseEstimator<N>>,
    stats: RunningStatistics,

    // Used to determine wen the 95% confidence interval determines that we are within the given
    // threshold of the mean.
    //
    // 0.1 is the typical default value.
    threshold: f64,
}

impl<const N: usize> ThreeAxisNoiseEstimator<N> {
    pub fn new() -> Self {
        let mut x = vec![];
        let mut y = vec![];
        let mut z = vec![];

        let freq_cnt = N / 2 - 10;

        for monitor_hz in 0..freq_cnt {
            x.push(NoiseEstimator::new(monitor_hz));
            y.push(NoiseEstimator::new(monitor_hz));
            z.push(NoiseEstimator::new(monitor_hz));
        }

        Self {
            x,
            y,
            z,
            stats: RunningStatistics::default(),

            threshold: 0.1,
        }
    }

    // Update estimate with new samples. Note - we assume noise is homogeneous across all axis.
    //
    // Returns true once the 95% CI width is within a given threshold of the mean.
    pub fn update(&mut self, x: f64, y: f64, z: f64) -> bool {
        for i in 0..self.x.len() {
            self.x[i].update(x);
            self.y[i].update(y);
            self.z[i].update(z);

            let var_x = self.x[i].variance();
            let var_y = self.y[i].variance();
            let var_z = self.z[i].variance();

            match (var_x, var_y, var_z) {
                (Some(var_x), Some(var_y), Some(var_z)) => {
                    self.stats.update(var_x);
                    self.stats.update(var_y);
                    self.stats.update(var_z);
                }
                _ => continue,
            }
        }

        let ratio = (2.0 * self.stats.ci95) / self.stats.mean;
        ratio < self.threshold
    }

    // Returns white noise variance estimates which is the mean of our
    // PSD estimates.
    pub fn mean_variance(&self) -> f64 {
        self.stats.mean
    }
}

// Similar to the noise estimator above for now, we need to use a multidimensional table from the
// original JS database - I have no idea where this table came from or how to create one for
// different frequencies, but it's a 60 hz table - so we might as well hard code for 60 hz anyways
// for now.
pub struct SixtyHzThreeAxisNoiseEstimator {
    x: [NoiseEstimator<60>; 20],
    y: [NoiseEstimator<60>; 20],
    z: [NoiseEstimator<60>; 20],
    stats: RunningStatistics,

    // Used to determine wen the 95% confidence interval determines that we are within the given
    // threshold of the mean.
    //
    // 0.1 is the typical default value.
    threshold: f64,
}

impl Default for SixtyHzThreeAxisNoiseEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl SixtyHzThreeAxisNoiseEstimator {
    // TODO: There *must* be a better way to do this.
    fn noise_estimators() -> [NoiseEstimator<60>; 20] {
        [
            NoiseEstimator::new(0),
            NoiseEstimator::new(1),
            NoiseEstimator::new(2),
            NoiseEstimator::new(3),
            NoiseEstimator::new(4),
            NoiseEstimator::new(5),
            NoiseEstimator::new(6),
            NoiseEstimator::new(7),
            NoiseEstimator::new(8),
            NoiseEstimator::new(9),
            NoiseEstimator::new(10),
            NoiseEstimator::new(11),
            NoiseEstimator::new(12),
            NoiseEstimator::new(13),
            NoiseEstimator::new(14),
            NoiseEstimator::new(15),
            NoiseEstimator::new(16),
            NoiseEstimator::new(17),
            NoiseEstimator::new(18),
            NoiseEstimator::new(19),
        ]
    }

    pub fn new() -> Self {
        Self {
            x: Self::noise_estimators(),
            y: Self::noise_estimators(),
            z: Self::noise_estimators(),
            stats: RunningStatistics::default(),

            threshold: 0.1,
        }
    }

    // Update estimate with new samples. Note - we assume noise is homogeneous across all axis.
    //
    // Returns true once the 95% CI width is within a given threshold of the mean.
    pub fn update(&mut self, x: f64, y: f64, z: f64) -> bool {
        for i in 0..20 {
            self.x[i].update(x);
            self.y[i].update(y);
            self.z[i].update(z);

            let var_x = self.x[i].variance();
            let var_y = self.y[i].variance();
            let var_z = self.z[i].variance();

            match (var_x, var_y, var_z) {
                (Some(var_x), Some(var_y), Some(var_z)) => {
                    self.stats.update(var_x);
                    self.stats.update(var_y);
                    self.stats.update(var_z);
                }
                _ => continue,
            }
        }

        let ratio = (2.0 * self.stats.ci95) / self.stats.mean;
        ratio < self.threshold
    }
}
