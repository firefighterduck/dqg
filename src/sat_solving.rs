use kissat_rs::Solver;

use crate::{encoding::Clause, Error};

#[cfg(not(tarpaulin_include))]
pub fn solve(formula: impl Iterator<Item = Clause>) -> Result<bool, Error> {
    Solver::decide_formula(formula).map_err(Error::from)
}
