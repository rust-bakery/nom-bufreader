//! This crate provides a `BufReader` alternative that can be used with
//! [nom parsers](http://docs.rs/nom)
//!
//! It will hide for you the [Incomplete](https://docs.rs/nom/7.0.0/nom/enum.Err.html#variant.Incomplete) handling in nom for streaming parsers, retrying and refilling buffers automatically.
//!
//! For synchronous io, use `bufreader::BufReader`, while for asynchronous
//! IO, you should use `async_bufreader::BufReader`
//!
//! # Examples
//!
//! ## sync
//!
//! ```rust,ignore
//! use nom_bufreader::bufreader::BufReader;
//! use nom_bufreader::{Error, Parse};
//! use std::{net::TcpListener, str::from_utf8};
//!
//! fn main() -> Result<(), Error<()>> {
//!     let listener = TcpListener::bind("127.0.0.1:8080")?;
//!     let mut i = BufReader::new(listener.incoming().next().unwrap()?);
//!
//!     // method, space and path are nom parsers
//!     let m = i.parse(method)?;
//!     let _ = i.parse(space)?;
//!     let p = i.parse(path)?;
//!     println!("got method {}, path {}", m, p);
//!     Ok(())
//! }
//! ```
//!
//! ### async
//!
//! #### tokio
//!
//! ```rust,ignore
//! use nom_bufreader::async_bufreader::BufReader;
//! use nom_bufreader::{AsyncParse, Error};
//! use std::str::from_utf8;
//! use tokio_util::compat::TokioAsyncReadCompatExt;
//! use tokio::net::TcpListener;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error<()>> {
//!     let listener = TcpListener::bind("127.0.0.1:8080").await?;
//!     let mut i = BufReader::new(listener.accept().await?.0.compat());
//!
//!     let m = i.parse(method).await?;
//!     let _ = i.parse(space).await?;
//!     let p = i.parse(path).await?;
//!     println!("got method {}, path {}", m, p);
//!     Ok(())
//! }
//! ```
//!
//! #### async-std
//!
//! ```rust,ignore
//! use nom_bufreader::async_bufreader::BufReader;
//! use nom_bufreader::{AsyncParse, Error};
//! use std::str::from_utf8;
//! use async_std::net::TcpListener;
//!
//! #[async_std::main]
//! async fn main() -> Result<(), Error<()>> {
//!     let listener = TcpListener::bind("127.0.0.1:8080").await?;
//!     let mut i = BufReader::new(listener.accept().await?.0);
//!
//!     let m = i.parse(method).await?;
//!     let _ = i.parse(space).await?;
//!     let p = i.parse(path).await?;
//!     println!("got method {}, path {}", m, p);
//!     Ok(())
//! }
//! ```
use futures_io::AsyncBufRead;
use nom::{Err, Needed, Offset, Parser};
use std::io::{self, BufRead, Read};

#[cfg(feature = "async")]
use async_trait::async_trait;
#[cfg(feature = "async")]
use futures_io::AsyncRead;
#[cfg(feature = "async")]
use futures_util::io::{AsyncBufReadExt, BufReader};

#[cfg(feature = "async")]
pub mod async_bufreader;
pub mod bufreader;

#[derive(Debug)]
pub enum Error<E> {
    Error(E),
    Failure(E),
    Io(io::Error),
    Eof,
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
        let mut eof = false;
        let mut error = None;
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
                    if eof {
                        return Err(Error::Eof);
                    }

                    if let Some(e) = error.take() {
                        return Err(Error::Io(e));
                    }

                    match self.fill_buf() {
                        Err(e) => error = Some(e),
                        Ok(s) => {
                            if s.is_empty() {
                                eof = true;
                            }
                        }
                    }
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

#[cfg(feature = "async")]
#[async_trait]
impl<R: AsyncBufRead + Unpin + Send, O: Send, E: Send, P> AsyncParse<O, E, P>
    for async_bufreader::BufReader<R>
{
    async fn parse(&mut self, mut p: P) -> Result<O, Error<E>>
    where
        for<'a> P: Parser<&'a [u8], O, E> + Send + 'async_trait,
    {
        loop {
            let buffer = self.buffer();
            match p.parse(buffer) {
                Err(Err::Error(e)) => return Err(Error::Error(e)),
                Err(Err::Failure(e)) => return Err(Error::Failure(e)),
                Err(Err::Incomplete(Needed::Unknown)) => {
                    self.my_fill_buf(None).await?;
                }
                Err(Err::Incomplete(Needed::Size(x))) => {
                    self.my_fill_buf(Some(x)).await?;
                }
                Ok((i, o)) => {
                    let offset = buffer.offset(i);
                    self.consume(offset);
                    return Ok(o);
                }
            };
        }
    }
}
