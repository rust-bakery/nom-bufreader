# nom-bufreader, adapters for BufReader around nom

**/!\Work in progress, if you put it in production, you fix it/!\**

With this crate, you can assemble a nom parser with a `BufReader` alternative, synchronous or asynchronous.
Due to incompatible buffering strategies, [std::io::BufReader](https://doc.rust-lang.org/stable/std/io/struct.BufReader.html)
and [futures::io::BufReader](https://docs.rs/futures/0.3.16/futures/io/struct.BufReader.html)
cannot be used directly. This crate proovide compatible forks instead, in the
`bufreader` and `async_bufreader` modules.

It will hide for you the [Incomplete](https://docs.rs/nom/7.0.0/nom/enum.Err.html#variant.Incomplete) handling in nom for streaming parsers, retrying and refilling buffers automatically.

## Examples

### sync

```rust
use nom_bufreader::bufreader::BufReader;
use nom_bufreader::{Error, Parse};
use std::{net::TcpListener, str::from_utf8};

fn main() -> Result<(), Error<()>> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    let mut i = BufReader::new(listener.incoming().next().unwrap()?);

    // method, space and path are nom parsers
    let m = i.parse(method)?;
    let _ = i.parse(space)?;
    let p = i.parse(path)?;
    println!("got method {}, path {}", m, p);
    Ok(())
}
```

### async

#### tokio

```rust
use nom_bufreader::async_bufreader::BufReader;
use nom_bufreader::{AsyncParse, Error};
use std::str::from_utf8;
use tokio_util::compat::TokioAsyncReadCompatExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Error<()>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let mut i = BufReader::new(listener.accept().await?.0.compat());

    let m = i.parse(method).await?;
    let _ = i.parse(space).await?;
    let p = i.parse(path).await?;
    println!("got method {}, path {}", m, p);
    Ok(())
}
```

#### async-std

```rust
use nom_bufreader::async_bufreader::BufReader;
use nom_bufreader::{AsyncParse, Error};
use std::str::from_utf8;
use async_std::net::TcpListener;

#[async_std::main]
async fn main() -> Result<(), Error<()>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let mut i = BufReader::new(listener.accept().await?.0);

    let m = i.parse(method).await?;
    let _ = i.parse(space).await?;
    let p = i.parse(path).await?;
    println!("got method {}, path {}", m, p);
    Ok(())
}
```
