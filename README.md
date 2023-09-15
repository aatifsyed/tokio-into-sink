<!-- cargo-rdme start -->

Use an [`AsyncWrite`] as a [`Sink`]`<Item: AsRef<[u8]>`.

This adapter produces a sink that will write each value passed to it into the underlying writer.
Note that this function consumes the given writer, returning a wrapped version.

```rust
use tokio_into_sink::IntoSinkExt as _;
use futures::{stream, StreamExt as _};
use std::io;

let stream = stream::iter(["hello", "world"]).map(io::Result::Ok);
let write = tokio::fs::File::create("/dev/null").await.unwrap();
let sink = write.into_sink();
stream.forward(sink).await.unwrap();
```

Ported from [`futures::io::AsyncWriteExt::into_sink`](https://docs.rs/futures/0.3.28/futures/io/trait.AsyncWriteExt.html#method.into_sink).

<!-- cargo-rdme end -->
