//! Models for reading spec manifests.

use serde::{Deserialize, Serialize};
use std::ffi::OsStr;

/// Config file format for declaring the set of ruby/spec suites to run.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Specs for core language features.
    ///
    /// For example, `Regexp` literal support.
    pub language: Option<Vec<Suite>>,
    /// Specs for the core objects and their API compatibility.
    ///
    /// For example, the behavior of `Comparable#==` when called with `self` as
    /// the argument.
    pub core: Option<Vec<Suite>>,
    /// Specs for the packages in the standard library and their API
    /// compatibility.
    ///
    /// For example, the behavior of `StringScanner` when dealing with invalid
    /// UTF-8 `String`s.
    pub library: Option<Vec<Suite>>,
}

impl Config {
    /// Lookup a suite.
    pub fn suites_for_family(&self, family: &OsStr) -> Option<&[Suite]> {
        match family {
            family if family == OsStr::new("lanugage") => self.language.as_deref(),
            family if family == OsStr::new("core") => self.core.as_deref(),
            family if family == OsStr::new("library") => self.library.as_deref(),
            _ => None,
        }
    }
}

/// The specs to run for a suite or API group.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Suite {
    /// Suite name.
    ///
    /// For example, `array`.
    pub suite: String,
    /// List of specs. Specs correspond to individual mspec files in `ruby/spec`.
    ///
    /// For example, `any`, `append`, and `assoc` for `array`.
    pub specs: Option<Vec<String>>,
    /// List of specs to always skip because they are known to fail.
    ///
    /// Specs in this list will override an explicit include in the `specs`
    /// field.
    pub skip: Option<Vec<String>>,
}