use kissat_rs::Solver;

use crate::encoding::Formula;

pub fn solve(formula: Formula) -> bool {
    if let Ok(Some(_)) = Solver::solve_formula(formula) {
        true
    } else {
        false
    }
}
