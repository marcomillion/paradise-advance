//! Portable browser-facing components for the real Paradise emulator.
//! This is not a complete emulator: the CPU, memory bus, loader and peripherals
//! still need to be ported before firmware can execute.

mod display;
use wasm_bindgen::prelude::*;
pub use display::{DisplayController, LCD_HEIGHT, LCD_WIDTH};

#[wasm_bindgen]
pub struct BrowserDisplay { controller: DisplayController }

#[wasm_bindgen]
impl BrowserDisplay {
    #[wasm_bindgen(constructor)]
    pub fn new() -> BrowserDisplay { BrowserDisplay { controller: DisplayController::new() } }
    pub fn write_pixel(&mut self, x: u32, y: u32, rgb565: u16) {
        self.controller.write_vram_pixel(x as usize, y as usize, rgb565);
    }
    pub fn rgba_framebuffer(&self) -> Vec<u8> { self.controller.rgba_bytes() }
    pub fn width(&self) -> u32 { self.controller.width() as u32 }
    pub fn height(&self) -> u32 { self.controller.height() as u32 }
    pub fn is_dirty(&self) -> bool { self.controller.dirty() }
    pub fn clear_dirty(&mut self) { self.controller.clear_dirty(); }
}
impl Default for BrowserDisplay { fn default() -> Self { Self::new() } }
