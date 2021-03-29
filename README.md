# lp-solvers

Library implementing interaction with various linear programming solvers.

It uses the [.lp file format] to interact with external solver binaries.

## Supported solvers

 - [gurobi](https://www.gurobi.com/)
 - [cplex](https://www.ibm.com/analytics/cplex-optimizer)
 - [cbc](https://www.coin-or.org/Cbc/)
 - [glpk](https://www.gnu.org/software/glpk/)

You need to have the solver you want to use installed on your machine already for this library to work.

## Example

```rust

use lp_solvers::lp_format::{Constraint, LpObjective};
use lp_solvers::problem::{Problem, StrExpression, Variable};
use lp_solvers::solvers::{CbcSolver, SolverTrait};
use lp_solvers::solvers::Status::Optimal;

fn solve_integer_problem_with_solver<S: SolverTrait>(solver: S) {
    let pb = Problem { // Alternatively, you can implement the LpProblem trait on your own structure
        name: "int_problem".to_string(),
        sense: LpObjective::Maximize,
        objective: StrExpression("x - y".to_string()), // You can use other expression representations
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
    // solution.results is now {"x":-1, "y":4}
}

fn main() {
    solve_integer_problem_with_solver(CbcSolver::default())
}
```