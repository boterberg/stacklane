import hmac
import json
import os
import re
import subprocess
import tempfile
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

REPOSITORY = os.environ.get("STACKLANE_REPO_DIR", "/srv/stacklane-apt")
TOKEN_FILE = os.environ.get("STACKLANE_REPO_TOKEN_FILE", "/etc/stacklane-repo/token")
LISTEN = ("127.0.0.1", int(os.environ.get("STACKLANE_REPO_PORT", "8091")))
MAX_SIZE = 200 * 1024 * 1024
VERSION_PATTERN = re.compile(r"^\d+\.\d+\.\d+\.\d+$")


def expected_token():
    with open(TOKEN_FILE, encoding="utf-8") as handle:
        return handle.read().strip()


def package_field(path, field):
    result = subprocess.run(["dpkg-deb", "--field", path, field], capture_output=True, text=True, check=True)
    return result.stdout.strip()


class Upload(BaseHTTPRequestHandler):
    server_version = "stacklane-repo"

    def reply(self, status, body):
        payload = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def do_POST(self):
        if self.path != "/apt/upload":
            return self.reply(404, {"error": "not found"})

        given = self.headers.get("Authorization", "").removeprefix("Bearer ").strip()
        if not given or not hmac.compare_digest(given, expected_token()):
            return self.reply(401, {"error": "invalid token"})

        length = int(self.headers.get("Content-Length", "0"))
        if length <= 0 or length > MAX_SIZE:
            return self.reply(413, {"error": "package missing or too large"})

        with tempfile.NamedTemporaryFile(suffix=".deb", dir=os.path.join(REPOSITORY, "incoming")) as package:
            remaining = length
            while remaining:
                chunk = self.rfile.read(min(remaining, 1024 * 1024))
                if not chunk:
                    return self.reply(400, {"error": "upload ended early"})
                package.write(chunk)
                remaining -= len(chunk)
            package.flush()

            try:
                name = package_field(package.name, "Package")
                version = package_field(package.name, "Version")
            except subprocess.CalledProcessError:
                return self.reply(400, {"error": "not a debian package"})
            if name != "stacklane" or not VERSION_PATTERN.match(version):
                return self.reply(400, {"error": f"unexpected package {name} {version}"})

            result = subprocess.run(
                ["reprepro", "--basedir", REPOSITORY, "includedeb", "stable", package.name],
                capture_output=True,
                text=True,
            )
            if result.returncode != 0:
                return self.reply(409, {"error": result.stderr.strip()[-500:]})

        self.reply(201, {"package": name, "version": version})

    def do_GET(self):
        self.reply(404, {"error": "not found"})

    def log_message(self, format, *args):
        print(f"{self.address_string()} {format % args}", flush=True)


if __name__ == "__main__":
    ThreadingHTTPServer(LISTEN, Upload).serve_forever()
