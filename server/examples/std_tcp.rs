use std::{
    io::{Read, Result, Write},
    net::{TcpListener, TcpStream},
    thread,
};

use bin_layout::{Cursor, Decoder};

fn process(mut stream: TcpStream) -> Result<()> {
    let mut buf = [0; 80 * 1024];
    let len = stream.read(&mut buf)?;
    let data = &buf[..len];

    if data.starts_with(b"GET /chat HTTP/1.1") {
        let req = String::from_utf8(data.to_vec()).unwrap();

        println!("Received:\n\n{req}");

        let sec_key = req
            .lines()
            .filter_map(|line| line.split_once(": "))
            .find_map(|(key, value)| key.contains("Sec-WebSocket-Key").then_some(value))
            .unwrap();

        let res = ws_proto::handshake::response(sec_key);

        println!("Sending Responce:\n\n{res}");
        stream.write_all(res.as_bytes())?;

        // ----------------------------------------------------
        loop {
            // std::thread::sleep_ms(2000);
            let n = stream.read(&mut buf[..])?;
            let mut c = Cursor::new(&buf[..n]);
            
            println!("Received: {n} bytes");
            println!("{:#?}", ws_proto::Header::decoder(&mut c).unwrap());
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Goto: http://{}\n", listener.local_addr()?);

    loop {
        let (stream, addr) = listener.accept()?;
        thread::spawn(move || {
            println!("# Peer addr: {addr}\n");
            println!("{:#?}", process(stream));
        });
    }
}
