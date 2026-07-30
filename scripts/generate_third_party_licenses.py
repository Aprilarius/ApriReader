"""Generate a deterministic third-party license bundle from installed lockfile packages."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "release" / "THIRD_PARTY_LICENSES.md"
LICENSE_NAMES = ("license", "licence", "copying", "notice")
MAX_LICENSE_BYTES = 2 * 1024 * 1024


@dataclass(frozen=True)
class Component:
    ecosystem: str
    name: str
    version: str
    license_expression: str
    authors: str
    homepage: str
    package_root: Path | None
    explicit_license_file: Path | None = None

    @property
    def identifier(self) -> str:
        return f"{self.ecosystem}:{self.name}@{self.version}"


def resolve_tool(name: str) -> str:
    executable = shutil.which(name)
    if not executable and os.name == "nt" and name == "cargo":
        rustup_proxy = Path.home() / ".cargo" / "bin" / "cargo.exe"
        if os.path.lexists(rustup_proxy):
            executable = str(rustup_proxy)
    if not executable:
        raise RuntimeError(f"required release tool is unavailable: {name}")
    return executable


def run_text(command: list[str], *, offline_cargo: bool = False) -> str:
    command = [resolve_tool(command[0]), *command[1:]]
    environment = os.environ.copy()
    if offline_cargo:
        environment["CARGO_NET_OFFLINE"] = "true"
    result = subprocess.run(
        command,
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
        env=environment,
    )
    return result.stdout


def run_json(command: list[str]) -> object:
    return json.loads(run_text(command))


def npm_components() -> list[Component]:
    raw = run_json(["pnpm", "licenses", "list", "--prod", "--json"])
    if not isinstance(raw, dict):
        raise RuntimeError("pnpm license inventory is not an object")

    components: list[Component] = []
    for entries in raw.values():
        if not isinstance(entries, list):
            continue
        for entry in entries:
            paths = entry.get("paths") or []
            versions = entry.get("versions") or []
            if not paths or not versions:
                raise RuntimeError(f"pnpm license record is incomplete: {entry!r}")
            components.append(
                Component(
                    ecosystem="npm",
                    name=str(entry["name"]),
                    version=str(versions[0]),
                    license_expression=str(entry.get("license") or "UNKNOWN"),
                    authors="",
                    homepage=str(entry.get("homepage") or ""),
                    package_root=Path(paths[0]),
                )
            )
    return components


def cargo_components() -> list[Component]:
    tree = run_text(
        [
            "cargo",
            "tree",
            "--manifest-path",
            str(ROOT / "src-tauri" / "Cargo.toml"),
            "--locked",
            "--target",
            "x86_64-pc-windows-msvc",
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p}|{l}|{r}",
        ],
        offline_cargo=True,
    )
    cargo_registry = Path.home() / ".cargo" / "registry" / "src"
    registry_roots = (
        sorted(
            (path for path in cargo_registry.iterdir() if path.is_dir()),
            key=lambda path: (path.name.casefold(), path.name),
        )
        if cargo_registry.is_dir()
        else []
    )

    components_by_id: dict[tuple[str, str], Component] = {}
    for line in tree.splitlines():
        package, separator, metadata = line.partition("|")
        if not separator:
            raise RuntimeError(f"unexpected Cargo tree line: {line}")
        license_expression, _, homepage = metadata.partition("|")
        package = package.removesuffix(" (*)")
        match = re.fullmatch(r"(.+) v([^ ]+)(?: \(.+\))?", package)
        if not match:
            raise RuntimeError(f"unexpected Cargo package identity: {package}")
        name, version = match.groups()
        if name == "aprireader":
            continue
        package_root = next(
            (
                root / f"{name}-{version}"
                for root in registry_roots
                if (root / f"{name}-{version}").is_dir()
            ),
            None,
        )
        authors = ""
        if package_root:
            manifest = tomllib.loads((package_root / "Cargo.toml").read_text("utf-8"))
            package_metadata = manifest.get("package") or {}
            authors = ", ".join(str(author) for author in package_metadata.get("authors") or [])
            homepage = str(
                package_metadata.get("homepage")
                or package_metadata.get("repository")
                or homepage
            )
        components_by_id[(name, version)] = Component(
            ecosystem="cargo",
            name=name,
            version=version,
            license_expression=license_expression or "UNKNOWN",
            authors=authors,
            homepage=homepage,
            package_root=package_root,
        )
    return list(components_by_id.values())


def license_files(component: Component) -> list[Path]:
    candidates: list[Path] = []
    if component.explicit_license_file:
        candidates.append(component.explicit_license_file)
    if component.package_root and component.package_root.is_dir():
        for path in component.package_root.iterdir():
            if path.is_file() and path.name.casefold().startswith(LICENSE_NAMES):
                candidates.append(path)

    unique: dict[str, Path] = {}
    for path in candidates:
        resolved = path.resolve()
        if resolved.is_file():
            unique[str(resolved).casefold()] = resolved
    return sorted(
        unique.values(),
        key=lambda path: (path.name.casefold(), path.name, str(path)),
    )


def normalize_license_text(path: Path) -> str:
    size = path.stat().st_size
    if size > MAX_LICENSE_BYTES:
        raise RuntimeError(f"license file is unexpectedly large: {path}")
    raw = path.read_bytes()
    decoded = raw.decode("utf-8", errors="replace").replace("\r\n", "\n")
    text = "\n".join(line.rstrip() for line in decoded.splitlines()).strip()
    if not text:
        raise RuntimeError(f"license file is empty: {path}")
    return text


def markdown_escape(value: str) -> str:
    return value.replace("|", "\\|").replace("\n", " ").strip()


def repository_key(value: str) -> str:
    return value.casefold().removesuffix(".git").rstrip("/")


def generate() -> str:
    components = sorted(
        npm_components() + cargo_components(),
        key=lambda item: (item.ecosystem, item.name.casefold(), item.version),
    )
    if not components:
        raise RuntimeError("third-party component inventory is empty")

    files_by_component = {
        component.identifier: license_files(component) for component in components
    }
    files_by_repository: dict[str, list[Path]] = {}
    for component in components:
        files = files_by_component[component.identifier]
        if files and component.homepage:
            files_by_repository.setdefault(repository_key(component.homepage), files)

    texts: dict[str, dict[str, object]] = {}
    missing: list[str] = []
    for component in components:
        files = files_by_component[component.identifier]
        inherited = False
        if not files and component.homepage:
            files = files_by_repository.get(repository_key(component.homepage), [])
            inherited = bool(files)
        if not files:
            missing.append(component.identifier)
            continue
        for path in files:
            text = normalize_license_text(path)
            digest = hashlib.sha256(text.encode("utf-8")).hexdigest().upper()
            record = texts.setdefault(
                digest,
                {"text": text, "components": set(), "filenames": set()},
            )
            record["components"].add(component.identifier)
            filename = f"{path.name} (workspace)" if inherited else path.name
            record["filenames"].add(filename)

    lines = [
        "# Third-party licenses",
        "",
        "This deterministic report covers the installed production npm dependency",
        "graph and the locked Cargo dependency graph used to build ApriReader.",
        "Absolute local paths are intentionally omitted.",
        "",
        f"- Components: {len(components)}",
        f"- Unique license or notice texts: {len(texts)}",
        f"- Components without a packaged license file: {len(missing)}",
        "",
        "## Component inventory",
        "",
        "| Package | License | Authors | Project |",
        "| --- | --- | --- | --- |",
    ]
    for component in components:
        lines.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_escape(component.identifier)}`",
                    markdown_escape(component.license_expression),
                    markdown_escape(component.authors),
                    markdown_escape(component.homepage),
                ]
            )
            + " |"
        )

    lines.extend(["", "## Packaged license and notice texts", ""])
    for digest, record in sorted(texts.items()):
        components_for_text = sorted(record["components"])
        filenames = sorted(record["filenames"], key=lambda value: (value.casefold(), value))
        lines.extend(
            [
                f"### SHA-256 `{digest}`",
                "",
                f"Files: {', '.join(f'`{name}`' for name in filenames)}",
                "",
                "Applies to:",
                "",
                *[f"- `{identifier}`" for identifier in components_for_text],
                "",
                "```text",
                str(record["text"]).replace("```", "` ` `"),
                "```",
                "",
            ]
        )

    if missing:
        lines.extend(
            [
                "## Metadata-only license declarations",
                "",
                "These installed packages declare an SPDX license in package metadata but",
                "do not ship a top-level license or notice file in the installed package:",
                "",
                *[f"- `{identifier}`" for identifier in sorted(missing)],
                "",
            ]
        )

    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    generated = generate()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text("utf-8") != generated:
            print(
                f"third-party license bundle is stale: {OUTPUT.relative_to(ROOT)}",
                file=sys.stderr,
            )
            return 1
        print(f"Third-party license bundle is current: {OUTPUT.relative_to(ROOT)}")
        return 0

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(generated, encoding="utf-8", newline="\n")
    print(f"Wrote {OUTPUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
