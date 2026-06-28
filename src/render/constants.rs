// WebGPU usage bits. `web-sys` exposes these constants as numeric flags in the
// JavaScript API shape, so keeping the values here makes buffer creation explicit.
pub const GPU_BUFFER_USAGE_COPY_DST: u32 = 0x8;
pub const GPU_BUFFER_USAGE_INDEX: u32 = 0x10;
pub const GPU_BUFFER_USAGE_VERTEX: u32 = 0x20;
pub const GPU_BUFFER_USAGE_UNIFORM: u32 = 0x40;

pub const GPU_TEXTURE_USAGE_COPY_DST: u32 = 0x2;
pub const GPU_TEXTURE_USAGE_TEXTURE_BINDING: u32 = 0x4;
pub const GPU_TEXTURE_USAGE_RENDER_ATTACHMENT: u32 = 0x10;

// Visibilité des bindings dans un GpuBindGroupLayoutEntry (GPUShaderStage).
pub const GPU_SHADER_STAGE_VERTEX: u32 = 0x1;
pub const GPU_SHADER_STAGE_FRAGMENT: u32 = 0x2;

// Ico sphere vertex layout:
// Previous step: `ico_sphere::mesh()` interleaves position and normal values.
// What this describes: one vertex is position.xyz followed by normal.xyz.
// What this influences: shader input locations. These offsets must match the
// WGSL `@location(0)` and `@location(1)` declarations.
// Next step: `pipeline.rs` turns these constants into a `GpuVertexBufferLayout`.
pub const POSITION_ATTRIBUTE_OFFSET: u32 = 0;
pub const NORMAL_ATTRIBUTE_OFFSET: u32 = 12;
pub const VERTEX_STRIDE: u32 = 24;
