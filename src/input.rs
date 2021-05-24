//! Provides convenient functions to read
//! graphs from stdin. Uses similar commands
//! as dreadnaut.

use std::{
    env::current_dir,
    fs::read_to_string,
    io::{self, BufRead, Stdin, Write},
    path::PathBuf,
};
use structopt::StructOpt;

use crate::{
    graph::{Graph, VertexIndex},
    parser::parse_dreadnaut_input,
    statistics::{Statistics, StatisticsLevel},
    Error, Settings,
};

#[derive(StructOpt, Debug)]
#[structopt(name = "DQG")]
struct CommandLineOptions {
    /// Test whole powerset of the generators.
    #[structopt(short = "-p", long)]
    iter_powerset: bool,
    /// Read a file from command line.
    #[structopt(short = "-m", long)]
    read_memory_pipe: bool,
    /// Stops after computing the orbits and
    /// outputs these in a nauty-like fashion.
    #[structopt(short = "-o", long)]
    orbits_only: bool,
    /// Logs all orbit sizes in a HashMap.
    #[structopt(short = "-l", long)]
    log_orbits: bool,
    /// Print formula instead of solving it.
    #[structopt(short = "-f", long)]
    print_formula: bool,
    /// Graph is colored and colors should be
    /// included in the nauty computation.
    #[structopt(short = "-c", long)]
    colored_graph: bool,
    /// Level of detail for statistics.
    /// None if left out, basic if `-s`, full for more than one `-s`.
    #[structopt(short = "-s", parse(from_occurrences = StatisticsLevel::from))]
    statistics_level: StatisticsLevel,
    /// The input file to read from. Optional.
    /// Same path will be used for output.
    /// Reads through CLI if not specified.
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,
}

#[cfg(not(tarpaulin_include))]
fn read_graph_empty(stdin: &Stdin) -> Result<Graph, io::Error> {
    let mut buffer = String::new();
    let number_of_vertices;

    println!("Input graph size:");
    loop {
        print!("n=");
        io::stdout().flush()?;
        stdin.read_line(&mut buffer)?;

        match buffer.trim().parse::<usize>() {
            Ok(n) => {
                number_of_vertices = n;
                break;
            }
            Err(_) => println!("Please insert only natural numbers in decimal!"),
        }

        buffer.clear();
    }

    Ok(Graph::new_ordered(number_of_vertices))
}

#[cfg(not(tarpaulin_include))]
fn read_vertex(index: VertexIndex, graph: &mut Graph, stdin: &Stdin) -> Result<bool, Error> {
    let mut line_buffer = String::new();
    let mut should_continue = true;

    'input: loop {
        print!("Edges from {}: ", index);
        io::stdout().flush()?;

        line_buffer.clear();
        stdin.read_line(&mut line_buffer)?;
        for input_part in line_buffer.split_whitespace() {
            if let Ok(end) = input_part.parse::<VertexIndex>() {
                if end < graph.size() as i32 {
                    graph.add_edge(index, end)?;
                } else {
                    println!(
                        "Please only input valid vertex indices (i.e. between 0 and {})!",
                        graph.size()
                    );
                    continue 'input;
                }
            } else if input_part.chars().next().unwrap_or(' ') == ';' {
                break 'input;
            } else if input_part.chars().next().unwrap_or(' ') == '.' {
                should_continue = false;
                break 'input;
            } else {
                break;
            }
        }

        println!("Please insert the edges from vertex {} in this format: `Edges from {}: #1 #2 #3 (;|.)` where \
#i is the vertex number of the end node of an edge from this vertex. Also end the line with a `;` to continue \
with the next vertex or a `.` to end inputting edges.", index, index);
    }

    Ok(should_continue)
}

#[cfg(not(tarpaulin_include))]
pub fn read_graph() -> Result<(Graph, Option<Statistics>, Settings), Error> {
    let cl_options = CommandLineOptions::from_args();
    let mut graph;
    let mut out_file;
    let statistics;

    if let Some(path_to_graph_file) = cl_options.input {
        // Either read the graph from a file ..
        let file = read_to_string(&path_to_graph_file)?;
        graph = parse_dreadnaut_input(&file)?;

        out_file = path_to_graph_file;
        out_file.set_extension("dqg");
    } else {
        // ... or from stdin.
        let stdin = io::stdin();

        if cl_options.read_memory_pipe {
            // Stdin can either mean a memory pipe ...
            let file = stdin
                .lock()
                .lines()
                .map(|mut line| {
                    line.iter_mut().for_each(|line| line.push('\n'));
                    line
                })
                .fold(String::new(), |mut acc, line| {
                    line.iter().for_each(|line| acc.push_str(line));
                    acc
                });
            graph = parse_dreadnaut_input(&file)?;
        } else {
            // .... or the interactive command line interface.
            graph = read_graph_empty(&stdin)?;

            for i in 0..graph.size() {
                if !read_vertex(i as VertexIndex, &mut graph, &stdin)? {
                    break;
                }
            }
        }

        out_file =
            current_dir().expect("Statistics feature requires current directory to be accessible!");
        out_file.push("statistics.dqg");
    }

    // Start the statistics after the graph reading is done.
    if cl_options.statistics_level == StatisticsLevel::None {
        statistics = None;
    } else {
        statistics = Some(Statistics::new(
            cl_options.statistics_level,
            out_file,
            graph.size(),
        ));
    }

    let settings = Settings {
        iter_powerset: cl_options.iter_powerset,
        orbits_only: cl_options.orbits_only,
        log_orbits: cl_options.log_orbits,
        print_formula: cl_options.print_formula,
        colored_graph: cl_options.colored_graph,
    };

    Ok((graph, statistics, settings))
}
