[![Rust](https://github.com/firefighterduck/dqg/actions/workflows/rust.yml/badge.svg)](https://github.com/firefighterduck/dqg/actions/workflows/rust.yml)

## Purpose of this repository
This repository is the working repository for a research project about descriptive quotient graphs.
To be more precise, it contains a tool that takes graphs in dreadnaut-like style and computes for quotient graphs picked by a heuristic whether they are descriptive.
The quotient graphs are computed from automorphism group generators, which in turn are computed via nauty/Traces.
They are then checked for descriptiveness by the SAT solver kissat.

As long as the research project is ongoing, this repository will remain a simple working repository, i.e. issues and PRs will be ignored.
After the project is done, the tool will become open to contributions by others as well.
