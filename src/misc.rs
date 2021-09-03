use std::{fs::File, io::BufReader, str::FromStr};

use crate::debug::MetricError;
use crate::{
    metric::{BiggestOrbits, LeastOrbits, Metric, Sparsity},
    quotient::QuotientGraph,
};

#[cfg(not(tarpaulin_include))]
pub fn do_if_some<F, T>(optional: &mut Option<T>, f: F)
where
    F: FnOnce(&mut T),
{
    if let Some(val) = optional {
        f(val);
    }
}

#[derive(Debug)]
pub enum NautyTraces {
    /// Calls dense nauty
    Nauty,
    /// Calls sparse nauty
    SparseNauty,
    /// Calls Traces (only for sparse graphs)
    Traces,
}

impl Default for NautyTraces {
    fn default() -> Self {
        Self::Nauty
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricUsed {
    LeastOrbits,
    BiggestOrbits,
    Sparsity,
    Standard,
}

impl MetricUsed {
    #[cfg(not(tarpaulin_include))]
    pub fn compare_quotients(
        &self,
        left: &QuotientGraph,
        right: &QuotientGraph,
    ) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match &self {
            Self::LeastOrbits => LeastOrbits::compare_quotients(left, right),
            Self::BiggestOrbits => BiggestOrbits::compare_quotients(left, right),
            Self::Sparsity => Sparsity::compare_quotients(left, right),
            Self::Standard => Ordering::Less,
        }
    }
}

impl FromStr for MetricUsed {
    type Err = MetricError;

    #[cfg(not(tarpaulin_include))]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "least_orbits" {
            Ok(Self::LeastOrbits)
        } else if s == "biggest_orbit" {
            Ok(Self::BiggestOrbits)
        } else if s == "sparsity" {
            Ok(Self::Sparsity)
        } else if s == "standard" {
            Ok(Self::Standard)
        } else {
            Err(MetricError(s.to_string()))
        }
    }
}

impl Default for MetricUsed {
    #[cfg(not(tarpaulin_include))]
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Default)]
pub struct Settings {
    /// Iterate the whole powerset.
    pub iter_powerset: bool,
    /// Compute only orbits.
    pub orbits_only: bool,
    /// Log orbit sizes.
    pub log_orbits: bool,
    /// Print formula instead of solving it.
    pub print_formula: bool,
    /// Graph is colored and colors should be
    /// included in the nauty computation.
    pub colored_graph: bool,
    /// Search for the smallest non-descriptive quotient
    /// core in the first non-descriptive quotient graph.
    pub nondescriptive_core: bool,
    /// Search in the whole automorphism group instead
    /// of a set of generators.
    pub search_group: bool,
    /// Validate each descriptiveness result
    /// with exhaustive search for consistent
    /// transversals.
    pub validate: bool,
    /// Use the given metric to find the "best" quotient
    /// and use it as described by the other flags.
    pub metric: Option<MetricUsed>,
    /// Evaluate a log file as printed by
    /// the quotientPlanning tool.
    pub evaluate: Option<BufReader<File>>,
    ///  Call nauty or traces.
    pub nauyt_or_traces: NautyTraces,
}
