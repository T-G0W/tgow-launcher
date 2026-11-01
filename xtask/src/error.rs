use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum Error {
    Io(io::Error),
    Read(PathBuf, io::Error),
    Command(String, String),
    Usage(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Read(path, error) => write!(f, "cannot read {}: {error}", path.display()),
            Self::Command(program, output) => write!(f, "{program} failed: {output}"),
            Self::Usage(text) => write!(f, "{text}"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
