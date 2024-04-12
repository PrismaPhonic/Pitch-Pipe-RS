use one_euro_rs::OneEuroFilter;

use crate::{
    calibrator::TuningSettings,
    table::{B_DIM, FC_DIM, J_DIM},
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
}

impl Tuner {
    pub fn new(settings: TuningSettings) -> Self {
        Self {
            filter: OneEuroFilter::new(settings.sample_rate, 1.0, 1.0, 1.0),
            settings,
            current_filtered_val: 0.0,
        }
    }

    // TODO: Add support to handle ringing (Might require a different one euro filter library that
    // can expose alpha).
    pub fn lag_s(&mut self) -> f64 {
        let mut cnt = 0;

        // Warm at zero
        for _ in 0..2 {
            self.current_filtered_val = self.filter.filter(0.0);
        }

        loop {
            self.current_filtered_val = self.filter.filter(self.settings.max_amplitude);

            cnt += 1;

            let delta = (self.current_filtered_val - self.settings.max_amplitude).abs();

            if delta < self.settings.max_target_precision {
                return cnt as f64 / self.settings.sample_rate;
            }
        }
    }
}
