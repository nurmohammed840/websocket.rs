mod utils;

use std::io::{Error, ErrorKind, Result};
use tokio::net::TcpStream;
use web_socket::{client::WS, CloseCode, DataType, WebSocket, CLIENT};

const ADDR: &str = "localhost:9001";
const AGENT: &str = "agent=web-socket";

async fn get_case_count() -> Result<u32> {
    let mut ws = WS::connect(ADDR, "/getCaseCount").await?;
    let msg = read_msg!(ws)?;
    ws.close(CloseCode::Normal, "").await?;
    Ok(msg.parse().unwrap())
}

async fn run_test(case: u32) -> Result<()> {
    // println!("Running test case {case}");
    let mut ws = WS::connect(ADDR, format!("/runCase?case={case}&{AGENT}")).await?;
    match echo(&mut ws).await.err().unwrap().kind() {
        ErrorKind::NotConnected | ErrorKind::InvalidData => Ok(()),
        _ => ws.close(CloseCode::ProtocolError, "").await,
    }
}

async fn echo(ws: &mut WebSocket<CLIENT, tokio::io::BufReader<TcpStream>>) -> Result<()> {
    loop {
        let mut data = ws.recv().await?;

        let mut msg = vec![];
        data.read_to_end(&mut msg).await?;

        match data.ty {
            DataType::Binary => ws.send(&*msg).await?,
            DataType::Text => {
                let msg = std::str::from_utf8(&msg)
                    .map_err(|err| Error::new(ErrorKind::InvalidData, err))?;

                ws.send(msg).await?;
            }
        }
    }
}

async fn update_reports() -> Result<()> {
    let ws = WS::connect(ADDR, format!("/updateReports?{AGENT}")).await?;
    ws.close(CloseCode::Normal, "").await
}

#[tokio::main]
async fn main() {
    let total = get_case_count().await.expect("Error getting case count");
    for case in 1..=total {
        let _ = run_test(case).await;
    }
    update_reports().await.expect("Error updating reports");
}
