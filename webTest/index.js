import init, { Render, Transform } from "../pkg/alEngine.js";

const canvas    = document.querySelector("#canvas");
const status    = document.querySelector("#status");
const loading   = document.querySelector("#loading");
const modeBtn   = document.querySelector("#mode-btn");
const modeLabel = document.querySelector("#mode-label");
const hudHint   = document.querySelector("#hud-hint");

// ── Initialisation des dimensions canvas AVANT Render.create() ──────────────
// Le moteur lit canvas.width / canvas.height pour la depth texture et l'aspect.
function syncCanvasSize() {
    canvas.width  = Math.max(window.innerWidth,  1);
    canvas.height = Math.max(window.innerHeight, 1);
}
syncCanvasSize();

// ── Hints par mode ───────────────────────────────────────────────────────────
const HINTS = {
    orbit: "Gauche: orbite · Droit: pan · Scroll: zoom",
    fps:   "Clic gauche: regarder · WASD: déplacer · Space/Shift: monter/descendre",
};

function setMode(renderer) {
    const mode = renderer.camera_mode();
    modeLabel.textContent = mode === "orbit" ? "🔄 Orbit" : "🎮 FPS";
    hudHint.textContent   = HINTS[mode] ?? "";
}

// ── Démarrage ────────────────────────────────────────────────────────────────
try {
    if (!("gpu" in navigator)) {
        throw new Error("WebGPU n'est pas disponible dans ce navigateur. Essayez Chrome 113+ ou Edge 113+.");
    }

    await init();

    const renderer = await Render.create();

    // Sphère gauche — petite
    const t1 = Transform.identity();
    t1.x = -1.2; t1.sx = 0.5; t1.sy = 0.5; t1.sz = 0.5;
    renderer.add_sphere(t1, null);

    // Sphère droite — légèrement plus grande avec rotation
    const t2 = Transform.identity();
    t2.x = 1.2; t2.sx = 0.7; t2.sy = 0.7; t2.sz = 0.7;
    t2.rz = 0.707; t2.rw = 0.707;
    renderer.add_sphere(t2, null);

    // Sphère du haut — shader orange personnalisé
    const t3 = Transform.identity();
    t3.y = 1.4;
    renderer.add_sphere(t3, ORANGE_SHADER);

    // Cacher le spinner
    loading.classList.add("hidden");
    setMode(renderer);

    // ── Boucle de rendu ──────────────────────────────────────────────────────
    let lastTime = performance.now();

    function frame(now) {
        const dt = Math.min((now - lastTime) / 1000, 0.1);
        lastTime = now;
        try {
            renderer.tick(dt);
        } catch (e) {
            console.error("tick error:", e);
            status.textContent = "Erreur de rendu : " + (e?.message ?? String(e));
            return;
        }
        requestAnimationFrame(frame);
    }
    requestAnimationFrame(frame);

    // ── Événements souris ────────────────────────────────────────────────────
    canvas.addEventListener("contextmenu", e => e.preventDefault());

    canvas.addEventListener("mousemove", e => {
        renderer.on_mouse_move(e.movementX, e.movementY, e.buttons);
    });

    canvas.addEventListener("mousedown", e => {
        renderer.on_mouse_button(e.button, true);
        if (renderer.camera_mode() === "fps") canvas.requestPointerLock?.();
    });

    canvas.addEventListener("mouseup",   e => renderer.on_mouse_button(e.button, false));

    canvas.addEventListener("wheel", e => {
        e.preventDefault();
        renderer.on_scroll(e.deltaY);
    }, { passive: false });

    // ── Clavier ──────────────────────────────────────────────────────────────
    document.addEventListener("keydown", e => {
        renderer.on_key(e.code, true);
        if (e.code === "Tab") {
            e.preventDefault();
            renderer.toggle_camera_mode();
            setMode(renderer);
        }
    });
    document.addEventListener("keyup", e => renderer.on_key(e.code, false));

    // ── Bouton mode ──────────────────────────────────────────────────────────
    modeBtn.addEventListener("click", () => {
        renderer.toggle_camera_mode();
        setMode(renderer);
    });

    // ── Resize ───────────────────────────────────────────────────────────────
    window.addEventListener("resize", () => {
        syncCanvasSize();
        renderer.set_aspect(canvas.width / canvas.height);
        // TODO: recréer la depth texture au resize (nécessite API Rust supplémentaire)
    });

} catch (err) {
    console.error(err);
    loading.classList.add("hidden");
    status.textContent = err?.message ?? String(err);
}

// ── Shader orange — respecte les 2 bind groups ───────────────────────────────
const ORANGE_SHADER = `
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view_pos:  vec4<f32>,
}
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> model_matrix: mat4x4<f32>;

struct VertIn  { @location(0) position: vec3<f32>, @location(1) normal: vec3<f32> }
struct VertOut { @builtin(position) clip: vec4<f32>, @location(0) world_normal: vec3<f32>, @location(1) world_pos: vec3<f32> }

@vertex
fn vs_main(v: VertIn) -> VertOut {
    var o: VertOut;
    let wp    = model_matrix * vec4<f32>(v.position, 1.0);
    o.clip        = camera.view_proj * wp;
    o.world_pos   = wp.xyz;
    o.world_normal = normalize((model_matrix * vec4<f32>(v.normal, 0.0)).xyz);
    return o;
}

@fragment
fn fs_main(f: VertOut) -> @location(0) vec4<f32> {
    let light   = normalize(vec3<f32>(0.35, 0.55, 1.0));
    let view    = normalize(camera.view_pos.xyz - f.world_pos);
    let half_v  = normalize(light + view);
    let diffuse = max(dot(f.world_normal, light), 0.0);
    let spec    = pow(max(dot(f.world_normal, half_v), 0.0), 48.0) * 0.3;
    let color   = vec3<f32>(1.0, 0.45, 0.08) * (0.06 + diffuse * 0.85) + spec;
    return vec4<f32>(color, 1.0);
}
`;
