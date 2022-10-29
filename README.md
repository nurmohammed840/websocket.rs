# Web-Socket

This library provide WebSocket implementation for both client and server.

## Usage

Run:

```txt
cargo add web-socket
```

Or add this to your `Cargo.toml` file.

```toml
[dependencies]
web-socket = "0.2"
```

### Ping-Pong Example

You can run this example with: `cargo run --example ping_pong`

```rust no_run
use std::io::Result;
use web_socket::{WebSocket, Event};

async fn example() -> Result<()> {
    let mut ws = WebSocket::connect("ws.ifelse.io:80", "/").await?;

    ws.on_event = Box::new(|ev| {
        if let Event::Pong(_) = ev {
            println!("Pong: {}", ev.to_string());
        }
        Ok(())
    });

    for _ in 0..5 {
        ws.send(Event::Ping(b"Hello!")).await?;
        ws.send("Copy Cat!").await?;

        let mut data = ws.recv().await?;

        let mut buf = vec![];
        data.read_to_end(&mut buf).await?;
        println!("Text: {:?}", String::from_utf8(buf));
    }
    Ok(())
}
```

For more examples, see [./examples](https://github.com/nurmohammed840/websocket.rs/tree/master/examples) directory.

### Feature

- [x]  Support async API.
- [x]  Support [backpressure](https://developer.mozilla.org/en-US/docs/Web/API/Streams_API/Concepts#backpressure)
- [x]  Support fragmented messages.
- [x]  Allow sending control frame.
- [ ]  Support sync API
- [ ]  Client support wss connection over TLS v1.3

#### License

This project is licensed under [Apache License 2.0](https://github.com/nurmohammed840/websocket.rs/blob/master/LICENSE)