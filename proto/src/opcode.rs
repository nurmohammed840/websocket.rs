/// Defines the interpretation of the "Payload data".  If an unknown
/// opcode is received, the receiving endpoint MUST _Fail the WebSocket Connection_.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    /// The FIN and opcode fields work together to send a message split up into separate frames. This is called message fragmentation.
    ///
    /// ```txt
    /// Client: FIN=1, opcode=0x1, msg="hello"
    /// Server: (process complete message immediately) Hi.
    /// Client: FIN=0, opcode=0x1, msg="and a"
    /// Server: (listening, new message containing text started)
    /// Client: FIN=0, opcode=0x0, msg="happy new"
    /// Server: (listening, payload concatenated to previous message)
    /// Client: FIN=1, opcode=0x0, msg="year!"
    /// Server: (process complete message) Happy new year to you too!
    /// ```
    ///
    /// ### Note
    ///
    /// - Control frames MAY be injected in the middle of
    ///   a fragmented message.  Control frames themselves MUST NOT be
    ///   fragmented. An endpoint MUST be capable of handling control frames in the
    ///   middle of a fragmented message.
    ///
    Continue = 0,

    Text = 1,
    Binary = 2,

    // 3-7 are reserved for further non-control frames.
    /// - The Close frame MAY contain a body that indicates a reason for closing.
    ///
    /// - If there is a body, the first two bytes of the body MUST be a 2-byte unsigned integer (in network byte order: Big Endian)
    ///   representing a status code with value /code/ defined in [Section 7.4](https://datatracker.ietf.org/doc/html/rfc6455#section-7.4). Following the 2-byte integer,
    ///
    /// - Close frames sent from client to server must be masked.
    /// - The application MUST NOT send any more data frames after sending a `Close` frame.
    ///
    /// - If an endpoint receives a Close frame and did not previously send a
    ///   Close frame, the endpoint MUST send a Close frame in response.  (When
    ///   sending a Close frame in response, the endpoint typically echos the
    ///   status code it received.)  It SHOULD do so as soon as practical.  An
    ///   endpoint MAY delay sending a Close frame until its current message is
    ///   sent
    ///
    /// - After both sending and receiving a Close message, an endpoint
    ///   considers the WebSocket connection closed and MUST close the
    ///   underlying TCP connection.
    Close = 8,

    /// A Ping frame MAY include "Application data".
    /// Unless it already received a Close frame.  It SHOULD respond with Pong frame as soon as is practical.
    ///
    /// A Ping frame may serve either as a keepalive or as a means to verify that the remote endpoint is still responsive.
    Ping = 9,

    /// A Pong frame sent in response to a Ping frame must have identical
    /// "Application data" as found in the message body of the Ping frame being replied to.
    ///
    /// If an endpoint receives a Ping frame and has not yet sent Pong frame(s) in response to previous Ping frame(s), the endpoint MAY
    /// elect to send a Pong frame for only the most recently processed Ping frame.
    ///
    ///  A Pong frame MAY be sent unsolicited.  This serves as a unidirectional heartbeat.  A response to an unsolicited Pong frame is not expected.
    Pong = 10,
    // 11-15 are reserved for further control frames
}

impl Opcode {
    /// Whether the opcode indicates a control frame.
    #[inline]
    pub fn is_control(self) -> bool {
        self as u8 >= 8
    }
}

impl TryFrom<u8> for Opcode {
    type Error = &'static str;
    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Opcode::Continue,
            1 => Opcode::Text,
            2 => Opcode::Binary,
            8 => Opcode::Close,
            9 => Opcode::Ping,
            10 => Opcode::Pong,
            _ => return Err("Unknown opcode"),
        })
    }
}

impl From<Opcode> for u8 {
    #[inline]
    fn from(opcode: Opcode) -> Self {
        opcode as u8
    }
}
