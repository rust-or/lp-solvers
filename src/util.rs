//! Utilities to help with building problems
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Useful to generate a list of unique valid variable names
#[derive(Debug, Default)]
pub struct UniqueNameGenerator {
    names: HashMap<u64, usize>,
}

impl UniqueNameGenerator {
    /// Create a new variable. Returns a valid variable name, never returned before by this generator.
    ///
    /// ```
    /// use lp_solvers::util::UniqueNameGenerator;
    ///
    /// let mut gen = UniqueNameGenerator::default();
    /// assert_eq!(gen.add_variable("x"), "x");
    /// assert_eq!(gen.add_variable("y"), "y");
    /// assert_eq!(gen.add_variable("z"), "z");
    /// assert_eq!(gen.add_variable("!#?/"), "v"); // "!#?/" is not a valid variable name
    /// assert_eq!(gen.add_variable("x"), "x2"); // A variable with name x is already present
    /// ```
    pub fn add_variable<'a>(&mut self, name: &'a str) -> Cow<'a, str> {
        let mut stem = stem(name);
        let hash = calculate_hash(&stem);
        let n = self.names.entry(hash).or_insert(0);
        *n += 1;
        if *n >= 2 {
            stem = Cow::Owned(stem.into_owned() + &n.to_string());
        }
        stem
    }
}

fn stem(name: &str) -> Cow<str> {
    if name.contains(|c: char| !c.is_ascii_alphabetic()) || name.is_empty() {
        let mut owned = name.replace(|c: char| !c.is_ascii_alphabetic(), "");
        if owned.is_empty() {
            owned.push('v');
        }
        Cow::Owned(owned)
    } else {
        Cow::Borrowed(name)
    }
}

fn calculate_hash(t: &str) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
