//! Debug facilities.
use kissat_rs::Literal;
use nom::error::VerboseErrorKind;
use std::{
    fmt,
    io::{self, Write},
};

use crate::{
    encoding::{Clause, HighLevelEncoding},
    graph::GraphError,
    parser::ParseError,
    quotient::Orbits,
    statistics::OrbitStatistics,
};

// Error type and From<...> implementations

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Graph initialization error")]
    GraphError(GraphError),
    #[error("Error while parsing input file with graph description")]
    ParseError(Vec<VerboseErrorKind>),
    #[error("Error while parsing graph from command line")]
    CLIParseError(io::Error),
    #[error("Error while calling Kissat")]
    KissatError(kissat_rs::Error),
}

impl From<GraphError> for Error {
    #[cfg(not(tarpaulin_include))]
    fn from(ge: GraphError) -> Self {
        Self::GraphError(ge)
    }
}

impl<'a> From<nom::Err<ParseError<'a>>> for Error {
    #[cfg(not(tarpaulin_include))]
    fn from(pe: nom::Err<ParseError<'a>>) -> Self {
        match pe {
            nom::Err::Error(verbose) | nom::Err::Failure(verbose) => {
                Self::ParseError(verbose.errors.into_iter().map(|(_, kind)| kind).collect())
            }
            nom::Err::Incomplete(_) => unreachable!(),
        }
    }
}

impl From<kissat_rs::Error> for Error {
    #[cfg(not(tarpaulin_include))]
    fn from(ke: kissat_rs::Error) -> Self {
        Self::KissatError(ke)
    }
}

impl From<io::Error> for Error {
    #[cfg(not(tarpaulin_include))]
    fn from(ie: io::Error) -> Self {
        Self::CLIParseError(ie)
    }
}

// Custom debug methods

impl fmt::Debug for OrbitStatistics {
    #[cfg(not(tarpaulin_include))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.orbit_sizes.fmt(f)
    }
}

#[cfg(not(tarpaulin_include))]
fn print_clause<'a>(clause: impl Iterator<Item = &'a Literal>) {
    print!("(");
    itertools::Itertools::intersperse(
        clause.map(|literal| {
            if *literal < 0 {
                format!("¬{}", -1 * literal)
            } else {
                format!("{}", literal)
            }
        }),
        " ∨ ".to_string(),
    )
    .for_each(|part| print!("{}", part));

    println!(") ∧");
}

#[cfg(not(tarpaulin_include))]
pub fn print_formula(formula: impl Iterator<Item = Clause>) {
    formula.for_each(|clause| print_clause(clause.iter()));
    println!("True");
}

#[cfg(not(tarpaulin_include))]
pub fn print_orbits_nauty_style(orbits: Orbits) {
    // This is necessary to give a correct
    // start point for the output.
    println!("cpu time = 0.00 seconds");

    orbits
        .encode_high(false)
        .into_iter()
        .for_each(|(orbit, members)| {
            if members.len() > 1 {
                members.iter().for_each(|member| print!("{} ", member));
                print!("({}); ", members.len());
            } else {
                print!("{}; ", orbit);
            }
        });

    // Force new line and flush everything out.
    println!();
    std::io::stdout()
        .flush()
        .expect("Why would stdout not be flushed?");
}

// Custom formatter for debug printing

#[cfg(not(tarpaulin_include))]
pub fn opt_fmt<T: fmt::Debug>(option: &Option<T>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match option {
        Some(val) => val.fmt(f),
        None => write!(f, "None"),
    }
}

#[cfg(not(tarpaulin_include))]
pub fn result_fmt<T: fmt::Debug, E: fmt::Debug>(
    result: &Result<T, E>,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match result {
        Ok(val) => val.fmt(f),
        Err(e) => e.fmt(f),
    }
}

#[allow(clippy::ptr_arg)]
#[cfg(not(tarpaulin_include))]
pub fn bin_fmt(vec: &Vec<u64>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{{")?;
    for number in vec {
        write!(f, "{:#066b}", number)?;
    }
    write!(f, "}}")?;

    Ok(())
}

// Debug macros that allow to time single expressions

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
