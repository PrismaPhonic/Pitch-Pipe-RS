use one_euro_rs::OneEuroFilter;

use crate::{
    calibrator::TuningSettings,
    table::{B_DIM, FC_DIM, J_DIM, SIXTYHZ},
};

pub struct Grid {
    table: [[[f64; B_DIM]; FC_DIM]; J_DIM],
}

impl Grid {
    pub fn new(table: [[[f64; B_DIM]; FC_DIM]; J_DIM]) -> Self {
        Self { table }
    }

    // I don't really understand what's going on here, so this was copied verbatum from the js repo
    // created by the researchers.
    pub fn precision(&self, jitter: f64, cutoff_hz: f64, beta: f64) -> f64 {
        // Jitter level goes up in steps of 1/3 start at 1/3.
        let mut j_idx = 3.0 * jitter - 1.0;
        j_idx = j_idx.min((J_DIM - 1) as f64);
        let j_idx_lo = j_idx.floor();
        let j_idx_hi = j_idx.ceil();

        // Min cutoff goes up in steps of 0.05 starting at 0.05
        let fc_idx = cutoff_hz / 0.05 - 0.05;
        let fc_idx_lo = fc_idx.floor();
        let fc_idx_hi = fc_idx.ceil();

        let vals = Self::get_beta_index(beta);
        let b_idx = vals[0];
        let b_idx_lo = vals[1];
        let b_idx_hi = vals[2];

        let xd = if (j_idx_hi - j_idx_lo).abs() > f64::EPSILON {
            (j_idx - j_idx_lo) / (j_idx_hi - j_idx_lo)
        } else {
            0.0
        };

        let yd = if (fc_idx_hi - fc_idx_lo).abs() > f64::EPSILON {
            (fc_idx - fc_idx_lo) / (fc_idx_hi - fc_idx_lo)
        } else {
            0.0
        };

        let zd = if (b_idx_hi - b_idx_lo).abs() > f64::EPSILON {
            (b_idx - b_idx_lo) / (b_idx_hi - b_idx_lo)
        } else {
            0.0
        };

        let j_idx_lo = j_idx_lo as usize;
        let j_idx_hi = j_idx_hi as usize;

        let fc_idx_lo = fc_idx_lo as usize;
        let fc_idx_hi = fc_idx_hi as usize;

        let b_idx_lo = b_idx_lo as usize;
        let b_idx_hi = b_idx_hi as usize;

        let c000 = self.table[j_idx_lo][fc_idx_lo][b_idx_lo];
        let c100 = self.table[j_idx_hi][fc_idx_lo][b_idx_lo];
        let c010 = self.table[j_idx_lo][fc_idx_hi][b_idx_lo];
        let c110 = self.table[j_idx_hi][fc_idx_hi][b_idx_lo];
        let c001 = self.table[j_idx_lo][fc_idx_lo][b_idx_hi];
        let c101 = self.table[j_idx_hi][fc_idx_lo][b_idx_hi];
        let c011 = self.table[j_idx_lo][fc_idx_hi][b_idx_hi];
        let c111 = self.table[j_idx_hi][fc_idx_hi][b_idx_hi];

        let c00 = c000 * (1.0 - xd) + c100 * xd;
        let c01 = c001 * (1.0 - xd) + c101 * xd;
        let c10 = c010 * (1.0 - xd) + c110 * xd;
        let c11 = c011 * (1.0 - xd) + c111 * xd;

        let c0 = c00 * (1.0 - yd) + c10 * yd;
        let c1 = c01 * (1.0 - yd) + c11 * yd;

        c0 * (1.0 - zd) + c1 * zd
    }

    pub fn get_beta_index(beta: f64) -> [f64; 3] {
        let mut b_idx: f64 = 46.0;
        let mut beta = beta;
        while beta < 1.0 && b_idx > 0.0 {
            beta *= 10.00000001;
            b_idx -= 9.0;
        }

        if b_idx < 0.0 {
            return [0.0, 0.0, 0.0];
        }

        b_idx = (b_idx - 1.0).max(0.0);
        beta += b_idx;
        let b_idx_lo = beta.floor();
        let b_idx_hi = beta.ceil();

        [beta, b_idx_lo, b_idx_hi]
    }
}

pub struct Tuner {
    filter: OneEuroFilter<f64>,
    settings: TuningSettings,
    current_filtered_val: f64,
    grid: Grid,
}

impl Tuner {
    pub fn new(settings: TuningSettings) -> Self {
        Self {
            filter: OneEuroFilter::new(60.0, 1.0, 1.0, 1.0),
            settings,
            current_filtered_val: 0.0,
            grid: Grid::new(SIXTYHZ),
        }
    }

    // TODO: Add support to handle ringing (Might require a different one euro filter library that
    // can expose alpha, or we could try porting over the one euro filter design from the js
    // library.
    //
    // There is a bug in the parent JS library this is copied from though with an open ticket that
    // I would like resolved before attempting to add support for ringing. As far as I can tell
    // it's not correctly supported in the parent library.
    pub fn lag_s(&mut self, target_precision: f64) -> f64 {
        let mut cnt = 0;

        // Warm at zero
        for _ in 0..2 {
            self.current_filtered_val = self.filter.filter(0.0);
        }

        loop {
            self.current_filtered_val = self.filter.filter(self.settings.max_amplitude);

            cnt += 1;

            let delta = (self.current_filtered_val - self.settings.max_amplitude).abs();

            if delta < target_precision {
                return cnt as f64 / self.settings.sample_rate;
            }
        }
    }

    pub fn tune(&mut self) -> Option<FinalTuningSettings> {
        let noise_stddev = self.settings.noise_variance.sqrt();
        let mut best_precision = f64::MAX;
        let mut best_lag_s = f64::MAX;
        let mut best_min_cutoff_hz = None;
        let mut best_beta = 1.1;

        let mut target_precision = self.settings.max_target_precision;

        while best_precision == f64::MAX {
            for min_hz in (10..400).map(|x| x as f64 / 100.0) {
                self.filter.configuration.cutoff_min = min_hz;

                let mut beta = 1.0;
                for scale in 1..=5 {
                    let step = 10f64.powi(-scale) / 4.0;

                    for _ in 0..36 {
                        beta -= step;
                        beta = (beta * 1e6).round() / 1e6;

                        let precision = self.grid.precision(noise_stddev, min_hz, beta);

                        if precision > target_precision {
                            continue;
                        }

                        self.filter.configuration.beta = beta;

                        let lag_s = self.lag_s(target_precision);

                        let accept = if best_lag_s <= self.settings.max_lag_secs {
                            !(lag_s >= self.settings.max_lag_secs || precision > best_precision)
                        } else {
                            lag_s <= best_lag_s
                        };

                        if !accept {
                            continue;
                        }

                        best_precision = precision;
                        best_lag_s = lag_s;
                        best_beta = beta;
                        best_min_cutoff_hz = Some(min_hz);
                    }
                }
            }
            // Adjust target precision and try again if no configuration is good enough
            target_precision += 1.0 / 3.0;
        }

        best_min_cutoff_hz.map(|min_cutoff_hz| FinalTuningSettings {
            min_cutoff_hz,
            beta: best_beta,
        })
    }
}

pub struct FinalTuningSettings {
    pub min_cutoff_hz: f64,
    pub beta: f64,
}
