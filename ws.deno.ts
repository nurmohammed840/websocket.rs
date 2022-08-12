// @ts-nocheck

import { serve } from "https://deno.land/std/http/mod.ts";

async function reqHandler(req: Request) {
    if (req.headers.get("upgrade") != "websocket") {
        return new Response(null, { status: 501 });
    }
    const { socket: ws, response } = Deno.upgradeWebSocket(req);

    ws.onopen = () => {
        console.log("Socket Opened!");
    }
    ws.onmessage = (msg) => {
        console.log("Msg:", msg.data)
        ws.send(msg.data)
    }
    ws.onclose = () => {
        console.log("Socket Closed!");
    }
    ws.onerror = ev => {
        console.log(ev.message)
    }
    return response;
}

console.log("Server runing on http://localhost:8080");
serve(reqHandler, { port: 8080 });



// let msg = "Hello, World!\n";

// let ws = new WebSocket("ws://localhost:8080/chat");


// ws.onclose = console.log

// ws.onopen = () => {
//     console.log("Socket Opened!");
//     ws.send(msg);
//     console.log(`Sending: ${ws.bufferedAmount} bytes`)
// }

/* 
HTTP/1.1 101 Switching Protocols
upgrade: websocket
connection: Upgrade
sec-websocket-accept: a8fPQ+P35dQ4nfjxGg2kvIAH5yk=
vary: Accept-Encoding
date: Wed, 10 Aug 2022 21:04:36 GMT

Sending Responce:

HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: a8fPQ+P35dQ4nfjxGg2kvIAH5yk=
*/