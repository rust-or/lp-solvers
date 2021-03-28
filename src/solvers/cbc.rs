use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::format::lp_format::*;
use crate::solvers::{Solution, SolverProgram, SolverWithSolutionParsing, Status, WithMaxSeconds, WithNbThreads};

#[derive(Debug, Clone)]
pub struct CbcSolver {
    name: String,
    command_name: String,
    temp_solution_file: Option<PathBuf>,
    threads: Option<u32>,
    seconds: Option<u32>,
}

impl Default for CbcSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl CbcSolver {
    pub fn new() -> CbcSolver {
        CbcSolver {
            name: "Cbc".to_string(),
            command_name: "cbc".to_string(),
            temp_solution_file: None,
            threads: None,
            seconds: None,
        }
    }

    pub fn command_name(&self, command_name: String) -> CbcSolver {
        CbcSolver {
            name: self.name.clone(),
            command_name,
            temp_solution_file: self.temp_solution_file.clone(),
            threads: None,
            seconds: None,
        }
    }

    pub fn with_temp_solution_file(&self, temp_solution_file: String) -> CbcSolver {
        CbcSolver {
            name: self.name.clone(),
            command_name: self.command_name.clone(),
            temp_solution_file: Some(temp_solution_file.into()),
            threads: None,
            seconds: None,
        }
    }
}

impl SolverWithSolutionParsing for CbcSolver {
    fn read_specific_solution<'a, P: LpProblem<'a>>(
        &self,
        f: &File,
        problem: Option<&'a P>,
    ) -> Result<Solution, String> {
        let mut vars_value: HashMap<String, _> = HashMap::new();

        // populate default values for all vars
        // CBC keeps only non-zero values from a number of variables
        if let Some(p) = problem {
            for var in p.variables() {
                vars_value.insert(var.name().to_string(), 0.0);
            }
        }

        let mut file = BufReader::new(f);
        let mut buffer = String::new();
        let _ = file.read_line(&mut buffer);

        let status = if let Some(status) = buffer.split_whitespace().next() {
            match status {
                "Optimal" => Status::Optimal,
                // Infeasible status is either "Infeasible" or "Integer infeasible"
                "Infeasible" | "Integer" => Status::Infeasible,
                "Unbounded" => Status::Unbounded,
                // "Stopped" can be "on time", "on iterations", "on difficulties" or "on ctrl-c"
                "Stopped" => Status::SubOptimal,
                _ => Status::NotSolved,
            }
        } else {
            return Err("Incorrect solution format".to_string());
        };
        for line in file.lines() {
            let l = line.unwrap();
            let mut result_line: Vec<_> = l.split_whitespace().collect();
            if result_line[0] == "**" {
                result_line.remove(0);
            };
            if result_line.len() == 4 {
                match result_line[2].parse::<f32>() {
                    Ok(n) => {
                        vars_value.insert(result_line[1].to_string(), n);
                    }
                    Err(e) => return Err(e.to_string()),
                }
            } else {
                return Err("Incorrect solution format".to_string());
            }
        }
        Ok(Solution::new(status, vars_value))
    }
}

impl WithMaxSeconds<CbcSolver> for CbcSolver {
    fn max_seconds(&self) -> Option<u32> {
        self.seconds
    }
    fn with_max_seconds(&self, seconds: u32) -> CbcSolver {
        CbcSolver {
            seconds: Some(seconds),
            ..(*self).clone()
        }
    }
}
impl WithNbThreads<CbcSolver> for CbcSolver {
    fn nb_threads(&self) -> Option<u32> {
        self.threads
    }
    fn with_nb_threads(&self, threads: u32) -> CbcSolver {
        CbcSolver {
            threads: Some(threads),
            ..(*self).clone()
        }
    }
}

impl SolverProgram for CbcSolver {
    fn command_name(&self) -> &str {
        &self.command_name
    }

    fn arguments(&self, lp_file: &Path, solution_file: &Path) -> Vec<OsString> {
        let mut args = vec![lp_file.as_os_str().to_owned()];
        if let Some(s) = self.max_seconds() {
            args.push("seconds".into());
            args.push(s.to_string().into());
        }
        for (name, value) in [
            ("seconds", self.max_seconds()),
            ("threads", self.nb_threads()),
        ].iter() {
            if let Some(val) = value {
                args.push(name.into());
                args.push(val.to_string().into());
            }
        }
        args.extend_from_slice(&[
            "solve".into(),
            "solution".into(),
            solution_file.into(),
        ]);
        args
    }

    fn preferred_temp_solution_file(&self) -> Option<&Path> {
        self.temp_solution_file.as_deref()
    }
}
