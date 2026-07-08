//! The proprietary gurobi solver
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::lp_format::*;
use crate::solvers::{
    Solution, SolverProgram, SolverWithSolutionParsing, Status, WithMaxSeconds, WithMipGap,
    WithMipStart,
};
use crate::util::buf_contains;

use super::WithNbThreads;

/// The proprietary gurobi solver
#[derive(Debug, Clone)]
pub struct GurobiSolver {
    name: String,
    command_name: String,
    temp_solution_file: Option<PathBuf>,
    temp_mip_start_file: Option<PathBuf>,
    seconds: Option<u32>,
    mipgap: Option<f32>,
    threads: Option<u32>,
    algorithm: Option<u32>,
    crossover: Option<u32>,
    verbose: Option<u32>,
    predual: Option<u32>,
    barconvtol: Option<f32>
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
            temp_mip_start_file: None,
            seconds: None,
            mipgap: None,
            threads: None,
            algorithm: None,
            crossover: None,
            verbose: None,
            predual: None,
            barconvtol: None
        }
    }
    /// set the name of the commandline gurobi executable to use
    pub fn command_name(&self, command_name: String) -> GurobiSolver {
        GurobiSolver {
            name: self.name.clone(),
            command_name,
            temp_solution_file: self.temp_solution_file.clone(),
            temp_mip_start_file: self.temp_mip_start_file.clone(),
            seconds: None,
            mipgap: self.mipgap,
            threads: self.threads,
            algorithm: self.algorithm.clone(),
            crossover: self.crossover,
            verbose: self.verbose,
            predual: self.predual,
            barconvtol: self.barconvtol
        }
    }

    fn algorithm(&self) -> Option<u32> {
        self.algorithm
    }

    /// Add solving method using gurobi syntax
    pub fn with_algorithm(&self, algorithm: u32) -> GurobiSolver {
        GurobiSolver {
            algorithm: Some(algorithm),
            ..(*self).clone()
        }
    }

    fn crossover(&self) -> Option<u32> {
        self.crossover
    }

    /// Tell gurobi to use crossover
    pub fn with_crossover(&self, crossover: u32) -> GurobiSolver {
        GurobiSolver {
            crossover: Some(crossover),
            ..(*self).clone()
        }
    }

    fn verbose(&self) -> Option<u32> {
        self.verbose
    }

    /// Toggle verbose
    pub fn set_verbose(&self, verbose: u32) -> GurobiSolver {
        GurobiSolver {
            verbose: Some(verbose),
            ..(*self).clone()
        }
    }

    fn predual(&self) -> Option<u32> {
        self.predual
    }

    /// Toggle predual
    pub fn set_predual(&self, predual: u32) -> GurobiSolver {
        GurobiSolver {
            predual: Some(predual),
            ..(*self).clone()
        }
    }

    fn barconvtol(&self) -> Option<f32> {
        self.barconvtol
    }

    /// Toggle predual
    pub fn with_barconvtol(&self, barconvtol: f32) -> GurobiSolver {
        GurobiSolver {
            barconvtol: Some(barconvtol),
            ..(*self).clone()
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

impl WithMaxSeconds<GurobiSolver> for GurobiSolver {
    fn max_seconds(&self) -> Option<u32> {
        self.seconds
    }

    fn with_max_seconds(&self, seconds: u32) -> GurobiSolver {
        GurobiSolver {
            seconds: Some(seconds),
            ..(*self).clone()
        }
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


impl WithNbThreads<GurobiSolver> for GurobiSolver {
    fn nb_threads(&self) -> Option<u32> {
        self.threads
    }
    fn with_nb_threads(&self, threads: u32) -> GurobiSolver {
        GurobiSolver {
            threads: Some(threads),
            ..(*self).clone()
        }
    }
}
impl WithMipStart<GurobiSolver> for GurobiSolver {
    /// create a (temporary) mip start file (.mst) and store the path reference in the solver struct.
    /// file is persisted; caller may want to delete
    fn with_mip_start(&self, assignments: &HashMap<String, f32>) -> Result<GurobiSolver, String> {
        let mut tmp = tempfile::Builder::new()
            .prefix("lp-solvers-gurobi-")
            .suffix(".mst")
            .tempfile()
            .map_err(|e| e.to_string())?;

        writeln!(tmp, "# MIP start (generated by lp-solvers)").map_err(|e| e.to_string())?;

        let mut deterministic_assignments: Vec<_> = assignments.iter().collect();
        deterministic_assignments.sort_by(|(a, _), (b, _)| a.cmp(b));

        for (var, val) in deterministic_assignments {
            writeln!(tmp, "{} {}", var, val).map_err(|e| e.to_string())?;
        }
        tmp.flush().map_err(|e| e.to_string())?;

        let path = tmp.into_temp_path().keep().map_err(|e| e.to_string())?;

        Ok(GurobiSolver {
            temp_mip_start_file: Some(path),
            ..(*self).clone()
        })
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

        if let Some(threads) = self.nb_threads() {
            let mut arg_threads: OsString = "Threads=".into();
            arg_threads.push::<OsString>(threads.to_string().into());
            args.push(arg_threads);
        }

        if let Some(algo) = self.algorithm() {
            let mut arg_algo: OsString = "Method=".into();
            arg_algo.push::<OsString>(algo.to_string().into());
            args.push(arg_algo);
        }

        if let Some(crossover) = self.crossover() {
            let mut arg_crossover: OsString = "Crossover=".into();
            arg_crossover.push::<OsString>(crossover.to_string().into());
            args.push(arg_crossover);
        }

        if let Some(verbose) = self.verbose() {
            let mut arg_verbose: OsString = "LogToConsole=".into();
            arg_verbose.push::<OsString>(verbose.to_string().into());
            args.push(arg_verbose);
        }

        if let Some(predual) = self.predual() {
            let mut arg_predual: OsString = "PreDual=".into();
            arg_predual.push::<OsString>(predual.to_string().into());
            args.push(arg_predual);
        }

        if let Some(barconvtol) = self.barconvtol() {
            let mut arg_barconvtol: OsString = "BarConvTol=".into();
            arg_barconvtol.push::<OsString>(barconvtol.to_string().into());
            args.push(arg_barconvtol);
        }
        if let Some(seconds) = self.max_seconds() {
            let mut arg_timelimit: OsString = "TimeLimit=".into();
            arg_timelimit.push::<OsString>(seconds.to_string().into());
            args.push(arg_timelimit);
        }

        if let Some(mst_file_path) = &self.temp_mip_start_file {
            let mut arg_inputfile: OsString = "InputFile=".into();
            arg_inputfile.push::<OsString>(mst_file_path.clone().into_os_string());
            args.push(arg_inputfile);
        }

        args.push(lp_file.into());

        args
    }

    fn preferred_temp_solution_file(&self) -> Option<&Path> {
        self.temp_solution_file.as_deref()
    }

    fn solution_suffix(&self) -> Option<&str> {
        Some(".sol")
    }

    fn parse_stdout_status(&self, stdout: &[u8]) -> Option<Status> {
        if buf_contains(stdout, "Optimal solution found (tolerance 1.00e-04)") {
            Some(Status::Optimal)
        } else if buf_contains(stdout, "Optimal solution found (tolerance ") {
            Some(Status::MipGap)
        } else if buf_contains(stdout, "Time limit reached") {
            if buf_contains(stdout, "Best objective -,") {
                Some(Status::NotSolved)
            } else {
                Some(Status::TimeLimit)
            }
        } else if buf_contains(stdout, "infeasible") || buf_contains(stdout, "Infeasible model") {
            Some(Status::Infeasible)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::solvers::{GurobiSolver, SolverProgram, WithMaxSeconds, WithMipGap, WithMipStart};
    use std::collections::HashMap;
    use std::env::temp_dir;
    use std::ffi::{OsStr, OsString};
    use std::path::Path;

    #[test]
    fn cli_args_default() {
        let solver = GurobiSolver::new();
        let args = solver.arguments(Path::new("test.lp"), Path::new("test.sol"));

        let expected: Vec<OsString> = vec!["ResultFile=test.sol".into(), "test.lp".into()];

        assert_eq!(args, expected);
    }

    #[test]
    fn cli_args_seconds() {
        let solver = GurobiSolver::new().with_max_seconds(10);
        let args = solver.arguments(Path::new("test.lp"), Path::new("test.sol"));

        let expected: Vec<OsString> = vec![
            "ResultFile=test.sol".into(),
            "TimeLimit=10".into(),
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
    fn cli_args_input_file() {
        let solver = GurobiSolver::new()
            .with_mip_start(&HashMap::from([
                ("x".to_owned(), 1.0_f32),
                ("y".to_owned(), -2.5_f32),
            ]))
            .expect("mip start should be valid");

        let args = solver.arguments(Path::new("test.lp"), Path::new("test.sol"));

        let input_file_argument = args
            .iter()
            .find(|a| a.to_string_lossy().starts_with("InputFile="))
            .expect("expected an InputFile=... argument")
            .to_string_lossy()
            .to_string();

        let input_file_path = Path::new(input_file_argument.strip_prefix("InputFile=").unwrap());

        assert!(
            input_file_path.exists(),
            "MIP start file does not exist: {:?}",
            input_file_path
        );

        assert_eq!(
            input_file_path.extension(),
            Some(OsStr::new("mst")),
            "InputFile not an .mst: {:?}",
            input_file_path
        );

        assert!(
            input_file_path.starts_with(&temp_dir()),
            "InputFile not under temp dir.\n  temp: {:?}\n  file: {:?}",
            temp_dir(),
            input_file_path
        );
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
