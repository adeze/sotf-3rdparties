//! Canvas backing-store sizing for the web resize path.
//!
//! Kept free of `web_sys` types so the size resolution stays unit-testable on
//! the host; the rest of the crate is `wasm`-only.

/// Tolerance for accepting the browser-reported device-pixel box.
///
/// In the normal case the box equals `round(css * dpr)` exactly; the 1px
/// slack absorbs implementations that snap the box to the element's
/// fractional offset instead of scaling the size alone. Divergence under DPR
/// emulation grows with the scale gap (tens of pixels on real windows), far
/// above this slack, so emulated sizes still take the computed branch.
/// Raising this risks accepting an emulated box; lowering it to zero risks
/// rejecting exact-but-differently-snapped boxes.
const DEVICE_PIXEL_AGREEMENT_TOLERANCE: f64 = 1.0;

/// Resolve the canvas backing-store size from live resize inputs.
///
/// Prefers the browser-reported device-pixel box when it agrees with
/// `round(css * dpr)` and falls back to the computed size when the two
/// diverge, as happens under DevTools DPR emulation where the box tracks the
/// real display while `devicePixelRatio` reports the emulated value.
/// Float-to-int conversions saturate, so out-of-range inputs yield a clamped
/// size instead of panicking. Verified by the unit tests below.
pub(crate) fn resolve_physical_size(
    css_width: f64,
    css_height: f64,
    device_width: f64,
    device_height: f64,
    dpr: f64,
) -> (u32, u32) {
    (
        resolve_extent(css_width, device_width, dpr),
        resolve_extent(css_height, device_height, dpr),
    )
}

fn resolve_extent(css: f64, device: f64, dpr: f64) -> u32 {
    let expected = (css * dpr).round();
    if (device - expected).abs() <= DEVICE_PIXEL_AGREEMENT_TOLERANCE {
        device.round() as u32
    } else {
        expected as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_browser_size_when_scales_agree() {
        // Real display DPR 2, no emulation.
        assert_eq!(
            resolve_physical_size(800.0, 600.0, 1600.0, 1200.0, 2.0),
            (1600, 1200)
        );
    }

    #[test]
    fn prefers_computed_size_under_dpr_emulation() {
        // Real display DPR 2, emulated DPR 3: the device box still reports
        // 1600x1200 while the live DPR is 3.
        assert_eq!(
            resolve_physical_size(800.0, 600.0, 1600.0, 1200.0, 3.0),
            (2400, 1800)
        );
    }

    #[test]
    fn handles_emulated_dpr_below_real_scale() {
        assert_eq!(
            resolve_physical_size(800.0, 600.0, 1600.0, 1200.0, 1.0),
            (800, 600)
        );
    }

    #[test]
    fn tolerates_off_by_one_browser_snapping() {
        assert_eq!(
            resolve_physical_size(100.0, 100.0, 201.0, 199.0, 2.0),
            (201, 199)
        );
    }

    #[test]
    fn resolves_fractional_css_sizes() {
        assert_eq!(
            resolve_physical_size(100.5, 50.25, 201.0, 100.0, 2.0),
            (201, 100)
        );
    }

    #[test]
    fn clamps_degenerate_inputs_to_zero() {
        assert_eq!(resolve_physical_size(0.0, 0.0, 0.0, 0.0, 2.0), (0, 0));
        assert_eq!(resolve_physical_size(-10.0, 5.0, 0.0, 10.0, 2.0), (0, 10));
        assert_eq!(
            resolve_physical_size(800.0, 600.0, 1600.0, 1200.0, f64::NAN),
            (0, 0)
        );
    }
}
