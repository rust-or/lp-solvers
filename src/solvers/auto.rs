//! Auto solvers automatically find which of their child solvers is installed on
//! the user's computer and uses it. The [AllSolvers] solvers tries all the supported solvers.

use crate::lp_format::{LpObjective, LpProblem};
use crate::problem::{Problem, StrExpression, Variable};
#[cfg(feature = "cplex")]
use crate::solvers::cplex::Cplex;
use crate::solvers::{CbcSolver, GlpkSolver, GurobiSolver, Solution};

use super::SolverTrait;

/// A solver that tries multiple solvers
#[derive(Debug, Clone)]
pub struct AutoSolver<SOLVER, NEXT>(SOLVER, NEXT);

/// The tail of a list of solvers. This one has no children and never finds any solver.
#[derive(Debug, Clone, Default)]
pub struct NoSolver;

#[cfg(not(feature = "cplex"))]
type Cplex = NoSolver;

/// An [AutoSolver] that tries, in order: Gurobi, Cplex, Cbc and Glpk
pub type AllSolvers = AutoSolver<
    GurobiSolver,
    AutoSolver<Cplex, AutoSolver<CbcSolver, AutoSolver<GlpkSolver, NoSolver>>>,
>;

impl SolverTrait for NoSolver {
    fn run<'a, P: LpProblem<'a>>(&self, _problem: &'a P) -> Result<Solution, String> {
        Err("No solver available".to_string())
    }
}

/// The default AutoSolver contains all supported solvers
impl<A: Default, B: Default> Default for AutoSolver<A, B> {
    fn default() -> Self {
        AutoSolver(A::default(), B::default())
    }
}

impl<SOLVER: Default, NEXT: Default> AutoSolver<SOLVER, NEXT> {
    /// Instantiate an AutoSolver with all supported solvers
    pub fn new() -> Self {
        Self::default()
    }

    /// Instantiate an AutoSolver with the given solvers
    pub fn with_solver<NewSolver>(self, solver: NewSolver) -> AutoSolver<NewSolver, Self> {
        AutoSolver(solver, self)
    }
}

impl<S: SolverTrait, T: SolverTrait> SolverTrait for AutoSolver<S, T> {
    fn run<'a, P: LpProblem<'a>>(&self, problem: &'a P) -> Result<Solution, String> {
        // Try solving a dummy problem (to avoid writing a large problem to disk if not necessary)
        let works = self
            .0
            .run(&Problem {
                name: "dummy".to_string(),
                sense: LpObjective::Minimize,
                objective: StrExpression("x".to_string()),
                variables: vec![Variable {
                    name: "x".to_string(),
                    is_integer: false,
                    lower_bound: 0.0,
                    upper_bound: 1.0,
                }],
                constraints: vec![],
            })
            .is_ok();
        if works {
            self.0.run(problem)
        } else {
            self.1.run(problem)
        }
    }
}
