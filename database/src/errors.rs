use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum DbBuilderErrors {
    InvalidSystem,
    InvalidDirectory,
}

impl Display for DbBuilderErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Unable to build Embedded Database")
    }
}

impl Error for DbBuilderErrors {};