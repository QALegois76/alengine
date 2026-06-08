mod utils;
#[path = "3d_models/mod.rs"]
pub(crate) mod models_3d;
mod render;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub async fn run() -> Result<(), JsValue> {
    utils::set_panic_hook();

    let renderer = render::Render::new().await?;
    renderer.draw_frame()?;

    Ok(())
}
