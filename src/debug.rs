//! Debug facilities.
use flussab_cnf::cnf::{write_clause, write_header, Header};
use itertools::Itertools;
use kissat_rs::Literal;
use nom::error::{VerboseError, VerboseErrorKind};
use std::{
    fmt::{self, Debug, Display},
    io::{self, Write},
    time::Duration,
};

use crate::{
    encoding::{Clause, HighLevelEncoding, QuotientGraphEncoding},
    graph::{Graph, GraphError, VertexIndex},
    parser::{BinParseError, ParseError},
    permutation::Permutation,
    quotient::Orbits,
    statistics::{OrbitStatistics, Statistics},
};

// Error types and From<...> implementations

#[derive(Debug)]
pub struct MetricError(pub String);

impl Display for MetricError {
    #[cfg(not(tarpaulin_include))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

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
    #[error("Unknown metric used")]
    MetricError(MetricError),
}

impl From<GraphError> for Error {
    #[cfg(not(tarpaulin_include))]
    fn from(ge: GraphError) -> Self {
        Self::GraphError(ge)
    }
}

#[cfg(not(tarpaulin_include))]
fn handle_nom_verbose_error<E: Debug>(
    should_print: bool,
    verbose: VerboseError<E>,
) -> Vec<VerboseErrorKind> {
    verbose
        .errors
        .into_iter()
        .map(|(msg, kind)| {
            if should_print {
                eprintln!("{:?}", msg);
            }
            kind
        })
        .collect()
}

impl<'a> From<nom::Err<ParseError<'a>>> for Error {
    #[cfg(not(tarpaulin_include))]
    fn from(pe: nom::Err<ParseError<'a>>) -> Self {
        match pe {
            nom::Err::Error(verbose) | nom::Err::Failure(verbose) => {
                Self::ParseError(handle_nom_verbose_error(true, verbose))
            }
            nom::Err::Incomplete(_) => unreachable!(),
        }
    }
}

impl<'a> From<nom::Err<BinParseError<'a>>> for Error {
    #[cfg(not(tarpaulin_include))]
    fn from(pe: nom::Err<BinParseError<'a>>) -> Self {
        match pe {
            nom::Err::Error(verbose) | nom::Err::Failure(verbose) => {
                Self::ParseError(handle_nom_verbose_error(false, verbose))
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

impl From<MetricError> for Error {
    #[cfg(not(tarpaulin_include))]
    fn from(me: MetricError) -> Self {
        Self::MetricError(me)
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
pub fn write_formula_dimacs(
    writer: &mut impl Write,
    formula: &[Clause],
    variable_number: usize,
) -> Result<(), Error> {
    let header = Header {
        var_count: variable_number,
        clause_count: formula.len(),
    };
    write_header(writer, header)?;

    for clause in formula {
        write_clause(writer, clause)?;
    }

    writer.flush().map_err(Error::from)
}

#[cfg(not(tarpaulin_include))]
pub fn print_orbits_nauty_style(orbits: Orbits, statistics: Option<&Statistics>) {
    // This is necessary to give a correct
    // start point for the output.
    let runtime = if let Some(statistics) = statistics {
        statistics.start_time.elapsed()
    } else {
        Duration::ZERO
    };
    println!("cpu time = {:.6} seconds", runtime.as_secs_f64());

    orbits
        .encode_high()
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

#[cfg(not(tarpaulin_include))]
pub fn print_generator(mut generator: Permutation) {
    let cycles = generator.get_cycles();

    if cycles.is_empty() {
        println!("Identity permutation.");
        return;
    }

    for cycle in cycles {
        print!("(");
        Itertools::intersperse(
            cycle.iter().map(|vertex| vertex.to_string()),
            " ".to_string(),
        )
        .for_each(|ele| print!("{}", ele));
        print!(") ");
    }
    println!();
}

#[cfg(not(tarpaulin_include))]
pub fn print_dot(quotient_encoding: QuotientGraphEncoding, graph: &Graph) -> Result<(), Error> {
    println!("graph graphname {{");

    let colors = vec!["red", "green", "blue", "black", "yellow", "orange"]; // I don't expect to print more than 4 orbits at a time with one color per orbit.

    let mut vertices_in_core = quotient_encoding
        .1
        .iter()
        .map(|(_, vertices)| vertices)
        .cloned()
        .flatten()
        .collect::<Vec<VertexIndex>>();
    vertices_in_core.sort_unstable();

    for (orbit, color) in quotient_encoding.1.iter().zip(colors) {
        for vertex in orbit.1.iter() {
            println!("{:?} [color={:?}];", vertex, color);
            for end in graph.get_vertex(*vertex)?.edges_to.iter() {
                if vertex < end && vertices_in_core.binary_search(end).is_ok() {
                    println!("{:?} -- {:?};", vertex, end);
                }
            }
        }
    }

    println!("}}");
    Ok(())
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

#[macro_export]
macro_rules! parse_single_line {
    ($ret:ident, $exp:expr) => {
        let (res, $ret) = $exp?;
        eof::<crate::parser::Input<'_>, crate::parser::ParseError<'_>>(res)?;
    };
}

#[macro_export]
macro_rules! get_line {
    ($ret:ident, $lines:ident) => {
        let $ret = $lines.next().unwrap_or_else(|| {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Unexpected EOF!",
            ))
        })?;
    };
}

#[macro_export]
macro_rules! get_line_parse {
    ($lines:ident, $ret:ident, $exp:expr) => {
        crate::get_line!(line, $lines);
        let (res, $ret) = $exp(&line)?;
        eof::<crate::parser::Input<'_>, crate::parser::ParseError<'_>>(res)?;
    };
}

#[macro_export]
macro_rules! get_line_recognize {
    ($lines:ident, $exp:expr) => {
        crate::get_line!(line, $lines);
        let (res, _) = $exp(&line)?;
        eof::<crate::parser::Input<'_>, crate::parser::ParseError<'_>>(res)?;
    };
}
