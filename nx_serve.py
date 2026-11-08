from http.server import HTTPServer, SimpleHTTPRequestHandler
import sys

class NativXHandler(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
    print(f"🚀 Servidor WebGPU/SharedArrayBuffer activo en http://localhost:{port}")
    HTTPServer(("0.0.0.0", port), NativXHandler).serve_forever()
