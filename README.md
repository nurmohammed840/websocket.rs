# Web-Socket

This library provide [WebSocket](https://en.wikipedia.org/wiki/WebSocket) implementation for both client and server. It provides a simple, modern and 
intuitive WebSocket interface.

## Usage

Run:

```txt
cargo add web-socket
```

Or add this to your `Cargo.toml` file.

```toml
[dependencies]
web-socket = "0.5"
```

### Ping-Pong Example

You can run this example with: `cargo run --example minimal`

```rust no_run
use std::io::Result;
use web_socket::{client::WSS, DataType};

async fn example() -> Result<()> {
    let mut ws = WSS::connect("ws.ifelse.io:443", "/").await?;
    for _ in 0..3 {
        ws.send("Copy Cat!").await?;

        let mut data = ws.recv().await?;
        assert_eq!(data.ty, DataType::Text);

        let mut buf = vec![];
        data.read_to_end(&mut buf).await?;
        println!("Text: {:?}", String::from_utf8(buf));
    }
    Ok(())
}
```

For more examples, see [./examples](https://github.com/nurmohammed840/websocket.rs/tree/master/examples) directory.

It passed every test of the [autobahn testsuite](https://github.com/crossbario/autobahn-testsuite)

#### License

This project is licensed under [Apache License 2.0](https://github.com/nurmohammed840/websocket.rs/blob/master/LICENSE)