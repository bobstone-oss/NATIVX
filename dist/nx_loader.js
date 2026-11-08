// Archivo: dist/nx_loader.js (Cargador NativX WebGPU + SharedArrayBuffer Zero-Copy)
const canvas = document.getElementById("nx-surface");
const statusEl = document.getElementById("nx-status");
const startBtn = document.getElementById("nx-start");

// Tabla deOffsets en el Bloque de Memoria Compartida (64 bytes = 16 floats de 32 bits)
// [0..2]: Giroscopio (X,Y,Z) | [3..5]: Acelerómetro (X,Y,Z)
// [6..7]: Stick Izquierdo / Touch (X,Y) | [8..9]: Resolución Física (W,H) | [10]: DeltaTime
const SENSOR_FLOATS = 16;

async function bootNativX() {
  // 1. Verificación de Hardware WebGPU y Memoria Compartida (COOP/COEP)
  if (!navigator.gpu) {
    statusEl.textContent = "Error: Este navegador no soporta hardware WebGPU.";
    return;
  }
  if (!window.crossOriginIsolated) {
    console.warn("Aviso: SharedArrayBuffer requiere cabeceras COOP/COEP activas.");
  }

  // 2. Ajuste exacto a los píxeles físicos del monitor o teléfono (120Hz High-DPI)
  const dpr = window.devicePixelRatio || 1;
  const resizeSurface = () => {
    canvas.width = Math.floor(window.innerWidth * dpr);
    canvas.height = Math.floor(window.innerHeight * dpr);
  };
  window.addEventListener("resize", resizeSurface, { passive: true });
  resizeSurface();

  // 3. Sistema de Archivos Privado de Alta Velocidad (OPFS) para `fs.write_struct`
  const opfsRoot = await navigator.storage?.getDirectory?.().catch(() => null);
  let wasmMemory = null;

  // 4. Tabla de llamadas directas del Sistema Operativo / Navegador (Imports de NativX)
  const nxImports = {
    env: {
      // Háptica: Combina vibración del teléfono y motores DualSense/Xbox en navegador
      nx_haptic_pulse: (intensity, durationMs) => {
        if (navigator.vibrate) navigator.vibrate(durationMs);
        const pad = navigator.getGamepads?.()[0];
        pad?.vibrationActuator?.playEffect("dual-rumble", {
          duration: durationMs,
          strongMagnitude: intensity,
          weakMagnitude: intensity * 0.7,
        });
      },

      // Guardado Binario Directo desde un puntero de RAM del .wasm hacia el disco OPFS
      nx_fs_write_fast: async (namePtr, nameLen, dataPtr, dataLen) => {
        if (!opfsRoot || !wasmMemory) return;
        const memBytes = new Uint8Array(wasmMemory.buffer);
        const fileName = new TextDecoder().decode(memBytes.subarray(namePtr, namePtr + nameLen));
        const payload = memBytes.slice(dataPtr, dataPtr + dataLen);
        const fileHandle = await opfsRoot.getFileHandle(fileName, { create: true });
        const writable = await fileHandle.createWritable();
        await writable.write(payload);
        await writable.close();
      }
    }
  };

  // 5. Compilación en Streaming mientras se descargan los bytes del archivo .wasm
  statusEl.textContent = "Compilando binario NativX (.wasm) en streaming...";
  const { instance } = await WebAssembly.instantiateStreaming(
    fetch("./nativx_app.wasm"),
    nxImports
  );

  const exports = instance.exports;
  wasmMemory = exports.memory;

  // 6. Enlace Zero-Copy: Apuntamos un Float32Array directamente al puntero de sensores en Wasm
  const sensorPtr = exports.nx_get_sensor_buffer_ptr();
  const getSensorView = () => new Float32Array(wasmMemory.buffer, sensorPtr, SENSOR_FLOATS);

  // 7. Captura directa de Giroscopio, Touch y Gamepad sin intermediarios en el bucle
  const bindHardwareSensors = () => {
    window.addEventListener("deviceorientation", (e) => {
      const view = getSensorView();
      view[0] = (e.beta || 0) * 0.0174533;  // Pitch en radianes
      view[1] = (e.gamma || 0) * 0.0174533; // Roll en radianes
      view[2] = (e.alpha || 0) * 0.0174533; // Yaw en radianes
    }, { passive: true });

    window.addEventListener("pointermove", (e) => {
      const view = getSensorView();
      view[6] = (e.clientX / window.innerWidth) * 2.0 - 1.0;  // Normalizado -1.0 a 1.0
      view[7] = -((e.clientY / window.innerHeight) * 2.0 - 1.0);
    }, { passive: true });
  };

  // 8. Desbloqueo de permisos nativos (Requerido en iOS para Giroscopio y Pantalla Completa)
  startBtn.hidden = false;
  statusEl.textContent = "Binario nativo listo.";

  startBtn.onclick = async () => {
    if (typeof DeviceOrientationEvent?.requestPermission === "function") {
      await DeviceOrientationEvent.requestPermission().catch(() => {});
    }
    document.documentElement.requestFullscreen?.().catch(() => {});
    document.getElementById("nx-boot").remove();
    canvas.focus();

    bindHardwareSensors();
    exports.nx_init(canvas.width, canvas.height);

    // 9. Bucle de Fotograma a 120Hz sincronizado con el refresco del monitor
    let lastTime = performance.now();
    const frameLoop = (now) => {
      const dt = Math.min((now - lastTime) * 0.001, 0.1);
      lastTime = now;

      const view = getSensorView();
      const pad = navigator.getGamepads?.()[0];
      if (pad) {
        view[6] = pad.axes[0] || view[6];
        view[7] = -(pad.axes[1] || -view[7]);
      }
      view[8] = canvas.width;
      view[9] = canvas.height;
      view[10] = dt;

      // Ejecuta la física y el renderizado WebGPU en el binario .wasm
      exports.nx_frame(dt);
      requestAnimationFrame(frameLoop);
    };
    requestAnimationFrame(frameLoop);
  };
}

bootNativX();
