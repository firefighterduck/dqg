use kissat_rs::Solver;

use crate::{encoding::Formula, Error};

#[cfg(not(tarpaulin_include))]
pub fn solve(formula: Formula) -> Result<bool, Error> {
    Solver::decide_formula(formula).map_err(Error::from)
}
