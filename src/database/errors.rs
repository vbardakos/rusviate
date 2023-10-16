use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum BuildErrors {
    UnknownSystemError,
    VersionFetchError,
    VersionValueError,
    InvalidImageError,
    ImageNotFoundError,
    DefaultPathError,
}

impl Display for BuildErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Failed to build Database")
    }
}

impl Error for BuildErrors {}
