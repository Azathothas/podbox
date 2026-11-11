"""A loopback stand-in of the runner service, for experiment 428 (T-1706).

Test code. The shape is the broker-fake crate of PodUtils/podrunner, as its
COORD-003 (PodUtils/podrunner#50) gives it: POST /_apis/oauth2/token answers
a JWT; POST /session answers the session with an id; GET /message is the
long poll, 202 with no body after up to 2 s; DELETE /session answers 200.
Each value is made up.

Usage:
  python3 -I runner-standin.py serve PORT LOG
  python3 -I runner-standin.py files PORT DIR   (writes .runner,
      .credentials and .credentials_rsaparams into DIR, with a fresh RSA
      key from openssl)
"""

import base64
import http.server
import json
import re
import subprocess
import sys
import time


def b64url(raw):
    return base64.urlsafe_b64encode(raw).rstrip(b"=").decode()


def jwt():
    head = b64url(json.dumps({"alg": "RS256", "typ": "JWT"}).encode())
    claims = {"nameid": "standin", "scp": "standin", "iat": 1700000000,
              "nbf": 1700000000, "exp": 4102444800, "iss": "fake", "aud": "fake"}
    return f"{head}.{b64url(json.dumps(claims).encode())}.c2lnbmF0dXJl"


class Handler(http.server.BaseHTTPRequestHandler):
    log_path = ""

    def note(self, what):
        with open(self.log_path, "a") as f:
            f.write(f"{time.time():.3f} {what}\n")

    def reply(self, code, body=None):
        data = b"" if body is None else json.dumps(body).encode()
        self.send_response(code)
        if body is not None:
            self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def body(self):
        n = int(self.headers.get("Content-Length") or 0)
        return self.rfile.read(n) if n else b""

    def do_POST(self):
        path = self.path.split("?")[0]
        raw = self.body()
        self.note(f"POST {path}")
        if path == "/_apis/oauth2/token":
            self.reply(200, {"access_token": jwt(), "token_type": "bearer", "expires_in": 3600})
        elif path == "/session":
            s = json.loads(raw or b"{}")
            s["sessionId"] = "11111111-1111-1111-1111-111111111111"
            self.reply(200, s)
        else:
            self.reply(404, {"message": "not in the stand-in"})

    def do_GET(self):
        path = self.path.split("?")[0]
        self.note(f"GET {path}")
        if path == "/message":
            time.sleep(2)
            self.reply(202)
        else:
            self.reply(404, {"message": "not in the stand-in"})

    def do_DELETE(self):
        self.note(f"DELETE {self.path.split('?')[0]}")
        self.reply(200, {})

    def log_message(self, *args):
        pass


def serve(port, log):
    Handler.log_path = log
    http.server.ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()


def rsa_fields():
    pem = subprocess.run(["openssl", "genrsa", "-traditional", "2048"],
                         check=True, capture_output=True).stdout
    text = subprocess.run(["openssl", "rsa", "-text", "-noout"], input=pem,
                          check=True, capture_output=True).stdout.decode()
    fields, name, hexes = {}, None, []
    for line in text.splitlines():
        m = re.match(r"^(\w+):\s*(.*)$", line)
        if m:
            if name:
                fields[name] = "".join(hexes)
            name, rest = m.group(1), m.group(2)
            hexes = []
            e = re.match(r"(\d+) \(0x([0-9a-f]+)\)", rest)
            if e:
                h = e.group(2)
                fields[name] = h if len(h) % 2 == 0 else "0" + h
                name = None
        elif name:
            hexes.append(line.strip().replace(":", ""))
    if name:
        fields[name] = "".join(hexes)

    def b64(key):
        raw = bytes.fromhex(fields[key]).lstrip(b"\x00")
        return base64.b64encode(raw).decode()

    return {"d": b64("privateExponent"), "dp": b64("exponent1"), "dq": b64("exponent2"),
            "exponent": b64("publicExponent"), "inverseQ": b64("coefficient"),
            "modulus": b64("modulus"), "p": b64("prime1"), "q": b64("prime2")}


def files(port, d):
    base = f"http://127.0.0.1:{port}/"
    runner = {"agentId": 7, "agentName": "runner-u-1", "poolId": 1, "poolName": "podrunner",
              "serverUrl": base, "serverUrlV2": base, "useV2Flow": True,
              "gitHubUrl": "https://github.com/example-org", "workFolder": "_work",
              "ephemeral": True, "disableUpdate": True}
    creds = {"scheme": "OAuth", "data": {"clientId": "00000000-0000-0000-0000-000000000007",
                                         "authorizationUrl": base + "_apis/oauth2/token",
                                         "requireFipsCryptography": "False"}}
    for name, obj in [(".runner", runner), (".credentials", creds),
                      (".credentials_rsaparams", rsa_fields())]:
        with open(f"{d}/{name}", "w") as f:
            json.dump(obj, f)


if __name__ == "__main__":
    if sys.argv[1] == "serve":
        serve(int(sys.argv[2]), sys.argv[3])
    elif sys.argv[1] == "files":
        files(int(sys.argv[2]), sys.argv[3])
    else:
        sys.exit(2)
