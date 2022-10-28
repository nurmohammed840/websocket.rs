# Web-Socket

This library provide WebSocket implementation for both client and server.

### Usage

```yaml
cargo add web-socket
```

Or add this to your `Cargo.toml` file

```toml
[dependencies]
web-socket = "0.1"
```

### Client example

```rust
use web_socket::{WebSocket, DataType};

let mut ws = WebSocket::connect("ws://example.com/chat").await?;
ws.send(get_data_somehow()).await?;

loop {
    let mut data = ws.recv().await?;

    let mut buf = vec![];
    data.read_to_end(&mut buf).await?;

    match data.ty {
        DataType::Binary => println!("Data: {buf:?}"),
        DataType::Text => println!("Text: {:?}", String::from_utf8(buf)),
    }
}
```

### Ping-Pong Example

```rust
use web_socket::{WebSocket, Event};

let mut ws = WebSocket::connect("ws://ws.ifelse.io").await?;

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
```

You can run this example by: `cargo run --example ping_pong`

### Feature

- [x]  Support async API.
- [x]  Support [backpressure](https://developer.mozilla.org/en-US/docs/Web/API/Streams_API/Concepts#backpressure)
- [x]  Support fragmented messages.
- [x]  Allow sending control frame.

### Todo

- [ ] Add sync API
- [ ] Complete API docs
- [ ] Support wss connection over TLS


#### License

This project is licensed under [Apache License 2.0](https://github.com/nurmohammed840/websocket.rs/blob/master/LICENSE)