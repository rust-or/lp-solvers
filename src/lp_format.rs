//! Traits to be implemented by structures that can be dumped in the .lp format
//!
use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;
use std::io::prelude::*;
use std::io::Result;

use tempfile::NamedTempFile;

/// Optimization sense
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum LpObjective {
    /// min
    Minimize,
    /// max
    Maximize,
}

/// It's the user's responsibility to ensure
/// that the variable names used by types implementing this trait
/// follow the solver's requirements.
pub trait WriteToLpFileFormat {
    /// Write the object to the given formatter in the .lp format
    fn to_lp_file_format(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

impl<'a, T: WriteToLpFileFormat> WriteToLpFileFormat for &'a T {
    fn to_lp_file_format(&self, f: &mut Formatter) -> fmt::Result {
        (*self).to_lp_file_format(f)
    }
}

/// A type that represents a variable. See [crate::problem::Variable].
pub trait AsVariable {
    /// Variable name. Needs to be unique. See [crate::util::UniqueNameGenerator]
    fn name(&self) -> &str;
    /// Whether the variable is forced to take only integer values
    fn is_integer(&self) -> bool;
    /// Minimum allowed value for the variable
    fn lower_bound(&self) -> f64;
    /// Maximum allowed value for the variable
    fn upper_bound(&self) -> f64;
}

impl<'a, T: AsVariable> AsVariable for &'a T {
    fn name(&self) -> &str {
        (*self).name()
    }

    fn is_integer(&self) -> bool {
        (*self).is_integer()
    }

    fn lower_bound(&self) -> f64 {
        (*self).lower_bound()
    }

    fn upper_bound(&self) -> f64 {
        (*self).upper_bound()
    }
}

/// A constraint expressing a relation between two expressions
pub struct Constraint<E> {
    /// left hand side of the constraint
    pub lhs: E,
    /// '<=' '=' or '>='
    pub operator: Ordering,
    /// Right-hand side of the constraint
    pub rhs: f64,
}

impl<E: WriteToLpFileFormat> WriteToLpFileFormat for Constraint<E> {
    fn to_lp_file_format(&self, f: &mut Formatter) -> fmt::Result {
        self.lhs.to_lp_file_format(f)?;
        write!(
            f,
            " {} {}",
            match self.operator {
                Ordering::Equal => "=",
                Ordering::Less => "<=",
                Ordering::Greater => ">=",
            },
            self.rhs
        )
    }
}

/// Implemented by type that can be formatted as an lp problem
pub trait LpProblem<'a>: Sized {
    /// variable type
    type Variable: AsVariable;
    /// expression type
    type Expression: WriteToLpFileFormat;
    /// Iterator over constraints
    type ConstraintIterator: Iterator<Item = Constraint<Self::Expression>>;
    /// Iterator over variables
    type VariableIterator: Iterator<Item = Self::Variable>;

    /// problem name. "lp_solvers_problem" by default
    fn name(&self) -> &str { "lp_solvers_problem" }
    /// Variables of the problem
    fn variables(&'a self) -> Self::VariableIterator;
    /// Target objective function
    fn objective(&'a self) -> Self::Expression;
    /// Whether to maximize or minimize the objective
    fn sense(&'a self) -> LpObjective;
    /// List of constraints to apply
    fn constraints(&'a self) -> Self::ConstraintIterator;
    /// Write the problem in the lp file format to the given formatter
    fn to_lp_file_format(&'a self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\\ {}\n\n", self.name())?;
        objective_lp_file_block(self, f)?;
        write_constraints_lp_file_block(self, f)?;
        write_bounds_lp_file_block(self, f)?;
        write!(f, "\nEnd\n")?;
        Ok(())
    }
    /// Return an object whose [fmt::Display] implementation is the problem in the .lp format
    fn display_lp(&'a self) -> DisplayedLp<'_, Self>
    where
        Self: Sized,
    {
        DisplayedLp(&self)
    }

    /// Write the problem to a temporary file
    fn to_tmp_file(&'a self) -> Result<NamedTempFile>
    where
        Self: Sized,
    {
        let mut f = tempfile::Builder::new()
            .prefix(self.name())
            .suffix(".lp")
            .tempfile()?;
        write!(f, "{}", self.display_lp())?;
        f.flush()?;
        Ok(f)
    }
}

/// A problem whose `Display` implementation outputs valid .lp syntax
pub struct DisplayedLp<'a, P>(&'a P);

impl<'a, P: LpProblem<'a>> std::fmt::Display for DisplayedLp<'a, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.to_lp_file_format(f)
    }
}

fn objective_lp_file_block<'a>(
    prob: &'a impl LpProblem<'a>,
    f: &mut std::fmt::Formatter,
) -> std::fmt::Result {
    // Write objectives
    let obj_type = match prob.sense() {
        LpObjective::Maximize => "Maximize\n  ",
        LpObjective::Minimize => "Minimize\n  ",
    };
    write!(f, "{}obj: ", obj_type)?;
    prob.objective().to_lp_file_format(f)?;
    Ok(())
}

fn write_constraints_lp_file_block<'a>(
    prob: &'a impl LpProblem<'a>,
    f: &mut std::fmt::Formatter,
) -> std::fmt::Result {
    let mut wrote_header = false;
    for (idx, constraint) in prob.constraints().enumerate() {
        if !wrote_header {
            write!(f, "\n\nSubject To\n")?;
            wrote_header = true;
        }
        write!(f, "  c{}: ", idx)?;
        constraint.to_lp_file_format(f)?;
        writeln!(f)?;
    }
    Ok(())
}

fn write_bounds_lp_file_block<'a>(prob: &'a impl LpProblem<'a>, f: &mut Formatter) -> fmt::Result {
    let mut integers = vec![];
    write!(f, "\nBounds\n")?;
    for variable in prob.variables() {
        let low: f64 = variable.lower_bound();
        let up: f64 = variable.upper_bound();
        write!(f, "  ")?;
        if low > f64::NEG_INFINITY {
            write!(f, "{} <= ", low)?;
        }
        let name = variable.name().to_string();
        write!(f, "{}", name)?;
        if up < f64::INFINITY {
            write!(f, " <= {}", up)?;
        }
        if low.is_infinite() && up.is_infinite() {
            write!(f, " free")?;
        }
        writeln!(f)?;
        if variable.is_integer() {
            integers.push(name);
        }
    }
    if !integers.is_empty() {
        writeln!(f, "\nGenerals")?;
        for name in integers.iter() {
            writeln!(f, "  {}", name)?;
        }
    }
    Ok(())
}
