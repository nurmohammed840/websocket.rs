// #![allow(warnings)]
// use tokio::io::*;
// use web_socket::*;

// type WebSocket<const SIDE: bool> = web_socket::WebSocket<SIDE, DuplexStream>;
// const MSG: &str = "Hello, World";

// // macro_rules! code {
// //     [$ws: expr] => {
// //         $ws.send(MSG).await?;

// //         let mut data = $ws.recv().await?;
// //         assert_eq!(data.fin(), true);
// //         assert_eq!(data.len(), MSG.len());
// //         assert_eq!(data.ty, DataType::Text);

// //         let mut buf = vec![];
// //         data.read_to_end(&mut buf).await?;
// //         assert_eq!(Ok("Hello, World".into()), String::from_utf8(buf));
// //     };
// // }

// // async fn server(mut ws: WebSocket<SERVER>) -> Result<()> {
// //     code!(ws);
// //     Ok(())
// // }

// // async fn client(mut ws: WebSocket<CLIENT>) -> Result<()> {
// //     code!(ws);
// //     Ok(())
// // }

// async fn server(mut ws: WebSocket<SERVER>) -> Result<()> {
//     while let Ok(ev) = ws.recv().await {
//         println!("{:#?}", ev);
//     }
//     // code!(ws);
//     Ok(())
// }

// async fn client(mut ws: WebSocket<CLIENT>) -> Result<()> {
//     // code!(ws);
//     let mut buf = vec![];
//     encode::<CLIENT>(&mut buf, true, 2, b"Hello, World");
//     ws.stream.write_all(&buf).await?;
    
//     // let mut buf = vec![];
//     // encode::<CLIENT>(&mut buf, false, 1, b"Hello, World");
//     // ws.stream.write_all(&buf).await?;
//     Ok(())
// }

// #[test]
// fn example() -> Result<()> {
//     block_on(async {
//         let mut duplex = duplex(8192);
//         let server = tokio::spawn(server(WebSocket::from(duplex.0)));
//         let client = tokio::spawn(client(WebSocket::from(duplex.1)));
//         server.await??;
//         client.await??;
//         Ok(())
//     })
// }

// /// This function create a single threaded async runtime.  
// pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
//     tokio::runtime::Builder::new_current_thread()
//         .enable_all()
//         .build()
//         .unwrap()
//         .block_on(future)
// }

// fn encode<const SIDE: bool>(writer: &mut Vec<u8>, fin: bool, opcode: u8, data: &[u8]) {
//     let data_len = data.len();
//     writer.reserve(if SERVER == SIDE { 10 } else { 14 } + data_len);
//     unsafe {
//         let filled = writer.len();
//         let start = writer.as_mut_ptr().add(filled);

//         let mask_bit = if SERVER == SIDE { 0 } else { 0x80 };

//         start.write(((fin as u8) << 7) | opcode);
//         let len = if data_len < 126 {
//             start.add(1).write(mask_bit | data_len as u8);
//             2
//         } else if data_len < 65536 {
//             let [b2, b3] = (data_len as u16).to_be_bytes();
//             start.add(1).write(mask_bit | 126);
//             start.add(2).write(b2);
//             start.add(3).write(b3);
//             4
//         } else {
//             let [b2, b3, b4, b5, b6, b7, b8, b9] = (data_len as u64).to_be_bytes();
//             start.add(1).write(mask_bit | 127);
//             start.add(2).write(b2);
//             start.add(3).write(b3);
//             start.add(4).write(b4);
//             start.add(5).write(b5);
//             start.add(6).write(b6);
//             start.add(7).write(b7);
//             start.add(8).write(b8);
//             start.add(9).write(b9);
//             10
//         };

//         let header_len = if SERVER == SIDE {
//             std::ptr::copy_nonoverlapping(data.as_ptr(), start.add(len), data_len);
//             len
//         } else {
//             let mask = 0u32.to_ne_bytes();
//             let [a, b, c, d] = mask;
//             start.add(len).write(a);
//             start.add(len + 1).write(b);
//             start.add(len + 2).write(c);
//             start.add(len + 3).write(d);

//             let dist = start.add(len + 4);
//             for (index, byte) in data.iter().enumerate() {
//                 dist.add(index).write(byte ^ mask.get_unchecked(index % 4));
//             }
//             len + 4
//         };
//         writer.set_len(filled + header_len + data_len);
//     }
// }
