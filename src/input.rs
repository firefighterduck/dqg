//! Provides convenient functions to read
//! graphs from stdin. Uses similar commands
//! as dreadnaut.

use std::{
    env::current_dir,
    fs::File,
    io::{self, BufReader, Stdin, Write},
    path::PathBuf,
};
use structopt::StructOpt;

use crate::{
    graph::{Graph, VertexIndex},
    misc::CoreMetric,
    parser::{parse_csv_input, parse_dreadnaut_input, parse_txt_input},
    statistics::{Statistics, StatisticsLevel},
    Error, MetricUsed, NautyTraces, Settings,
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
    /// Outputs orbits in dreadnaut format.
    #[structopt(short = "-o", long)]
    output_orbits: bool,
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
    /// Use traces instead of nauty to compute
    /// the graphs automorphism group.
    #[structopt(short = "-t", long)]
    use_traces: bool,
    /// Use nondescriptive cores and the metric
    /// to guide the search.
    /// Possible values: recolor, pow_gen
    #[structopt(short = "-q", long)]
    nondescriptive_core: Option<CoreMetric>,
    /// Search in the whole automorphism group instead
    /// of a set of generators.
    #[structopt(short = "-g", long)]
    search_group: bool,
    /// Validate each descriptiveness result
    /// with exhaustive search for consistent
    /// transversals.
    #[structopt(short = "-v", long)]
    validate: bool,
    /// Operate in GAP mode.
    /// This means that DQG use GAP to
    /// search in the conjugacy classes.
    #[structopt(long)]
    gap_mode: bool,
    /// GIve graph size for file formats
    /// which don't contain the graph size.
    #[structopt(short = "-n", long)]
    graph_size: Option<usize>,
    /// Use the given metric to find the "best" quotient
    /// and use it as described by the other flags.
    /// Possible value: least_orbits, biggest_orbit, sparsity
    #[structopt(long)]
    metric: Option<MetricUsed>,
    /// Evaluate a log file as printed by
    /// the quotientPlanning tool.
    #[structopt(long, parse(from_os_str))]
    evaluate: Option<PathBuf>,
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
pub fn read_graph() -> Result<(Graph, Settings), Error> {
    let cl_options = CommandLineOptions::from_args();

    if let Some(eval_path) = cl_options.evaluate {
        let eval_file = File::open(eval_path)?;
        let buf = BufReader::new(eval_file);
        return Ok((
            Graph::new_ordered(0),
            Settings {
                evaluate: Some(buf),
                ..Default::default()
            },
        ));
    }

    let mut use_traces = cl_options.use_traces;
    let mut graph;
    let mut out_file;

    if let Some(path_to_graph_file) = cl_options.input {
        // Either read the graph from a file ..
        let file_buf = BufReader::new(File::open(&path_to_graph_file)?);
        let (parsed_graph, has_header) = match path_to_graph_file
            .as_path()
            .extension()
            .unwrap()
            .to_str()
            .unwrap()
        {
            "dre" => parse_dreadnaut_input(file_buf)?,
            "csv" => (
                parse_csv_input(cl_options.graph_size.unwrap(), file_buf)?,
                false,
            ),
            "txt" => (parse_txt_input(file_buf)?, false),
            _ => unimplemented!(),
        };
        use_traces |= has_header;
        graph = parsed_graph;

        out_file = path_to_graph_file;
        out_file.set_extension("dqg");
    } else {
        // ... or from stdin.
        let stdin = io::stdin();

        if cl_options.read_memory_pipe {
            // Stdin can either mean a memory pipe ...
            let file_buf = BufReader::new(stdin.lock());
            let (parsed_graph, has_header) = parse_dreadnaut_input(file_buf)?;
            use_traces |= has_header;
            graph = parsed_graph;
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
    let statistics = if cl_options.statistics_level == StatisticsLevel::None {
        None
    } else {
        Some(Statistics::new(
            cl_options.statistics_level,
            out_file,
            graph.size(),
        ))
    };

    let settings = Settings {
        iter_powerset: cl_options.iter_powerset,
        output_orbits: cl_options.output_orbits,
        log_orbits: cl_options.log_orbits,
        print_formula: cl_options.print_formula,
        colored_graph: cl_options.colored_graph,
        nondescriptive_core: cl_options.nondescriptive_core,
        search_group: cl_options.search_group,
        validate: cl_options.validate,
        gap_mode: cl_options.gap_mode,
        metric: cl_options.metric,
        evaluate: None,
        nauyt_or_traces: if use_traces {
            NautyTraces::Traces
        } else if graph.is_sparse() {
            NautyTraces::SparseNauty
        } else {
            NautyTraces::Nauty
        },
        statistics,
    };

    Ok((graph, settings))
}
