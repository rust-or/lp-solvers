//! The proprietary gurobi solver
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::lp_format::*;
use crate::solvers::{Solution, SolverProgram, SolverWithSolutionParsing, Status, WithMipGap};
use crate::util::buf_contains;

/// The proprietary gurobi solver
#[derive(Debug, Clone)]
pub struct GurobiSolver {
    name: String,
    command_name: String,
    temp_solution_file: Option<PathBuf>,
    mipgap: Option<f32>,
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
            mipgap: None,
        }
    }
    /// set the name of the commandline gurobi executable to use
    pub fn command_name(&self, command_name: String) -> GurobiSolver {
        GurobiSolver {
            name: self.name.clone(),
            command_name,
            temp_solution_file: self.temp_solution_file.clone(),
            mipgap: self.mipgap,
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

impl WithMipGap<GurobiSolver> for GurobiSolver {
    fn mip_gap(&self) -> Option<f32> {
        self.mipgap
    }

    fn with_mip_gap(&self, mipgap: f32) -> Result<GurobiSolver, String> {
        if mipgap.is_sign_positive() && mipgap.is_finite() {
            Ok(GurobiSolver {
                mipgap: Some(mipgap),
                ..(*self).clone()
            })
        } else {
            Err("Invalid MIP gap: must be positive and finite".to_string())
        }
    }
}

impl SolverProgram for GurobiSolver {
    fn command_name(&self) -> &str {
        &self.command_name
    }

    fn arguments(&self, lp_file: &Path, solution_file: &Path) -> Vec<OsString> {
        let mut arg0: OsString = "ResultFile=".into();
        arg0.push(solution_file.as_os_str());

        let mut args = vec![arg0];

        if let Some(mipgap) = self.mip_gap() {
            let mut arg_mipgap: OsString = "MIPGap=".into();
            arg_mipgap.push::<OsString>(mipgap.to_string().into());
            args.push(arg_mipgap);
        }

        args.push(lp_file.into());

        args
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

#[cfg(test)]
mod tests {
    use crate::solvers::{GurobiSolver, SolverProgram, WithMipGap};
    use std::ffi::OsString;
    use std::path::Path;

    #[test]
    fn cli_args_default() {
        let solver = GurobiSolver::new();
        let args = solver.arguments(Path::new("test.lp"), Path::new("test.sol"));

        let expected: Vec<OsString> = vec![
            "ResultFile=test.sol".into(),
            "test.lp".into(),
        ];

        assert_eq!(args, expected);
    }

    #[test]
    fn cli_args_mipgap() {
        let solver = GurobiSolver::new()
            .with_mip_gap(0.05)
            .expect("mipgap should be valid");

        let args = solver.arguments(Path::new("test.lp"), Path::new("test.sol"));

        let expected: Vec<OsString> = vec![
            "ResultFile=test.sol".into(),
            "MIPGap=0.05".into(),
            "test.lp".into(),
        ];

        assert_eq!(args, expected);
    }

    #[test]
    fn cli_args_mipgap_negative() {
        let solver = GurobiSolver::new().with_mip_gap(-0.05);
        assert!(solver.is_err());
    }

    #[test]
    fn cli_args_mipgap_infinite() {
        let solver = GurobiSolver::new().with_mip_gap(f32::INFINITY);
        assert!(solver.is_err());
    }
}
