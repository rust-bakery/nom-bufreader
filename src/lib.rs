use nom::{Err, Offset, Parser};
use std::io::{self, BufRead, Read};

#[cfg(feature = "async")]
use async_trait::async_trait;
#[cfg(feature = "async")]
use futures::{
    io::{AsyncBufReadExt, BufReader},
    AsyncRead,
};

pub mod bufreader;

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

pub trait Parse<O, E, P> {
    fn parse(&mut self, p: P) -> Result<O, Error<E>>
    where
        for<'a> P: Parser<&'a [u8], O, E>;
}

impl<R: Read, O, E, P> Parse<O, E, P> for std::io::BufReader<R> {
    fn parse(&mut self, mut p: P) -> Result<O, Error<E>>
    where
        for<'a> P: Parser<&'a [u8], O, E>,
    {
        loop {
            let opt =
                    //match p(input.buffer()) {
                    match p.parse(self.buffer()) {
                        Err(Err::Error(e)) => return Err(Error::Error(e)),
                        Err(Err::Failure(e)) => return Err(Error::Failure(e)),
                        Err(Err::Incomplete(_)) => {
                            None
                        },
                        Ok((i, o)) => {
                            let offset = self.buffer().offset(i);
                            Some((offset, o))
                        },
                };

            match opt {
                Some((sz, o)) => {
                    self.consume(sz);
                    return Ok(o);
                }
                None => {
                    self.fill_buf()?;
                }
            }
        }
    }
}

impl<R: Read, O, E, P> Parse<O, E, P> for bufreader::BufReader<R> {
    fn parse(&mut self, mut p: P) -> Result<O, Error<E>>
    where
        for<'a> P: Parser<&'a [u8], O, E>,
    {
        loop {
            let opt =
                    //match p(input.buffer()) {
                    match p.parse(self.buffer()) {
                        Err(Err::Error(e)) => return Err(Error::Error(e)),
                        Err(Err::Failure(e)) => return Err(Error::Failure(e)),
                        Err(Err::Incomplete(_)) => {
                            None
                        },
                        Ok((i, o)) => {
                            let offset = self.buffer().offset(i);
                            Some((offset, o))
                        },
                };

            match opt {
                Some((sz, o)) => {
                    self.consume(sz);
                    return Ok(o);
                }
                None => {
                    self.fill_buf()?;
                }
            }
        }
    }
}

#[cfg(feature = "async")]
#[async_trait]
pub trait AsyncParse<O, E, P> {
    async fn parse(&mut self, p: P) -> Result<O, Error<E>>
    where
        for<'a> P: Parser<&'a [u8], O, E> + Send + 'async_trait;
}

#[cfg(feature = "async")]
#[async_trait]
impl<R: AsyncRead + Unpin + Send, O: Send, E, P> AsyncParse<O, E, P> for BufReader<R> {
    async fn parse(&mut self, mut p: P) -> Result<O, Error<E>>
    where
        for<'a> P: Parser<&'a [u8], O, E> + Send + 'async_trait,
    {
        loop {
            let opt =
                    //match p(input.buffer()) {
                    match p.parse(self.buffer()) {
                        Err(Err::Error(e)) => return Err(Error::Error(e)),
                        Err(Err::Failure(e)) => return Err(Error::Failure(e)),
                        Err(Err::Incomplete(_)) => {
                            None
                        },
                        Ok((i, o)) => {
                            let offset = self.buffer().offset(i);
                            Some((offset, o))
                        },
                };

            match opt {
                Some((sz, o)) => {
                    self.consume_unpin(sz);
                    return Ok(o);
                }
                None => {
                    self.fill_buf().await?;
                }
            }
        }
    }
}
