use kissat_rs::Solver;

use crate::encoding::Formula;

pub fn solve(formula: Formula) -> bool {
    matches!(Solver::solve_formula(formula), Ok(Some(_)))
}
