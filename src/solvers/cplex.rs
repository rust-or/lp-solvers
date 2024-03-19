//! The IBM CPLEX optimizer.
//! You need to activate the "cplex" feature of this crate to use this solver.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::lp_format::LpProblem;
use crate::solvers::{Solution, SolverProgram, SolverWithSolutionParsing, Status, WithMipGap};
use crate::util::buf_contains;

/// IBM cplex optimizer
#[derive(Debug, Clone)]
pub struct Cplex {
    command: String,
    mipgap: Option<f32>,
}

impl Default for Cplex {
    fn default() -> Self {
        Self {
            command: "cplex".into(),
            mipgap: None,
        }
    }
}

impl Cplex {
    /// Create a cplex solver from the given binary
    pub fn with_command(command: String) -> Self {
        Self {
            command,
            mipgap: None,
        }
    }
}

impl WithMipGap<Cplex> for Cplex {
    fn mip_gap(&self) -> Option<f32> {
        self.mipgap
    }

    fn with_mip_gap(&self, mipgap: f32) -> Result<Cplex, String> {
        if mipgap.is_sign_positive() && mipgap.is_finite() {
            Ok(Cplex {
                mipgap: Some(mipgap),
                ..(*self).clone()
            })
        } else {
            Err("Invalid MIP gap: must be positive and finite".to_string())
        }
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
        let mut args = vec!["-c".into(), format_osstr!("READ \"" lp_file "\"")];

        if let Some(mipgap) = self.mip_gap() {
            args.push(format_osstr!("set mip tolerances mipgap " mipgap.to_string()));
        }

        args.push("optimize".into());
        args.push(format_osstr!("WRITE \"" solution_file "\""));

        args
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

fn extract_variable_name_and_value_from_event(
    variable_event: BytesStart,
) -> Result<(String, f32), String> {
    let mut name = None;
    let mut value = None;
    for attribute in variable_event.attributes() {
        let attribute = attribute.map_err(|e| format!("attribute error: {}", e))?;
        match attribute.key.as_ref() {
            b"name" => name = Some(String::from_utf8_lossy(attribute.value.as_ref()).to_string()),
            b"value" => {
                value = Some(
                    String::from_utf8_lossy(attribute.value.as_ref())
                        .parse()
                        .map_err(|e| format!("invalid variable value for {:?}: {}", name, e))?,
                );
            }
            _ => {}
        }
    }

    name.and_then(|name| value.map(|value| (name, value)))
        .ok_or_else(|| "name and value not found for variable".to_string())
}

fn read_specific_solution(f: &File, variables_len: Option<usize>) -> Result<Solution, String> {
    let results = variables_len
        .map(HashMap::with_capacity)
        .unwrap_or_default();

    let mut solution = Solution {
        status: Status::Optimal,
        results,
    };

    let f = BufReader::new(f);
    let mut reader = Reader::from_reader(f);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => {
                return Err(format!(
                    "Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                ))
            }
            // exits the loop when reaching end of file
            Ok(Event::Eof) => {
                break;
            }
            // we reached the "variables" section, where the variables to parse are
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"variables" => loop {
                match reader.read_event_into(&mut buf) {
                    // we matched either the start of a "variable" tag, or a "variable" tag without body
                    Ok(Event::Empty(e)) | Ok(Event::Start(e))
                        if e.local_name().as_ref() == b"variable" =>
                    {
                        // let's try to parse the variable name and value
                        let (name, value) = extract_variable_name_and_value_from_event(e)?;
                        solution.results.insert(name, value);
                    }
                    // we reached the end of the "variables" section, at this point all the variables should have been parsed.
                    // we can safely return
                    Ok(Event::End(e)) if e.local_name().as_ref() == b"variables" => {
                        return Ok(solution);
                    }
                    Err(e) => {
                        return Err(format!(
                            "Error at position {}: {:?}",
                            reader.buffer_position(),
                            e
                        ))
                    }
                    // an end-of-file here would be an error, since the 'variables' section would not be terminated
                    Ok(Event::Eof) => {
                        return Err(format!(
                            "Error at position {}: Unterminated variables section",
                            reader.buffer_position(),
                        ))
                    }
                    _ => {}
                }
            },
            // There are several other `Event`s we do not consider here
            _ => {}
        }
    }

    Ok(solution)
}

impl SolverWithSolutionParsing for Cplex {
    fn read_specific_solution<'a, P: LpProblem<'a>>(
        &self,
        f: &File,
        problem: Option<&'a P>,
    ) -> Result<Solution, String> {
        let len = problem.map(|p| p.variables().size_hint().0);
        read_specific_solution(f, len)
    }
}

#[cfg(test)]
mod tests {
    use super::read_specific_solution;
    use crate::solvers::{Cplex, SolverProgram, WithMipGap};
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::io::{Seek, Write};
    use std::path::Path;

    const SAMPLE_SOL_FILE: &str = r##"<?xml version = "1.0" standalone="yes"?>
<?xml-stylesheet href="https://www.ilog.com/products/cplex/xmlv1.0/solution.xsl" type="text/xsl"?>
<CPLEXSolution version="1.2">
 <header
   problemName="../../../examples/data/mexample.mps"
   solutionName="incumbent"
   solutionIndex="-1"
   objectiveValue="-122.5"
   solutionTypeValue="3"
   solutionTypeString="primal"
   solutionStatusValue="101"
   solutionStatusString="integer optimal solution"
   solutionMethodString="mip"
   primalFeasible="1"
   dualFeasible="1"
   MIPNodes="0"
   MIPIterations="3"/>
 <quality
   epInt="1e-05"
   epRHS="1e-06"
   maxIntInfeas="0"
   maxPrimalInfeas="0"
   maxX="40"
   maxSlack="2"/>
 <linearConstraints>
  <constraint name="c1" index="0" slack="0"/>
  <constraint name="c2" index="1" slack="2"/>
  <constraint name="c3" index="2" slack="0"/>
 </linearConstraints>
 <variables>
  <variable name="x1" index="0" value="40"/>
  <variable name="x2" index="1" value="10.5"/>
  <variable name="x3" index="2" value="19.5"/>
  <variable name="x4" index="3" value="3"/>
 </variables>
</CPLEXSolution>"##;

    #[test]
    fn sol_file_parsing() {
        let mut tmpfile = tempfile::tempfile().expect("unable to create tempfile");
        tmpfile
            .write_all(SAMPLE_SOL_FILE.as_bytes())
            .expect("unable to write sol file to tempfile");
        tmpfile.rewind().expect("unable to rewind sol file");

        let solution = read_specific_solution(&tmpfile, None).expect("failed to read sol file");

        assert_eq!(
            solution.results,
            HashMap::from([
                ("x1".to_owned(), 40.0),
                ("x2".to_owned(), 10.5),
                ("x3".to_owned(), 19.5),
                ("x4".to_owned(), 3.0)
            ])
        );
    }

    #[test]
    fn cli_args_default() {
        let solver = Cplex::default();
        let args = solver.arguments(Path::new("test.lp"), Path::new("test.sol"));

        let expected: Vec<OsString> = vec![
            "-c".into(),
            "READ \"test.lp\"".into(),
            "optimize".into(),
            "WRITE \"test.sol\"".into(),
        ];

        assert_eq!(args, expected);
    }

    #[test]
    fn cli_args_mipgap() {
        let solver = Cplex::default()
            .with_mip_gap(0.5)
            .expect("mipgap should be valid");

        let args = solver.arguments(Path::new("test.lp"), Path::new("test.sol"));

        let expected: Vec<OsString> = vec![
            "-c".into(),
            "READ \"test.lp\"".into(),
            "set mip tolerances mipgap 0.5".into(),
            "optimize".into(),
            "WRITE \"test.sol\"".into(),
        ];

        assert_eq!(args, expected);
    }

    #[test]
    fn cli_args_mipgap_negative() {
        let solver = Cplex::default().with_mip_gap(-0.05);
        assert!(solver.is_err());
    }

    #[test]
    fn cli_args_mipgap_infinite() {
        let solver = Cplex::default().with_mip_gap(f32::INFINITY);
        assert!(solver.is_err());
    }
}
