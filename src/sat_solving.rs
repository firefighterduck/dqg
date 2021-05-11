use kissat_rs::Solver;

use crate::encoding::Formula;

#[cfg(not(tarpaulin_include))]
pub fn solve(formula: Formula) -> bool {
    matches!(Solver::solve_formula(formula), Ok(Some(_)))
}
