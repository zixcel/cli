#!/usr/bin/env python3
"""Synthetic acceptance on an isolated DBus and persistent GNOME keyring."""
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import subprocess
import sys
import tempfile
import time

SECRET = b"synthetic-zix-keyring-secret-never-print"


def main():
    zix, probe = map(lambda p: str(Path(p).resolve()), sys.argv[1:3])
    if os.environ.get("ZIX_ISOLATED_KEYRING") != "1":
        with tempfile.TemporaryDirectory(prefix="zix-keyring-") as temporary:
            root = Path(temporary)
            root.chmod(0o700)
            env = {k: v for k, v in os.environ.items()
                   if k not in ("DBUS_SESSION_BUS_ADDRESS", "GNOME_KEYRING_CONTROL", "GITHUB_ACTIONS")
                   and not k.startswith("ZIX_")}
            for key, name in (("XDG_DATA_HOME", "data"), ("XDG_CONFIG_HOME", "config"),
                              ("XDG_RUNTIME_DIR", "runtime")):
                directory = root / name
                directory.mkdir(mode=0o700)
                env[key] = str(directory)
            env["ZIX_ISOLATED_KEYRING"] = "1"
            subprocess.run(["dbus-run-session", "--", sys.executable, __file__, zix, probe],
                           env=env, check=True, timeout=180)
        return
    subprocess.run(["test", "-d", os.environ["XDG_RUNTIME_DIR"]], check=True)
    daemon = None
    log = open(Path(os.environ["XDG_RUNTIME_DIR"]) / "keyring.log", "wb")

    def dbus(path, method, *args):
        return subprocess.run(["gdbus", "call", "--session", "--dest", "org.freedesktop.secrets",
                               "--object-path", path, "--method", method, *args],
                              capture_output=True, timeout=5)

    def start():
        nonlocal daemon
        daemon = subprocess.Popen(["gnome-keyring-daemon", "--foreground", "--unlock",
                                   "--components=secrets", "--control-directory=" +
                                   os.environ["XDG_RUNTIME_DIR"] + "/keyring"],
                                  stdin=subprocess.PIPE, stdout=log, stderr=log)
        daemon.stdin.write(b"synthetic-acceptance-keyring-password")
        daemon.stdin.close()
        control = Path(os.environ["XDG_RUNTIME_DIR"]) / "keyring/control"
        for _ in range(100):
            if control.is_socket():
                break
            time.sleep(0.1)
        else:
            raise AssertionError("isolated provider control socket unavailable")
        for _ in range(100):
            ready = dbus("/org/freedesktop/secrets/collection/login",
                         "org.freedesktop.DBus.Properties.Get",
                         "org.freedesktop.Secret.Collection", "Locked")
            if ready.returncode == 0 and b"false" in ready.stdout:
                alias = dbus("/org/freedesktop/secrets", "org.freedesktop.Secret.Service.SetAlias",
                             "default", "/org/freedesktop/secrets/collection/login")
                assert alias.returncode == 0, alias.stderr
                return
            time.sleep(0.1)
        raise AssertionError("isolated provider unavailable")

    def run(args, success=True, env=None):
        result = subprocess.run([zix, *args], capture_output=True, timeout=50, env=env)
        assert SECRET not in result.stdout + result.stderr
        assert (result.returncode == 0) == success, result.stderr
        return result

    def terminal(args, steps=(), success=True):
        master, slave = pty.openpty()
        child = subprocess.Popen([zix, *args], stdin=slave, stderr=slave, stdout=subprocess.PIPE)
        os.close(slave)
        pending, output = list(steps), b""
        deadline = time.monotonic() + 50
        try:
            while child.poll() is None:
                assert time.monotonic() < deadline, "terminal operation timed out"
                if select.select([master], [], [], 0.1)[0]:
                    try:
                        chunk = os.read(master, 4096)
                    except OSError:
                        break
                    output += chunk
                    if pending and pending[0][0] in output:
                        _, answer = pending.pop(0)
                        os.write(master, answer)
            stdout, _ = child.communicate(timeout=5)
            assert SECRET not in output + stdout
            assert not pending
            assert (child.returncode == 0) == success, output
            return stdout
        finally:
            if child.poll() is None:
                child.kill()
                child.wait()
            os.close(master)

    collection = ["--collection", "default"]
    ref = [*collection, "--reference", "local/provider/api/sample/main"]
    try:
        start()
        assert json.loads(run(["credential", "status", *collection]).stdout)["result"]["locked"] is False
        # Use only a synthetic registered provider, never a real network API.
        root = Path(os.environ["XDG_CONFIG_HOME"])
        manifest = {"schema": "zix-extension/v1", "package": "sample-provider", "version": "0.1.0",
                    "namespace": "sample", "platform": "linux-x86_64", "protocol": "zix-provider/v1",
                    "timeout_ms": 1000, "credential_environment": ["ZIX_TEST_ACCESS_TOKEN"],
                    "commands": [{"id": "sample.inspect", "words": ["inspect"], "effect": "read",
                                  "description": "Synthetic inspection", "required_permissions": [],
                                  "arguments": [{"name": "mode", "required": True, "maximum": 32}]}]}
        path = root / "sample.json"
        path.write_text(json.dumps(manifest))
        path.chmod(0o600)
        digest = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
        run(["extensions", "add", "--manifest", str(path), "--manifest-sha256", digest(path),
             "--executable", probe, "--executable-sha256", digest(probe)])
        terminal(["credential", "set", *ref, "--host", "api.example.com"],
                 [(b"Type yes", b"yes\n"), (b"Secret (hidden", SECRET + b"\n")])
        before = json.loads(run(["credential", "inspect", *ref]).stdout)
        terminal(["credential", "set", *ref, "--host", "api.example.com"],
                 [(b"Type yes", b"yes\n"), (b"Secret (hidden", b"\x03")], success=False)
        assert json.loads(run(["credential", "inspect", *ref]).stdout) == before
        terminal(["credential", "set", *ref, "--host", "api.example.com"],
                 [(b"Type yes", b"yes\n"), (b"Secret (hidden", SECRET + b"\n")])
        rotated = json.loads(run(["credential", "inspect", *ref]).stdout)
        assert rotated["result"]["revision"] != before["result"]["revision"]
        before = rotated
        use = ["credential", "run", *ref, "--host", "api.example.com", "--environment", "ZIX_TEST_ACCESS_TOKEN",
               "--manifest-sha256", digest(path), "--executable-sha256", digest(probe),
               "--", "sample", "inspect", "--mode", "normal"]
        assert json.loads(run(use).stdout)["result"]["selected_environment"] is True
        assert json.loads(run(["sample", "inspect", "--mode", "normal"]).stdout)["result"]["selected_environment"] is False
        leak = use[:-1] + ["secret"]
        assert b"provider_secret_output" in run(leak, False).stderr
        wrong_host = use.copy()
        wrong_host[wrong_host.index("api.example.com")] = "other.example.com"
        run(wrong_host, False)
        # New process and provider restart must preserve the credential revision.
        daemon.terminate()
        daemon.wait(timeout=5)
        start()
        assert json.loads(run(["credential", "inspect", *ref]).stdout) == before
        run(use)
        terminal(["credential", "lock", *collection])
        assert json.loads(run(["credential", "status", *collection]).stdout)["result"]["locked"] is True
        assert b"credential_store_locked_or_denied" in run(use, False).stderr
        # The daemon owns unlocking; a restart uses only the public test password.
        daemon.terminate()
        daemon.wait(timeout=5)
        start()
        terminal(["credential", "remove", *ref], [(b"Type yes", b"yes\n")])
        assert b"credential_not_found" in run(use, False).stderr
        absent = dict(os.environ, DBUS_SESSION_BUS_ADDRESS="unix:path=" + os.environ["XDG_RUNTIME_DIR"] + "/absent")
        assert b"credential_store_unavailable" in run(["credential", "status", *collection], False, absent).stderr
        print("PASS: isolated enrollment, bound use, redaction, restart, lock, deletion, unavailable provider")
    finally:
        if daemon and daemon.poll() is None:
            daemon.terminate()
            daemon.wait(timeout=5)
        log.close()


if __name__ == "__main__":
    main()
