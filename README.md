# Web-Socket

This library provide a simple, modern and 
intuitive [WebSocket](https://en.wikipedia.org/wiki/WebSocket) implementation for both client and server.

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

### Example

You can run this example with: `cargo run --example minimal`

```rust no_run
use tokio::io::*;
use web_socket::*;

async fn example<IO>(mut ws: WebSocket<CLIENT, IO>) -> Result<()>
where
    IO: Unpin + AsyncRead + AsyncWrite,
{
    let _ = ws.recv().await?; // ignore message: Request served by 4338e324
    for _ in 0..3 {
        ws.send("Copy Cat!").await?;

        match ws.recv().await? {
            Event::Data { ty, data } => {
                assert_eq!(ty, DataType::Complete(MessageType::Text));
                assert_eq!(&*data, b"Copy Cat!");
            }
            Event::Ping(data) => ws.send_ping(data).await?,
            Event::Pong(..) => {}
            Event::Error(..) => return ws.close(CloseCode::ProtocolError).await,
            Event::Close { .. } => return ws.close(()).await,
        }
    }
    ws.close("bye!").await
}

```

For more examples, see [./examples](https://github.com/nurmohammed840/websocket.rs/tree/master/examples) directory.

It passed every test of the [autobahn testsuite](https://github.com/crossbario/autobahn-testsuite)

#### License

This project is licensed under [Apache License 2.0](https://github.com/nurmohammed840/websocket.rs/blob/master/LICENSE)