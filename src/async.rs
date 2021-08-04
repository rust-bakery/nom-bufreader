use super::Error;
use async_trait::async_trait;
use futures::{
    io::{AsyncBufReadExt, BufReader},
    AsyncRead,
};
use nom::{Err, Offset, Parser};

#[async_trait]
trait AsyncParse<O, E, P> {
    async fn parse(&mut self, p: P) -> Result<O, Error<E>>
    where
        for<'a> P: Parser<&'a [u8], O, E> + Send + 'async_trait;
}

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
