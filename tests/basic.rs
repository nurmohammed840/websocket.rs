use std::io;
use web_socket::*;

const DATA: &str = "Hello";

#[tokio::test]
async fn unmasked() -> io::Result<()> {
    let mut writer = vec![];

    // ----------------------- send txt message -----------------------

    let mut ws = WebSocket::server(&mut writer);
    ws.send(DATA).await?;
    assert_eq!(writer, [0x81, 5, b'H', b'e', b'l', b'l', b'o']);

    // ----------------------- send ping request ----------------------
    writer.clear();

    let mut ws = WebSocket::server(&mut writer);
    ws.send_ping(DATA).await?;
    assert_eq!(writer, [0x89, 5, b'H', b'e', b'l', b'l', b'o']);

    // ----------------------- streaming text -------------------------
    writer.clear();

    let mut ws = WebSocket::server(&mut writer);
    ws.send(Frame {
        fin: false,
        opcode: MessageType::Text as u8,
        data: b"Hel",
    })
    .await?;

    ws.send(Frame {
        fin: true,
        opcode: 0,
        data: b"lo",
    })
    .await?;
    assert_eq!(
        writer,
        [
            0x01, 3, b'H', b'e', b'l', // fragmented frame
            0x80, 2, b'l', b'o', // final frame
        ]
    );
    Ok(())
}
