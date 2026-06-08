use wasm_bindgen::{prelude::*, JsCast};
use web_sys::HtmlCanvasElement;

use super::gpu_error;

pub fn canvas_from_document() -> Result<HtmlCanvasElement, JsValue> {
    // Step: Browser DOM access.
    let window = web_sys::window().ok_or_else(|| gpu_error("window is not available"))?;
    let document = window
        .document()
        .ok_or_else(|| gpu_error("document is not available"))?;
    let canvas = document
        .get_element_by_id("canvas")
        .ok_or_else(|| gpu_error("missing canvas element with id `canvas`"))?
        .dyn_into::<HtmlCanvasElement>()?;

    resize_canvas_to_css_size(&canvas);
    Ok(canvas)
}

fn resize_canvas_to_css_size(canvas: &HtmlCanvasElement) {
    // Step: Canvas pixel sizing.
    let width = canvas.client_width().max(1) as u32;
    let height = canvas.client_height().max(1) as u32;
    canvas.set_width(width);
    canvas.set_height(height);
}
