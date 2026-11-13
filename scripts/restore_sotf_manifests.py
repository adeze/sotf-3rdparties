#!/usr/bin/env python3
"""Restore sotf-* manifest policy after re-vendoring GPUI from Zed.

The importer (gpui-toolkit/scripts/import_gpui_upstream.py) emits upstream
package names, `publish = false`, and git+tag dependencies. This pass merges
each regenerated manifest with the pre-upgrade local manifest snapshot:

- package identity (sotf-* name, version, repository, lib name) from the
  snapshot; new crates get the same treatment by the naming table below;
- [features] from upstream (feature names are referenced by the new code),
  minus entries in FEATURE_REMOVALS;
- dependencies present in both keep the snapshot spec (local policy wins);
  upstream-only dependencies are added (internal crates become
  workspace/path deps, never git); snapshot-only dependencies are dropped
  unless listed in KEEP_DEPS;
- DROP_DEPS are never re-added (yanked/replaced crates).

Every divergence is printed as a report; unknown git dependencies abort.
Run from the sotf-3rdparties worktree root:

    python3 scripts/restore_sotf_manifests.py \
        --pre /Volumes/home_tmp/tmp/zed-upstream-cache/manifests-pre \
        --regen /Volumes/home_tmp/tmp/regen-v1.25.2 \
        --crates gpui/crates
"""
from __future__ import annotations

import argparse
import sys
import tomllib
from pathlib import Path

import gpui_toml

# New crates admitted at v1.25.2: upstream lib name -> sotf package name.
NEW_CRATE_PACKAGES = {
    "gpui_apple": "sotf-gpui-toolkit-gpui-apple",
    "path": "sotf-gpui-toolkit-path",
    "bench_metrics": "sotf-gpui-toolkit-bench-metrics",
}
# Crates whose sources live in gpui/crates but are not importer-vendored
# (pinned support crates); deps on them become workspace deps like the rest.
INTERNAL_CRATES = {
    "bench_metrics", "collections", "gpui", "gpui_apple", "gpui_linux",
    "gpui_macos", "gpui_macros", "gpui_shared_string", "gpui_util",
    "gpui_web", "gpui_wgpu", "gpui_windows", "http_client", "path",
    "perf", "refineable", "derive_refineable", "scheduler", "sum_tree",
    "util", "util_macros",
}
DROP_DEPS = {"spin", "postage"}
# Hand-maintained crates: never rewritten by this pass.
SKIP_CRATES = {"gpui_wgpu", "gpui_windows"}
# Upstream git deps with a local sibling/registry equivalent. Extras
# (default-features/features/optional) are taken from the upstream spec.
GIT_DEP_MAP = {
    "font-kit": {"package": "sotf-zed-font-kit", "version": "0.14.1-zed", "path": "../../../zed-font-kit"},
    "scap": {"package": "sotf-zed-scap", "version": "0.0.8-zed", "path": "../../../scap"},
    "xim": {"package": "sotf-zed-xim", "version": "0.4.0-zed", "path": "../../../xim-rs"},
    "wasm_thread": {"version": "0.3"},
    "async-tar": {"version": "0.6.1", "default-features": False, "features": ["runtime-tokio", "xattr"]},
    "proptest": {"workspace": True},
}
KEEP_DEPS = {("gpui", "async-channel")}
# Deps whose feature set must be the union of local + upstream (upstream code
# needs new features; local code needs the old ones). Version/default-features
# follow upstream; local package/path (sibling forks) are preserved.
UNION_FEATURE_DEPS = {
    ("gpui", "accesskit"), ("gpui", "bitflags"), ("gpui", "windows"),
    ("gpui_linux", "accesskit"), ("gpui_linux", "ashpd"),
    ("gpui_macos", "accesskit"), ("gpui_macos", "objc2-app-kit"),
    ("gpui_macos", "objc2-foundation"), ("gpui_web", "web-sys"),
    ("util", "nix"), ("util", "rust-embed"),
}
FEATURE_REMOVALS = {
    ("gpui", "test-support"): {"dep:postage"},
    ("http_client", "test-support"): {"dep:log"},
    ("util", "test-support"): {"dep:tempfile"},
}
# Features neither side declares but the merged code needs (feature-unification
# gaps that a full-workspace build would mask).
EXTRA_FEATURES = {("gpui", "objc2-core-video"): {"CVBuffer", "CVImageBuffer"}}
# Deps the merged code uses unconditionally; never made optional by dep: refs.
FORCE_NON_OPTIONAL = {("http_client", "log"), ("util", "tempfile")}
# Deps where the local spec provably breaks the new upstream code; the
# upstream spec wins verbatim (syn 3 local bump vs syn-2 macro parsers).
FORCE_UPSTREAM_DEPS = set()
DEP_SECTIONS = ("dependencies", "dev-dependencies", "build-dependencies")
REPO = "https://github.com/pierreaubert/sotf-3rdparties"


def read_toml(path: Path) -> dict:
    return tomllib.loads(path.read_text())


def iter_dep_tables(manifest: dict):
    for section in DEP_SECTIONS:
        yield section, None, manifest.get(section, {})
    for cfg, table in manifest.get("target", {}).items():
        for section in DEP_SECTIONS:
            yield section, cfg, table.get(section, {})


def transform_new_dep(crate: str, name: str, spec, report: list[str], emitter):
    """Map an upstream-only dep spec to local policy; abort on unknown git."""
    if isinstance(spec, dict) and "git" in spec:
        if name in INTERNAL_CRATES:
            out = {"workspace": True}
            for key in ("default-features", "features", "optional"):
                if key in spec:
                    out[key] = spec[key]
            report.append(f"{crate}: new internal dep {name} -> workspace")
            return out
        if name in GIT_DEP_MAP:
            out = dict(GIT_DEP_MAP[name])
            for key in ("default-features", "features", "optional"):
                if key in spec and key not in out:
                    out[key] = spec[key]
            report.append(f"{crate}: git dep {name} -> local {out!r}")
            return out
        raise SystemExit(f"error: {crate}: unknown git dep {name}: {spec}")
    report.append(f"{crate}: new dep {name} = {spec!r}")
    return spec


def merge_deps(crate: str, pre: dict, regen: dict, report: list[str], emitter) -> dict:
    out_sections: dict[tuple[str, str | None], dict] = {}
    regen_tables = {(s, c): t for s, c, t in iter_dep_tables(regen)}
    pre_tables = {(s, c): t for s, c, t in iter_dep_tables(pre)}
    for key in sorted(set(regen_tables) | set(pre_tables), key=lambda k: (k[0], k[1] or "")):
        rdeps = regen_tables.get(key, {})
        pdeps = pre_tables.get(key, {})
        merged: dict = {}
        for name, rspec in rdeps.items():
            if name in DROP_DEPS:
                report.append(f"{crate}: drop upstream dep {name} (DROP_DEPS)")
                continue
            if name in pdeps:
                if (crate, name) in FORCE_UPSTREAM_DEPS:
                    merged[name] = rspec
                    report.append(f"{crate}: dep {name} forced upstream {rspec!r}")
                    continue
                if (crate, name) in UNION_FEATURE_DEPS:
                    pspec = {"version": pdeps[name]} if isinstance(pdeps[name], str) else dict(pdeps[name])
                    uspec = {"version": rspec} if isinstance(rspec, str) else dict(rspec)
                    out = dict(uspec)
                    for pkey in ("package", "path"):
                        if pkey in pspec:
                            out[pkey] = pspec[pkey]
                    out["features"] = sorted(set(pspec.get("features", [])) | set(uspec.get("features", [])))
                    if pspec.get("version", uspec.get("version")) != uspec.get("version"):
                        out["version"] = pspec.get("version", uspec.get("version"))
                    merged[name] = out
                    report.append(f"{crate}: dep {name} feature-union {out!r}")
                    continue
                if pdeps[name] != rspec:
                    report.append(f"{crate}: dep {name} kept local {pdeps[name]!r} (upstream {rspec!r})")
                merged[name] = pdeps[name]
            else:
                merged[name] = transform_new_dep(crate, name, rspec, report, emitter)
        for name, pspec in pdeps.items():
            if name not in rdeps:
                if (crate, name) in KEEP_DEPS:
                    merged[name] = pspec
                    report.append(f"{crate}: keep local-only dep {name} (KEEP_DEPS)")
                else:
                    report.append(f"{crate}: drop local-only dep {name} = {pspec!r}")
        if merged:
            out_sections[key] = merged
    return out_sections


def place_sections(doc: dict, sections: dict) -> None:
    for (section, cfg), deps in sections.items():
        if cfg is None:
            doc[section] = deps
        else:
            doc.setdefault("target", {}).setdefault(cfg, {})[section] = deps


def merge_manifest(crate: str, pre: dict | None, regen: dict, report: list[str], emitter) -> dict:
    if pre is None:
        doc = {"package": dict(regen.get("package", {}))}
        doc["package"]["name"] = NEW_CRATE_PACKAGES[crate]
        doc["package"]["version"] = "0.1.0"
        doc["package"]["repository"] = REPO
        doc["package"].pop("publish", None)
        lib = dict(regen.get("lib", {}))
        lib.setdefault("name", crate)
        doc["lib"] = lib
        report.append(f"{crate}: NEW crate, package {doc['package']['name']}")
        pre = {}
    else:
        doc = {"package": dict(pre.get("package", {}))}
        if "lib" in pre:
            doc["lib"] = pre["lib"]
    if "features" in regen:
        features = {}
        for fname, fvals in regen["features"].items():
            removals = FEATURE_REMOVALS.get((crate, fname), set())
            kept = [v for v in fvals if v not in removals]
            if removals:
                report.append(f"{crate}: feature {fname} removals {sorted(removals)}")
            features[fname] = kept
        doc["features"] = features
    sections = merge_deps(crate, pre, regen, report, emitter)
    # Upstream features may reference dep:X where the local spec kept X
    # non-optional; Cargo rejects that. Follow upstream optionality there.
    dep_refs = {v[4:] for vals in doc.get("features", {}).values() for v in vals if v.startswith("dep:")}
    for deps in sections.values():
        for name in dep_refs & set(deps):
            if (crate, name) in FORCE_NON_OPTIONAL:
                continue
            spec = deps[name]
            if isinstance(spec, str):
                deps[name] = {"version": spec, "optional": True}
                report.append(f"{crate}: dep {name} made optional (dep: feature ref)")
            elif isinstance(spec, dict) and not spec.get("optional"):
                spec["optional"] = True
                report.append(f"{crate}: dep {name} made optional (dep: feature ref)")
    for deps in sections.values():
        for (c, name), extras in EXTRA_FEATURES.items():
            if c == crate and name in deps and isinstance(deps[name], dict):
                deps[name]["features"] = sorted(set(deps[name].get("features", [])) | extras)
    for (section, _cfg), deps in sections.items():
        if section == "dev-dependencies":
            for spec in deps.values():
                if isinstance(spec, dict):
                    spec.pop("optional", None)
    place_sections(doc, sections)
    for key, value in regen.items():
        if key not in doc and key not in ("package", "lib", "features", "target", *DEP_SECTIONS):
            doc[key] = value
            report.append(f"{crate}: copy upstream top-level [{key}]")
    for key, value in pre.items():
        if key not in doc and key not in ("package", "lib", "features", "target", *DEP_SECTIONS):
            doc[key] = value
            report.append(f"{crate}: keep local top-level [{key}]")
    return doc


def emit_manifest(doc: dict, emitter) -> str:
    """Emit TOML; array-of-tables keys ([[bin]] etc.) go last, manually."""
    plain = {k: v for k, v in doc.items()
             if not (isinstance(v, list) and v and isinstance(v[0], dict))}
    arrays = {k: v for k, v in doc.items() if k not in plain}
    text = emitter.dumps_toml(plain)
    for key, entries in arrays.items():
        for entry in entries:
            text += f"\n[[{key}]]\n"
            for k, v in entry.items():
                text += f"{k} = {emitter._toml_value(v)}\n"
    return text


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pre", required=True, help="snapshot root containing gpui/crates/*")
    parser.add_argument("--regen", required=True, help="pristine regenerated vendor root")
    parser.add_argument("--crates", default="gpui/crates", help="destination crates dir")
    parser.add_argument("--only", action="append", default=[], help="limit to crate name(s)")
    args = parser.parse_args(argv)
    emitter = gpui_toml
    pre_root = Path(args.pre) / "gpui" / "crates"
    regen_root = Path(args.regen)
    dest_root = Path(args.crates)
    crates = sorted(p.name for p in regen_root.iterdir() if p.is_dir())
    if args.only:
        crates = [c for c in crates if c in set(args.only)]
    report: list[str] = []
    for crate in crates:
        if crate in SKIP_CRATES:
            print(f"skipped {crate} (hand-maintained)")
            continue
        regen = read_toml(regen_root / crate / "Cargo.toml")
        pre_path = pre_root / crate / "Cargo.toml"
        pre = read_toml(pre_path) if pre_path.is_file() else None
        if pre is None and crate not in NEW_CRATE_PACKAGES:
            raise SystemExit(f"error: {crate}: no snapshot manifest and not a known new crate")
        doc = merge_manifest(crate, pre, regen, report, emitter)
        (dest_root / crate / "Cargo.toml").write_text(emit_manifest(doc, emitter))
        print(f"restored {crate}")
    print("\n--- divergence report ---")
    print("\n".join(report) if report else "(none)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
