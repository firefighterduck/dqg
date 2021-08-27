use crate::quotient::QuotientGraph;

pub trait Metric {
    fn compare_quotients(left: &QuotientGraph, right: &QuotientGraph) -> std::cmp::Ordering;
}

/// The quotient with the least number of orbits
/// has the highest priority.
#[derive(Debug)]
pub struct LeastOrbits;
impl Metric for LeastOrbits {
    #[cfg(not(tarpaulin_include))]
    fn compare_quotients(left: &QuotientGraph, right: &QuotientGraph) -> std::cmp::Ordering {
        left.quotient_graph.size().cmp(&right.quotient_graph.size())
    }
}

/// The quotient with the biggest maximum orbit size
/// has the highest priority.
#[derive(Debug)]
pub struct BiggestOrbits;
impl Metric for BiggestOrbits {
    #[cfg(not(tarpaulin_include))]
    fn compare_quotients(left: &QuotientGraph, right: &QuotientGraph) -> std::cmp::Ordering {
        let left_biggest = left.get_orbit_sizes().1;
        let right_biggest = right.get_orbit_sizes().1;
        left_biggest.cmp(&right_biggest).reverse()
    }
}

/// The sparsest quotient
/// has the highest priority.
#[derive(Debug)]
pub struct Sparsity;
impl Metric for Sparsity {
    #[cfg(not(tarpaulin_include))]
    fn compare_quotients(left: &QuotientGraph, right: &QuotientGraph) -> std::cmp::Ordering {
        let left_sparsity_coefficient =
            left.quotient_graph.number_edges() as f64 / left.quotient_graph.size() as f64;
        let right_sparsity_coefficient =
            right.quotient_graph.number_edges() as f64 / right.quotient_graph.size() as f64;
        left_sparsity_coefficient
            .partial_cmp(&right_sparsity_coefficient)
            .expect("Sparsity coefficients should be comparable")
    }
}
