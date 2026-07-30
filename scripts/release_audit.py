"""Fail closed on ApriReader's repository-level release security gates."""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN_FILES = {
    "aprireader_steam_bridge.dll",
    "steam_api64.dll",
    "steam_appid.txt",
}
IGNORED_DIRS = {".git", "node_modules", "target", "dist"}
APPROVED_NPM = {
    "@tauri-apps/api": "MIT OR Apache-2.0",
    "@tauri-apps/plugin-dialog": "MIT OR Apache-2.0",
    "pdfjs-dist": "Apache-2.0",
    "react": "MIT",
    "react-dom": "MIT",
}
APPROVED_CARGO = {
    "base64": "MIT OR Apache-2.0",
    "ort": "MIT OR Apache-2.0",
    "quick-xml": "MIT",
    "rars": "MIT OR Apache-2.0",
    "rusqlite": "MIT",
    "serde": "MIT OR Apache-2.0",
    "serde_json": "MIT OR Apache-2.0",
    "sha2": "MIT OR Apache-2.0",
    "tauri": "MIT OR Apache-2.0",
    "tauri-plugin-dialog": "MIT OR Apache-2.0",
    "thiserror": "MIT OR Apache-2.0",
    "ureq": "MIT OR Apache-2.0",
    "zip": "MIT",
}


def fail(message: str) -> None:
    print(f"release audit failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def check_forbidden_files() -> None:
    for path in ROOT.rglob("*"):
        if any(part in IGNORED_DIRS for part in path.relative_to(ROOT).parts):
            continue
        if path.is_file() and path.name.casefold() in FORBIDDEN_FILES:
            fail(f"protected artifact present: {path.relative_to(ROOT)}")


def check_direct_dependencies() -> None:
    package = json.loads((ROOT / "package.json").read_text("utf-8"))
    npm = set(package["dependencies"])
    if npm != set(APPROVED_NPM):
        fail(f"npm dependency review is stale: {sorted(npm ^ set(APPROVED_NPM))}")
    cargo = tomllib.loads((ROOT / "src-tauri" / "Cargo.toml").read_text("utf-8"))
    rust = set(cargo["dependencies"])
    if rust != set(APPROVED_CARGO):
        fail(f"Cargo dependency review is stale: {sorted(rust ^ set(APPROVED_CARGO))}")


def check_tauri_boundary() -> None:
    config = json.loads((ROOT / "src-tauri" / "tauri.conf.json").read_text("utf-8"))
    security = config["app"]["security"]
    csp = security["csp"]
    if "'unsafe-eval'" in csp or "script-src *" in csp:
        fail("CSP permits dynamic or unrestricted script execution")
    if security["capabilities"] != ["default"]:
        fail("unexpected Tauri capability enabled")
    capability = json.loads(
        (ROOT / "src-tauri" / "capabilities" / "default.json").read_text("utf-8")
    )
    allowed = {"core:default", "dialog:allow-open", "dialog:allow-save"}
    if set(capability["permissions"]) != allowed:
        fail("desktop capability permissions changed without security review")
    scopes = set(security["assetProtocol"]["scope"])
    expected = {
        "$APPLOCALDATA/covers/**",
        "$APPLOCALDATA/fonts/**",
        "$APPLOCALDATA/readers/**",
    }
    if scopes != expected:
        fail("asset protocol scope changed without security review")

    bundle = config["bundle"]
    if bundle.get("active") is not True or bundle.get("targets") != ["nsis"]:
        fail("public Windows candidate must produce only the reviewed NSIS bundle")
    if bundle.get("useLocalToolsDir") is not True:
        fail("Windows bundler tools must remain in the project-local target cache")
    if bundle.get("license") != "Apache-2.0" or bundle.get("licenseFile") != "../LICENSE":
        fail("installer license metadata changed without review")
    nsis = bundle.get("windows", {}).get("nsis", {})
    if nsis.get("installMode") != "currentUser":
        fail("installer must remain scoped to the current Windows user")
    if nsis.get("languages") != ["English", "Russian"]:
        fail("installer language set changed without review")
    if nsis.get("displayLanguageSelector") is not True:
        fail("installer must expose the reviewed RU/EN language selector")


def check_candidate_provenance() -> None:
    builder = (ROOT / "scripts" / "build_beta_candidate.ps1").read_text("utf-8")
    required_markers = {
        "sourceTreeState": "source tree state",
        "sourceChangedFileCount": "changed-file count",
        "SOURCE_SHA256SUMS.txt": "source manifest",
        "sourceManifestSha256": "source manifest hash",
        '"release-candidate", "github-release"': "public channel guard",
        "Release-candidate and GitHub release builds require -RequireCleanTree.": (
            "mandatory clean-tree public-build guard"
        ),
        "The release source tree changed while the candidate was building.": (
            "mid-build source mutation guard"
        ),
    }
    missing = [
        description
        for marker, description in required_markers.items()
        if marker not in builder
    ]
    if missing:
        fail(f"candidate provenance gate is stale: {', '.join(missing)}")
    if not (ROOT / "release" / "THIRD_PARTY_LICENSES.md").is_file():
        fail("generated third-party license bundle is missing")


def main() -> int:
    check_forbidden_files()
    check_direct_dependencies()
    check_tauri_boundary()
    check_candidate_provenance()
    print("Release security and direct-license gates passed.")
    for name, license_id in sorted(APPROVED_NPM.items() | APPROVED_CARGO.items()):
        print(f"  {name}: {license_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
