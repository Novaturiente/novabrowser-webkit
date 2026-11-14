"""Cross-origin API for index.html: CORS allows only content-type, like data.dataforseo.com.

Run: python3 api.py  (port 8766). Logs whether each request carried Sec-GPC.
"""
from http.server import BaseHTTPRequestHandler, HTTPServer

ORIGIN = "http://localhost:8765"


class H(BaseHTTPRequestHandler):
    def cors(self):
        self.send_header("Access-Control-Allow-Origin", ORIGIN)
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "origin, content-type, accept")

    def do_OPTIONS(self):
        print("preflight", self.headers.get("Access-Control-Request-Headers"), flush=True)
        self.send_response(204)
        self.cors()
        self.end_headers()

    def do_GET(self):
        print("GET sec-gpc=", self.headers.get("Sec-GPC"), flush=True)
        self.send_response(200)
        self.cors()
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(b"{}")

    def log_message(self, *a):
        pass


HTTPServer(("127.0.0.1", 8766), H).serve_forever()
