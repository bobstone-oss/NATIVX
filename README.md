# ⚡ Ecosistema NativX (v1.2)
**El Lenguaje Universal de Juegos para Navegadores, Móviles (Android/iOS) y Desktop**

NativX es un lenguaje y motor de videojuegos de alto rendimiento diseñado desde cero para ejecutarse directamente en hardware nativo y navegadores modernos a través de WebGPU, WebAssembly y abstracciones directas de sistema operativo (NDK en Android, Metal en iOS).

---

## 🌟 Características Principales

- **Compilador NativX (`nxc`)**: Transpila código fuente `.nx` tanto a código Rust optimizado para CPU como a Shaders WGSL para GPU, garantizando cero sobrecarga.
- **Arquitectura ECS Cache-Friendly**: Sistema de entidades por componentes (`SparseSet`), jerarquía espacial padre-hijo sin recursión y física de colisiones AABB.
- **Memoria Zero-GC**:
  - `FrameArena`: Asignador de memoria lineal de ciclo de fotograma (cero recolección de basura).
  - `GenerationalPool`: Acceso seguro a entidades y objetos con protección Use-After-Free de 64 bits.
  - `SpscRingBuffer`: Colas circulares lock-free entre hilos de física/red y GPU/audio.
- **Empaquetado Zero-Copy (`.nxpak`)**: Formato de contenedores binarios alineados a 64 bytes para mapeo directo con `mmap` o `SyncAccessHandle` (OPFS en navegador).
- **Audio Espacial 3D Binaural**: Mezclador multicanal a 48 kHz con paneo de potencia constante y procesador `AudioWorklet`.
- **Multijugador Autoritativo**: Paquetes de 32 bytes con sincronización nativa UDP + WebTransport (QUIC) en `nx_server`.
- **Capa HAL Multi-Plataforma**:
  - **Móvil**: Giroscopio, acelerómetro, háptica, ciclo de vida (`nx_hal_sensors`), Bluetooth Low Energy (`nx_hal_ble`) y pagos integrados (`nx_hal_store`).
  - **Android & iOS**: Configuración de `NativeActivity` en `AndroidManifest.xml` y puente `CAMetalLayer` / Objective-C en `Info.plist`.
- **Extensión VS Code**: Resaltado de sintaxis TextMate, validación en tiempo real y snippets para `.nx`.

---

## 📁 Estructura del Workspace

```text
├── Cargo.toml                    # Configuración raíz del workspace
├── crates/
│   ├── nx_core/                  # Matemáticas 3D (Vec, Mat, Quat), FrameArena, Pools y RNG
│   ├── nx_ecs/                   # Motor ECS (SparseSet, World, Jerarquía y Física AABB)
│   ├── nxc_lexer_parser/         # Compilador de NativX (.nx -> Rust + Shaders WGSL)
│   ├── nx_hal_sensors/           # Giroscopio, acelerómetro, háptica, lifecycle y puentes nativos
│   ├── nx_hal_fs/                # Almacenamiento Zero-Copy y empaquetador .nxpak
│   ├── nx_hal_audio/             # Audio espacial 3D y mezclador estéreo
│   ├── nx_hal_net/               # Transporte de red binario ultracompacto (32 bytes)
│   ├── nx_hal_ble/               # Periféricos Bluetooth Low Energy (GATT)
│   ├── nx_hal_store/             # Monetización unificada (Google Play / StoreKit / Web)
│   ├── nx_hal_ui/                # Primitivas SDF de interfaz gráfica de usuario
│   ├── nx_server/                # Servidor autoritativo cross-play (UDP + WebTransport)
│   ├── nx_cli/                   # Herramienta de línea de comandos (.nxpak, importador de activos 3D)
│   ├── nx_runtime_app/           # Runtime interactivo 3D con WGPU y Winit
│   ├── nx_android_runner/        # Runner nativo para Android (NativeActivity)
│   └── nx_ios_runner/            # Runner nativo para iOS (Metal)
├── dist/                         # Cargador WebGPU, OPFS Worker y Audio Worklet
├── nx_serve.py                   # Servidor local con encabezados COOP/COEP
├── tools/vscode-nativx/          # Extensión de Visual Studio Code para NativX
├── .github/workflows/            # Pipeline de CI/CD multiplataforma
└── src/                          # Scripts de prueba en lenguaje NativX (.nx)
```

---

## 🚀 Guía de Inicio Rápido

### 1. Compilación y Tests de Todo el Workspace
```bash
cargo test --workspace
```

### 2. Inicializar y Empaquetar Activos en `.nxpak`
```bash
cargo run -p nx_cli -- init-assets assets
cargo run -p nx_cli -- pack assets dist/game_data.nxpak
```

### 3. Ejecutar la Aplicación Interactiva 3D
```bash
cargo run -p nx_runtime_app --release
```

### 4. Servidor Web Local (WebGPU + SharedArrayBuffer)
```bash
python3 nx_serve.py 8080
```
Abre en tu navegador compatible con WebGPU: `http://localhost:8080/dist/`
