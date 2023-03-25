mod answer;
mod utils;

use crate::utils::connect;
use answer::echo;
use std::{io::Result, str};
use web_socket::*;

const ADDR: &str = "localhost:9001";
const AGENT: &str = "agent=web-socket";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let total = get_case_count().await.expect("unable to get case count");
    for case in 1..=total {
        let ws = connect(ADDR, &format!("/runCase?case={case}&{AGENT}")).await?;
        tokio::spawn(async move {
            if let Err(err) = echo(ws).await {
                eprintln!("ws error: {err:#?}")
            }
        });
    }
    update_reports().await
}

async fn get_case_count() -> Option<u32> {
    let mut ws = connect(ADDR, "/getCaseCount").await.unwrap();
    if let Event::Data { data, .. } = ws.recv().await.unwrap() {
        return std::str::from_utf8(&data).ok()?.parse().ok();
    }
    None
}

async fn update_reports() -> Result<()> {
    let ws = connect(ADDR, &format!("/updateReports?{AGENT}")).await?;
    ws.close(()).await
}
