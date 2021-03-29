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
use std::ffi::OsString;
use std::fs::File;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::NamedTempFile;

use crate::lp_format::LpProblem;

pub use self::cbc::*;
pub use self::glpk::*;
pub use self::gurobi::*;

pub mod cbc;
pub mod glpk;
pub mod gurobi;

/// Solution status
#[derive(Debug, PartialEq, Clone)]
pub enum Status {
    /// the best possible solution was found
    Optimal,
    /// A solution was found; it may not be the best one.
    SubOptimal,
    /// There is no solution for the problem
    Infeasible,
    /// There is no single finite optimum for the problem
    Unbounded,
    /// Unable to solve
    NotSolved,
}

/// A solution to a problem
#[derive(Debug, Clone)]
pub struct Solution {
    /// solution state
    pub status: Status,
    /// map from variable name to variable value
    pub results: HashMap<String, f32>,
}

impl Solution {
    /// Create a solution
    pub fn new(status: Status, results: HashMap<String, f32>) -> Solution {
        Solution { status, results }
    }
}

/// A solver that can take a problem and return a solution
pub trait SolverTrait {
    /// Run the solver on the given problem
    fn run<'a, P: LpProblem<'a>>(&self, problem: &'a P) -> Result<Solution, String>;
}

/// An external commandline solver
pub trait SolverProgram {
    /// Returns the commandline program name
    fn command_name(&self) -> &str;
    /// Returns the commandline arguments
    fn arguments(&self, lp_file: &Path, solution_file: &Path) -> Vec<OsString>;
    /// If there is a predefined solution filename
    fn preferred_temp_solution_file(&self) -> Option<&Path> {
        None
    }
    /// Parse the output of the program
    fn parse_stdout_status(&self, _stdout: &[u8]) -> Option<Status> {
        None
    }
}

/// A solver that can parse a solution file
pub trait SolverWithSolutionParsing {
    /// Use read_solution_from_path instead.
    #[deprecated]
    fn read_solution<'a, P: LpProblem<'a>>(
        &self,
        temp_solution_file: &str,
        problem: Option<&'a P>,
    ) -> Result<Solution, String> {
        Self::read_solution_from_path(self, &PathBuf::from(temp_solution_file), problem)
    }
    /// Read a solution
    fn read_solution_from_path<'a, P: LpProblem<'a>>(
        &self,
        temp_solution_file: &Path,
        problem: Option<&'a P>,
    ) -> Result<Solution, String> {
        match File::open(temp_solution_file) {
            Ok(f) => {
                let res = self.read_specific_solution(&f, problem)?;
                Ok(res)
            }
            Err(_) => Err("Cannot open file".to_string()),
        }
    }
    /// Read a solution from a file
    fn read_specific_solution<'a, P: LpProblem<'a>>(
        &self,
        f: &File,
        problem: Option<&'a P>,
    ) -> Result<Solution, String>;
}

impl<T: SolverWithSolutionParsing + SolverProgram> SolverTrait for T {
    fn run<'a, P: LpProblem<'a>>(&self, problem: &'a P) -> Result<Solution, String> {
        let command_name = self.command_name();
        let file_model = problem
            .to_tmp_file()
            .map_err(|e| format!("Unable to create {} problem file: {}", command_name, e))?;

        let named_temp;
        let temp_solution_file = if let Some(p) = self.preferred_temp_solution_file() {
            p
        } else {
            named_temp = NamedTempFile::new().map_err(|e| e.to_string())?;
            named_temp.path()
        };
        let output = Command::new(command_name)
            .args(self.arguments(file_model.path(), temp_solution_file))
            .output()
            .map_err(|e| format!("Error while running {}: {}", command_name, e))?;
        if !output.status.success() {
            return Err(format!(
                "{} exited with status {}",
                command_name, output.status
            ));
        }
        self.read_solution_from_path(temp_solution_file, Some(problem))
    }
}

/// Configure the max allowed runtime
pub trait WithMaxSeconds<T> {
    /// get max runtime
    fn max_seconds(&self) -> Option<u32>;
    /// set max runtime
    fn with_max_seconds(&self, seconds: u32) -> T;
}

/// A solver where the parallelism can be configured
pub trait WithNbThreads<T> {
    /// get thread count
    fn nb_threads(&self) -> Option<u32>;
    /// set thread count
    fn with_nb_threads(&self, threads: u32) -> T;
}


/// A static version of a solver, where the solver itself doesn't hold any data
///
/// ```
/// use lp_solvers::solvers::{StaticSolver, CbcSolver};
/// const STATIC_SOLVER : StaticSolver<CbcSolver> = StaticSolver::new();
/// ```
#[derive(Default, Copy, Clone)]
pub struct StaticSolver<T>(PhantomData<T>);

impl<T> StaticSolver<T> {
    /// Create a new static solver
    pub const fn new() -> Self {
        StaticSolver(PhantomData)
    }
}

impl<T: SolverTrait + Default> SolverTrait for StaticSolver<T> {
    fn run<'a, P: LpProblem<'a>>(
        &self,
        problem: &'a P,
    ) -> Result<Solution, String> {
        let solver = T::default();
        SolverTrait::run(&solver, problem)
    }
}
