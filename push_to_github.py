import subprocess, json, base64, sys, os

repo = "E7G/wmpf-debugger-rust-v2"

def gh_api(endpoint, method="POST", data=None):
    cmd = ["gh", "api", f"repos/{repo}/{endpoint}", "-X", method]
    if data is not None:
        cmd.extend(["--input", "-"])
    r = subprocess.run(cmd, input=json.dumps(data) if data is not None else None,
                       capture_output=True, text=True)
    if r.returncode != 0:
        print(f"  API error {endpoint}: {r.stderr.strip()}", file=sys.stderr)
        return None
    try:
        return json.loads(r.stdout) if r.stdout.strip() else {}
    except json.JSONDecodeError:
        return {"raw": r.stdout}

# Get tracked files
result = subprocess.run(["git", "ls-files"], capture_output=True, text=True)
files = [f.strip() for f in result.stdout.splitlines() if f.strip()]
print(f"Found {len(files)} files")

# Get current HEAD
head = gh_api("git/refs/heads/main", "GET")
if not head:
    sys.exit(1)
head_sha = head["object"]["sha"]
print(f"HEAD: {head_sha[:7]}")

# Create blobs
blob_shas = {}
for f in files:
    with open(f, "rb") as fh:
        content = base64.b64encode(fh.read()).decode()
    blob = gh_api("git/blobs", data={"encoding": "base64", "content": content})
    if not blob or "sha" not in blob:
        print(f"Failed blob for {f}: {blob}")
        sys.exit(1)
    blob_shas[f] = blob["sha"]
    print(f"  {f:45s} {blob['sha'][:7]}")

print("Building tree...")

tree_items = [{"path": f, "mode": "100644", "type": "blob", "sha": blob_shas[f]} for f in files]
tree = gh_api("git/trees", data={"base_tree": head_sha, "tree": tree_items})
if not tree or "sha" not in tree:
    print(f"Failed tree: {tree}")
    sys.exit(1)
tree_sha = tree["sha"]
print(f"Tree: {tree_sha[:7]}")

commit = gh_api("git/commits", data={
    "message": "feat: initial Rust implementation of WMPF Debugger\n\n- Pure Rust WebSocket servers (tokio-tungstenite)\n- Protobuf encode/decode via prost\n- Zlib compression support\n- Hand-written Frida C FFI bindings (behind frida-link feature)\n- Frida integration: process discovery, script injection\n- CDP proxy for Chrome DevTools connection\n- Version-specific hook configs for 45 WMPF versions",
    "tree": tree_sha,
    "parents": [head_sha]
})
if not commit or "sha" not in commit:
    print(f"Failed commit: {commit}")
    sys.exit(1)
commit_sha = commit["sha"]
print(f"Commit: {commit_sha[:7]}")

gh_api("git/refs/heads/main", "PATCH", {"sha": commit_sha, "force": True})
print(f"\nDone! {len(files)} files pushed to https://github.com/{repo}")
