//! Native window composition is not ported (see PATCHES.md). Retained
//! records still carry the composition surface a view was drawn in, which
//! is always `None` here, so retained reuse never has to exclude composed
//! content.

use crate::Window;

/// An opaque native-surface id; never constructed in this port.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CompositionSurfaceId(pub(crate) u64);

pub(crate) fn input_surface(_window: &Window) -> Option<CompositionSurfaceId> {
    None
}
