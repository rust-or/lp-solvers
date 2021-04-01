//! The IBM CPLEX optimizer.
//! You need to activate the "cplex" feature of this crate to use this solver.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::path::Path;

use xml::reader::XmlEvent;
use xml::EventReader;

use crate::lp_format::LpProblem;
use crate::solvers::{Solution, SolverProgram, SolverWithSolutionParsing, Status};
use crate::util::buf_contains;

/// IBM cplex optimizer
#[derive(Debug, Clone)]
pub struct Cplex {
    command: String,
}

impl Default for Cplex {
    fn default() -> Self {
        Self {
            command: "cplex".into(),
        }
    }
}

impl Cplex {
    /// Create a cplex solver from the given binary
    pub fn with_command(command: String) -> Self {
        Self { command }
    }
}

macro_rules! format_osstr {
    ($($parts:expr)*) => {{
        let mut s = OsString::new();
        $(s.push($parts);)*
        s
    }}
}

impl SolverProgram for Cplex {
    fn command_name(&self) -> &str {
        &self.command
    }

    fn arguments(&self, lp_file: &Path, solution_file: &Path) -> Vec<OsString> {
        vec![
            "-c".into(),
            format_osstr!("READ \"" lp_file "\""),
            "optimize".into(),
            format_osstr!("WRITE \"" solution_file "\""),
        ]
    }

    fn parse_stdout_status(&self, stdout: &[u8]) -> Option<Status> {
        if buf_contains(stdout, "No solution exists") {
            Some(Status::Infeasible)
        } else {
            None
        }
    }

    fn solution_suffix(&self) -> Option<&str> {
        Some(".sol")
    }
}

impl SolverWithSolutionParsing for Cplex {
    fn read_specific_solution<'a, P: LpProblem<'a>>(
        &self,
        f: &File,
        problem: Option<&'a P>,
    ) -> Result<Solution, String> {
        let len = problem.map(|p| p.variables().size_hint().0).unwrap_or(0);
        let parser = EventReader::new(f);
        let mut solution = Solution {
            status: Status::Optimal,
            results: HashMap::with_capacity(len),
        };
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => {
                    if name.local_name == "variable" {
                        let mut name = None;
                        let mut value = None;
                        for attr in attributes {
                            match attr.name.local_name.as_str() {
                                "name" => name = Some(attr.value),
                                "value" => {
                                    let parsed = attr.value.parse().map_err(|e| {
                                        format!("invalid variable value for {:?}: {}", name, e)
                                    })?;
                                    value = Some(parsed)
                                }
                                _ => {}
                            };
                        }
                        if let (Some(name), Some(value)) = (name, value) {
                            solution.results.insert(name, value);
                        }
                    }
                }
                Err(e) => return Err(format!("xml error: {}", e)),
                _ => {}
            }
        }
        Ok(solution)
    }
}
