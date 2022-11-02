mod utils;
use std::io::Result;
use web_socket::{client::WSS, DataType, Event};

async fn ping_pong() -> Result<()> {
    let mut ws = WSS::connect("ws.ifelse.io:443", "/").await?;

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
        assert_eq!(data.ty, DataType::Text);

        let mut buf = vec![];
        data.read_to_end(&mut buf).await?;
        println!("Text: {:?}", String::from_utf8(buf));
    }
    Ok(())
}

fn main() {
    println!("Status: {:?}", utils::block_on(ping_pong()));
}
