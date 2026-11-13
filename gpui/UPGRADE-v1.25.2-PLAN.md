# Phase A execution plan — re-vendor GPUI at Zed v1.25.2 + patch port (cargo check green)

Status: plan only. No vendoring, patching, or manifest edits are executed by this document.
Goal of Phase A: replace the script-vendored GPUI sources at Zed v1.9.0 with Zed
v1.25.2 (upstream commit `d25fa01` as cited in the evidence), re-apply every
still-needed local patch, restore the `sotf-*` manifest policy, and reach a
green `cargo check` for the vendored workspace. Phase A deliberately does
**not** include the Phase B crates.io dependency refresh (~144 per-crate
requirement rows; see §8).

Source of truth for this plan: the compact evidence packet only (six prior
results: import-tooling, patch-surface, upstream-delta, consumer-impact,
dep-inventory, critic). The full bodies of those results were not available
to this synthesis (see §7); every step below that depends on a detail not in
the packet is marked as a verify-at-execution gate rather than asserted.

Key facts the plan is built on:

- Closure grows 12 -> 15 crates: new `path`, `bench_metrics`, `gpui_apple`
  (a macOS/iOS split carrying the old `gpui` + `gpui_macos` platform deps).
  No new excluded-crate or GPL-licensed vendored crate.
- Toolchain pin moves 1.95.0 -> 1.98.1; edition / `rust-version` unchanged.
- Biggest dependency risks: `wgpu` 29.0.3 (Zed fork) -> 30.0.0 (gfx-rs rev
  `3b3506ea`) vs. local `sotf-wgpu` 29.0.0 pin; `taffy` upstream 0.10 -> 0.13
  while local vendored `taffy` is already `=0.14.0` (local ahead of upstream);
  root `[patch]` fork bumps (`calloop`, `async-task`) that the importer does
  not propagate.
- The importer as-is is not runnable for this upgrade (§1).
- Two crates are hand-maintained and must never be overwritten by the
  importer: `gpui_wgpu`, `gpui_windows`. Foreign-parent files inside
  `gpui_linux` / `gpui_windows` are de-facto in the same class.
- Support-crate delta v1.9.0 -> v1.25.2 is fully resolved by the critic:
  `gpui_util` and `gpui_shared_string` changed; `collections`, `refineable`
  (+ derive) unchanged; `media` deleted upstream; `get_powershell` (in
  `gpui_util`) is a hard dependency of `gpui_windows`.

Reference artifacts (already on disk, do not regenerate blindly):

- `/Volumes/home_tmp/tmp/zed-upstream-cache/` — tarballs `zed-1.9.0` /
  `zed-1.25.2`, extracted checkouts `zed-1.9.0`, `zed-1.25.2`,
  `regen-proof/gpui+gpui_macos`.
- `/Volumes/home_tmp/tmp/zed-v1.9.0-closure.txt`,
  `/Volumes/home_tmp/tmp/zed-v1.25.2-closure.txt`,
  `/Volumes/home_tmp/tmp/zed-delta-report.txt` (workspace-resolved dep specs
  for all 12 common crates + 3 new).
- Target worktree named by the critic:
  `/Volumes/home_worktrees/sotf-3rdparties-gpui-v1.25.2` (contains
  `gpui/Cargo.toml`, 19 crate manifests, `verify.py`, `release.sh`,
  `FORKS.json`, `README` / `PREVIOUS_VENDOR_NOTES`).

Conventions: `CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home` for all cargo
commands. Work in the `sotf-3rdparties` worktree above; do not vendor into
the gpui-toolkit workspace directly. Run every step in order; a step that
says STOP is a hard gate.

---

## 1. Importer fixes (do these before any vendor run)

Script: `scripts/import_gpui_upstream.py` (in the sotf-3rdparties repo).

1.1 **Fix the output path.** As-is, `--ref v1.25.2` writes to a nonexistent
sibling path. Fix the script (or its path argument) so output lands in the
real vendored tree of the target worktree
(`<worktree>/gpui/crates/...` layout as currently committed). Gate: run the
importer in dry-run / `--help` mode first and print the resolved output
directory; STOP if it does not exist or is not inside the worktree.

1.2 **Do not expect the importer to produce shippable manifests.** Even
retargeted, it emits unrenamed `publish = false` manifests full of
`git`+tag dependencies. Those fail `verify.py` / `release.sh` on every
count. Treat importer output as *source input only*; §4 (manifest pass) is
mandatory, not optional. Do not "fix" this by loosening `verify.py`.

1.3 **Confirm the rename step does not exist as a tool.** The `sotf-*`
renaming was a one-off manual commit (`c3315a6`); no rename tool was
retained. Phase A therefore needs a new small scripted post-vendor manifest
pass (§4). Write it as a checked-in script this time; do not repeat the
one-off manual edit.

1.4 **Teach / confirm the `--skip` set (§2) is honoured for hand-maintained
crates.** The importer must not create, overwrite, or delete anything under
the skipped crates. Gate: after a trial run into a scratch directory,
`diff -r` the skipped trees against the committed ones; any difference is
an importer bug — fix before the real run.

1.5 **Propagate what the importer does not:** the upstream root `[patch]`
fork bumps (`calloop`, `async-task`) and the toolchain pin
(1.95.0 -> 1.98.1) are not propagated by the importer. Handle both manually
in §4.5 / §4.6; record them in the vendor notes as manual steps.

1.6 **Protect unrecorded local code from deletion.** An importer refresh
would delete foreign-parent files in `gpui_linux` / `gpui_windows` (they
are absent from the VENDORED/PATCHES records the importer consults). Before
the real run, snapshot the entire current vendored tree (§2 step 0) and
list every file present locally but absent upstream; that list is the
must-restore set for §3. STOP if any such file is unaccounted for after
the run.

Importer acceptance test (from import-tooling evidence): the existing
regen proof under `regen-proof/gpui+gpui_macos` shows what a correct
regeneration of the current version looks like. Re-run the fixed importer
at the *old* ref first and confirm it reproduces that proof before
pointing it at v1.25.2.

---

## 2. Exact vendor commands (incl. `--skip` set and external-crate handling)

All commands run from the sotf-3rdparties worktree root unless stated.
Flag spellings other than `--ref` / `--skip` were not in the evidence
packet: confirm them with `--help` in step 0 and adjust names, not
semantics. Do not invent a different ref: the target is `v1.25.2`.

Step 0 — snapshot and preflight (no writes to the tree):

```sh
cd /Volumes/home_worktrees/sotf-3rdparties-gpui-v1.25.2
python3 scripts/import_gpui_upstream.py --help
cp gpui/Cargo.lock /Volumes/home_tmp/tmp/zed-upstream-cache/Cargo.lock.pre-v1.25.2
# snapshot every manifest + full tree listing for the must-restore diff (§1.6)
find gpui/crates -name Cargo.toml -exec cp --parents {} /Volumes/home_tmp/tmp/zed-upstream-cache/manifests-pre/ \;
find gpui/crates -type f | sort > /Volumes/home_tmp/tmp/zed-upstream-cache/tree-pre.txt
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo metadata --manifest-path gpui/Cargo.toml --no-deps --format-version 1 > /Volumes/home_tmp/tmp/zed-upstream-cache/metadata-pre.json
```

Use the already-downloaded tarballs / extracted checkouts in
`/Volumes/home_tmp/tmp/zed-upstream-cache/` (`zed-1.9.0`, `zed-1.25.2`)
and the closure files `zed-v1.9.0-closure.txt` /
`zed-v1.25.2-closure.txt` as the expected crate list. Do not re-derive
the closure with a different tool.

Step 1 — trial run at the old ref (importer acceptance, §1):

```sh
python3 scripts/import_gpui_upstream.py --ref v1.9.0 --skip gpui_wgpu --skip gpui_windows
# compare gpui + gpui_macos output against regen-proof; STOP on unexplained diff
diff -r gpui/crates/gpui /Volumes/home_tmp/tmp/zed-upstream-cache/regen-proof/gpui
git checkout -- gpui   # discard trial output
```

Step 2 — the real vendor run at v1.25.2:

```sh
python3 scripts/import_gpui_upstream.py --ref v1.25.2 --skip gpui_wgpu --skip gpui_windows
find gpui/crates -type f | sort > /Volumes/home_tmp/tmp/zed-upstream-cache/tree-post.txt
diff /Volumes/home_tmp/tmp/zed-upstream-cache/tree-pre.txt /Volumes/home_tmp/tmp/zed-upstream-cache/tree-post.txt
```

**`--skip` set (hand-maintained, class (e) — never importer-written):**

- `gpui_wgpu` — hand-maintained; carries the local `WgpuCustomDraw` half
  of the CustomDraw stack and the `sotf-wgpu` 29.0.0 pin.
- `gpui_windows` — hand-maintained; the only real consumer of
  `[workspace.dependencies]`; carries the WS_CHILD creation hook and
  `foreign_parent` module; hard-depends on `gpui_util::get_powershell`.

Skipped crates still need *manual* §3/§4 attention (API drift against the
new `gpui`), they are only skipped as importer targets.

**De-facto skip / restore set (files, not whole crates):** foreign-parent
files inside `gpui_linux` and `gpui_windows` (Linux stack 2814 lines,
commit `d7b2222`; Windows stack 419 lines, commit `dc7afc9`). The
importer will delete the `gpui_linux` ones; restore them from the step-0
snapshot in §3 and verify with the tree diff above. Any other file in
`tree-pre.txt` missing from `tree-post.txt` must be classified in §3
before proceeding — unexplained deletion is a STOP.

**New crates to admit (closure 12 -> 15):** `path`, `bench_metrics`,
`gpui_apple`. They arrive via the importer like any other vendored
crate and then go through the same §4 manifest pass. `gpui_apple` is the
macOS/iOS split carrying the old `gpui` + `gpui_macos` platform deps —
expect file moves out of both crates; §3 remaps patches accordingly.
No closure member is excluded and none is GPL-licensed, per
upstream-delta; do not add exclusions.

**Deleted upstream:** `media`. Remove the vendored crate and every
reference (workspace members, path deps, `FORKS.json`, vendor notes) in
§4; do not let the importer leave a stale copy behind.

**External-crate handling (what the vendor run must NOT do):**

- No `cargo update` / crates.io refresh in Phase A. The vendored tree
  currently has **no git dependencies**; the importer will introduce
  `git`+tag deps — §4 rewrites them back to the local policy. Upstream
  requirement bumps at v1.25.2 are the floor, not crates.io latest.
- 7 sibling collections are path-pinned outside `gpui/` and do not move
  with any refresh (e.g. `scap = ../scap`; siblings `../async-task`,
  `../block`, `../objc` exist but are consumed from the registry, not by
  path — leave exactly as-is unless §4.5 says otherwise). Never run
  `cargo update` against the siblings; their pins live in `FORKS.json`.
- Keep banned/transitive guards intact: `cargo tree -i spin -i
  proc-macro-error2` must stay empty and `stacksafe` must stay 1.x
  (dep-inventory §C). Check in §6, not during the vendor run.
- `wgpu`: upstream v1.25.2 wants 30.0.0 (gfx-rs rev `3b3506ea`); local
  `sotf-wgpu` is pinned at 29.0.0. Phase A does **not** silently take
  upstream wgpu — §4.4 records the decision gate. `taffy`: local
  `=0.14.0` is ahead of upstream 0.13; do not downgrade (§4.4).

---

## 3. Per-crate patch port checklist (drop-if-upstream marked)

Classification key (patch-surface): (a) behavioral — port to v1.25.2;
(b) upstream backport — **check at `d25fa01` and drop if present**;
(c) manifest-only policy — handled in §4, not as code patches;
(d) lint allows — re-apply verbatim; (e) hand-maintained,
do-not-overwrite; (f) unclassified — STOP and classify before porting.
Order within this section is dependency order: support crates, then
`gpui`, then platform/renderer crates. Tick each box only with the
`d25fa01` check for (b) items actually performed (symbol grep against
`/Volumes/home_tmp/tmp/zed-upstream-cache/zed-1.25.2`), recording
file remaps where `gpui_apple` extraction or wgpu bind-group caching
moved the code.

### 3.0 The six (b) backports — check first, port only if absent

All six merged upstream Jul–Sep 2026, and the critic reports their
symbols present at v1.25.2 — they are **expected DROP**, but each must
still be verified individually at `d25fa01`; a present symbol with a
changed signature is a port, not a drop.

- [ ] **DROP-IF-UPSTREAM** PR #56315 — grep its symbols in zed-1.25.2; drop local patch if present.
- [ ] **DROP-IF-UPSTREAM** PR #60482 — same check; watch for macOS atlas code moved into `gpui_apple`.
- [ ] **DROP-IF-UPSTREAM** PR #60574 — same check.
- [ ] **DROP-IF-UPSTREAM** PR #61707 — same check.
- [ ] **DROP-IF-UPSTREAM** PR #62743 — same check; watch for wgpu atlas / bind-group caching remap.
- [ ] **DROP-IF-UPSTREAM** PR #64623 — same check.

For each: record in the vendor notes either "present at d25fa01 in
<file>, local patch dropped" or "absent/changed, ported to <file>".
Never keep both the upstream change and the local backport.

### 3.1 Support crates (port first; everything depends on them)

- [ ] `collections` — unchanged v1.9.0 -> v1.25.2 (critic). Accept
      importer output as-is; only §4 manifest pass applies.
- [ ] `refineable` + `derive_refineable` — unchanged. Same handling.
      Note `syn` split (3.0.4 vs 2.0.101 in derive) is Phase B, not A.
- [ ] `gpui_shared_string` — changed upstream. Accept new sources;
      re-apply any local (a)/(d) items from the patch record; §4 pass.
- [ ] `gpui_util` — changed upstream. Accept new sources **but verify
      `get_powershell` still exists with the expected signature** —
      hand-maintained `gpui_windows` hard-depends on it. If upstream
      moved/renamed it, port a shim here (class (a)), do not edit
      `gpui_windows` to chase it silently.
- [ ] `media` — deleted upstream. Delete crate + all references (§4).
      Confirm no remaining path dep points at it (was a workspace path
      dep; `bindgen`/`core-video` etc. consumers were `media`-only).
- [ ] Remaining support crates in the closure (`sum_tree`, `scheduler`,
      `http_client`, `util`, `util_macros`, `gpui_macros`, `perf`,
      plus new `path`, `bench_metrics`): the packet gives no per-crate
      patch list for these beyond the dep inventory — for each, diff
      local vs zed-1.9.0 to enumerate local patches, classify (a)–(f),
      then port or drop per class. Any (f) item is a STOP (§7).

### 3.2 `gpui` (core)

- [ ] `CustomDraw` core API (class (a)) — port. This is a **4-crate
      stack that must move together**: `gpui` + `gpui_wgpu` +
      `gpui_macos` + `gpui_windows` (no-op). Do not land any leg alone;
      consumers are `WgpuCustomDraw` / `MetalCustomDraw` (§5).
- [ ] `ScrollAnchor` (class (a), **unrecorded** — absent from
      VENDORED/PATCHES records). Port from the step-0 snapshot; add it
      to the patch record this time so the next upgrade sees it.
- [ ] `clone_app_cell` (class (a), **unrecorded local patch**: critic
      proves it absent at both v1.9.0 and v1.25.2 upstream — it is not
      a backport and must not be dropped). Port; record it. Consumers:
      the 6 foreign-parent bindings' clone + sync-run pattern (§5).
- [ ] `run_embedded` surface (class (a) as consumed by
      `gpui-miniapp`, `gpui-hello-web`) — verify signature against
      v1.25.2 and port local deltas; record any drift for §5.
- [ ] (b) items landing in `gpui` from §3.0 — drop or port per check.
- [ ] (d) lint allows — re-apply verbatim; do not fix upstream lints
      in Phase A.
- [ ] Remap any patch whose target file moved into `gpui_apple`;
      record old -> new path in the vendor notes.

### 3.3 `gpui_apple` (new) + `gpui_macos`

- [ ] Admit `gpui_apple` (§2). It now carries the old `gpui` +
      `gpui_macos` macOS/iOS platform deps — re-home every local
      macOS patch to whichever of `gpui_apple` / `gpui_macos` now
      owns the file; never duplicate a patch in both.
- [ ] `MetalCustomDraw` leg of the CustomDraw stack (`gpui_macos`
      and/or `gpui_apple` after the split) — port with §3.2.
- [ ] (b) atlas-related backport(s) — **DROP-IF-UPSTREAM** after
      checking the post-split location (macos atlas may live in
      `gpui_apple` now).
- [ ] iOS-relevant code now shared via `gpui_apple`: flag every
      signature change for the `gpui-ios` / `gpui-au` consumers (§5).

### 3.4 `gpui_wgpu` — class (e), hand-maintained, skipped by importer

- [ ] Do NOT accept importer output. Manually reconcile against the
      new `gpui`: `WgpuCustomDraw` leg of the CustomDraw stack, atlas
      code (watch upstream bind-group caching — a (b) remap may make
      part of the local delta **DROP-IF-UPSTREAM**), and the
      `sotf-wgpu` 29.0.0 pin vs upstream wgpu 30.0.0 gate (§4.4).
- [ ] `InstanceUploadCache` fix (iOS; critic: absent upstream at
      v1.25.2, and absent from VENDORED/PATCHES records). This is a
      local (a) patch — port it forward explicitly and record it.
      It must not be treated as a backport.

### 3.5 `gpui_windows` — class (e), hand-maintained, skipped by importer

- [ ] Do NOT accept importer output. Preserve: WS_CHILD creation hook
      (gpui_windows fork), `foreign_parent` module, CustomDraw no-op
      leg, and its `[workspace.dependencies]` consumption.
- [ ] Foreign-parent Windows stack (419 lines, commit `dc7afc9`,
      **absent from VENDORED/PATCHES records** — de-facto class (e)).
      Reconcile manually against new `gpui`/`gpui_util`
      (`get_powershell` above); record the stack in the patch record.
- [ ] Re-check `windows 0.62` / `windows-core 0.62` + ~50-feature set
      still satisfies the fork code; manifest changes only via §4.

### 3.6 `gpui_linux`

- [ ] Accept importer output for upstream files, then **restore the
      foreign-parent Linux stack** (2814 lines, commit `d7b2222`,
      X11 child windows + Wayland subsurfaces, creation hooks in the
      gpui_linux fork; **absent from the patch records**, importer
      deletes it). Restore from the step-0 snapshot, port to the new
      `gpui` API, and record the stack in the patch record.
- [ ] Root `[patch]` sensitivity: `calloop 0.14.3` /
      `calloop-wayland-source 0.4.1` vs upstream fork bumps (§4.5) —
      verify the restored stack compiles against whichever pin §4.5
      lands on.
- [ ] (b)/(d) items from the patch record — drop-if-upstream / verbatim.

### 3.7 `gpui_web`

- [ ] Accept importer output; port local (a) items. Note:
      `gpui_web` foreign attach **does not exist yet** (§16 of the
      foreign-parent contract) — Phase A must not invent it; only
      keep existing web/wasm surfaces (incl. `run_embedded` consumers)
      compiling.
- [ ] wasm-only deps (`getrandom`, `wasm-bindgen`, `web-sys`, …) stay
      target-gated exactly as inventoried; no Phase B bumps.

### 3.8 Close-out for §3

- [ ] Every file in the §1.6 must-restore list is either restored,
      ported, or recorded as intentionally dropped (with the §3.0
      `d25fa01` evidence for drops). Zero unaccounted files.
- [ ] Update VENDORED.md / PATCHES records: add the four previously
      unrecorded stacks (`ScrollAnchor`, `clone_app_cell`, iOS
      `InstanceUploadCache`, both foreign-parent stacks) and the
      drop/port outcome of all six (b) PRs. This record update is a
      Phase A deliverable, not cleanup.

---

## 4. Manifest rename / version restoration

The importer emits unrenamed `publish = false` manifests with `git`+tag
deps; the committed policy is the `sotf-*` rename from commit `c3315a6`.
Implement this section as one checked-in scripted pass (new script,
e.g. `scripts/restore_sotf_manifests.py`) run over all 15 closure
crates **after** §2 and **before** any `cargo` command. The pre-vendor
manifests snapshotted in §2 step 0 are the template for every rule
below — diff the pass output against them rule by rule.

4.1 **Package rename.** Restore the `sotf-*` package names (and matching
`[lib] name` / path-dep `package =` aliases) exactly as in the
snapshotted manifests, for all previously vendored crates. New crates
(`path`, `bench_metrics`, `gpui_apple`) get the same treatment by the
same rule; record their new names in `FORKS.json` and the vendor notes.
Internal path deps must point at `crates/*` as today
(`collections, gpui, gpui_macros, gpui_shared_string, gpui_util,
gpui_wgpu, http_client, perf, refineable, scheduler, sum_tree, util,
util_macros` — minus `media`, plus the 3 new crates).

4.2 **Publish / version policy.** Restore the committed `publish`
setting and version numbers from the snapshot; never ship the
importer's `publish = false` + upstream versions. Bump the vendored
version marker to record v1.25.2 provenance wherever the snapshot
records v1.9.0 (vendor notes + `FORKS.json` pins).

4.3 **Kill every `git` dependency.** Rewrite each importer-emitted
`git`+tag dep back to the local form: internal crates -> `path`;
external crates -> the registry requirement from the snapshot (per-crate
reqs, since `[workspace.dependencies]` is nearly vestigial — only
`gpui_windows` consumes it). Gate: `grep -rn "git =" gpui/crates
gpui/Cargo.toml` returns nothing, matching today's "no git deps" state
(except a deliberate, recorded §4.4 wgpu decision).

4.4 **Requirement deltas — take upstream as floor, nothing more.**
Diff upstream v1.25.2 reqs (from `zed-delta-report.txt`) against the
dep inventory; where v1.25.2 requires newer than the snapshot, raise
the per-crate req to the upstream floor only. Explicit gates:

- `wgpu`: upstream 30.0.0 (gfx-rs rev `3b3506ea`) vs local
  `sotf-wgpu` 29.0.0. STOP and decide (recorded in vendor notes):
  either hold 29.0.0 and port `gpui_wgpu` shims, or adopt 30.0.0 and
  re-pin `sotf-wgpu` — Phase A is not green until one is chosen and
  `cargo check` passes with it. Default assumed in §8: hold local pin
  first, escalate only on compile proof.
- `taffy`: keep local `=0.14.0` (ahead of upstream 0.13; upstream
  moved 0.10 -> 0.13). Never downgrade; verify `gpui` layout code
  compiles against 0.14 as it does today.
- Exact pins stay exact until code compiles: `cocoa =0.26.0`,
  `cocoa-foundation =0.2.0`, `core-foundation =0.10.1`,
  `objc2-foundation =0.3.2` (and `taffy` above).
- Known divergences stay Phase B: `rand` 0.9 per-crate vs 0.10.2 in
  workspace deps; `syn` 3.0.4 vs 2.0.101 (`derive_refineable`);
  `anyhow`/`log`/`image` re-declared per crate. Do not "fix" them in
  Phase A beyond upstream floors; do not convert crates to
  `workspace = true` in Phase A.
- Guards: `stacksafe` stays 1.x; nothing may pull `spin` or
  `proc-macro-error2` (checked in §6).

4.5 **Root `[patch]` forks (importer does not propagate).** Compare
the v1.25.2 root `[patch]` (`calloop`, `async-task` fork bumps) with
the committed workspace root. Apply the bump only as a recorded manual
edit in `gpui/Cargo.toml` + `FORKS.json`, then re-verify §3.6. Note
`async-task 4.7` is currently consumed from the registry (sibling
`../async-task` exists but is not path-referenced) — keep that shape
unless the upstream patch entry forces otherwise, and say so.

4.6 **Toolchain.** Raise the pin 1.95.0 -> 1.98.1 (`rust-toolchain.toml`
as extracted from the v1.25.2 tarball). Edition / `rust-version`
unchanged — reject any importer output that changes them.

4.7 **Workspace shape.** Update workspace members: + `path`,
+ `bench_metrics`, + `gpui_apple`, − `media`. Keep
`[workspace.dependencies]` externals as snapshotted (`anyhow 1.0`,
`futures 0.3`, `image 0.25`, `itertools 0.15`, `log 0.4.28`,
`parking_lot 0.12`, `proptest 1.9`, `rand 0.10.2`, `smallvec 1.6`,
`uuid 1`, `windows 0.62`, `windows-core 0.62`) — they serve
`gpui_windows` (and `image`: `gpui_wgpu`/`gpui_windows`; `proptest`:
`gpui`/`sum_tree`) and Phase A does not widen their use.
`scap = ../scap` and the other 6 sibling path pins stay untouched.

4.8 **Regenerate the lockfile minimally.** `cargo update --dry-run`
first; then only `cargo update -p <dep>` for deps §4.4 moved (add
`--precise <ver>` where the floor must be exact). No blanket
`cargo update`. Diff `Cargo.lock` against
`Cargo.lock.pre-v1.25.2` and account for every hunk in the notes.

---

## 5. Compile-break order and consumer crates at risk

Fix compile breaks in this order (dependency order inside the vendored
workspace first, then toolkit consumers in break-first order from
consumer-impact). Do not chase consumer errors until the layer below
them checks green; most consumer breaks are signature drift from §3.

**Layer 0 — support crates.** `collections`, `refineable` (+derive),
`gpui_shared_string`, `gpui_util`, `sum_tree`, `scheduler`,
`http_client`, `util`, `util_macros`, `gpui_macros`, `perf`, `path`,
`bench_metrics`. Expected breaks: upstream req floors (§4.4),
`media` removal fallout, `gpui_util::get_powershell` drift.

**Layer 1 — `gpui` core.** Expected breaks: `gpui_apple` file moves,
CustomDraw / ScrollAnchor / `clone_app_cell` / `run_embedded` ports
(§3.2), `taffy` 0.14 held-pin friction, (b) drop/port fallout.

**Layer 2 — platform & renderer crates.** `gpui_apple`, `gpui_macos`,
`gpui_wgpu` (hand-maintained), `gpui_windows` (hand-maintained),
`gpui_linux` (incl. restored foreign-parent stack), `gpui_web`.
Expected breaks: CustomDraw legs, wgpu 29-vs-30 gate, atlas /
bind-group remaps, foreign-parent hook drift, `calloop` patch pin.

**Layer 3 — toolkit consumers, break-first order:**

1. **In-tree platform backends (break first):** `gpui-au`,
   `gpui-ios`, `gpui-android` — Android is duplicated via
   `SharedPlatform`, so one upstream platform-API change breaks it
   twice; fix the shared side once. iOS additionally depends on the
   `InstanceUploadCache` port (§3.4) and the `gpui_apple` split.
2. **Patched custom-draw signatures:** `gpui-d3rs`, `gpui-scene3d`
   via `WgpuCustomDraw` / `MetalCustomDraw`. Any signature change in
   the §3.2 stack lands here next; keep the 4-crate stack atomic.
3. **`run_embedded` consumers:** `gpui-miniapp`, `gpui-hello-web`.
4. **`clone_app_cell` + sync-run foreign pattern:** the 6
   foreign-parent bindings (incl. `gpui-foreign-parent`,
   `gpui-web-foreign`, `gpui-win-foreign`, `gpui-linux-foreign`).
   `gpui_web` foreign attach still does not exist (§16) — a missing
   attach here is pre-existing, not a Phase A regression.
5. **Fork-only `foreign_parent` modules** in `gpui_windows` /
   `gpui_linux` as consumed by the bindings above (§3.5/§3.6).

Not at risk (recorded non-issues): no toolkit crate consumes a gpui
`canvas_size()` helper, so its presence/absence upstream breaks
nothing. Everything else in the toolkit consumes `gpui` through the
layers above and is checked in §6 only after Layers 0–3 are green.

Per-layer loop: `cargo check -p <crate>` (vendored name), fix,
`cargo clippy -p <crate>`, then move up. Never silence a break by
re-adding a dropped (b) backport or by loosening a §4 pin.

---

## 6. Verification commands

Run in order; Phase A is done only when all of 6.1–6.5 pass and 6.6
is recorded. All cargo commands use
`CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home` and toolchain 1.98.1.

6.1 Vendor-policy gates (must pass before cargo):

```sh
cd /Volumes/home_worktrees/sotf-3rdparties-gpui-v1.25.2
python3 verify.py
./release.sh            # dry/verify mode if it offers one; STOP on any publish/git-dep complaint
grep -rn "git =" gpui/crates gpui/Cargo.toml        # expect: no output (§4.3)
grep -rn "publish = false" gpui/crates/*/Cargo.toml # expect: matches snapshot policy only
diff /Volumes/home_tmp/tmp/zed-upstream-cache/tree-pre.txt /Volumes/home_tmp/tmp/zed-upstream-cache/tree-post.txt  # every delta accounted in §3.8
```

6.2 Vendored workspace checks:

```sh
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo check --manifest-path gpui/Cargo.toml --workspace --all-targets
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo clippy --manifest-path gpui/Cargo.toml --workspace --all-targets
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo test --manifest-path gpui/Cargo.toml --workspace
```

6.3 Dependency guards (dep-inventory §C):

```sh
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo tree --manifest-path gpui/Cargo.toml -d
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo tree --manifest-path gpui/Cargo.toml -i spin -i proc-macro-error2   # expect: empty
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo tree --manifest-path gpui/Cargo.toml -i stacksafe                  # expect: 1.x only
```

Duplicate majors from `cargo tree -d` are acceptable only where the
dep inventory already records them (`rand`, `syn`); any *new*
duplicate is a Phase A regression to resolve, not waive.

6.4 Cross-target checks (per-OS / wasm surfaces the host check misses).
Use whatever targets CI already uses for this repo; at minimum cover
the target-gated dep groups in the inventory (macOS, Linux, Windows,
wasm). Pattern per target:

```sh
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo check --manifest-path gpui/Cargo.toml --workspace --all-targets --target <triple>
```

In gpui-toolkit, the wasm closure gate is:

```sh
just wasm-check
```

6.5 Consumer checks in gpui-toolkit (Layer 3, §5 order):

```sh
cd /Volumes/home_tmp/src/gpui-toolkit
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo check -p gpui-au -p gpui-ios -p gpui-android
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo check -p gpui-d3rs -p gpui-scene3d -p gpui-px
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo check -p gpui-miniapp -p gpui-hello-web
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo check -p gpui-foreign-parent -p gpui-web-foreign -p gpui-win-foreign -p gpui-linux-foreign
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo check --workspace
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo clippy --workspace
CARGO_HOME=/Volumes/home_tmp/tmp/cargo-home cargo run -p gpui-toolkit-cli -- doctor
```

(Crate names above follow the toolkit crate table; adjust to the exact
`-p` names `cargo metadata` reports. Also run the vendored-patches
release gate the critic inspected: `gpui-release-gates` /
`vendored_patches.rs`.)

6.6 Record (not a pass/fail gate): commit the updated VENDORED.md /
PATCHES records (§3.8), the `Cargo.lock` diff account (§4.8), the
§3.0 drop/port table, and the §4.4 wgpu decision. Update
`PREVIOUS_VENDOR_NOTES` so the next upgrade does not rediscover any
of §7.

Explicitly out of Phase A verification: `cargo outdated` / `cargo
upgrade` and any crates.io-latest comparison — that is Phase B
(dep-inventory §C steps 1–3) and must start from the green Phase A
tree, diffing upstream reqs against the inventory first.

---

## 7. Unresolved items (verbatim)

The evidence packet handed to this synthesis carried `"unresolved":[]`
at the top level, but the six prior-result bodies were not delivered.
Verbatim, from the runtime-prepared handoff:

> `{"label":4,"gap":"result belongs to another parent Run or Workflow"}`
> `{"label":5,"gap":"result belongs to another parent Run or Workflow"}`
> `{"label":1,"gap":"result belongs to another parent Run or Workflow"}`
> `{"label":3,"gap":"result belongs to another parent Run or Workflow"}`
> `{"label":2,"gap":"result belongs to another parent Run or Workflow"}`
> `{"label":6,"gap":"result belongs to another parent Run or Workflow"}`

> "import-tooling: large submission retained at prior result 4 in durable session storage / MSP subagent view; contents uninspected."
> "patch-surface: large submission retained at prior result 5 in durable session storage / MSP subagent view; contents uninspected."
> "upstream-delta: large submission retained at prior result 1 in durable session storage / MSP subagent view; contents uninspected."
> "consumer-impact: large submission retained at prior result 3 in durable session storage / MSP subagent view; contents uninspected."
> "dep-inventory: large submission retained at prior result 2 in durable session storage / MSP subagent view; contents uninspected."
> "critic: large submission retained at prior result 6 in durable session storage / MSP subagent view; contents uninspected."

Consequences kept open in this plan (resolve at execution, do not
guess): the full per-crate (a)–(f) patch lists beyond the items named
in §3; the exact importer flag spellings beyond `--ref`/`--skip`
(§2 step 0 `--help` gate); which specific PR in §3.0 maps to which
crate/file; and any (f) unclassified items. Each is a STOP-and-classify
gate where §3 says so.

---

## 8. Policy defaults assumed

Where the packet left a choice open, this plan assumes the following.
Change any of them explicitly in the vendor notes if execution proves
otherwise; do not drift silently.

1. **Scope.** Phase A = re-vendor + patch port + `cargo check` green.
   Phase B (crates.io refresh of ~144 per-crate req rows, optional
   `workspace = true` migration, `rand` 0.9->0.10 and `syn` 2->3
   unifications) is a separate, later change on top of green Phase A.
2. **No default case.** Unknown patch class, unknown file deletion, or
   unknown upstream symbol state = STOP and classify (§3 (f), §1.6).
   The plan never defaults an unknown item to "drop" or "port".
3. **Importer output is untrusted input.** Sources only; manifests
   always go through the §4 scripted pass; `verify.py`/`release.sh`
   are never loosened to accept importer output.
4. **Hand-maintained means importer-blind, not change-blind.**
   `gpui_wgpu`/`gpui_windows` (and foreign-parent files) are reconciled
   manually against the new `gpui`, with the reconciliation recorded.
5. **Drop-if-upstream requires local proof.** The critic's "(b)
   symbols present at 1.25" sets the expectation; each PR still gets
   its own `d25fa01` grep (§3.0) before dropping, with file remaps
   recorded (gpui_apple extraction, wgpu bind-group caching).
6. **Upstream reqs are a floor, local pins win ties.** Where local is
   ahead (`taffy =0.14.0`), keep local. Where upstream jumps a major
   the local stack pins (`wgpu` 30 vs `sotf-wgpu` 29.0.0), hold the
   local pin first and adopt upstream only on compile evidence,
   recorded as an explicit decision (§4.4).
7. **No git deps, no sibling bumps.** The 7 path-pinned sibling
   collections and `FORKS.json` pins move only via their own
   collections, never as a side effect of this vendor run.
8. **Locks are minimal.** Per-dep `cargo update -p` only (§4.8);
   `spin` / `proc-macro-error2` stay absent; `stacksafe` stays 1.x.
9. **Records are a deliverable.** The four unrecorded stacks
   (`ScrollAnchor`, `clone_app_cell`, iOS `InstanceUploadCache`,
   foreign-parent Windows/Linux) get written into VENDORED/PATCHES as
   part of Phase A, so no future importer run can delete them unseen.
10. **Verification style.** `cargo check` then `cargo clippy` then
    crate tests, per crate in §5 layer order, before any
    workspace-wide claim of green.





