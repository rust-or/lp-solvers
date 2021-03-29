use std::fmt;
use std::fmt::Formatter;

use crate::lp_format::{AsVariable, Constraint, LpObjective, LpProblem, WriteToLpFileFormat};

/// A string that is a valid expression in the .lp format for the solver you are using
pub struct StrExpression(pub String);

/// A variable to optimize
pub struct Variable {
    /// The variable name should be unique in the problem and have a name accepted by the solver
    pub name: String,
    pub is_integer: bool,
    /// -INFINITY if there is no lower bound
    pub lower_bound: f64,
    /// INFINITY if there is no upper bound
    pub upper_bound: f64,
}

impl WriteToLpFileFormat for StrExpression {
    fn to_lp_file_format(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsVariable for Variable {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_integer(&self) -> bool {
        self.is_integer
    }

    fn lower_bound(&self) -> f64 {
        self.lower_bound
    }

    fn upper_bound(&self) -> f64 {
        self.upper_bound
    }
}

pub struct Problem<EXPR = StrExpression, VAR = Variable> {
    pub name: String,
    pub sense: LpObjective,
    pub objective: EXPR,
    pub variables: Vec<VAR>,
    pub constraints: Vec<Constraint<EXPR>>,
}

impl<'a, EXPR: 'a, VAR: 'a> LpProblem<'a> for Problem<EXPR, VAR>
where
    &'a VAR: AsVariable,
    &'a EXPR: WriteToLpFileFormat,
{
    type Variable = &'a VAR;
    type Expression = &'a EXPR;
    type ConstraintIterator = Box<dyn Iterator<Item = Constraint<&'a EXPR>> + 'a>;
    type VariableIterator = std::slice::Iter<'a, VAR>;

    fn name(&self) -> &str {
        &self.name
    }

    fn variables(&'a self) -> Self::VariableIterator {
        self.variables.iter()
    }

    fn objective(&'a self) -> Self::Expression {
        &self.objective
    }

    fn sense(&self) -> LpObjective {
        self.sense
    }

    fn constraints(&'a self) -> Self::ConstraintIterator {
        Box::new(
            self.constraints
                .iter()
                .map(|Constraint { lhs, operator, rhs }| Constraint {
                    lhs,
                    operator: *operator,
                    rhs: *rhs,
                }),
        )
    }
}
