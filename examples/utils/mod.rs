#![allow(dead_code)]
// pub mod ws;

use std::{thread, time::Duration};

/// This function create a single threaded async runtime.  
pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

pub async fn sleep(ms: u64) {
    tokio::task::spawn_blocking(move || thread::sleep(Duration::from_millis(ms)))
        .await
        .unwrap();
}
