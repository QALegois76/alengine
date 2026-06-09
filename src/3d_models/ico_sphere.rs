use std::collections::HashMap;

pub const SHADER: &str = include_str!("ico_sphere.wgsl");
pub const SUBDIVISION_LEVELS: u32 = 3;
pub const BASE_VERTEX_COUNT: u32 = 12;
pub const BASE_INDEX_COUNT: u32 = 60;

pub struct MeshData {
    pub vertices: Vec<f32>,
    pub indices: Vec<u16>,
}

impl MeshData {
    // pub fn index_count(&self) -> u32 {
    //     self.indices.len() as u32
    // }
}

pub fn mesh() -> MeshData {
    let mut positions = POSITIONS.to_vec();
    let mut indices = INDICES.to_vec();

    let mut level = 0;
    while level < SUBDIVISION_LEVELS {
        let mut next_indices = Vec::with_capacity(indices.len() * 4);
        let mut midpoints = HashMap::new();

        for triangle in indices.chunks_exact(3) {
            let a = triangle[0];
            let b = triangle[1];
            let c = triangle[2];
            let ab = midpoint_index(a, b, &mut positions, &mut midpoints);
            let bc = midpoint_index(b, c, &mut positions, &mut midpoints);
            let ca = midpoint_index(c, a, &mut positions, &mut midpoints);

            next_indices.extend_from_slice(&[a, ab, ca, b, bc, ab, c, ca, bc, ab, bc, ca]);
        }

        indices = next_indices;
        level += 1;
    }

    MeshData {
        vertices: interleaved_vertices(&positions),
        indices,
    }
}

// Unit-radius icosahedron base data. Subdivision projects every new midpoint
// back onto the unit sphere.
pub const POSITIONS: [[f32; 3]; BASE_VERTEX_COUNT as usize] = [
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

pub const INDICES: [u16; BASE_INDEX_COUNT as usize] = [
    0, 11, 5, 0, 5, 1, 0, 1, 7, 0, 7, 10, 0, 10, 11, 1, 5, 9, 5, 11, 4, 11, 10, 2, 10, 7, 6, 7, 1,
    8, 3, 9, 4, 3, 4, 2, 3, 2, 6, 3, 6, 8, 3, 8, 9, 4, 9, 5, 2, 4, 11, 6, 2, 10, 8, 6, 7, 9, 8, 1,
];

fn midpoint_index(
    a: u16,
    b: u16,
    positions: &mut Vec<[f32; 3]>,
    midpoints: &mut HashMap<(u16, u16), u16>,
) -> u16 {
    let key = if a < b { (a, b) } else { (b, a) };
    if let Some(index) = midpoints.get(&key) {
        return *index;
    }

    let midpoint = normalize([
        (positions[a as usize][0] + positions[b as usize][0]) * 0.5,
        (positions[a as usize][1] + positions[b as usize][1]) * 0.5,
        (positions[a as usize][2] + positions[b as usize][2]) * 0.5,
    ]);
    let index = positions.len() as u16;
    positions.push(midpoint);
    midpoints.insert(key, index);
    index
}

fn interleaved_vertices(positions: &[[f32; 3]]) -> Vec<f32> {
    let mut vertices = Vec::with_capacity(positions.len() * 6);

    for position in positions {
        let normal = normalize(*position);
        vertices.extend_from_slice(position);
        vertices.extend_from_slice(&normal);
    }

    vertices
}

fn normalize(value: [f32; 3]) -> [f32; 3] {
    let length = (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt();
    [value[0] / length, value[1] / length, value[2] / length]
}
