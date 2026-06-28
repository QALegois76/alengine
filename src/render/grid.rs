// Repères de debug, style Blender — tout est calculé en plein écran (un triangle
// couvrant l'écran), sans géométrie longue, donc vraiment infini et antialiasé.
//
// - GRID_SHADER : grille de plan (XY/XZ/YZ) avec LOD continu (les lignes se
//   subdivisent en douceur au zoom), fondu radial + fondu rasant.
// - AXIS_SHADER : un axe coloré infini (X rouge / Y vert / Z bleu), tracé sur le
//   plan qui le contient le plus face caméra, antialiasé et fondu.
// - LINE_SHADER + origin_mesh : petit cube fil-de-fer marquant l'origine.
//
// Les deux shaders plein écran reconstruisent le rayon monde de chaque pixel via
// camera.inv_view_proj puis l'intersectent avec le plan voulu, et écrivent
// frag_depth pour s'intégrer correctement à la profondeur de la scène.

// ── Shader lignes colorées (origine) ─────────────────────────────────────────
pub const LINE_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view_pos:  vec4<f32>,
}
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> model_matrix: mat4x4<f32>;

struct VIn  { @location(0) position: vec3<f32>, @location(1) color: vec3<f32> }
struct VOut { @builtin(position) clip: vec4<f32>, @location(0) color: vec3<f32> }

@vertex
fn vs_main(v: VIn) -> VOut {
    var o: VOut;
    o.clip  = camera.view_proj * model_matrix * vec4<f32>(v.position, 1.0);
    o.color = v.color;
    return o;
}

@fragment
fn fs_main(i: VOut) -> @location(0) vec4<f32> {
    return vec4<f32>(i.color, 1.0);
}
"#;

// Préambule commun aux shaders plein écran (caméra + triangle + utilitaires).
const FULLSCREEN_HEADER: &str = r#"
struct CameraUniform {
    view_proj:     mat4x4<f32>,
    view_pos:      vec4<f32>,
    inv_view_proj: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct Params { info: vec4<u32> }   // info.x : id (plan ou axe)
@group(1) @binding(0) var<uniform> params: Params;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) near_pt: vec3<f32>,
}

struct FOut {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

fn unproject(xy: vec2<f32>, z: f32) -> vec3<f32> {
    let p = camera.inv_view_proj * vec4<f32>(xy, z, 1.0);
    return p.xyz / p.w;
}

// Rayon monde du pixel : depuis l'œil, vers le point du plan proche.
// On évite le point du plan lointain (mal conditionné avec un far élevé →
// direction bruitée → les lignes fines "tremblent").
fn pixel_ray_dir(near_pt: vec3<f32>) -> vec3<f32> {
    return normalize(near_pt - camera.view_pos.xyz);
}

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let ndc = corners[vid];
    var o: VOut;
    o.clip    = vec4<f32>(ndc, 0.0, 1.0);
    o.near_pt = unproject(ndc, 0.0);
    return o;
}

fn log10(x: f32) -> f32 { return log2(x) / log2(10.0); }

// Couverture antialiasée d'une grille de pas `cell`.
fn grid_cov(coord: vec2<f32>, cell: f32) -> f32 {
    let c = coord / cell;
    let d = fwidth(c);
    let g = abs(fract(c - 0.5) - 0.5) / max(d, vec2<f32>(1e-8));
    return 1.0 - clamp(min(g.x, g.y), 0.0, 1.0);
}
"#;

// ── Shader grille infinie (plan XY / XZ / YZ) ────────────────────────────────
pub fn grid_shader() -> String {
    let body = r#"
@fragment
fn fs_main(in: VOut) -> FOut {
    let pid = params.info.x;
    let ro = camera.view_pos.xyz;
    let rd = pixel_ray_dir(in.near_pt);

    var normal: vec3<f32>;
    if (pid == 0u) { normal = vec3<f32>(0.0, 0.0, 1.0); }      // XY
    else if (pid == 1u) { normal = vec3<f32>(0.0, 1.0, 0.0); } // XZ (sol)
    else { normal = vec3<f32>(1.0, 0.0, 0.0); }                // YZ

    let denom = dot(rd, normal);
    let safe  = select(denom, 1e-6, abs(denom) < 1e-6);
    let t     = -dot(ro, normal) / safe;
    if (t <= 0.0) { discard; }
    let world = ro + t * rd;

    let cam = camera.view_pos.xyz;
    var coord: vec2<f32>;
    var camc:  vec2<f32>;
    if (pid == 0u) { coord = world.xy; camc = cam.xy; }
    else if (pid == 1u) { coord = world.xz; camc = cam.xz; }
    else { coord = world.yz; camc = cam.yz; }

    // LOD continu : cellule = puissance de 10 telle que les lignes fines fassent
    // au moins ~8 px (jamais d'aliasing), avec fondu doux entre décades (pas de
    // "pop"). Permet des cellules < 1 unité en zoom rapproché.
    let w_pp = max(fwidth(coord).x, fwidth(coord).y); // unités monde par pixel
    let lod  = log10(max(w_pp * 8.0, 1e-8));
    let f    = ceil(lod) - lod;        // fondu du niveau fin dans la décade
    let c0   = pow(10.0, ceil(lod));   // cellules fines (≥ ~8 px)
    let c1   = c0 * 10.0;              // cellules larges (toutes les 10)

    let g0 = grid_cov(coord, c0);
    let g1 = grid_cov(coord, c1);
    let intensity = max(g1, g0 * f);

    // Fondu radial (en cellules → rayon écran ~constant) + fondu rasant.
    let radial = length(coord - camc) / c1;
    let fade   = 1.0 - smoothstep(25.0, 55.0, radial);
    let graze  = abs(dot(normalize(rd), normal));
    let gfade  = smoothstep(0.0, 0.10, graze);

    let alpha = intensity * fade * gfade;
    if (alpha < 0.003) { discard; }

    let color = vec3<f32>(0.36, 0.38, 0.43);
    let clip  = camera.view_proj * vec4<f32>(world, 1.0);

    var out: FOut;
    out.depth = clamp(clip.z / clip.w, 0.0, 1.0);
    out.color = vec4<f32>(color, alpha);
    return out;
}
"#;
    format!("{FULLSCREEN_HEADER}{body}")
}

// ── Shader axe infini (X / Y / Z) ────────────────────────────────────────────
pub fn axis_shader() -> String {
    let body = r#"
@fragment
fn fs_main(in: VOut) -> FOut {
    let aid = params.info.x;
    var d: vec3<f32>;
    var color: vec3<f32>;
    if (aid == 0u) { d = vec3<f32>(1.0, 0.0, 0.0); color = vec3<f32>(0.91, 0.30, 0.34); }
    else if (aid == 1u) { d = vec3<f32>(0.0, 1.0, 0.0); color = vec3<f32>(0.55, 0.78, 0.27); }
    else { d = vec3<f32>(0.0, 0.0, 1.0); color = vec3<f32>(0.27, 0.49, 0.90); }

    let ro = camera.view_pos.xyz;
    let rd = pixel_ray_dir(in.near_pt);

    // Points/distance les plus proches entre le rayon (ro, rd) et l'axe (0, d).
    // Méthode stable : aucune sélection de plan (donc pas de clignotement).
    let b     = dot(rd, d);
    let denom = max(1.0 - b * b, 1e-6);   // → 0 si on regarde le long de l'axe
    let rdo   = dot(rd, ro);
    let dro   = dot(d, ro);
    let sc    = (b * dro - rdo) / denom;  // paramètre le long du rayon
    let tc    = (dro - b * rdo) / denom;  // paramètre le long de l'axe

    let pr = ro + sc * rd;                // point du rayon le plus proche
    let pa = tc * d;                      // point de l'axe le plus proche
    let dist = length(pr - pa);           // distance rayon ↔ axe (monde)

    let dpix = fwidth(dist);
    let cov  = 1.0 - clamp(dist / max(dpix * 1.5, 1e-8), 0.0, 1.0);

    if (sc <= 0.0 || cov < 0.01) { discard; }

    let clip = camera.view_proj * vec4<f32>(pa, 1.0);
    var out: FOut;
    out.depth = clamp(clip.z / clip.w, 0.0, 1.0);
    out.color = vec4<f32>(color, cov);
    return out;
}
"#;
    format!("{FULLSCREEN_HEADER}{body}")
}

// ── Géométrie lignes (origine) ───────────────────────────────────────────────

type Seg = ([f32; 3], [f32; 3], [f32; 3]);

fn build(segments: &[Seg]) -> (Vec<f32>, Vec<u16>) {
    let mut vertices = Vec::with_capacity(segments.len() * 12);
    let mut indices = Vec::with_capacity(segments.len() * 2);
    for (start, end, color) in segments {
        let base = (vertices.len() / 6) as u16;
        vertices.extend_from_slice(&[start[0], start[1], start[2], color[0], color[1], color[2]]);
        vertices.extend_from_slice(&[end[0], end[1], end[2], color[0], color[1], color[2]]);
        indices.push(base);
        indices.push(base + 1);
    }
    (vertices, indices)
}

// Petit cube fil-de-fer blanc marquant l'origine du monde (0, 0, 0).
pub fn origin_mesh(size: f32) -> (Vec<f32>, Vec<u16>) {
    let h = size * 0.5;
    let c = [0.95_f32, 0.95, 0.95];
    let v = [
        [-h, -h, -h], [h, -h, -h], [h, -h, h], [-h, -h, h],
        [-h, h, -h],  [h, h, -h],  [h, h, h],  [-h, h, h],
    ];
    let edges = [
        (0, 1), (1, 2), (2, 3), (3, 0),
        (4, 5), (5, 6), (6, 7), (7, 4),
        (0, 4), (1, 5), (2, 6), (3, 7),
    ];
    let segs: Vec<Seg> = edges.iter().map(|(a, b)| (v[*a], v[*b], c)).collect();
    build(&segs)
}
