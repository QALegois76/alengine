use cgmath::{perspective, Deg, InnerSpace, Matrix4, Point3, SquareMatrix, Vector3, Vector4};

pub enum CameraMode {
    Orbit,
    Fps,
}

pub struct Camera {
    pub mode: CameraMode,

    // Projection
    pub fov_y: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,

    // Orbit state
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw: f32,   // rotation horizontale autour de Y (radians)
    pub pitch: f32, // élévation (radians, clampée à ±89°)

    // FPS state
    pub fps_position: [f32; 3],
    pub fps_yaw: f32,
    pub fps_pitch: f32,

    // Sensibilités
    pub orbit_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub fps_look_sensitivity: f32,
    pub fps_move_speed: f32,
}

impl Camera {
    pub fn default() -> Self {
        Self {
            mode: CameraMode::Orbit,
            fov_y: 60.0,
            aspect: 1.0,
            near: 0.01,
            far: 10_000.0,
            target: [0.0, 0.0, 0.0],
            distance: 3.5,
            yaw: 0.6,
            pitch: 0.35,
            fps_position: [0.0, 1.0, 4.0],
            fps_yaw: std::f32::consts::PI,
            fps_pitch: 0.0,
            orbit_sensitivity: 0.005,
            pan_sensitivity: 0.0015,
            zoom_sensitivity: 0.001,
            fps_look_sensitivity: 0.003,
            fps_move_speed: 5.0,
        }
    }

    // Position monde de la caméra en mode orbit.
    fn orbit_eye(&self) -> [f32; 3] {
        let d = self.distance;
        [
            self.target[0] + d * self.pitch.cos() * self.yaw.sin(),
            self.target[1] + d * self.pitch.sin(),
            self.target[2] + d * self.pitch.cos() * self.yaw.cos(),
        ]
    }

    // Vecteur avant de la caméra FPS.
    fn fps_forward(&self) -> Vector3<f32> {
        Vector3::new(
            self.fps_yaw.sin() * self.fps_pitch.cos(),
            self.fps_pitch.sin(),
            self.fps_yaw.cos() * self.fps_pitch.cos(),
        )
        .normalize()
    }

    // Vecteur droite de la caméra FPS (perpendiculaire au forward et à Y monde).
    fn fps_right(&self) -> Vector3<f32> {
        self.fps_forward()
            .cross(Vector3::new(0.0, 1.0, 0.0))
            .normalize()
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        match self.mode {
            CameraMode::Orbit => {
                let e = self.orbit_eye();
                Matrix4::look_at_rh(
                    Point3::new(e[0], e[1], e[2]),
                    Point3::new(self.target[0], self.target[1], self.target[2]),
                    Vector3::new(0.0, 1.0, 0.0),
                )
            }
            CameraMode::Fps => {
                let p = self.fps_position;
                let eye = Point3::new(p[0], p[1], p[2]);
                let fwd = self.fps_forward();
                Matrix4::look_at_rh(eye, eye + fwd, Vector3::new(0.0, 1.0, 0.0))
            }
        }
    }

    pub fn projection_matrix(&self) -> Matrix4<f32> {
        // cgmath génère une projection OpenGL (z ∈ [-1,1]).
        // WebGPU attend z ∈ [0,1] → correction de depth range.
        let correction: Matrix4<f32> = Matrix4::new(
            1.0, 0.0, 0.0, 0.0, // col 0
            0.0, 1.0, 0.0, 0.0, // col 1
            0.0, 0.0, 0.5, 0.0, // col 2
            0.0, 0.0, 0.5, 1.0, // col 3
        );
        correction * perspective(Deg(self.fov_y), self.aspect, self.near, self.far)
    }

    pub fn position(&self) -> [f32; 3] {
        match self.mode {
            CameraMode::Orbit => self.orbit_eye(),
            CameraMode::Fps => self.fps_position,
        }
    }

    // Rayon monde (origine = œil, direction unitaire) passant par le pixel NDC.
    pub fn screen_ray(&self, ndc_x: f32, ndc_y: f32) -> ([f32; 3], [f32; 3]) {
        let vp = self.projection_matrix() * self.view_matrix();
        let inv = vp.invert().unwrap_or_else(Matrix4::identity);
        let p = inv * Vector4::new(ndc_x, ndc_y, 0.0, 1.0);
        let near = Vector3::new(p.x / p.w, p.y / p.w, p.z / p.w);
        let eye = self.position();
        let dir = (near - Vector3::new(eye[0], eye[1], eye[2])).normalize();
        (eye, [dir.x, dir.y, dir.z])
    }

    // Réoriente l'orbite (centrée sur le globe) pour amener `point` au nadir,
    // c.-à-d. juste sous l'œil. Garde la cible (centre) et la distance ; seuls
    // yaw/pitch changent. Évite que la caméra n'entre dans le globe.
    pub fn orbit_to_point(&mut self, point: [f32; 3]) {
        let len =
            (point[0] * point[0] + point[1] * point[1] + point[2] * point[2]).sqrt().max(1e-6);
        let dx = point[0] / len;
        let dy = point[1] / len;
        let dz = point[2] / len;
        self.pitch = dy.clamp(-0.999, 0.999).asin();
        self.yaw = dx.atan2(dz);
    }

    // Données uploadées en uniform buffer :
    //   view_proj     (16 f32, offset 0)
    //   view_pos      (4 f32,  offset 64)
    //   inv_view_proj (16 f32, offset 80)   → 144 bytes
    // Les shaders qui n'ont besoin que des deux premiers champs déclarent un
    // struct de 80 octets : le binding plus grand reste valide.
    pub fn uniform_data(&self) -> [f32; 36] {
        let vp_mat = self.projection_matrix() * self.view_matrix();
        let vp: [[f32; 4]; 4] = vp_mat.into();
        let inv: [[f32; 4]; 4] = vp_mat.invert().unwrap_or_else(Matrix4::identity).into();
        let pos = self.position();
        [
            vp[0][0], vp[0][1], vp[0][2], vp[0][3],
            vp[1][0], vp[1][1], vp[1][2], vp[1][3],
            vp[2][0], vp[2][1], vp[2][2], vp[2][3],
            vp[3][0], vp[3][1], vp[3][2], vp[3][3],
            pos[0], pos[1], pos[2], 1.0,
            inv[0][0], inv[0][1], inv[0][2], inv[0][3],
            inv[1][0], inv[1][1], inv[1][2], inv[1][3],
            inv[2][0], inv[2][1], inv[2][2], inv[2][3],
            inv[3][0], inv[3][1], inv[3][2], inv[3][3],
        ]
    }

    // --- Contrôles orbit ---

    // Rotation autour de la cible. dx = horizontal, dy = vertical.
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * self.orbit_sensitivity;
        self.pitch = (self.pitch + dy * self.orbit_sensitivity)
            .clamp(-1.55, 1.55); // ±~89°
    }

    // Pan : déplace la cible dans le plan de la caméra.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        let view = self.view_matrix();
        // Vecteurs right/up dans l'espace monde depuis la vue (lignes de la rotation).
        let right = Vector3::new(view[0][0], view[1][0], view[2][0]);
        let up    = Vector3::new(view[0][1], view[1][1], view[2][1]);
        let scale = self.distance * self.pan_sensitivity;
        let offset = right * (-dx * scale) + up * (dy * scale);
        self.target[0] += offset.x;
        self.target[1] += offset.y;
        self.target[2] += offset.z;
    }

    // Zoom : rapproche/éloigne la caméra de la cible.
    // delta > 0 → zoom out, delta < 0 → zoom in (convention scroll wheel).
    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance * (1.0 + delta * self.zoom_sensitivity)).max(0.001);
    }

    // --- Contrôles FPS ---

    pub fn fps_look(&mut self, dx: f32, dy: f32) {
        self.fps_yaw   -= dx * self.fps_look_sensitivity;
        self.fps_pitch  = (self.fps_pitch - dy * self.fps_look_sensitivity)
            .clamp(-1.55, 1.55);
    }

    // forward/right/up sont des scalaires (-1, 0, 1) depuis les touches.
    pub fn fps_move(&mut self, forward: f32, right: f32, up: f32, dt: f32) {
        if forward == 0.0 && right == 0.0 && up == 0.0 {
            return;
        }
        let fwd = self.fps_forward();
        let rgt = self.fps_right();
        let spd = self.fps_move_speed * dt;
        self.fps_position[0] += (fwd.x * forward + rgt.x * right) * spd;
        self.fps_position[1] += up * spd;
        self.fps_position[2] += (fwd.z * forward + rgt.z * right) * spd;
    }

    // --- Toggle de mode ---

    pub fn toggle_mode(&mut self) {
        match self.mode {
            CameraMode::Orbit => {
                // Positionne la FPS à la position actuelle de la caméra orbit.
                let eye = self.orbit_eye();
                self.fps_position = eye;
                // La caméra orbit regarde vers la cible, on inverse pour le FPS.
                self.fps_yaw   = self.yaw + std::f32::consts::PI;
                self.fps_pitch = -self.pitch;
                self.mode = CameraMode::Fps;
            }
            CameraMode::Fps => {
                // Reprend la cible existante ; ajuste la distance depuis la position FPS.
                let pos = self.fps_position;
                let d = Vector3::new(
                    pos[0] - self.target[0],
                    pos[1] - self.target[1],
                    pos[2] - self.target[2],
                )
                .magnitude();
                self.distance = d.max(0.1);
                self.yaw   = self.fps_yaw - std::f32::consts::PI;
                self.pitch = -self.fps_pitch;
                self.mode = CameraMode::Orbit;
            }
        }
    }

    pub fn mode_name(&self) -> &'static str {
        match self.mode {
            CameraMode::Orbit => "orbit",
            CameraMode::Fps => "fps",
        }
    }
}
