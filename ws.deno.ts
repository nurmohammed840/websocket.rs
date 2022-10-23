// function connect() {
//     const ws = new WebSocket("ws://127.0.0.1:8080");

//     ws.onerror = err => {
//         console.log("Websocket error", (err as any)?.message)
//     };

//     ws.onopen = () => {
//         console.log("Socket Opened!");
//         ws.send("Hello, World!");
//         console.log(`Sending: ${ws.bufferedAmount} bytes`)

//         setTimeout(() => {
//             ws.send("HelloWorld 3");
//             console.log(`Sending: ${ws.bufferedAmount} bytes`)
//         }, 2000)

//     }

//     ws.onmessage = (msg) => {
//         console.log(msg?.data)
//     }
// }

import { serve } from "https://deno.land/std@0.134.0/http/mod.ts";

function reqHandler(req: Request) {
    if (req.headers.get("upgrade") != "websocket") {
        return new Response(null, { status: 501 });
    }
    const { socket: ws, response } = Deno.upgradeWebSocket(req);

    ws.onopen = () => {
        console.log("Socket Opened!");
        ws.send("Hi")
    }
    ws.onmessage = (msg) => {
        console.log("Msg:", msg.data)
        ws.send(msg.data)
    }
    ws.onclose = () => {
        console.log("Socket Closed!");
    }
    ws.onerror = err => {
        console.log((err as any)?.message)
    }
    return response;
}

console.log("Server runing on http://localhost:8080");
serve(reqHandler, { port: 8080 });


