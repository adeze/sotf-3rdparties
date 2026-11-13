# Vello

- Upstream: https://github.com/linebender/vello
- Base: crates.io 0.10.0, cached locally; no network source was downloaded.
- Upstream commit: fc0baddd06c63287ef516180d276333aa2401e6e
- License: Apache-2.0 OR MIT; original license files and README are retained.
- Archive checksum: recorded in FORKS.json.

The renamed local GPUI renderer exposes types from sotf-wgpu. Upstream Vello
depends on the registry package wgpu, whose Device, Queue and TextureView types
have a different Cargo package identity. That prevented desktop chart compilation.

The original adaptation was a manifest-only adapter: the package is sotf-vello, consumers retain
the vello dependency alias, and its wgpu dependency selects ../wgpu/wgpu under
the wgpu alias. The macOS release lint repairs below are the additional Rust source delta.
Cargo.toml.orig records the upstream manifest.

Verify the desktop's actual selected feature combination with:
cargo check --manifest-path sotf/Cargo.toml -p sotf-gpui --features dev-api.
Native UI checks still follow that build. Optional upstream hot_reload and
wgpu-profiler closures are not qualified by this desktop check.

Owner: SOTF dependency maintainers. Review every 90 days and on GPU dependency
updates. Remove this adapter once the upstream renderer and GPUI can share the
selected GPU package identity directly.

macOS release checks (2026-10-30): strict all-target Clippy and cargo test
compilation pass. The packaged crate has zero unit tests and one ignored
doctest; nextest exits 4 for no tests. This is not runtime rendering coverage.
Removed five stale lifetime lint expectations, used standard Waker::noop,
and replaced RGBA slice chunks with fixed-size chunks and array destructuring.
Private issue: http://192.168.1.32:3001/pierre/sotf-3rdparties/issues/18.
Evidence: aggregate tmp/release-qa/vello-verified.log. Consumer adoption and
native rendering validation remain pending.
