// Composant Camera — définit les paramètres de projection.
//
// L'entité Camera doit aussi posséder un LocalTransform (position / orientation).
// CameraSystem calcule view_proj = projection * view et met à jour le CameraUniform
// uploadé en bind group 0 de chaque render pass.
//
// Bind groups conventionnels :
//   group(0) : CameraUniform  (partagé pour toute la frame)
//   group(1) : ModelUniform   (par objet : model matrix + material params)

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Camera {
    pub fov_y_radians: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn perspective(fov_degrees: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        Self {
            fov_y_radians: fov_degrees.to_radians(),
            aspect_ratio,
            near,
            far,
        }
    }
}

// Données uploadées au GPU (uniform buffer, 144 bytes total).
// Doit être aligné sur 256 bytes pour WebGPU (remplissage implicite dans le buffer).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],   // 64 bytes
    pub view_position: [f32; 4],    // 16 bytes (w ignoré)
}

impl CameraUniform {
    pub fn identity() -> Self {
        Self {
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            view_position: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

// Contrôleur orbite (mise à jour depuis les events souris JS).
// Sera traité par InputSystem avant CameraSystem.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct OrbitController {
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw: f32,    // rotation horizontale (radians)
    pub pitch: f32,  // rotation verticale (radians)
}

impl OrbitController {
    pub fn new(distance: f32) -> Self {
        Self {
            target: [0.0, 0.0, 0.0],
            distance,
            yaw: 0.0,
            pitch: 0.3,
        }
    }
}
