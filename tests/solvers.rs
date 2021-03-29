extern crate lp_solvers;

use std::path::PathBuf;

use lp_solvers::problem::Problem;
use lp_solvers::solvers::{CbcSolver, GlpkSolver, Solution, SolverWithSolutionParsing, Status};

fn sol_file(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("solution_files")
        .join(file)
}

#[test]
fn cbc_optimal() {
    let solver = CbcSolver::new();
    let Solution {
        status,
        results: mut variables,
    } = solver
        .read_solution_from_path::<Problem>(&sol_file("cbc_optimal.sol"), None)
        .unwrap();
    assert_eq!(status, Status::Optimal);
    assert_eq!(variables.remove("a"), Some(5f32));
    assert_eq!(variables.remove("b"), Some(6f32));
    assert_eq!(variables.remove("c"), Some(0f32));
}

#[test]
fn cbc_infeasible() {
    let solver = CbcSolver::new();
    let Solution { status, .. } = solver
        .read_solution_from_path::<Problem>(&sol_file("cbc_infeasible.sol"), None)
        .unwrap();
    assert_eq!(status, Status::Infeasible);
}

#[test]
// created from:
// minimize
//   obj: a + b
// subject to
//   c1: a + b <= 1
//   c2: a + b >= 2
// binaries
//   a b
// end
fn cbc_infeasible_alternative_format() {
    let Solution {
        status,
        results: mut variables,
        ..
    } = CbcSolver::new()
        .read_solution_from_path::<Problem>(
            &sol_file("cbc_infeasible_alternative_format.sol"),
            None,
        )
        .unwrap();
    assert_eq!(status, Status::Infeasible);
    assert_eq!(variables.remove("a"), Some(2f32));
    assert_eq!(variables.remove("b"), Some(0f32));
}

#[test]
fn cbc_unbounded() {
    let solver = CbcSolver::new();
    let Solution { status, .. } = solver
        .read_solution_from_path::<Problem>(&sol_file("cbc_unbounded.sol"), None)
        .unwrap();
    assert_eq!(status, Status::Unbounded);
}

#[test]
fn glpk_optimal() {
    let solver = GlpkSolver::new();
    let Solution {
        status,
        results: mut variables,
        ..
    } = solver
        .read_solution_from_path::<Problem>(&sol_file("glpk_optimal.sol"), None)
        .unwrap();
    assert_eq!(status, Status::Optimal);
    assert_eq!(variables.remove("a"), Some(0f32));
    assert_eq!(variables.remove("b"), Some(5f32));
    assert_eq!(variables.remove("c"), Some(0f32));
}

#[test]
fn glpk_infeasible() {
    let solver = GlpkSolver::new();
    let Solution { status, .. } = solver
        .read_solution_from_path::<Problem>(&sol_file("glpk_infeasible.sol"), None)
        .unwrap();
    assert_eq!(status, Status::Infeasible);
}

#[test]
fn glpk_unbounded() {
    let solver = GlpkSolver::new();
    let Solution { status, .. } = solver
        .read_solution_from_path::<Problem>(&sol_file("glpk_unbounded.sol"), None)
        .unwrap();
    assert_eq!(status, Status::Unbounded);
}

#[test]
fn glpk_empty_col_bounds() {
    let solver = GlpkSolver::new();
    let Solution {
        status,
        results: solution,
        ..
    } = solver
        .read_solution_from_path::<Problem>(&sol_file("glpk_empty_col_bounds.sol"), None)
        .unwrap();
    assert_eq!(status, Status::Optimal);
    assert_eq!(1.0, *solution.get("a").unwrap());
    assert_eq!(0.0, *solution.get("b").unwrap());
}
