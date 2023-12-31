use std::error::Error;
use std::fmt::{Display, Formatter};


#[derive(Debug)]
pub enum ImageConfigError {
    InvalidImageError,
    UnsupportedArchError,
    InvalidVersionError,
    FetchVersionError,
    CompilationError,
}

#[derive(Debug)]
pub enum DatabaseConfigError {
    Generic,
}

#[derive(Debug)]
pub enum DatabaseError {
    TcpError,
}

impl Display for ImageConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Failed to parse parameters in Image Configuration")
    }
}

impl Display for DatabaseConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Failed to parse parameters in Database Configuration")
    }
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Database Unexpected Error")
    }
}

impl Error for ImageConfigError {}
impl Error for DatabaseConfigError {}
impl Error for DatabaseError {}