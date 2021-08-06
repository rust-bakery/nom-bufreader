# nom-bufreader, adapters for BufReader around nom

**/!\Work in progress, if you put it in production, you fix it/!\**

With this crate, you can assemble a nom parser with a `BufReader` alternative, synchronous or asynchronous.
Due to incompatible buffering strategies, [std::io::BufReader](https://doc.rust-lang.org/stable/std/io/struct.BufReader.html)
and [futures::io::BufReader](https://docs.rs/futures/0.3.16/futures/io/struct.BufReader.html)
cannot be used directly. This crate proovide compatible forks instead, in the
`bufreader` and `async_bufreader` modules.

It will hide for you the [Incomplete](https://docs.rs/nom/6.2.1/nom/enum.Err.html#variant.Incomplete) handling in nom for streaming parsers, retrying and refilling buffers automatically.

See the `examples/` directory for usage
