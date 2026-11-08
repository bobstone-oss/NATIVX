// Archivo: dist/nx_opfs_worker.js
// Mantiene los archivos .nxsave y .nxmesh abiertos en modo acceso directo de hardware (SyncAccessHandle)

const openHandles = new Map();
let opfsRoot = null;

export async function initOpfsCache(preloadFiles = ["progreso.bin"]) {
  if (!navigator.storage?.getDirectory) return;
  opfsRoot = await navigator.storage.getDirectory();

  for (const name of preloadFiles) {
    const fileHandle = await opfsRoot.getFileHandle(name, { create: true });
    // En un WebWorker dedicado, createSyncAccessHandle opera a velocidad de disco nativo (POSIX)
    if (fileHandle.createSyncAccessHandle) {
      const syncHandle = await fileHandle.createSyncAccessHandle();
      openHandles.set(name, syncHandle);
    }
  }
}

export function createOpfsImports(getWasmMemory) {
  const decoder = new TextDecoder();

  return {
    // Escritura binaria directa desde la dirección de memoria de Rust hacia el SSD del dispositivo
    nx_opfs_write_sync: (namePtr, nameLen, dataPtr, dataLen) => {
      const mem = new Uint8Array(getWasmMemory().buffer);
      const fileName = decoder.decode(mem.subarray(namePtr, namePtr + nameLen));
      const payload = mem.subarray(dataPtr, dataPtr + dataLen);

      const handle = openHandles.get(fileName);
      if (handle) {
        handle.truncate(0);
        handle.write(payload, { at: 0 });
        handle.flush();
        return 0; // Éxito instantáneo (0ms bloqueo)
      }
      return -1;
    },

    // Lectura binaria directa desde el SSD hacia el struct en la memoria de Rust
    nx_opfs_read_sync: (namePtr, nameLen, outPtr, maxLen) => {
      const mem = new Uint8Array(getWasmMemory().buffer);
      const fileName = decoder.decode(mem.subarray(namePtr, namePtr + nameLen));
      const targetSlice = mem.subarray(outPtr, outPtr + maxLen);

      const handle = openHandles.get(fileName);
      if (handle) {
        return handle.read(targetSlice, { at: 0 });
      }
      return -1;
    }
  };
}
