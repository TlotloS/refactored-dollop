// WinExt — library entry point.
// Modules are mostly Windows-specific stubs for now (capture / encoder
// / input lands in M2..M4). The transport module is platform-agnostic
// and is what M1 fleshes out.

pub mod capture;
pub mod encoder;
pub mod input;
pub mod transport;
