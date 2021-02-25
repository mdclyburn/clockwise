use std::cmp::{Ord, Ordering, PartialOrd, Reverse};
use std::collections::BinaryHeap;
use std::convert::From;
use std::error;
use std::fmt;
use std::fmt::Display;
use std::iter::IntoIterator;
use std::time::{Duration, Instant};

use crate::io;
use crate::io::{IOPin, Mapping};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::IO(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IO(ref e) => write!(f, "I/O error: {}", e),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Signal {
    High(u8),
    Low(u8),
}

impl Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Signal::High(pin) => write!(f, "DIGITAL HIGH\tP{:02}", pin),
            Signal::Low(pin) => write!(f, "DIGITAL LOW\tP{:02}", pin),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Operation {
    pub time: u64,
    pub input: Signal,
}

impl Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\tinput: {}", self.time, self.input)
    }
}

impl Ord for Operation {
    fn cmp(&self, b: &Self) -> Ordering {
        self.time.cmp(&b.time)
    }
}

impl PartialOrd for Operation {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        self.time.partial_cmp(&b.time)
    }
}

#[derive(Copy, Clone)]
pub struct Response {
    pub time: u64,
    pub output: Signal,
}

impl Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\toutput: {}", self.time, self.output)
    }
}

#[derive(Clone, Debug)]
pub enum Criterion {
    Response(u8),
}

#[derive(Clone, Debug)]
pub struct Execution {
    duration: Duration,
}

impl Execution {
    fn new(duration: Duration) -> Execution {
        Execution {
            duration
        }
    }

    pub fn get_duration(&self) -> &Duration {
        &self.duration
    }
}

#[derive(Clone)]
pub struct Test {
    id: String,
    actions: BinaryHeap<Reverse<Operation>>,
    criteria: Vec<Criterion>,
}

impl Test {
    pub fn new<'a, T, U>(id: &str, ops: T, criteria: U) -> Test where
        T: IntoIterator<Item = &'a Operation>,
        U: IntoIterator<Item = &'a Criterion> {
        Test {
            id: id.to_string(),
            actions: ops.into_iter().map(|x| Reverse(*x)).collect(),
            criteria: criteria.into_iter().cloned().collect(),
        }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_criteria(&self) -> &Vec<Criterion> {
        &self.criteria
    }

    pub fn execute(&self, t0: Instant, mapping: &Mapping) -> Result<Execution> {
        let timeline = self.actions.iter()
            .map(|Reverse(op)| (t0 + Duration::from_millis(op.time), op.input));
        for (t, input) in timeline {
            while Instant::now() < t {  } // spin wait
            match input {
                Signal::High(pin_no) =>
                    (*mapping.get_pin(pin_no)?)
                    .expect_output()?
                    .set_high(),
                Signal::Low(pin_no) =>
                    (*mapping.get_pin(pin_no)?)
                    .expect_output()?
                    .set_low(),
            };
            println!("{:?}", input);
        }

        Ok(Execution::new(Instant::now() - t0))
    }
}

impl Display for Test {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Test: {}\n", self.id)?;
        write!(f, "Operations =====\n")?;
        for Reverse(ref action) in &self.actions {
            write!(f, "{}\n", action)?;
        }

        Ok(())
    }
}
