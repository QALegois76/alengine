// Composant Transform — position dans l'espace monde (local ou global).
//
// `LocalTransform`  : position relative au parent (ou monde si pas de parent).
// `GlobalTransform` : matrice monde calculée par TransformSystem (mise en cache).
//
// Le shader reçoit GlobalTransform.matrix via un uniform buffer (bind group 1).

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct LocalTransform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],    // quaternion xyzw
    pub scale: [f32; 3],
}

impl LocalTransform {
    pub fn identity() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    pub fn at(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: [x, y, z],
            ..Self::identity()
        }
    }
}

// Matrice monde 4×4 calculée à chaque frame par TransformSystem.
// Stockée séparément pour ne pas recalculer si le LocalTransform n'a pas changé.
// TODO : ajouter un flag `dirty` pour n'updater que les entités modifiées.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GlobalTransform {
    pub matrix: [[f32; 4]; 4],
}

impl GlobalTransform {
    pub fn identity() -> Self {
        Self {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}
