#[path = "3d_models/mod.rs"]
pub(crate) mod models_3d;
mod utils;

#[path = "./models/World.rs"]
pub(crate) mod models;
mod render;
mod shaders;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub async fn run() -> Result<(), JsValue> {
    utils::set_panic_hook();

    let mut renderer = render::Render::create().await?;
    
    let mut transform = models::Transform::identity();
    transform.x = 0.0;
    transform.y = 0.0;
    transform.z = 0.0;
    renderer.add_sphere(transform, None)?;
    
    renderer.draw_frame()?;

    Ok(())
}
