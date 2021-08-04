use super::Error;
use nom::{Err, Offset, Parser};
use std::io::{BufRead, BufReader, Read};

trait Parse<O, E, P> {
    fn parse(&mut self, p: P) -> Result<O, Error<E>>
    where
        for<'a> P: Parser<&'a [u8], O, E>;
}

impl<R: Read, O, E, P> Parse<O, E, P> for BufReader<R> {
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
