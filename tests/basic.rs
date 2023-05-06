use web_socket::{Frame, *};
const DATA: &str = "Hello";

#[tokio::test]
async fn unmasked_txt_msg() {
    let mut writer = vec![];
    let mut ws = WebSocket::server(&mut writer);
    ws.send(DATA).await.unwrap();
    assert_eq!(writer, [0x81, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f]);
}

#[tokio::test]
async fn unmasked_ping_req() {
    let mut bytes = vec![];
    let mut ws = WebSocket::server(&mut bytes);
    ws.send_ping(DATA).await.unwrap();
    assert_eq!(bytes, [0x89, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f,]);
}

#[tokio::test]
async fn fragmented_unmasked_txt_msg() {
    let mut bytes = vec![];
    let mut ws = WebSocket::server(&mut bytes);

    ws.send(Frame {
        fin: false,
        opcode: 1,
        data: b"Hel",
    })
    .await
    .unwrap();

    ws.send(Frame {
        fin: true,
        opcode: 0,
        data: b"lo",
    })
    .await
    .unwrap();

    assert_eq!(
        bytes,
        [
            0x01, 0x03, 0x48, 0x65, 0x6c, // fragmented frame
            0x80, 0x02, 0x6c, 0x6f, // final frame
        ]
    );
}
