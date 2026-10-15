# Releasing sotf-* crates to crates.io

Every crate in this collection is published under an `sotf-` prefixed name,
because every crate here is a patched or pinned fork and must never be
confused with its upstream on crates.io. The rule is absolute: the published
name is always `sotf-xxx`, never `xxx`.

## Naming and compatibility guarantees

- Package `xxx` publishes as `sotf-xxx` (mechanical prefix, full old name
  kept: `zed-scap` -> `sotf-zed-scap`).
- Rust `[lib]` names and `[[bin]]` names are pinned to the old defaults, so
  no Rust source changes when switching: `use foo::` keeps compiling and
  installed binaries keep their names.
- Consumers switch with a rename, keeping their import paths:
  `foo = { package = "sotf-foo", version = "..." }`.
- Intra-collection `path` dependencies always carry `package` + `version`
  keys, which is what `cargo publish` rewrites to registry requirements.
- `scripts/verify.py` enforces these invariants (names, dep keys, no git
  dependencies in normal/build tables).

## What publishes

Libraries, binaries and proc-macros publish (111 crates). Examples, test
harnesses, benches, `xtask`/fuzz/snapshot helpers, the wgpu `player`,
`lock-analyzer`, `cts_runner` and `deno_webgpu` are `publish = false`
(they are still renamed for consistency). Two test helpers that publishable
crates dev-depend on (`naga-test`, `hlsl-snapshots`) do publish, because
cargo validates dev-dependencies at publish time.

## Publish order

Dependencies must be live on crates.io before their dependents. Publish
level by level:

- L0 (leaves): async-task baseview block coreaudio-rs cosmic-text
  derive_more-impl dst-decoder fontconfig-parser headers librespot
  metaheuristics-nature-rs nalgebra nnnoiseless objc oo7 predicates rdsd2pcm
  rfd rubato rust-assert-no-alloc rusty-fork scap stacker tflitec vst3-sys
  wgpu xim-rs zed-font-kit
- L1: derive_more nih-plug tract
- L2: gpui

## How to release

Prerequisites: `cargo-release` installed, a crates.io token authorized for
the `sotf-*` crates (`CARGO_REGISTRY_TOKEN`), and a clean git worktree
(cargo-release refuses to run on a dirty tree).

```sh
# Dry run everything (default; safe on a dirty tree, reports per group).
./scripts/release.sh
# Dry run one level or one group.
./scripts/release.sh L0
./scripts/release.sh block
# Publish for real (strict clean-tree gate, prompts unless --yes).
./scripts/release.sh --execute
./scripts/release.sh --execute --yes L1
```

The script runs `scripts/verify.py` first, then `cargo release publish`
per group in level order (`--workspace` for the eight workspaces).
Each crate carries `[package.metadata.release] tag-name =
"{{crate_name}}-v{{version}}"`, so tags are per-crate
(`sotf-block-v0.1.6`) and never collide in this monorepo.
Versions are independent per crate; bump with
`cargo release version --execute` when a fork changes.

Exception: `sotf-derive_more` publishes with `--no-verify` because its
upstream sources intentionally fail a default-feature build
("at least one derive feature must be enabled"); that is also how upstream
ships it. It is covered by `cargo check --features full` instead.

## Migrating consumers

Sibling repos (`sotf`, `sotf-daw`, ...) reference this collection by path
and break on the rename until migrated. The migration needs no Rust edits:

```sh
# From the collection root, for each consumer repo:
python3 scripts/migrate-consumer.py --collection . ../sotf         # report
python3 scripts/migrate-consumer.py --apply --collection . ../sotf # write
```

Default is the path flow: path deps gain `package = "sotf-..."`, registry
deps that had a matching `[patch]` entry are converted to path deps, and
dead `[patch]` entries pointing into the collection are removed (a patch
must carry the patched crate's name, so renamed crates cannot patch
upstream names). Registry deps without a patch entry are left alone and
reported for review. Re-running is a no-op once migrated.

Registry flow alternative: depend on `sotf-*` from crates.io and keep a
`[patch]` entry keyed by the new name for local overrides. The migration
script never touches patches already keyed by `sotf-...` names.

## Dual-universe warning (read before depending on sotf-objc/sotf-block)

A renamed fork cannot share Rust type identity with its upstream: if one
crate in a build uses `sotf-objc` types and another (e.g. `cocoa` from
crates.io) uses upstream `objc` types, mixing them fails to compile. For
that reason the in-repo users of `block`, `objc` and `async-task` resolve
those crates from upstream (single universe, verified by `cargo tree`);
`sotf-block`, `sotf-objc` and `sotf-async-task` still publish for
completeness, but nothing in this collection uses them today. Consumers
must apply the same rule: within one dependency graph, use either the
`sotf-` fork or upstream for these type-identity crates, never both.

The same applies to any future fork whose types cross crate boundaries:
proc-macros and leaf utilities are safe to duplicate, shared runtimes and
FFI bindings are not.
