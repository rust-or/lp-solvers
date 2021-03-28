//! This module provides the interface to different solvers.
//!
//! Both [`coin_cbc`](https://docs.rs/coin_cbc/latest/coin_cbc/) and
//! [`minilp`](https://docs.rs/minilp/0.2.2/minilp/) are available as cargo
//! [features](https://doc.rust-lang.org/cargo/reference/features.html). To use
//! them, specify your dependency to `lp_modeler` accordingly in your `Cargo.toml`
//! (note the name difference of the `native_coin_cbc` feature for the `coin_cbc` crate):
//! ```toml
//! [dependencies.lp_modeler]
//! version = "4.3"
//! features = "native_coin_cbc"
//! ```
//! or:
//! ```toml
//! [dependencies.lp_modeler]
//! version = "4.3"
//! features = "minilp"
//! ```
//! For `coin_cbc` to compile, the `Cbc` library files need to be available on your system.
//! See the [`coin_cbc` project README](https://github.com/KardinalAI/coin_cbc) for more infos.
//!
//! The other solvers need to be installed externally on your system.
//! The respective information is provided in the project's README in the section on
//! [installing external solvers](https://github.com/jcavat/rust-lp-modeler#installing-external-solvers).

use std::collections::HashMap;
use std::fs;
use std::fs::File;

use crate::format::lp_format::LpProblem;

pub use self::cbc::*;
pub use self::glpk::*;
pub use self::gurobi::*;

pub mod cbc;
pub mod glpk;
pub mod gurobi;

#[derive(Debug, PartialEq, Clone)]
pub enum Status {
    Optimal,
    SubOptimal,
    Infeasible,
    Unbounded,
    NotSolved,
}

#[derive(Debug, Clone)]
pub struct Solution {
    pub status: Status,
    pub results: HashMap<String, f32>,
}

impl Solution {
    pub fn new(status: Status, results: HashMap<String, f32>) -> Solution {
        Solution { status, results }
    }
}

pub trait SolverTrait {
    fn run<'a, P: LpProblem<'a>>(&self, problem: &'a P) -> Result<Solution, String>;
}

pub trait SolverWithSolutionParsing {
    fn read_solution<'a, P: LpProblem<'a>>(
        &self,
        temp_solution_file: &str,
        problem: Option<&'a P>,
    ) -> Result<Solution, String> {
        match File::open(temp_solution_file) {
            Ok(f) => {
                let res = self.read_specific_solution(&f, problem)?;
                let _ = fs::remove_file(temp_solution_file);
                Ok(res)
            }
            Err(_) => Err("Cannot open file".to_string()),
        }
    }
    fn read_specific_solution<'a, P: LpProblem<'a>>(
        &self,
        f: &File,
        problem: Option<&'a P>,
    ) -> Result<Solution, String>;
}

pub trait WithMaxSeconds<T> {
    fn max_seconds(&self) -> Option<u32>;
    fn with_max_seconds(&self, seconds: u32) -> T;
}

pub trait WithNbThreads<T> {
    fn nb_threads(&self) -> Option<u32>;
    fn with_nb_threads(&self, threads: u32) -> T;
}
