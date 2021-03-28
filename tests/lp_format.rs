use std::cmp::Ordering;

use lp_solvers::format::lp_format::{Constraint, LpObjective, LpProblem};
use lp_solvers::problem::{Problem, StrExpression, Variable};

#[test]
fn simple_problem() {
    let pb = Problem {
        name: "my_problem".to_string(),
        sense: LpObjective::Minimize,
        objective: StrExpression("2 x + y".to_string()),
        variables: vec![
            Variable {
                name: "x".to_string(),
                is_integer: false,
                lower_bound: f64::NEG_INFINITY,
                upper_bound: f64::INFINITY,
            },
            Variable {
                name: "y".to_string(),
                is_integer: false,
                lower_bound: 0.0,
                upper_bound: f64::INFINITY,
            },
            Variable {
                name: "z".to_string(),
                is_integer: false,
                lower_bound: 1.,
                upper_bound: 10.,
            },
        ],
        constraints: vec![Constraint {
            lhs: StrExpression("x + y + z".to_string()),
            operator: Ordering::Greater,
            rhs: 5.0,
        }],
    };
    let expected_str = "\\ my_problem

Minimize
  obj: 2 x + y

Subject To
  c0: x + y + z >= 5

Bounds
  x free
  0 <= y
  1 <= z <= 10

End
";
    assert_eq!(pb.display_lp().to_string(), expected_str);
}

#[test]
fn with_integers() {
    let pb = Problem {
        name: "int_problem".to_string(),
        sense: LpObjective::Maximize,
        objective: StrExpression("x - y".to_string()),
        variables: vec![
            Variable {
                name: "x".to_string(),
                is_integer: true,
                lower_bound: -10.,
                upper_bound: 10.,
            },
            Variable {
                name: "y".to_string(),
                is_integer: true,
                lower_bound: f64::NEG_INFINITY,
                upper_bound: 16.5,
            },
        ],
        constraints: vec![Constraint {
            lhs: StrExpression("x - y".to_string()),
            operator: Ordering::Less,
            rhs: -5.0,
        }],
    };
    let expected_str = "\\ int_problem

Maximize
  obj: x - y

Subject To
  c0: x - y <= -5

Bounds
  -10 <= x <= 10
  y <= 16.5

Generals
  x
  y

End
";
    assert_eq!(pb.display_lp().to_string(), expected_str);
}
