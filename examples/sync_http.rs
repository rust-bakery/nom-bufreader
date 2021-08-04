use nom::{
    branch::alt,
    bytes::streaming::{tag, take_until},
    character::streaming::space0,
    combinator::map_res,
    Err, IResult, Offset, Parser,
};
use nom_bufreader::{sync::Parse, Error};
use std::{
    io::{self, BufRead, BufReader, Read},
    net::TcpListener,
    str::from_utf8,
};

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

fn main() -> Result<(), Error<()>> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    let mut i = BufReader::new(listener.incoming().next().unwrap()?);

    let m = i.parse(method)?;
    let _ = i.parse(space)?;
    let p = i.parse(path)?;
    let _ = i.parse(space)?;
    let _ = i.parse(http_version)?;
    println!("got method {}, path {}", m, p);
    Ok(())
}
