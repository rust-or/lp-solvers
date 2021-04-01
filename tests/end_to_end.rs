use std::cmp::Ordering;
use std::collections::HashMap;

use lp_solvers::lp_format::{Constraint, LpObjective};
use lp_solvers::problem::{Problem, StrExpression, Variable};
use lp_solvers::solvers::Status::{Infeasible, Optimal};
use lp_solvers::solvers::{AllSolvers, CbcSolver, SolverTrait};

#[test]
fn solve_integer_problem_with_cbc() {
    let solver = CbcSolver::default();
    solve_integer_problem_with_solver(&solver);
    infeasible(&solver);
}

#[test]
fn solve_integer_problem_with_auto_solver() {
    let solver = AllSolvers::new();
    solve_integer_problem_with_solver(&solver);
    infeasible(&solver);
}

#[cfg(feature = "cplex")]
#[test]
fn solve_integer_problem_with_cplex() {
    use lp_solvers::solvers::cplex::Cplex;
    let command = std::env::var("CPLEX_BINARY").unwrap_or("cplex".to_string());
    let solver = Cplex::with_command(command);
    solve_integer_problem_with_solver(&solver);
    infeasible(&solver);
}

fn solve_integer_problem_with_solver<S: SolverTrait>(solver: &S) {
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
    let solution = solver.run(&pb).expect("Failed to run solver");
    assert_eq!(solution.status, Optimal);
    let expected_results: HashMap<String, f32> =
        vec![("x".to_string(), -1.), ("y".to_string(), 4.)]
            .into_iter()
            .collect();
    assert_eq!(solution.results, expected_results);
}

fn infeasible<S: SolverTrait>(solver: &S) {
    let pb = Problem {
        name: "impossible".to_string(),
        sense: LpObjective::Maximize,
        objective: StrExpression("x".to_string()),
        variables: vec![Variable {
            name: "x".to_string(),
            is_integer: false,
            lower_bound: 0.,
            upper_bound: 100.,
        }],
        constraints: vec![Constraint {
            lhs: StrExpression("x".to_string()),
            operator: Ordering::Less,
            rhs: -5.,
        }],
    };
    let solution = solver.run(&pb).expect("Failed to run solver");
    assert_eq!(solution.status, Infeasible);
}
