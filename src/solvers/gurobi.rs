extern crate uuid;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::Command;

use format::lp_format::*;
use solvers::{Solution, SolverTrait, SolverWithSolutionParsing, Status};

use self::uuid::Uuid;

pub struct GurobiSolver {
    name: String,
    command_name: String,
    temp_solution_file: String,
}

impl Default for GurobiSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl GurobiSolver {
    pub fn new() -> GurobiSolver {
        GurobiSolver {
            name: "Gurobi".to_string(),
            command_name: "gurobi_cl".to_string(),
            temp_solution_file: format!("{}.sol", Uuid::new_v4().to_string()),
        }
    }
    pub fn command_name(&self, command_name: String) -> GurobiSolver {
        GurobiSolver {
            name: self.name.clone(),
            command_name,
            temp_solution_file: self.temp_solution_file.clone(),
        }
    }
}

impl SolverWithSolutionParsing for GurobiSolver {
    fn read_specific_solution<'a, P: LpProblem<'a>>(&self, f: &File, _problem: Option<&'a P>) -> Result<Solution, String> {
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

impl SolverTrait for GurobiSolver {
    fn run<'a, P: LpProblem<'a>>(&self, problem: &'a P) -> Result<Solution, String> {
        let file_model = problem.to_tmp_file()
            .map_err(|e| format!("Unable to create gurobi problem file: {}", e))?;

        let r = Command::new(&self.command_name)
            .arg(format!("ResultFile={}", self.temp_solution_file))
            .arg(file_model.path())
            .output()
            .map_err(|e| format!("Error running the {} solver: {}", self.name, e))?;
        let mut status = Status::SubOptimal;
        let result = String::from_utf8(r.stdout).expect("");
        if result.contains("Optimal solution found")
        {
            status = Status::Optimal;
        } else if result.contains("infeasible") {
            status = Status::Infeasible;
        }
        if !r.status.success() {
            return Err(r.status.to_string())
        }
        self.read_solution(&self.temp_solution_file, Some(problem))
            .map(|solution| Solution { status, ..solution })
    }
}
