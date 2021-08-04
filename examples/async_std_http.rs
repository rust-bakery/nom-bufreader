use futures::{
    io::{AsyncBufRead, AsyncBufReadExt, BufReader},
    AsyncRead,
};
use nom::{
    branch::alt,
    bytes::streaming::{tag, take_until},
    character::streaming::space0,
    combinator::map_res,
    Err, IResult, Offset, Parser,
};
use nom_bufreader::{r#async::AsyncParse, Error};
use std::str::from_utf8;

fn method(i: &[u8]) -> IResult<&[u8], String, ()> {
    map_res(alt((tag("GET"), tag("POST"), tag("HEAD"))), |s| {
        from_utf8(s).map(|s| s.to_string())
    })(i)
}

fn path(i: &[u8]) -> IResult<&[u8], String, ()> {
    map_res(take_until(" "), |s| from_utf8(s).map(|s| s.to_string()))(i)
}

fn space(i: &[u8]) -> IResult<&[u8], (), ()> {
    let (i, _) = space0(i)?;
    Ok((i, ()))
}

fn http_version(i: &[u8]) -> IResult<&[u8], (), ()> {
    let (i, _) = tag("HTTP/1.1")(i)?;
    Ok((i, ()))
}

fn crlf(i: &[u8]) -> IResult<&[u8], (), ()> {
    let (i, _) = tag("r\n")(i)?;
    Ok((i, ()))
}

#[async_std::main]
async fn main() -> Result<(), Error<()>> {
    let listener = async_std::net::TcpListener::bind("127.0.0.1:8080").await?;
    let mut i = BufReader::new(listener.accept().await?.0);

    let m = i.parse(method).await?;
    let _ = i.parse(space).await?;
    let p = i.parse(path).await?;
    println!("got method {}, path {}", m, p);
    Ok(())
}
