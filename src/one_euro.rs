use nalgebra::Point3;
use one_euro::{OneEuroFilter, OneEuroState};

pub struct ThreeAxisFilter {
    state: OneEuroState<f64, 3>,
    filter: OneEuroFilter<f64>,
    sample_rate: f64,
}

impl ThreeAxisFilter {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            state: Point3::new(0.0, 0.0, 0.0).coords.into(),
            filter: OneEuroFilter::<f64>::default(),
            sample_rate,
        }
    }

    pub fn set_dcutoff(&mut self, dcutoff: f64) {
        self.filter.set_dcutoff(dcutoff)
    }

    pub fn set_mincutoff(&mut self, mincutoff: f64) {
        self.filter.set_mincutoff(mincutoff)
    }

    pub fn set_beta(&mut self, beta: f64) {
        self.filter.set_beta(beta)
    }

    pub fn filter(&mut self, data: Point3<f64>) -> Point3<f64> {
        self.filter
            .filter(&mut self.state, &data.coords, self.sample_rate);
        (*self.state.data()).into()
    }
}
