// Archivo: tools/vscode-nativx/extension.js
const vscode = require("vscode");
const { exec } = require("child_process");

const NATIVX_API_DOCS = {
  temp: "**Memoria Efímera (`FrameArena`)**\n\nAloja la variable en un bloque lineal de CPU que se libera automáticamente en **0.0001 ms** al finalizar el fotograma actual, sin usar Garbage Collector.",
  "@gpu_shared": "**Estructura Compartida CPU ↔ GPU (STD140)**\n\nEl compilador `nxc` alinea automáticamente los campos `vec3` a múltiplos de 16 bytes e inyecta padding invisible para transferencias Zero-Copy hacia Vulkan, Metal y WebGPU.",
  "@system": "**Sistema ECS de Alto Rendimiento (`nx_ecs`)**\n\nRecibe automáticamente la `FrameArena` del fotograma actual y opera sobre los `SparseSet` en caché L1.",
  "gyroscope": "`device.imu.gyroscope() -> vec3`\n\nLee las velocidades angulares $(X, Y, Z)$ directamente desde `ASensorEventQueue` (Android NDK a 125Hz), `CMMotionManager` (iOS) o `SharedArrayBuffer` (Web).",
  "impact": "`device.haptics.impact(intensidad: f32, duracion_ms: u32)`\n\nDispara un pulso háptico físico en el Taptic Engine (iOS), NDK Vibrator (Android) o motores DualSense/Xbox (Web).",
  "query_mut": "`world.query_mut(CompA, CompB)`\n\nItera simultáneamente con acceso mutable `(&mut CompA, &mut CompB)` verificando en tiempo de compilación que ambos tipos sean disjuntos."
};

const MODULE_COMPLETIONS = {
  device: [
    { label: "imu.gyroscope()", insert: "imu.gyroscope()", detail: "-> vec3 (125Hz Lock-Free IMU)" },
    { label: "haptics.impact", insert: "haptics.impact(intensidad: ${1:0.9}, duracion_ms: ${2:40})", detail: "Pulso háptico nativo" },
    { label: "input.stick_left()", insert: "input.stick_left()", detail: "-> vec2 (Touch / Gamepad)" }
  ],
  world: [
    { label: "spawn_spatial", insert: "spawn_spatial(LocalTransform.from_xyz(${1:0.0}, ${2:0.0}, ${3:0.0}))", detail: "-> EntityId (Crea entidad 3D)" },
    { label: "insert", insert: "insert(${1:entidad}, ${2:Componente})", detail: "Añade componente en O(1)" },
    { label: "set_parent", insert: "set_parent(${1:hijo}, ${2:padre})", detail: "Enlaza jerarquía 3D Padre -> Hijo" },
    { label: "query_mut", insert: "query_mut(${1:LocalTransform}, ${2:RigidBody3D})", detail: "Consulta mutable de 2 componentes" },
    { label: "step_physics", insert: "step_physics(vec3(0.0, -9.81, 0.0), dt)", detail: "Integración SIMD + Colisiones AABB" },
    { label: "propagate_transforms", insert: "propagate_transforms()", detail: "Actualiza GlobalTransform (Mat4) de padres a hijos" }
  ],
  fs: [
    { label: "write_struct", insert: "write_struct(\"${1:progreso.bin}\", ${2:estado})", detail: "Guardado instantáneo mmap / OPFS" },
    { label: "load_mesh", insert: "load_mesh(\"${1:assets/nave.nxmesh}\")", detail: "Carga Zero-Copy desde contenedor .nxpak" }
  ],
  gpu: [
    { label: "init", insert: "init(fps_objetivo: 120, vsync: true)", detail: "Inicializa Vulkan / Metal / WebGPU" },
    { label: "on_frame", insert: "on_frame(fn(dt: f32) {\n\t$0\n})", detail: "Bucle principal sincronizado a 120Hz" }
  ]
};

function activate(context) {
  const diagnosticCollection = vscode.languages.createDiagnosticCollection("nativx");
  context.subscriptions.push(diagnosticCollection);

  // 1. Proveedor de Autocompletado Contextual (.)
  const completionProvider = vscode.languages.registerCompletionItemProvider(
    "nativx",
    {
      provideCompletionItems(document, position) {
        const linePrefix = document.lineAt(position).text.slice(0, position.character);

        for (const [modName, items] of Object.entries(MODULE_COMPLETIONS)) {
          if (linePrefix.endsWith(`${modName}.`)) {
            return items.map((item) => {
              const ci = new vscode.CompletionItem(item.label, vscode.CompletionItemKind.Method);
              ci.insertText = new vscode.SnippetString(item.insert);
              ci.detail = item.detail;
              return ci;
            });
          }
        }
        return undefined;
      }
    },
    "."
  );

  // 2. Proveedor de Documentación Hover
  const hoverProvider = vscode.languages.registerHoverProvider("nativx", {
    provideHover(document, position) {
      const range = document.getWordRangeAtPosition(position, /@?[a-zA-Z_][a-zA-Z0-9_]*/);
      if (!range) return null;
      const word = document.getText(range);
      if (NATIVX_API_DOCS[word]) {
        return new vscode.Hover(new vscode.MarkdownString(NATIVX_API_DOCS[word]));
      }
      return null;
    }
  });

  // 3. Linter Estático Instantáneo en el Editor (Reglas de Hardware GPU vs CPU)
  const validateDocument = (doc) => {
    if (doc.languageId !== "nativx") return;
    const diagnostics = [];
    const text = doc.getText();
    const lines = text.split(/\r?\n/);

    let insideGpuShader = false;
    let braceDepth = 0;

    lines.forEach((line, i) => {
      if (/@gpu_(vertex|fragment|compute)/.test(line)) {
        insideGpuShader = true;
      }
      if (insideGpuShader) {
        braceDepth += (line.match(/\{/g) || []).length;
        braceDepth -= (line.match(/\}/g) || []).length;

        if (/\btemp\b/.test(line)) {
          diagnostics.push(
            new vscode.Diagnostic(
              new vscode.Range(i, 0, i, line.length),
              "Error Semántico NativX: `temp` aloja en la FrameArena de CPU y no puede usarse dentro de un Shader de GPU.",
              vscode.DiagnosticSeverity.Error
            )
          );
        }
        if (/\b(device|fs|world)\./.test(line)) {
          diagnostics.push(
            new vscode.Diagnostic(
              new vscode.Range(i, 0, i, line.length),
              "Violación de Hardware NativX: No se puede acceder a sensores, disco o ECS (`device`, `fs`, `world`) desde un Shader de GPU.",
              vscode.DiagnosticSeverity.Error
            )
          );
        }
        if (braceDepth <= 0 && line.includes("}")) {
          insideGpuShader = false;
          braceDepth = 0;
        }
      }
    });

    diagnosticCollection.set(doc.uri, diagnostics);
  };

  vscode.workspace.onDidChangeTextDocument((e) => validateDocument(e.document), null, context.subscriptions);
  vscode.workspace.onDidOpenTextDocument(validateDocument, null, context.subscriptions);

  // 4. Comandos de Compilación Directa desde la Paleta de VS Code (Ctrl+Shift+P)
  const registerCliTask = (cmdId, cliArgs) => {
    return vscode.commands.registerCommand(cmdId, () => {
      const terminal = vscode.window.activeTerminal || vscode.window.createTerminal("NativX Compiler");
      terminal.show();
      terminal.sendText(`cargo run -p nx_cli -- ${cliArgs}`);
    });
  };

  context.subscriptions.push(
    completionProvider,
    hoverProvider,
    registerCliTask("nativx.buildWeb", "build --target web"),
    registerCliTask("nativx.buildAndroid", "build --target android"),
    registerCliTask("nativx.packAssets", "pack assets dist/game_data.nxpak")
  );
}

function deactivate() {}

module.exports = { activate, deactivate };
