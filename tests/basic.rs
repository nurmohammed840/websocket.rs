#![allow(warnings)]
use tokio::io::*;
use web_socket::*;

type WebSocket<const SIDE: bool> = web_socket::WebSocket<SIDE, BufReader<DuplexStream>>;
const MSG: &str = "Hello, World";

macro_rules! code {
    [$ws: expr] => {
        $ws.send(MSG).await?;

        let mut data = $ws.recv().await?;
        assert_eq!(data.fin(), true);
        assert_eq!(data.len(), MSG.len());
        assert_eq!(data.ty, DataType::Text);
    
        let mut buf = vec![];
        data.read_to_end(&mut buf).await?;
        assert_eq!(Ok("Hello, World".into()), String::from_utf8(buf));    
    };
}

async fn server(mut ws: WebSocket<SERVER>) -> Result<()> {
    code!(ws);
    Ok(())
}

async fn client(mut ws: WebSocket<CLIENT>) -> Result<()> {
    code!(ws);
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
