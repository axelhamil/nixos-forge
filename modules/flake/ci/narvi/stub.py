import http.server
import json
import sys
import time

log_path, port_path, mode_path = sys.argv[1:4]


def mode():
    with open(mode_path) as source:
        return source.read().strip()


class Stub(http.server.BaseHTTPRequestHandler):
    def record(self, body):
        try:
            body = json.loads(body)
        except ValueError:
            pass
        with open(log_path, "a") as log:
            log.write(json.dumps({
                "method": self.command,
                "path": self.path,
                "authorization": self.headers.get("authorization", ""),
                "x-api-key": self.headers.get("x-api-key", ""),
                "anthropic-version": self.headers.get("anthropic-version", ""),
                "body": body,
            }) + "\n")

    def answer(self, status, payload):
        data = payload if isinstance(payload, bytes) else json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        self.record("")
        if self.path.startswith("/oidc?") and self.headers.get("authorization") == "Bearer gh-request-token":
            self.answer(200, {"value": "jwt-from-github"})
        else:
            self.answer(403, {"message": "no"})

    def do_POST(self):
        length = int(self.headers.get("content-length", "0"))
        self.record(self.rfile.read(length).decode())
        current = mode()
        if self.path == "/v1/oauth/token":
            if current == "slow-exchange":
                time.sleep(3)
            if current == "exchange-fails":
                self.answer(401, {"type": "error", "error": {"type": "authentication_error", "message": "Authentication failed"}})
            else:
                self.answer(200, {"access_token": "sk-ant-oat01-stub", "expires_in": 600, "token_type": "Bearer"})
        elif self.path == "/v1/messages":
            if current == "error":
                self.answer(500, {"type": "error", "error": {"type": "api_error", "message": "boom"}})
            elif current in ("slow", "slow-exchange"):
                time.sleep(10)
                self.answer(200, {"content": [{"type": "text", "text": "too late"}]})
            elif current == "malformed":
                self.answer(200, b"<html>not json</html>")
            elif current == "hostile":
                self.answer(200, {"content": [{"type": "text", "text": "x <!-- @user [a](http://e) ![i](http://e)\n\nhttps://evil.example | # done"}]})
            elif current == "empty":
                self.answer(200, {"content": []})
            else:
                self.answer(200, {"content": [{"type": "text", "text": "The kernel moves to 6.18 \u2014 Firefox jumps a major version."}]})
        else:
            self.answer(404, {"message": "unknown path"})

    def log_message(self, *args):
        pass


server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Stub)
with open(port_path, "w") as port:
    port.write(str(server.server_port))
server.serve_forever()
