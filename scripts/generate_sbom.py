"""Generate a deterministic CycloneDX inventory from ApriReader lockfiles."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import tomllib
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "release" / "aprireader-sbom.cdx.json"


def cargo_components() -> list[dict[str, str]]:
    lock = tomllib.loads((ROOT / "src-tauri" / "Cargo.lock").read_text("utf-8"))
    return [
        {
            "type": "library",
            "group": "cargo",
            "name": package["name"],
            "version": package["version"],
            "purl": f"pkg:cargo/{package['name']}@{package['version']}",
        }
        for package in lock["package"]
        if package["name"] != "aprireader"
    ]


def pnpm_components() -> list[dict[str, str]]:
    lock = (ROOT / "pnpm-lock.yaml").read_text("utf-8")
    packages = lock.split("\npackages:\n", 1)[1].split("\nsnapshots:\n", 1)[0]
    components: list[dict[str, str]] = []
    for raw_key in re.findall(
        r"^  (?=\S)(['\"]?)(.+?)\1:[ \t]*$", packages, re.MULTILINE
    ):
        key = raw_key[1]
        if key.startswith("@"):
            name, version = key.rsplit("@", 1)
        else:
            name, version = key.split("@", 1)
        version = version.split("(", 1)[0]
        components.append(
            {
                "type": "library",
                "group": "npm",
                "name": name,
                "version": version,
                "purl": f"pkg:npm/{name.replace('@', '%40')}@{version}",
            }
        )
    return components


def build_document() -> dict[str, object]:
    app_version = json.loads((ROOT / "package.json").read_text("utf-8"))["version"]
    lock_bytes = (ROOT / "pnpm-lock.yaml").read_bytes() + (
        ROOT / "src-tauri" / "Cargo.lock"
    ).read_bytes()
    digest = hashlib.sha256(lock_bytes).hexdigest()
    components = cargo_components() + pnpm_components()
    components.sort(key=lambda item: (item["group"], item["name"], item["version"]))
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "serialNumber": f"urn:uuid:{uuid.uuid5(uuid.NAMESPACE_URL, digest)}",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "name": "ApriReader",
                "version": app_version,
                "licenses": [{"license": {"id": "Apache-2.0"}}],
            },
            "properties": [
                {"name": "aprireader:lockfiles-sha256", "value": digest},
                {
                    "name": "aprireader:scope",
                    "value": "locked Cargo and pnpm package inventory",
                },
            ],
        },
        "components": components,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    content = json.dumps(build_document(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text("utf-8") != content:
            print("SBOM is missing or stale; run: python scripts/generate_sbom.py")
            return 1
        print(f"SBOM is current: {OUTPUT.relative_to(ROOT)}")
        return 0
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(content, "utf-8")
    print(f"Wrote {OUTPUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
