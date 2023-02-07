#![allow(warnings)]
use tokio::io::*;
use web_socket::*;

type WebSocket<const SIDE: bool> = web_socket::WebSocket<SIDE, BufReader<DuplexStream>>;
const MSG: &str = "Hello, World";

async fn server(mut ws: WebSocket<SERVER>) -> Result<()> {
    ws.send(MSG).await?;

    // Sending raw bytes:
    // ws.stream
    //     .write_all(&[
    //         0x01, 0x03, 0x48, 0x65, 0x6c, // fragmented frame
    //         0x80, 0x02, 0x6c, 0x6f, // final frame
    //     ])
    //     .await?;

    Ok(())
}

async fn client(mut ws: WebSocket<CLIENT>) -> Result<()> {
    let mut data = ws.recv().await?;
    assert_eq!(data.fin(), true);
    assert_eq!(data.len(), MSG.len());
    assert_eq!(data.ty, DataType::Text);

    let mut buf = vec![];
    data.read_to_end(&mut buf).await?;
    println!("{:?}", String::from_utf8(buf));
    Ok(())
}

#[test]
fn example() -> Result<()> {
    block_on(async {
        let mut duplex = duplex(8192);
        let server = tokio::spawn(server(WebSocket::from(BufReader::new(duplex.0))));
        let client = tokio::spawn(client(WebSocket::from(BufReader::new(duplex.1))));

        server.await??;
        client.await??;
        Ok(())
    })
}

/// This function create a single threaded async runtime.  
pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}
