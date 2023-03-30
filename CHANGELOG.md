# Change Log

## [0.6] - 30 March 2023

- Added: `PartialEq<u16>` trait for `CloseCode`
- Added: `Ping` struct for sending ping response
- Added: `Pong` struct for sending pong response
- Removed: `WebSocket::send_ping`, use `ws.send(Ping(..))` instead
- Removed: `WebSocket::send_pong`, use `ws.send(Pong(..))` instead

## [0.5] - 27 March 2023

Initial release