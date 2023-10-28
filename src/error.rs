use crate::rope;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Unhandled,
    Io(std::io::Error),
    Rope(rope::Error),
}

impl From<rope::Error> for Error {
    fn from(err: rope::Error) -> Error {
        Error::Rope(err)
    }
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

impl From<tokio::task::JoinError> for Error {
    fn from(_err: tokio::task::JoinError) -> Self {
        Error::Unhandled
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(_err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Error::Unhandled
    }
}
