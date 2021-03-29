//! The proprietary gurobi solver
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::lp_format::*;
use crate::solvers::{Solution, SolverProgram, SolverWithSolutionParsing, Status};

/// The proprietary gurobi solver
#[derive(Debug, Clone)]
pub struct GurobiSolver {
    name: String,
    command_name: String,
    temp_solution_file: Option<PathBuf>,
}

impl Default for GurobiSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl GurobiSolver {
    /// create a solver instance
    pub fn new() -> GurobiSolver {
        GurobiSolver {
            name: "Gurobi".to_string(),
            command_name: "gurobi_cl".to_string(),
            temp_solution_file: None,
        }
    }
    /// set the name of the commandline gurobi executable to use
    pub fn command_name(&self, command_name: String) -> GurobiSolver {
        GurobiSolver {
            name: self.name.clone(),
            command_name,
            temp_solution_file: self.temp_solution_file.clone(),
        }
    }
}

impl SolverWithSolutionParsing for GurobiSolver {
    fn read_specific_solution<'a, P: LpProblem<'a>>(
        &self,
        f: &File,
        _problem: Option<&'a P>,
    ) -> Result<Solution, String> {
        let mut vars_value: HashMap<_, _> = HashMap::new();
        let mut file = BufReader::new(f);
        let mut buffer = String::new();
        let _ = file.read_line(&mut buffer);

        if buffer.split(' ').next().is_some() {
            for line in file.lines() {
                let l = line.unwrap();

                // Gurobi version 7 add comments on the header file
                if let Some('#') = l.chars().next() {
                    continue;
                }

                let result_line: Vec<_> = l.split_whitespace().collect();
                if result_line.len() == 2 {
                    match result_line[1].parse::<f32>() {
                        Ok(n) => {
                            vars_value.insert(result_line[0].to_string(), n);
                        }
                        Err(e) => return Err(e.to_string()),
                    }
                } else {
                    return Err("Incorrect solution format".to_string());
                }
            }
        } else {
            return Err("Incorrect solution format".to_string());
        }
        Ok(Solution::new(Status::Optimal, vars_value))
    }
}

impl SolverProgram for GurobiSolver {
    fn command_name(&self) -> &str {
        &self.command_name
    }

    fn arguments(&self, lp_file: &Path, solution_file: &Path) -> Vec<OsString> {
        let mut arg0: OsString = "ResultFile=".into();
        arg0.push(solution_file.as_os_str());
        vec![arg0, lp_file.into()]
    }

    fn preferred_temp_solution_file(&self) -> Option<&Path> {
        self.temp_solution_file.as_deref()
    }

    fn parse_stdout_status(&self, stdout: &[u8]) -> Option<Status> {
        if buf_contains(stdout, "Optimal solution found") {
            Some(Status::Optimal)
        } else if buf_contains(stdout, "infeasible") {
            Some(Status::Infeasible)
        } else {
            None
        }
    }
}

fn buf_contains(haystack: &[u8], needle: &str) -> bool {
    let needle = needle.as_bytes();
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
