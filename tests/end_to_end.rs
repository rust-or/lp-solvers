use std::cmp::Ordering;
use std::collections::HashMap;

use lp_solvers::lp_format::{Constraint, LpObjective};
use lp_solvers::problem::{Problem, StrExpression, Variable};
use lp_solvers::solvers::Status::Optimal;
use lp_solvers::solvers::{CbcSolver, SolverTrait};

#[test]
fn solve_integer_problem_with_cbc() {
    let pb = Problem {
        name: "int_problem".to_string(),
        sense: LpObjective::Maximize,
        objective: StrExpression("x - y".to_string()),
        variables: vec![
            Variable {
                name: "x".to_string(),
                is_integer: true,
                lower_bound: -10.,
                upper_bound: -1.,
            },
            Variable {
                name: "y".to_string(),
                is_integer: true,
                lower_bound: 4.,
                upper_bound: 7.,
            },
        ],
        constraints: vec![Constraint {
            lhs: StrExpression("x - y".to_string()),
            operator: Ordering::Less,
            rhs: -4.5,
        }],
    };
    let solution = CbcSolver::new().run(&pb).expect("Failed to run cbc");
    assert_eq!(solution.status, Optimal);
    let expected_results: HashMap<String, f32> =
        vec![("x".to_string(), -1.), ("y".to_string(), 4.)]
            .into_iter()
            .collect();
    assert_eq!(solution.results, expected_results);
}
