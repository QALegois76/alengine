use wasm_bindgen::prelude::*;
use web_sys::{GpuBuffer, GpuBufferDescriptor, GpuDevice};

pub fn create_buffer_with_data<T>(
    device: &GpuDevice,
    data: &[T],
    usage: u32,
) -> Result<GpuBuffer, JsValue> {
    // Step: Typed CPU data to GPU buffer.
    let bytes = bytes_of(data);
    let buffer = device.create_buffer(&GpuBufferDescriptor::new(bytes.len() as u32, usage))?;
    device
        .queue()
        .write_buffer_with_u32_and_u8_slice(&buffer, 0, bytes)?;
    Ok(buffer)
}

fn bytes_of<T>(data: &[T]) -> &[u8] {
    // Step: Raw byte view.
    unsafe {
        std::slice::from_raw_parts(
            data.as_ptr().cast::<u8>(),
            std::mem::size_of_val(data),
        )
    }
}
