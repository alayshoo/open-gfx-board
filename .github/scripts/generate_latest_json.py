#!/usr/bin/env python3
"""
Generates the latest.json update manifest consumed by tauri-plugin-updater.

Each platform entry contains:
  - url:       direct download link to the updater bundle on GitHub Releases
  - signature: the content of the .sig file produced by `tauri signer`

The Ed25519 signature is verified by the app against the public key that was
embedded at compile time (TAURI_SIGNING_PUBLIC_KEY), so a tampered binary or
a bundle from an untrusted source will be rejected.

Expects the following environment variables (set by the workflow):
  TAG   – the git tag, e.g. "v0.1.3"
  REPO  – owner/repo,  e.g. "alayshoo/open-gfx-board"
"""

import glob
import json
import os
import sys
from datetime import datetime, timezone
from urllib.parse import quote

tag = os.environ["TAG"]          # e.g. "v0.1.3"
repo = os.environ["REPO"]        # e.g. "alayshoo/open-gfx-board"
version = tag.lstrip("v")        # e.g. "0.1.3"
base_url = f"https://github.com/{repo}/releases/download/{tag}"


def read_sig(path: str) -> str:
    """Return the trimmed content of a .sig file."""
    with open(path, "r", encoding="utf-8") as f:
        return f.read().strip()


def find_one(pattern: str) -> str | None:
    """Return the first file matching *pattern*, or None."""
    matches = glob.glob(pattern, recursive=True)
    return matches[0] if matches else None


platforms: dict = {}

# ── Windows (x86_64) ─────────────────────────────────────────────────────────
# Tauri produces an NSIS installer: "Open GFX Board_X.X.X_x64-setup.exe"
nsis_exe = find_one("artifacts/windows-artifacts/**/*.exe")
nsis_sig = find_one("artifacts/windows-artifacts/**/*.exe.sig")

if nsis_exe and nsis_sig:
    platforms["windows-x86_64"] = {
        "signature": read_sig(nsis_sig),
        "url": f"{base_url}/{quote(os.path.basename(nsis_exe))}",
    }
    print(f"[windows-x86_64] {os.path.basename(nsis_exe)}")
else:
    print("WARNING: Windows NSIS artifacts not found – windows-x86_64 will be omitted",
          file=sys.stderr)

# ── macOS ARM (aarch64 / Apple Silicon) ───────────────────────────────────────
# Tauri produces a compressed app bundle: "Open GFX Board_X.X.X_aarch64.app.tar.gz"
arm_tar = find_one("artifacts/macos-artifacts/**/*.app.tar.gz")
arm_sig = find_one("artifacts/macos-artifacts/**/*.app.tar.gz.sig")

if arm_tar and arm_sig:
    platforms["darwin-aarch64"] = {
        "signature": read_sig(arm_sig),
        "url": f"{base_url}/{quote(os.path.basename(arm_tar))}",
    }
    print(f"[darwin-aarch64] {os.path.basename(arm_tar)}")
else:
    print("WARNING: macOS ARM artifacts not found – darwin-aarch64 will be omitted",
          file=sys.stderr)

if not platforms:
    print("ERROR: No platform artifacts were found. Aborting.", file=sys.stderr)
    sys.exit(1)

manifest = {
    "version": version,
    "notes": f"See the release page for full notes.",
    "pub_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": platforms,
}

with open("latest.json", "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=2)

print(f"\nGenerated latest.json for version {version}:")
print(json.dumps(manifest, indent=2))
