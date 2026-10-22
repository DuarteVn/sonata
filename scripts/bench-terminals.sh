#!/bin/sh
# Opens Sonata with N terminals that print continuously, spread over W workspaces, in a
# throwaway data folder. Used to check SC-003 (30 terminals) and SC-004 (100 terminals).
# Usage: scripts/bench-terminals.sh N [W]
set -eu
n=${1:?usage: scripts/bench-terminals.sh N [workspaces]}
w=${2:-1}
cd "$(dirname "$0")/.."
home=$(mktemp -d)/sonata
project=$(mktemp -d)
python3 - "$home" "$project" "$n" "$w" <<'PY'
import json, os, sys, uuid, datetime
home, project, n, w = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
command = 'while :; do printf "%s \\033[3%dmline %05d\\033[0m %s\\n" "$(date +%T)" $((RANDOM % 7 + 1)) $RANDOM "lorem ipsum dolor sit amet"; sleep 0.05; done'
order = []
for k in range(w):
    ws_id = str(uuid.uuid4())
    order.append(ws_id)
    count = n // w + (1 if k < n % w else 0)
    nodes = []
    for i in range(count):
        nodes.append({
            "id": str(uuid.uuid4()),
            "frame": {"x": (i % 6) * 680, "y": (i // 6) * 440, "width": 640, "height": 400},
            "z": i + 1, "createdAt": now, "modifiedAt": now,
            "content": {"terminal": {"name": f"Bench {k + 1}.{i + 1}", "launchCommand": command, "saveHistory": False}},
        })
    payload = {"id": ws_id, "name": f"Bench {k + 1}", "icon": "zap", "color": "#3B82F6",
               "workingDirectory": project, "createdAt": now, "lastOpenedAt": now,
               "camera": {"x": 0, "y": 0, "zoom": 0.5}, "nodes": nodes}
    d = os.path.join(home, "workspaces", ws_id)
    os.makedirs(os.path.join(d, "notes"), mode=0o700, exist_ok=True)
    with open(os.path.join(d, "workspace.json"), "w") as f:
        json.dump({"type": "workspace", "schemaVersion": 1, "payload": payload}, f, indent=2, sort_keys=True)
state = {"activeWorkspaceId": order[0], "recentWorkspaceIds": order, "workspaceOrder": order,
         "cleanShutdown": True, "outlivedLaunchWindow": True, "consecutiveEarlyCrashes": 0}
with open(os.path.join(home, "app-state.json"), "w") as f:
    json.dump({"type": "appState", "schemaVersion": 1, "payload": state}, f, indent=2, sort_keys=True)
PY
echo "Data folder: $home ($n terminals in $w workspace(s))"
SONATA_HOME="$home" exec npm run tauri dev
