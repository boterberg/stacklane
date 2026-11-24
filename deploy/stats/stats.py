#!/usr/bin/env python3
import glob
import gzip
import hashlib
import hmac
import json
import os
import re
import secrets
import time
from datetime import datetime, timedelta, timezone

LOGS = os.environ.get("STACKLANE_STATS_LOGS", "/var/log/caddy/stacklane-apt*.log*")
STATE = os.environ.get("STACKLANE_STATS_STATE", "/var/lib/stacklane-stats/state.json")
OUTPUT = os.environ.get("STACKLANE_STATS_OUTPUT", "/srv/stacklane-stats/stats.json")
IGNORED = set(filter(None, os.environ.get("STACKLANE_STATS_IGNORE", "127.0.0.1 ::1").split()))
KEEP_DAYS = 2
HISTORY_DAYS = 30

DEB = re.compile(r"^/apt/pool/.+/stacklane_([0-9.]+)_[a-z0-9]+\.deb$")
INDEX = re.compile(r"^/apt/dists/stable/main/binary-[a-z0-9]+/Packages$")
PANEL = re.compile(r"^stacklane/([0-9.]+)")


def load_state():
    try:
        with open(STATE) as handle:
            return json.load(handle)
    except FileNotFoundError:
        return {"last_ts": 0, "since": None, "downloads": {}, "days": {}, "salts": {}}


def entries():
    for path in sorted(glob.glob(LOGS)):
        opener = gzip.open if path.endswith(".gz") else open
        with opener(path, "rt", errors="replace") as handle:
            for line in handle:
                try:
                    yield json.loads(line)
                except ValueError:
                    continue


def count(values):
    result = {}
    for value in values:
        result[value] = result.get(value, 0) + 1
    return result


def day_of(ts):
    return datetime.fromtimestamp(ts, timezone.utc).strftime("%Y-%m-%d")


def main():
    state = load_state()
    last_ts = state["last_ts"]
    newest = last_ts
    for entry in entries():
        ts = entry.get("ts", 0)
        if ts <= last_ts:
            continue
        newest = max(newest, ts)
        request = entry.get("request", {})
        if request.get("method") != "GET" or entry.get("status") != 200:
            continue
        uri = request.get("uri", "").split("?", 1)[0]
        ip = request.get("client_ip") or request.get("remote_ip") or ""
        if ip in IGNORED:
            continue
        day = day_of(ts)
        state["since"] = state["since"] or day
        found = DEB.match(uri)
        if found:
            version = found.group(1)
            state["downloads"][version] = state["downloads"].get(version, 0) + 1
            continue
        agent = (request.get("headers", {}).get("User-Agent") or [""])[0]
        panel = PANEL.match(agent)
        if INDEX.match(uri) and panel:
            salt = state["salts"].setdefault(day, secrets.token_hex(32))
            key = hmac.new(bytes.fromhex(salt), ip.encode(), hashlib.sha256).hexdigest()[:24]
            state["days"].setdefault(day, {})[key] = panel.group(1)
    state["last_ts"] = newest

    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    for day, keys in state["days"].items():
        if day < today:
            state.setdefault("history", {})[day] = {"panels": len(keys), "versions": count(keys.values())}
    cutoff = (datetime.now(timezone.utc) - timedelta(days=KEEP_DAYS)).strftime("%Y-%m-%d")
    state["days"] = {day: keys for day, keys in state["days"].items() if day > cutoff}
    state["salts"] = {day: salt for day, salt in state["salts"].items() if day > cutoff}
    oldest = (datetime.now(timezone.utc) - timedelta(days=HISTORY_DAYS)).strftime("%Y-%m-%d")
    state["history"] = {day: value for day, value in state.get("history", {}).items() if day > oldest}

    finished = sorted(state["history"].items())
    if finished:
        latest_day, latest = finished[-1]
    else:
        latest_day = today
        keys = state["days"].get(today, {})
        latest = {"panels": len(keys), "versions": count(keys.values())}

    os.makedirs(os.path.dirname(STATE), exist_ok=True)
    temporary = STATE + ".tmp"
    with open(temporary, "w") as handle:
        json.dump(state, handle)
    os.replace(temporary, STATE)

    public = {
        "updated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "since": state["since"],
        "downloads": sum(state["downloads"].values()),
        "downloads_by_version": dict(sorted(state["downloads"].items())),
        "active_panels": latest["panels"],
        "active_panels_day": latest_day,
        "panels_by_version": dict(sorted(latest["versions"].items())),
        "panels_per_day": {day: value["panels"] for day, value in finished},
    }
    os.makedirs(os.path.dirname(OUTPUT), exist_ok=True)
    temporary = OUTPUT + ".tmp"
    with open(temporary, "w") as handle:
        json.dump(public, handle)
    os.chmod(temporary, 0o644)
    os.replace(temporary, OUTPUT)


if __name__ == "__main__":
    main()
