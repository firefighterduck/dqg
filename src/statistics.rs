use custom_debug_derive::Debug;
use itertools::{Itertools, MinMaxResult};
use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{graph::VertexIndex, Error};

#[macro_export]
macro_rules! time {
    ($i:ident, $ret:ident, $exp:expr) => {
        let before = std::time::Instant::now();
        let $ret = $exp;
        let $i = before.elapsed();
    };
}

#[macro_export]
macro_rules! print_time {
    ($name:expr, $ret:ident, $exp:expr) => {
        let before = std::time::Instant::now();
        let $ret = $exp;
        println!("{} took {:?}", $name, before.elapsed());
    };
}

#[macro_export]
macro_rules! time_mut {
    ($i:ident, $ret:ident, $exp:expr) => {
        let before = std::time::Instant::now();
        let mut $ret = $exp;
        let $i = before.elapsed();
    };
}

#[macro_export]
macro_rules! print_time_mut {
    ($name:expr, $ret:ident, $exp:expr) => {
        let before = std::time::Instant::now();
        let mut $ret = $exp;
        println!("{} took {:?}", $name, before.elapsed());
    };
}

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

#[derive(Debug)]
pub struct QuotientStatistics {
    pub quotient_size: usize,
    pub max_orbit_size: usize,
    pub min_orbit_size: usize,
    pub descriptive: Result<bool, Error>,
    pub quotient_handling_time: Duration,
    pub kissat_time: Duration,
    pub orbit_gen_time: Duration,
    pub quotient_gen_time: Duration,
    pub encoding_time: Duration,
    pub log_orbit_time: Duration,
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
    // Meta information
    #[debug(skip)]
    level: StatisticsLevel,
    #[debug(skip)]
    out_file: PathBuf,
    // Timings
    #[debug(skip)]
    start_time: Instant,
    nauty_done_time: Option<Duration>,
    end_time: Option<Duration>,
    // Graph statistics
    graph_size: usize,
    number_of_generators: Option<usize>,
    max_orbit_size: usize,
    max_quotient_graph_size: usize,
    number_of_descriptive: usize,
    max_quotient_handling_time: Option<Duration>,
    max_kissat_time: Option<Duration>,
    quotient_statistics: Vec<QuotientStatistics>,
}

impl Statistics {
    #[cfg(not(tarpaulin_include))]
    pub fn new(level: StatisticsLevel, out_file: PathBuf, graph_size: usize) -> Self {
        assert!(level != StatisticsLevel::None);

        Statistics {
            level,
            out_file,
            start_time: Instant::now(),
            nauty_done_time: None,
            end_time: None,
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
