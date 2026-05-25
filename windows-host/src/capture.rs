// Frame capture.
//
// Phase 1: DXGI Desktop Duplication API on the primary monitor.
//   - Create D3D11 device.
//   - IDXGIOutput1::DuplicateOutput.
//   - Loop AcquireNextFrame -> shared texture.
//
// Phase 2: replace with IddCx swap-chain consumption (see docs/DESIGN.md §3.1).
//   The driver is a separate UMDF binary; this module receives shared
//   D3D11 texture handles over a named pipe and forwards them downstream
//   instead of running DDA itself.

use anyhow::Result;
use tracing::info;

pub fn probe_primary() -> Result<()> {
    info!("capture::probe_primary stub - DXGI Desktop Duplication wiring lands here");
    Ok(())
}
