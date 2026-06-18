import init, { run, Render, Transform } from "../pkg/alEngine.js";

const status = document.querySelector("#status");

try {
    console.log("Hello alEngine");
    if (!("gpu" in navigator)) {
        throw new Error("WebGPU is not available in this browser.");
    }

    await init();
    
    // You can still call the default run()
    // await run();

    // Or use the new API to add spheres manually
    const renderer = await Render.create();

    // Add a default sphere
    const t1 = Transform.identity();
    t1.x = -0.8;
    t1.sx = 0.5;
    t1.sy = 0.5;
    t1.sz = 0.5;
    renderer.add_sphere(t1, null);

    // Add a sphere with custom rotation and scale
    const t2 = Transform.identity();
    t2.x = 0.8;
    t2.sx = 0.7;
    t2.sy = 0.7;
    t2.sz = 0.7;
    // Simple rotation around Z axis (quat: [0, 0, sin(pi/4), cos(pi/4)])
    t2.rz = 0.707;
    t2.rw = 0.707;
    renderer.add_sphere(t2, null);

    // Add a sphere with a custom shader
    const customShader = `
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> model_matrix: mat4x4<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = model_matrix * vec4<f32>(input.position, 1.0);
    output.normal = (model_matrix * vec4<f32>(input.normal, 0.0)).xyz;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.5, 0.2, 1.0); // Orange color
}
    `;
    const t3 = Transform.identity();
    t3.y = 0.8;
    renderer.add_sphere(t3, customShader);

    renderer.draw_frame();

} catch (error) {
    console.error(error);
    status.textContent = error?.message ?? String(error);
}
