#!/usr/bin/env python3
"""Check fork documentation and the aggregate's reviewed patch sources."""

import json
from pathlib import Path
import subprocess
import tomllib

from gpui_provenance import update as check_gpui_provenance


def is_block_compatibility_patch(source: str, package: str, dependency: object) -> bool:
    """Recognize the retained upstream-name patch; see Capture's manifest.

    The current local fork is named sotf-block and cannot replace block in
    upstream consumers. Keep this exception exact so other Git pins still
    require review.
    """
    return source == "crates-io" and package == "block" and dependency == {
        "git": "https://github.com/pierreaubert/sotf-3rdparties",
        "rev": "5a4634af0916b5f53effa5ef6340efd90af32285",
    }


def main() -> None:
    collection = Path(__file__).resolve().parents[1]
    aggregate = collection.parent
    inventory = json.loads((collection / "FORKS.json").read_text())
    errors: list[str] = []
    directories: set[str] = set()
    required = ["upstream_url", "upstream_revision_status", "reason", "retained_changes",
                "owner", "review_cadence_days", "removal_condition", "verification_command"]
    for fork in inventory["forks"]:
        name = fork["directory"]
        if name in directories:
            errors.append(f"Duplicate fork: {name}")
        directories.add(name)
        for field in required:
            if not fork.get(field):
                errors.append(f"{name}: missing {field}")
        for relative in [name + "/Cargo.toml", fork["maintenance_document"], fork["detail_document"]]:
            if not (collection / relative).is_file():
                errors.append(f"{name}: missing {relative}")
        if not fork["license_files"] and not fork.get("license_status"):
            errors.append(f"{name}: no preserved license files or documented license evidence gap")
        for relative in fork["license_files"]:
            if not (collection / name / relative).is_file():
                errors.append(f"{name}: missing license {relative}")
    repos = ["sotf", "sotf-daw", "sotf-capture", "sotf-systemwide", "autoeq",
             "math-audio", "gpui-toolkit", "sofa-reader", "symphonia-add-ons"]
    patches = 0
    for repo in repos:
        folder = aggregate / repo
        if not folder.exists():
            # This collection can be checked standalone; consumer checks need the aggregate.
            continue
        tracked = subprocess.check_output(["git", "-C", str(folder), "ls-files", "-z"], text=True)
        for relative in tracked.split("\0"):
            if not relative.endswith("Cargo.toml") or any(
                part in {"audit", "artifacts", "target"} for part in Path(relative).parts
            ):
                continue
            manifest = folder / relative
            if not manifest.exists():
                continue
            config = tomllib.loads(manifest.read_text())
            for source, entries in config.get("patch", {}).items():
                for package, dependency in entries.items():
                    patches += 1
                    if is_block_compatibility_patch(source, package, dependency):
                        continue
                    if not isinstance(dependency, dict) or "path" not in dependency:
                        errors.append(f"{manifest}: {source}/{package} is not a local path")
                        continue
                    target = (manifest.parent / dependency["path"]).resolve()
                    if collection not in target.parents:
                        errors.append(f"{manifest}: {package} resolves outside the collection: {target}")
                    elif not (target / "Cargo.toml").is_file():
                        errors.append(f"{manifest}: missing patched crate {target}")
                    elif target.relative_to(collection).parts[0] not in directories:
                        errors.append(f"{manifest}: undocumented patch {target}")
    errors.extend(check_gpui_provenance(collection))
    check_release_naming(collection, errors)
    if errors:
        raise SystemExit("\n".join(errors))
    print(f"Verified {len(directories)} fork records, licenses and {patches} reviewed patch declarations.")


def check_release_naming(collection: Path, errors: list[str]) -> None:
    """Enforce the crates.io release invariants (issue #3).

    Every package in the collection must be named sotf-*, intra-collection
    path dependencies must carry matching package/version keys (required for
    `cargo publish` path rewriting), and publishable crates must not use git
    dependencies in normal/build tables.
    """
    manifests = sorted(p for p in collection.rglob("Cargo.toml")
                       if "target/" not in p.parts)
    names: dict[str, str] = {}  # crate dir -> package name
    versions: dict[str, str | None] = {}
    parsed: dict[str, dict] = {}
    for manifest in manifests:
        rel = str(manifest.parent.relative_to(collection))
        parsed[rel] = tomllib.loads(manifest.read_text())
        name = parsed[rel].get("package", {}).get("name")
        if isinstance(name, str):
            if not name.startswith("sotf-"):
                errors.append(f"{rel}/Cargo.toml: package name must start with sotf-: {name}")
            names[rel] = name
            version = parsed[rel].get("package", {}).get("version")
            versions[rel] = version if isinstance(version, str) else None
    for manifest in manifests:
        rel = str(manifest.parent.relative_to(collection))
        config = parsed[rel]
        tables = [config.get("dependencies", {}), config.get("dev-dependencies", {}),
                  config.get("build-dependencies", {}),
                  config.get("workspace", {}).get("dependencies", {})]
        for target in config.get("target", {}).values():
            if isinstance(target, dict):
                for section in ("dependencies", "dev-dependencies", "build-dependencies"):
                    tables.append(target.get(section, {}))
        for deps in tables:
            if not isinstance(deps, dict):
                continue
            for key, dep in deps.items():
                if not isinstance(dep, dict):
                    continue
                if "git" in dep:
                    continue  # dev-only git deps and [patch] git overrides are allowed;
                    # normal/build git deps are checked structurally below
                if "path" not in dep:
                    continue
                target = (manifest.parent / dep["path"]).resolve()
                try:
                    trel = str(target.relative_to(collection))
                except ValueError:
                    continue
                if trel not in names:
                    errors.append(f"{rel}/Cargo.toml: path dep {key} has no collection manifest: {trel}")
                    continue
                if dep.get("package", key) != names[trel]:
                    errors.append(f"{rel}/Cargo.toml: dep {key} must set package = \"{names[trel]}\"")
                if "version" not in dep and versions[trel] is not None:
                    errors.append(f"{rel}/Cargo.toml: dep {key} needs version = \"{versions[trel]}\"")
        for section in ("dependencies", "build-dependencies"):
            for key, dep in config.get(section, {}).items():
                if isinstance(dep, dict) and "git" in dep:
                    errors.append(f"{rel}/Cargo.toml: git dependency forbidden in [{section}]: {key}")
        for key, dep in config.get("workspace", {}).get("dependencies", {}).items():
            if isinstance(dep, dict) and "git" in dep:
                errors.append(f"{rel}/Cargo.toml: git dependency forbidden in [workspace.dependencies]: {key}")
        for source, entries in config.get("patch", {}).items():
            if not isinstance(entries, dict):
                continue
            for key, dep in entries.items():
                if not (isinstance(dep, dict) and "path" in dep):
                    continue
                target = (manifest.parent / dep["path"]).resolve()
                try:
                    trel = str(target.relative_to(collection))
                except ValueError:
                    continue
                if trel in names and key != names[trel]:
                    errors.append(f"{rel}/Cargo.toml: [patch.{source}] {key} must be keyed "
                                  f"\"{names[trel]}\" to match its renamed target")


if __name__ == "__main__":
    main()
