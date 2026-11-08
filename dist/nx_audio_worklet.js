// Archivo: dist/nx_audio_worklet.js
class NativXSpatialProcessor extends AudioWorkletProcessor {
  constructor(options) {
    super();
    // Recibe la vista compartida (SharedArrayBuffer) directamente desde el binario .wasm
    this.sharedFloatRing = new Float32Array(options.processorOptions.sharedBuffer);
    this.readCursor = 0;
  }

  process(_inputs, outputs) {
    const output = outputs[0];
    const left = output[0];
    const right = output[1];
    const frames = left.length; // 128 muestras por bloque (2.66 ms @ 48kHz)

    for (let i = 0; i < frames; i++) {
      const idx = (this.readCursor + i * 2) % this.sharedFloatRing.length;
      left[i] = this.sharedFloatRing[idx];
      right[i] = this.sharedFloatRing[idx + 1];
    }
    this.readCursor = (this.readCursor + frames * 2) % this.sharedFloatRing.length;
    return true; // Mantiene el motor de hardware activo permanentemente
  }
}

registerProcessor("nativx-spatial-processor", NativXSpatialProcessor);
