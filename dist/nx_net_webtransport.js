// Archivo: dist/nx_net_webtransport.js
let datagramWriter = null;
const latestIncomingDatagram = new Uint8Array(32);
let hasUnreadDatagram = false;

export async function connectNativXWebTransport(serverUrl) {
  const transport = new WebTransport(serverUrl);
  await transport.ready;

  datagramWriter = transport.datagrams.writable.getWriter();
  const reader = transport.datagrams.readable.getReader();

  // Hilo asíncrono que vuelca el último datagrama UDP recibido al bloque de 32 bytes
  (async () => {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      if (value && value.byteLength === 32) {
        latestIncomingDatagram.set(value);
        hasUnreadDatagram = true;
      }
    }
  })();
}

export function createWebTransportImports(getWasmMemory) {
  return {
    nx_webtransport_send_datagram: (ptr, len) => {
      if (!datagramWriter) return;
      const mem = new Uint8Array(getWasmMemory().buffer, ptr, len);
      // Envío UDP no-bloqueante directo al socket QUIC
      datagramWriter.write(mem.slice()).catch(() => {});
    },

    nx_webtransport_poll_datagram: (outPtr, maxLen) => {
      if (!hasUnreadDatagram || maxLen < 32) return 0;
      const target = new Uint8Array(getWasmMemory().buffer, outPtr, 32);
      target.set(latestIncomingDatagram);
      hasUnreadDatagram = false;
      return 32;
    }
  };
}
