use std::time::{Duration, Instant, SystemTime};

#[derive(Debug)]
struct QuotientStatistics {
    qutoient_size: usize,
    max_orbit_size: usize,
    min_orbit_size: usize,
    formula_size: usize,
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
}
