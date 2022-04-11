//! ### Keeping track of clients
//!
//! This doesn't directly relate to the WebSocket protocol,
//! but it's worth mentioning here: your server must keep track of clients' sockets so you don't keep handshaking again with clients who have already completed the handshake.
//! The same client IP address can try to connect multiple times.
//! However, the server can deny them if they attempt too many connections in order to save itself from [Denial-of-Service attack](https://en.wikipedia.org/wiki/Denial-of-service_attack).
//!
//!
//! For example, you might keep a table of usernames or ID numbers along with the corresponding WebSocket and other data that you need to associate with that connection.

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
