// @ts-nocheck

import { serve } from "https://deno.land/std/http/mod.ts";

async function reqHandler(req: Request) {
    if (req.headers.get("upgrade") != "websocket") {
        return new Response(null, { status: 501 });
    }
    console.log("req headers:", req.headers)
    const { socket: _ws, response } = Deno.upgradeWebSocket(req);
    console.log("response:", response);
    return response;
}

console.log("Server runing on http://localhost:8000");
serve(reqHandler, { port: 8000 });