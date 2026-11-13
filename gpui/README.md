# GPUI framework and support sources

This workspace owns the GPUI vendor and support crates listed below. SOTF-owned toolkit UI components, themes, builders and apps remain in the sibling `gpui-toolkit` repository.

## Current source and packages

<!-- current-gpui-provenance:start -->
Current upstream ref: `v1.25.2`. Support source revision: `d25fa01ccd788d812f236810b71af4feff6b2e38`.

| Crate | Local package | Version |
| --- | --- | --- |
| `gpui` | `sotf-gpui-toolkit-gpui` | `0.2.3` |
| `gpui_apple` | `sotf-gpui-toolkit-gpui-apple` | `0.1.0` |
| `gpui_linux` | `sotf-gpui-toolkit-gpui-linux` | `0.1.0` |
| `gpui_macos` | `sotf-gpui-toolkit-gpui-macos` | `0.1.2` |
| `gpui_macros` | `sotf-gpui-toolkit-gpui-macros` | `0.1.0` |
| `gpui_web` | `sotf-gpui-toolkit-gpui-web` | `0.1.1` |
| `gpui_wgpu` | `sotf-gpui-toolkit-gpui-wgpu` | `0.1.3` |
| `gpui_windows` | `sotf-gpui-toolkit-gpui-windows` | `0.1.2` |
| `http_client` | `sotf-gpui-toolkit-http-client` | `0.1.0` |
| `perf` | `sotf-gpui-toolkit-perf` | `0.1.0` |
| `scheduler` | `sotf-gpui-toolkit-scheduler` | `0.1.0` |
| `sum_tree` | `sotf-gpui-toolkit-sum-tree` | `0.1.0` |
| `util` | `sotf-gpui-toolkit-util` | `0.1.1` |
| `util_macros` | `sotf-gpui-toolkit-util-macros` | `0.1.0` |
| `gpui_util` | `sotf-gpui_util` | `0.1.0` |
| `refineable` | `sotf-refineable` | `0.1.0` |
| `gpui_shared_string` | `sotf-gpui_shared_string` | `0.1.0` |
| `collections` | `sotf-collections` | `0.1.0` |
| `path` | `sotf-gpui-toolkit-path` | `0.1.0` |
| `bench_metrics` | `sotf-gpui-toolkit-bench-metrics` | `0.1.0` |
| `refineable/derive_refineable` | `sotf-derive_refineable` | `0.1.0` |
<!-- current-gpui-provenance:end -->

The source and package table is generated from `FORKS.json` and the active manifests. Run `python3 scripts/gpui_provenance.py --write` after updating them. Per-crate vendoring notes record patch history. `PREVIOUS_VENDOR_NOTES.md` is historical evidence.

## Workspace and assets

`Cargo.toml` preserves the former toolkit's inherited dependency, feature and
lint policies for the source members. Its shared support dependencies
point at the local source above. `assets/fonts` preserves the embedded font bytes,
readmes and licenses needed by native/web rendering. The source include paths
retain their original relative layout.

## Updating and validation

From `all_of_sotf`, run the collection verifier and the SOTF GPUI shipping build:

```sh
python3 sotf-3rdparties/scripts/verify.py
cargo check --manifest-path sotf/Cargo.toml -p sotf-gpui --features shipping
```

Run the relevant per-crate regressions when changing behavior; macOS compilation
does not qualify Windows/Linux/web targets. The import script in the sibling
toolkit (`scripts/import_gpui_upstream.py`) now targets this group's `crates`
directory. Preserve local patches and source pins when using it.
Follow the [collection update procedure](../README.md#updating-a-fork).

The renderer's `cosmic-text` pin and Zed `scap` / `xim-rs` forks are maintained as
separate source groups in the parent collection. Their revisions and purposes
are listed in `../FORKS.json`; GPUI dependency paths resolve to those local copies.
