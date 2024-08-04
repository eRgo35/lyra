use std::fmt;

#[derive(Debug)]
pub enum ParseError {
    InvalidUrl,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid URL")
    }
}

impl std::error::Error for ParseError {}
