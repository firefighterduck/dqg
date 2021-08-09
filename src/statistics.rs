//! Statistics about different parts of the program.

use custom_debug_derive::Debug;
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::debug::{opt_fmt, result_fmt};
use crate::{encoding::OrbitEncoding, Error};

#[derive(Debug, PartialEq, Eq)]
pub enum StatisticsLevel {
    None,
    Basic,
    Full,
}

impl From<u64> for StatisticsLevel {
    #[cfg(not(tarpaulin_include))]
    fn from(level: u64) -> Self {
        match level {
            0 => Self::None,
            1 => Self::Basic,
            _ => Self::Full,
        }
    }
}

/// Counts how many orbits have the same size.
/// Stores the as a map from orbit size to number
/// of orbits with this size.
#[derive(Default)]
pub struct OrbitStatistics {
    pub orbit_sizes: HashMap<usize, usize>,
}

impl OrbitStatistics {
    #[cfg(not(tarpaulin_include))]
    pub fn log_orbit(&mut self, orbit: &OrbitEncoding) {
        let orbit_size = orbit.1.len();
        match self.orbit_sizes.get_mut(&orbit_size) {
            Some(number) => *number += 1,
            None => {
                self.orbit_sizes.insert(orbit_size, 1);
            }
        };
    }
}

#[derive(Debug)]
pub struct QuotientStatistics {
    pub quotient_size: usize,
    pub max_orbit_size: usize,
    pub min_orbit_size: usize,
    #[debug(with = "result_fmt")]
    pub descriptive: Result<bool, Error>,
    #[debug(with = "opt_fmt")]
    pub validated: Option<bool>,
    pub quotient_handling_time: Duration,
    pub kissat_time: Duration,
    pub orbit_gen_time: Duration,
    pub quotient_gen_time: Duration,
    pub encoding_time: Duration,
    pub orbit_sizes: OrbitStatistics,
}

#[derive(Debug)]
pub struct Statistics {
    // Meta information
    #[debug(skip)]
    level: StatisticsLevel,
    #[debug(skip)]
    out_file: PathBuf,
    // Timings
    #[debug(skip)]
    start_time: Instant,
    #[debug(with = "opt_fmt")]
    nauty_done_time: Option<Duration>,
    #[debug(with = "opt_fmt")]
    end_time: Option<Duration>,
    #[debug(with = "opt_fmt")]
    graph_sort_time: Option<Duration>,
    // Graph statistics
    graph_size: usize,
    #[debug(with = "opt_fmt")]
    number_of_generators: Option<usize>,
    max_orbit_size: usize,
    max_quotient_graph_size: usize,
    number_of_descriptive: usize,
    #[debug(with = "opt_fmt")]
    max_quotient_handling_time: Option<Duration>,
    #[debug(with = "opt_fmt")]
    max_kissat_time: Option<Duration>,
    quotient_statistics: Vec<QuotientStatistics>,
}

impl Statistics {
    #[cfg(not(tarpaulin_include))]
    pub fn new(level: StatisticsLevel, out_file: PathBuf, graph_size: usize) -> Self {
        debug_assert!(level != StatisticsLevel::None);

        Statistics {
            level,
            out_file,
            start_time: Instant::now(),
            nauty_done_time: None,
            end_time: None,
            graph_sort_time: None,
            graph_size,
            number_of_generators: None,
            max_orbit_size: 0,
            max_quotient_graph_size: 0,
            number_of_descriptive: 0,
            max_quotient_handling_time: None,
            max_kissat_time: None,
            quotient_statistics: Vec::new(),
        }
    }

    #[cfg(not(tarpaulin_include))]
    pub fn log_nauty_done(&mut self) {
        self.nauty_done_time = Some(self.start_time.elapsed());
    }

    #[cfg(not(tarpaulin_include))]
    pub fn log_graph_sorted(&mut self, duration: Duration) {
        self.graph_sort_time = Some(duration);
    }

    #[cfg(not(tarpaulin_include))]
    pub fn log_end(&mut self) {
        self.end_time = Some(self.start_time.elapsed());
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
        self.number_of_descriptive += if *quotient_statistic.descriptive.as_ref().unwrap_or(&false)
        {
            1
        } else {
            0
        };
        self.max_quotient_handling_time = if let Some(qh_time) = self.max_quotient_handling_time {
            Some(qh_time.max(quotient_statistic.quotient_handling_time))
        } else {
            Some(quotient_statistic.quotient_handling_time)
        };
        self.max_kissat_time = if let Some(ks_time) = self.max_kissat_time {
            Some(ks_time.max(quotient_statistic.kissat_time))
        } else {
            Some(quotient_statistic.kissat_time)
        };

        if self.level == StatisticsLevel::Full {
            self.quotient_statistics.push(quotient_statistic);
        }
    }

    #[cfg(not(tarpaulin_include))]
    pub fn save_statistics(&self) -> Result<(), Error> {
        let mut statistics_file = File::create(&self.out_file)?;
        write!(statistics_file, "Raw Statistics: {:#?}", self).map_err(Error::from)
    }
}
