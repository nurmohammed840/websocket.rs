use hyper::{
    body::Incoming, header::*, server::conn::http1, service::service_fn, Request, Response,
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Listening on http://{}", listener.local_addr()?);
    loop {
        let (stream, addr) = listener.accept().await?;
        println!("Incoming {addr}");

        if let Err(err) = http1::Builder::new()
            .serve_connection(stream, service_fn(handle))
            .with_upgrades()
            .await
        {
            println!("Failed to serve connection: {err:#?}");
        }
    }
}

async fn handle(req: Request<Incoming>) -> hyper::Result<Response<String>> {
    // let f = header::SEC_WEBSOCKET_KEY;
    // let body = format!("{:#?}", req);
    // println!("{body}");

    let headers: &HeaderMap = req.headers();
    if let Some(true) = headers.get(UPGRADE).map(|h| h == "websocket") {
        
    }
    // if headers.contains_key(UPGRADE) {
    //     // hyper::upgrade::on(req);
    // }
    // Ok(Response::new(body))
    todo!()
}


fn upgrade(headers: &HeaderMap) -> Option<()> {
    if headers.get(UPGRADE)? != "websocket" {
        return None
    }
    
    None
}