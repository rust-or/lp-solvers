//! GNU's glpk solver
//! [https://www.gnu.org/software/glpk/]
//!
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, Error};
use std::path::{Path, PathBuf};

use crate::lp_format::*;
use crate::solvers::{
    Solution, SolverProgram, SolverWithSolutionParsing, Status, WithMaxSeconds, WithMipGap,
};

/// glpk solver
#[derive(Debug, Clone)]
pub struct GlpkSolver {
    name: String,
    command_name: String,
    temp_solution_file: Option<PathBuf>,
    seconds: Option<u32>,
    mipgap: Option<f32>,
}

impl Default for GlpkSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl GlpkSolver {
    /// New glpk solver instance
    pub fn new() -> GlpkSolver {
        GlpkSolver {
            name: "Glpk".to_string(),
            command_name: "glpsol".to_string(),
            temp_solution_file: None,
            seconds: None,
            mipgap: None,
        }
    }
    /// Set the glpk command name
    pub fn command_name(&self, command_name: String) -> GlpkSolver {
        GlpkSolver {
            name: self.name.clone(),
            command_name,
            temp_solution_file: self.temp_solution_file.clone(),
            seconds: self.seconds,
            mipgap: self.mipgap,
        }
    }
    /// Set the temporary solution file to use
    pub fn with_temp_solution_file(&self, temp_solution_file: String) -> GlpkSolver {
        GlpkSolver {
            name: self.name.clone(),
            command_name: self.command_name.clone(),
            temp_solution_file: Some(temp_solution_file.into()),
            seconds: self.seconds,
            mipgap: self.mipgap,
        }
    }
}

impl SolverWithSolutionParsing for GlpkSolver {
    fn read_specific_solution<'a, P: LpProblem<'a>>(
        &self,
        f: &File,
        _problem: Option<&'a P>,
    ) -> Result<Solution, String> {
        fn read_size(line: Option<Result<String, Error>>) -> Result<usize, String> {
            match line {
                Some(Ok(l)) => match l.split_whitespace().nth(1) {
                    Some(value) => match value.parse::<usize>() {
                        Ok(v) => Ok(v),
                        _ => Err("Incorrect solution format".to_string()),
                    },
                    _ => Err("Incorrect solution format".to_string()),
                },
                _ => Err("Incorrect solution format".to_string()),
            }
        }
        let mut vars_value: HashMap<_, _> = HashMap::new();

        let file = BufReader::new(f);

        let mut iter = file.lines();
        let row = read_size(iter.nth(1))?;
        let col = read_size(iter.next())?;
        let status = match iter.nth(1) {
            Some(Ok(status_line)) => match &status_line[12..] {
                "INTEGER OPTIMAL" | "OPTIMAL" => Status::Optimal,
                "INTEGER NON-OPTIMAL" | "FEASIBLE" => Status::SubOptimal,
                "INFEASIBLE (FINAL)" | "INTEGER EMPTY" => Status::Infeasible,
                "UNDEFINED" => Status::NotSolved,
                "INTEGER UNDEFINED" | "UNBOUNDED" => Status::Unbounded,
                _ => return Err("Incorrect solution format: Unknown solution status".to_string()),
            },
            _ => return Err("Incorrect solution format: No solution status found".to_string()),
        };
        let mut result_lines = iter.skip(row + 7);
        for _ in 0..col {
            let line = match result_lines.next() {
                Some(Ok(l)) => l,
                _ => {
                    return Err("Incorrect solution format: Not all columns are present".to_string())
                }
            };
            let result_line: Vec<_> = line.split_whitespace().collect();
            if result_line.len() >= 4 {
                match result_line[3].parse::<f32>() {
                    Ok(n) => {
                        vars_value.insert(result_line[1].to_string(), n);
                    }
                    Err(e) => return Err(e.to_string()),
                }
            } else {
                return Err(
                    "Incorrect solution format: Column specification has to few fields".to_string(),
                );
            }
        }
        Ok(Solution::new(status, vars_value))
    }
}

impl WithMaxSeconds<GlpkSolver> for GlpkSolver {
    fn max_seconds(&self) -> Option<u32> {
        self.seconds
    }

    fn with_max_seconds(&self, seconds: u32) -> GlpkSolver {
        GlpkSolver {
            seconds: Some(seconds),
            ..(*self).clone()
        }
    }
}

impl WithMipGap<GlpkSolver> for GlpkSolver {
    fn mip_gap(&self) -> Option<f32> {
        self.mipgap
    }

    fn with_mip_gap(&self, mipgap: f32) -> Result<GlpkSolver, String> {
        if mipgap.is_sign_positive() && mipgap.is_finite() {
            Ok(GlpkSolver {
                mipgap: Some(mipgap),
                ..(*self).clone()
            })
        } else {
            Err("Invalid MIP gap: must be positive and finite".to_string())
        }
    }
}

impl SolverProgram for GlpkSolver {
    fn command_name(&self) -> &str {
        &self.command_name
    }

    fn arguments(&self, lp_file: &Path, solution_file: &Path) -> Vec<OsString> {
        let mut args = vec![
            "--lp".into(),
            lp_file.into(),
            "-o".into(),
            solution_file.into(),
        ];

        if let Some(seconds) = self.max_seconds() {
            args.push("--tmlim".into());
            args.push(seconds.to_string().into());
        }

        if let Some(mipgap) = self.mip_gap() {
            args.push("--mipgap".into());
            args.push(mipgap.to_string().into());
        }

        args
    }

    fn preferred_temp_solution_file(&self) -> Option<&Path> {
        self.temp_solution_file.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use crate::solvers::{GlpkSolver, SolverProgram, WithMaxSeconds, WithMipGap};
    use std::ffi::OsString;
    use std::path::Path;

    #[test]
    fn cli_args_default() {
        let solver = GlpkSolver::new();
        let args = solver.arguments(Path::new("test.lp"), Path::new("test.sol"));

        let expected: Vec<OsString> = vec![
            "--lp".into(),
            "test.lp".into(),
            "-o".into(),
            "test.sol".into(),
        ];

        assert_eq!(args, expected);
    }

    #[test]
    fn cli_args_seconds() {
        let solver = GlpkSolver::new().with_max_seconds(10);
        let args = solver.arguments(Path::new("test.lp"), Path::new("test.sol"));

        let expected: Vec<OsString> = vec![
            "--lp".into(),
            "test.lp".into(),
            "-o".into(),
            "test.sol".into(),
            "--tmlim".into(),
            "10".into(),
        ];

        assert_eq!(args, expected);
    }

    #[test]
    fn cli_args_mipgap() {
        let solver = GlpkSolver::new()
            .with_mip_gap(0.05)
            .expect("mipgap should be valid");

        let args = solver.arguments(Path::new("test.lp"), Path::new("test.sol"));

        let expected: Vec<OsString> = vec![
            "--lp".into(),
            "test.lp".into(),
            "-o".into(),
            "test.sol".into(),
            "--mipgap".into(),
            "0.05".into(),
        ];

        assert_eq!(args, expected);
    }

    #[test]
    fn cli_args_mipgap_negative() {
        let solver = GlpkSolver::new().with_mip_gap(-0.05);
        assert!(solver.is_err());
    }

    #[test]
    fn cli_args_mipgap_infinite() {
        let solver = GlpkSolver::new().with_mip_gap(f32::INFINITY);
        assert!(solver.is_err());
    }

    #[test]
    fn cli_args_multiple() {
        let solver = GlpkSolver::new()
            .with_max_seconds(10)
            .with_mip_gap(0.05)
            .expect("mipgap should be valid");

        let args = solver.arguments(Path::new("test.lp"), Path::new("test.sol"));

        let expected: Vec<OsString> = vec![
            "--lp".into(),
            "test.lp".into(),
            "-o".into(),
            "test.sol".into(),
            "--tmlim".into(),
            "10".into(),
            "--mipgap".into(),
            "0.05".into(),
        ];

        assert_eq!(args, expected);
    }
}
