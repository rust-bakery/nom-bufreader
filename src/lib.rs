#[cfg(feature = "async")]
pub mod r#async;
pub mod sync;

use std::io;

#[derive(Debug)]
pub enum Error<E> {
    Error(E),
    Failure(E),
    Io(io::Error),
}

impl<E> From<io::Error> for Error<E> {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}
