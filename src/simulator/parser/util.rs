use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

/// Represents any kind of error the parser may find
#[derive(Debug)]
pub enum Error {
    /// Not the parser's fault, some std::io went wrong
    IO(io::Error),
    LabelNotFound(String),
    Nom(String, nom::error::ErrorKind), // I'm feeling lazy
    RegisterNotFound(String),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(e)
    }
}

impl<'a> From<nom::Err<(&'a str, nom::error::ErrorKind)>> for Error {
    fn from(err: nom::Err<(&'a str, nom::error::ErrorKind)>) -> Self {
        use nom::Err as e;
        match err {
            e::Incomplete(_) => unreachable!("nom::Err::Incomplete should only exist in streaming parsers"),
            e::Error((i, e)) => Error::Nom(i.into(), e),
            e::Failure((i, e)) => Error::Nom(i.into(), e),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self) // muahahahaha
    }
}

impl std::error::Error for Error {}

/// Returns an iterator over the lines of a file
pub fn file_lines(filepath: String) -> Result<impl Iterator<Item = String>, Error> {
    let reader = File::open(Path::new(&filepath)).map(BufReader::new)?;
    Ok(reader.lines().map(|x| x.unwrap()))
}