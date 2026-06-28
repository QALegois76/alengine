// Système de coordonnées géographiques pour la visualisation SIG.
//
// - Interface `CoordinateTransform` : transforme des coordonnées terrestres
//   (lon/lat, ou d'autres CRS ajoutés plus tard) en position monde WebGPU.
// - Maths de tuiles Web Mercator (TILEMATRIXSET=PM) pour découper l'imagerie.
// - Génération de mailles courbées drapées sur le globe (sphère centrée origine).
//
// Modèle borné simple : globe de rayon fixe `GLOBE_RADIUS` centré à l'origine.
// Convention : pôle nord = +Y, longitude 0° = +Z, longitude 90°E = +X.

use std::f64::consts::PI;

// Rayon du globe en unités monde.
pub const GLOBE_RADIUS: f32 = 1.0;

// Interface de transformation de coordonnées terrestres → monde WebGPU.
// D'autres CRS (Lambert, UTM, …) implémenteront ce trait par la suite.
pub trait CoordinateTransform {
    // lon/lat en degrés, alt en unités monde au-dessus de la surface.
    fn to_world(&self, lon: f64, lat: f64, alt: f64) -> [f32; 3];
}

// Transform géographique WGS84 (lon/lat) → point sur la sphère.
pub struct Geographic {
    pub radius: f64,
}

impl CoordinateTransform for Geographic {
    fn to_world(&self, lon: f64, lat: f64, alt: f64) -> [f32; 3] {
        lonlat_to_world(lon, lat, self.radius + alt)
    }
}

// lon/lat (deg) + rayon → position cartésienne monde.
pub fn lonlat_to_world(lon_deg: f64, lat_deg: f64, radius: f64) -> [f32; 3] {
    let lon = lon_deg.to_radians();
    let lat = lat_deg.to_radians();
    let cos_lat = lat.cos();
    [
        (radius * cos_lat * lon.sin()) as f32,
        (radius * lat.sin()) as f32,
        (radius * cos_lat * lon.cos()) as f32,
    ]
}

// Direction monde unitaire → (lon, lat) en degrés (inverse de lonlat_to_world).
pub fn world_dir_to_lonlat(dir: [f32; 3]) -> (f64, f64) {
    let x = dir[0] as f64;
    let y = dir[1] as f64;
    let z = dir[2] as f64;
    let lat = y.clamp(-1.0, 1.0).asin().to_degrees();
    let lon = x.atan2(z).to_degrees();
    (lon, lat)
}

// Latitude (deg) de la ligne horizontale de tuile `y` à `n = 2^z` tuiles.
fn mercator_tile_lat(y: f64, n: f64) -> f64 {
    (PI * (1.0 - 2.0 * y / n)).sinh().atan().to_degrees()
}

// Bornes (lon0, lon1, lat_haut, lat_bas) en degrés d'une tuile Web Mercator.
pub fn tile_bounds(z: u32, x: u32, y: u32) -> (f64, f64, f64, f64) {
    let n = (1u64 << z) as f64;
    let lon0 = x as f64 / n * 360.0 - 180.0;
    let lon1 = (x as f64 + 1.0) / n * 360.0 - 180.0;
    let lat_top = mercator_tile_lat(y as f64, n);
    let lat_bottom = mercator_tile_lat(y as f64 + 1.0, n);
    (lon0, lon1, lat_top, lat_bottom)
}

// (lon, lat) deg + zoom → coordonnées de tuile fractionnaires (col, row).
pub fn lonlat_to_tile(lon: f64, lat: f64, z: u32) -> (f64, f64) {
    let n = (1u64 << z) as f64;
    let col = (lon + 180.0) / 360.0 * n;
    let lat_rad = lat.to_radians();
    let row = (1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / PI) / 2.0 * n;
    (col, row)
}

// Maille courbée d'une tuile drapée sur le globe.
// Sommets interleavés [x, y, z, u, v, 0] (UV dans le 2e vec3), indices triangles.
pub fn build_tile_mesh(z: u32, x: u32, y: u32, radius: f64) -> (Vec<f32>, Vec<u16>) {
    // Plus de subdivisions à bas zoom (grandes tuiles très courbées).
    let seg: u32 = if z <= 3 { 16 } else { 8 };
    let (lon0, lon1, lat_top, lat_bottom) = tile_bounds(z, x, y);

    let mut vertices = Vec::with_capacity(((seg + 1) * (seg + 1) * 6) as usize);
    for j in 0..=seg {
        let v = j as f64 / seg as f64; // v=0 au nord (lat_top)
        let lat = lat_top + v * (lat_bottom - lat_top);
        for i in 0..=seg {
            let u = i as f64 / seg as f64; // u=0 à l'ouest (lon0)
            let lon = lon0 + u * (lon1 - lon0);
            let p = lonlat_to_world(lon, lat, radius);
            vertices.extend_from_slice(&[p[0], p[1], p[2], u as f32, v as f32, 0.0]);
        }
    }

    let mut indices = Vec::with_capacity((seg * seg * 6) as usize);
    let stride = seg + 1;
    for j in 0..seg {
        for i in 0..seg {
            let a = (j * stride + i) as u16;
            let b = (j * stride + i + 1) as u16;
            let c = ((j + 1) * stride + i) as u16;
            let d = ((j + 1) * stride + i + 1) as u16;
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }
    (vertices, indices)
}
