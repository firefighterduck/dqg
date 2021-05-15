use crate::{graph::VertexIndex, Error};
use itertools::{Itertools, MinMaxResult};
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug)]
pub struct QuotientStatistics {
    pub quotient_size: usize,
    pub max_orbit_size: usize,
    pub min_orbit_size: usize,
    pub formula_size: usize,
    pub descriptive: Result<bool, Error>,
}

impl QuotientStatistics {
    #[cfg(not(tarpaulin_include))]
    pub fn log_orbit_sizes(orbits: &[VertexIndex]) -> (usize, usize) {
        let mut counter = vec![0usize; orbits.len()];
        orbits
            .iter()
            .for_each(|orbit| counter[*orbit as usize] += 1);
        match counter.iter().filter(|size| **size > 0).minmax() {
            MinMaxResult::NoElements => (0, 0),
            MinMaxResult::OneElement(m) => (*m, *m),
            MinMaxResult::MinMax(min, max) => (*min, *max),
        }
    }
}

#[derive(Debug)]
pub struct Statistics {
    // Timings
    start_time: Instant,
    start_time_sys: SystemTime,
    nauty_done_time: Option<Duration>,
    nauty_done_time_sys: Option<Duration>,
    end_time: Option<Duration>,
    end_time_sys: Option<Duration>,
    // Graph statistics
    graph_size: usize,
    number_of_generators: Option<usize>,
    max_orbit_size: usize,
    max_quotient_graph_size: usize,
    max_formula_size: usize,
    number_of_descriptive: usize,
    #[cfg(feature = "full-statistics")]
    quotient_statistics: Vec<QuotientStatistics>,
}

impl Statistics {
    #[cfg(not(tarpaulin_include))]
    pub fn new(graph_size: usize) -> Self {
        Statistics {
            start_time: Instant::now(),
            start_time_sys: SystemTime::now(),
            nauty_done_time: None,
            nauty_done_time_sys: None,
            end_time: None,
            end_time_sys: None,
            graph_size,
            number_of_generators: None,
            max_orbit_size: 0,
            max_quotient_graph_size: 0,
            max_formula_size: 0,
            number_of_descriptive: 0,
            #[cfg(feature = "full-statistics")]
            quotient_statistics: Vec::new(),
        }
    }

    #[cfg(not(tarpaulin_include))]
    pub fn log_nauty_done(&mut self) {
        self.nauty_done_time = Some(self.start_time.elapsed());
        self.nauty_done_time_sys = self.start_time_sys.elapsed().ok();
    }

    #[cfg(not(tarpaulin_include))]
    pub fn log_end(&mut self) {
        self.end_time = Some(self.start_time.elapsed());
        self.end_time_sys = self.start_time_sys.elapsed().ok();
    }

    #[cfg(not(tarpaulin_include))]
    pub fn log_number_of_generators(&mut self, number_of_generators: usize) {
        self.number_of_generators = Some(number_of_generators);
    }

    #[cfg(not(tarpaulin_include))]
    pub fn log_quotient_statistic(&mut self, quotient_statistic: QuotientStatistics) {
        self.max_orbit_size = self.max_orbit_size.max(quotient_statistic.max_orbit_size);
        self.max_quotient_graph_size = self
            .max_quotient_graph_size
            .max(quotient_statistic.quotient_size);
        self.max_formula_size = self.max_formula_size.max(quotient_statistic.formula_size);
        self.number_of_descriptive += if *quotient_statistic.descriptive.as_ref().unwrap_or(&false)
        {
            1
        } else {
            0
        };
        #[cfg(feature = "full-statistics")]
        self.quotient_statistics.push(quotient_statistic);
    }
}
