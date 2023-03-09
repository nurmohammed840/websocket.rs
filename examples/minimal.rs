mod utils;
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

fn main() {
    println!("Status: {:?}", utils::block_on(example()));
}
