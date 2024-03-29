//! Flexible testbed and test providers.

use std::fmt::Debug;

use crate::test::Test;
use crate::testbed::Testbed;

/// Adapter producing a testbed from some input source.
pub trait TestbedProvider: Debug {
    /// Create a configured testbed ready to run tests.
    fn create(&self) -> Result<Testbed, String>;
}

/// Data adapter producing tests read from an input source.
pub trait TestProvider: Debug {
    /// Create a Test-producing iterator.
    fn tests<'a>(&'a self) -> Box<dyn Iterator<Item = Test> + 'a>;
}
