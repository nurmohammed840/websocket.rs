// import { serve } from "https://deno.land/std@0.134.0/http/mod.ts";

// console.log("Server runing on http://localhost:8080");

// serve(req => {
//     if (req.headers.get("upgrade") != "websocket") {
//         return new Response(null, { status: 501 });
//     }
//     const { socket: ws, response } = Deno.upgradeWebSocket(req);

//     ws.onopen = () => console.log("Incoming!");
//     ws.onmessage = (msg) => {
//         console.log("Msg:", msg.data);
//         ws.send(msg.data);
//         console.log(`Sending: ${ws.bufferedAmount} bytes`)
//     }
//     ws.onclose = () => console.log("Socket Closed!");
//     ws.onerror = err => console.log(err.message);

//     return response;
// }, { port: 8080 });

// -------------------------------------------------------

const ws = new WebSocket("ws://127.0.0.1:8080");

ws.onerror = err => {
    console.log("Websocket error", err?.message)
};

ws.onopen = () => {
    console.log("Socket Opened!");
    ws.send("Hello, World!");
    console.log(`Sending: ${ws.bufferedAmount} bytes`)
}

ws.onmessage = (msg) => {
    console.log(msg?.data)
}