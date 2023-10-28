#[derive(Debug)]
pub enum Error {
    Unhandled,
    Io(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_err: std::sync::PoisonError<T>) -> Error {
        Error::Unhandled
    }
}

pub type Result<T> = std::result::Result<T, Error>;
