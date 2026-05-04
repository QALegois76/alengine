#![allow(dead_code)]

pub const SHADER: &str = include_str!("ico_sphere.wgsl");
pub const VERTEX_COUNT: u32 = 12;
pub const INDEX_COUNT: u32 = 60;
pub const TRIANGLE_COUNT: u32 = 20;
pub const VERTEX_FLOAT_COUNT: usize = VERTEX_COUNT as usize * 6;

// Unit-radius icosahedron data. For this base ico sphere, normals are the same
// as normalized positions.
pub const POSITIONS: [[f32; 3]; VERTEX_COUNT as usize] = [
    [-0.5257311, 0.8506508, 0.0],
    [0.5257311, 0.8506508, 0.0],
    [-0.5257311, -0.8506508, 0.0],
    [0.5257311, -0.8506508, 0.0],
    [0.0, -0.5257311, 0.8506508],
    [0.0, 0.5257311, 0.8506508],
    [0.0, -0.5257311, -0.8506508],
    [0.0, 0.5257311, -0.8506508],
    [0.8506508, 0.0, -0.5257311],
    [0.8506508, 0.0, 0.5257311],
    [-0.8506508, 0.0, -0.5257311],
    [-0.8506508, 0.0, 0.5257311],
];

pub const NORMALS: [[f32; 3]; VERTEX_COUNT as usize] = POSITIONS;

pub const INDICES: [u16; INDEX_COUNT as usize] = [
    0, 11, 5,
    0, 5, 1,
    0, 1, 7,
    0, 7, 10,
    0, 10, 11,
    1, 5, 9,
    5, 11, 4,
    11, 10, 2,
    10, 7, 6,
    7, 1, 8,
    3, 9, 4,
    3, 4, 2,
    3, 2, 6,
    3, 6, 8,
    3, 8, 9,
    4, 9, 5,
    2, 4, 11,
    6, 2, 10,
    8, 6, 7,
    9, 8, 1,
];

pub fn interleaved_vertices() -> [f32; VERTEX_FLOAT_COUNT] {
    let mut vertices = [0.0; VERTEX_FLOAT_COUNT];
    let mut i = 0;

    while i < VERTEX_COUNT as usize {
        let vertex_offset = i * 6;
        vertices[vertex_offset] = POSITIONS[i][0];
        vertices[vertex_offset + 1] = POSITIONS[i][1];
        vertices[vertex_offset + 2] = POSITIONS[i][2];
        vertices[vertex_offset + 3] = NORMALS[i][0];
        vertices[vertex_offset + 4] = NORMALS[i][1];
        vertices[vertex_offset + 5] = NORMALS[i][2];
        i += 1;
    }

    vertices
}
