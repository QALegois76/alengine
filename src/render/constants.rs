// WebGPU usage bits. `web-sys` exposes these constants as numeric flags in the
// JavaScript API shape, so keeping the values here makes buffer creation explicit.
pub const GPU_BUFFER_USAGE_COPY_DST: u32 = 0x8;
pub const GPU_BUFFER_USAGE_INDEX: u32 = 0x10;
pub const GPU_BUFFER_USAGE_VERTEX: u32 = 0x20;

// Ico sphere vertex layout:
// Previous step: `ico_sphere::mesh()` interleaves position and normal values.
// What this describes: one vertex is position.xyz followed by normal.xyz.
// What this influences: shader input locations. These offsets must match the
// WGSL `@location(0)` and `@location(1)` declarations.
// Next step: `pipeline.rs` turns these constants into a `GpuVertexBufferLayout`.
pub const POSITION_ATTRIBUTE_OFFSET: u32 = 0;
pub const NORMAL_ATTRIBUTE_OFFSET: u32 = 12;
pub const VERTEX_STRIDE: u32 = 24;
