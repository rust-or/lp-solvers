use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;
use std::fs::File;
use std::io::prelude::*;
use std::io::Result;

use tempfile::NamedTempFile;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum LpObjective {
    Minimize,
    Maximize,
}

pub trait WriteToLpFileFormat {
    fn to_lp_file_format(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result;
}

pub trait Variable {
    fn name(&self) -> &str;
    fn is_integer(&self) -> bool;
    fn lower_bound(&self) -> f64;
    fn upper_bound(&self) -> f64;
}

pub trait Constraint<E: WriteToLpFileFormat> {
    fn lhs(&self) -> E;
    fn rhs(&self) -> E;
    fn operator(&self) -> std::cmp::Ordering;
    fn to_lp_file_format(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.lhs().to_lp_file_format(f)?;
        write!(f, " {} ", match self.operator() {
            Ordering::Less => "<=",
            Ordering::Equal => "=",
            Ordering::Greater => ">=",
        })?;
        self.rhs().to_lp_file_format(f)?;
        Ok(())
    }
}

pub trait LpProblem: Sized {
    type Variable: Variable;
    type Expression: WriteToLpFileFormat;
    type Constraint: Constraint<Self::Expression>;
    type ConstraintIterator: Iterator<Item=Self::Constraint>;
    type VariableIterator: Iterator<Item=Self::Variable>;

    fn name(&self) -> &str;
    fn variables(&self) -> Self::VariableIterator;
    fn objective(&self) -> Self::Expression;
    fn sense(&self) -> LpObjective;
    fn constraints(&self) -> Self::ConstraintIterator;
    fn to_lp_file_format(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\\ {}\n\n", self.name());
        objective_lp_file_block(self, f);
        write_constraints_lp_file_block(self, f)?;
        write_bounds_lp_file_block(self, f)?;
        write!(f, "\nEnd\n")?;
        Ok(())
    }
    fn display_lp(&self) -> DisplayedLp<'_, Self> where Self: Sized {
        DisplayedLp(&self)
    }
    fn to_tmp_file(&self) -> Result<NamedTempFile> where Self: Sized {
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

impl<'a, P: LpProblem> std::fmt::Display for DisplayedLp<'a, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.to_lp_file_format(f)
    }
}

fn objective_lp_file_block(prob: &impl LpProblem, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    // Write objectives
    let obj_type = match prob.sense() {
        LpObjective::Maximize => "Maximize\n  ",
        LpObjective::Minimize => "Minimize\n  "
    };
    write!(f, "{}obj: ", obj_type)?;
    prob.objective().to_lp_file_format(f)?;
    Ok(())
}

fn write_constraints_lp_file_block(prob: &impl LpProblem, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    let mut wrote_header = false;
    for (idx, constraint) in prob.constraints().enumerate() {
        if !wrote_header {
            write!(f, "\n\nSubject To\n")?;
        }
        write!(f, "  c{}: ", idx);
        constraint.to_lp_file_format(f)?;
        write!(f, "\n")?;
    }
    Ok(())
}

fn write_bounds_lp_file_block<P: LpProblem>(prob: &P, f: &mut Formatter) -> fmt::Result {
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
        write!(f, "\n")?;
        if variable.is_integer() {
            integers.push(name);
        }
    }
    if !integers.is_empty() {
        write!(f, "Generals\n")?;
        for name in integers.iter() {
            write!(f, "  {}\n", name)?;
        }
    }
    Ok(())
}