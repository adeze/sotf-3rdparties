# block Vendoring Notes

## Upstream

- Project: https://github.com/SSheldon/rust-block
- Base crate: `block` 0.1.6 from crates.io
- License: MIT as declared by the upstream crate manifest.
- Local status: consolidated `sotf-block` fork. Consumers requiring the upstream
  package name use compatibility revision
  `5a4634af0916b5f53effa5ef6340efd90af32285` of the collection.
- Last reviewed: 2026-10-30

## Why Vendored

`block` 0.1.6 declares the Objective-C runtime symbol
`_NSConcreteStackBlock` as a Rust static with an uninhabited `Class` type.
Current Rust accepts this with a future-incompatibility warning, but Cargo
reports that it will become a hard error in a future compiler.

The workspace uses `block` through Zed/GPUI's Objective-C and Cocoa dependency
chain, so the release cannot leave this warning invisible.

## Local Changes

- Keep the upstream `Class` marker for block layout pointers.
- Declare `_NSConcreteStackBlock` as an opaque `u8` extern static.
- Store the block `isa` pointer with `ptr::addr_of!(_NSConcreteStackBlock).cast::<Class>()`.
- Use explicit `extern "C"` ABI spellings for block invoke function pointers.
- Add a local `[workspace]` table so the vendored crate can be checked directly
  without being treated as an undeclared member of the repository workspace.
- Restore the upstream `objc_test_utils` helper from rust-block revision
  `642ea4a4a5853a21b55b05c34832a5f1bb1af61c` so native C block interop tests run.
  Its four files are under `test_utils/`; the C source is unchanged. The helper
  uses current `cc` instead of deprecated `gcc`, explicit Rust 2015 and C ABI,
  and the collection package prefix `sotf-` (library name unchanged).
- Document existing unsafe API contracts under Safety headings, spell out
  existing function-pointer transmute types, and use field shorthand. No block
  ABI, ownership behavior or callback semantics changed.
- Make the upstream default Rust 2015 edition explicit so direct Cargo checks
  do not emit an edition-default warning.

## Upgrade Guidance

1. Check whether upstream `block` or GPUI's dependency stack has removed the
   `block` 0.1.x dependency.
2. If this crate is still required, reapply only the opaque-symbol change above.
3. Run:
   - `cargo check --manifest-path sotf-3rdparties/block/Cargo.toml --lib`
   - a dependent GPUI/toolkit check
   - `cargo report future-incompatibilities --id <build-id>` if Cargo emits a
     future-incompatibility report.
4. Remove this vendored patch if the dependency is no longer present or the
   upstream crate carries an equivalent fix.

## Verification

Run the direct manifest check and dependent GPUI/toolkit check listed in the
upgrade guidance. Review the delta with
`git diff block-0.1.6 -- sotf-3rdparties/block` before updating the base.

Release review on 2026-10-30 verified strict workspace Clippy, tests, and
nextest, including six native block interop tests and two doctests with no
skips. The restored helper is covered by the workspace run. Dependent GPUI
validation also passed with the upstream-name compatibility revision and no
future-incompatibility warning. See the aggregate release-progress.md entries
and block-workspace-final.log for the recorded verification.
