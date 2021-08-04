use futures::io::BufReader;
use nom::{
    branch::alt,
    bytes::streaming::{tag, take_until},
    character::streaming::space0,
    combinator::map_res,
    IResult,
};
use nom_bufreader::{AsyncParse, Error};
use std::str::from_utf8;
use tokio_util::compat::TokioAsyncReadCompatExt;

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

#[tokio::main]
async fn main() -> Result<(), Error<()>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    let mut i = BufReader::new(listener.accept().await?.0.compat());

    let m = i.parse(method).await?;
    let _ = i.parse(space).await?;
    let p = i.parse(path).await?;
    println!("got method {}, path {}", m, p);
    Ok(())
}
