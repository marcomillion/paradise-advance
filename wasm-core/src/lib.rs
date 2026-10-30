//! Portable browser-facing components for the real Paradise emulator.
//! Firmware inspection is real; CPU execution and decryption are not yet integrated.

mod display;
mod sonix;
mod memory;
use wasm_bindgen::prelude::*;
pub use display::{DisplayController, LCD_HEIGHT, LCD_WIDTH};
pub use sonix::{find_tables, CipherMode, LoadTable, Region};
pub use memory::{MemoryStorage, RegionKind};

#[wasm_bindgen]
pub struct BrowserDisplay { controller: DisplayController }
#[wasm_bindgen]
impl BrowserDisplay {
    #[wasm_bindgen(constructor)]
    pub fn new() -> BrowserDisplay { BrowserDisplay { controller: DisplayController::new() } }
    pub fn write_pixel(&mut self, x: u32, y: u32, rgb565: u16) { self.controller.write_vram_pixel(x as usize, y as usize, rgb565); }
    pub fn rgba_framebuffer(&self) -> Vec<u8> { self.controller.rgba_bytes() }
    pub fn width(&self) -> u32 { self.controller.width() as u32 }
    pub fn height(&self) -> u32 { self.controller.height() as u32 }
    pub fn is_dirty(&self) -> bool { self.controller.dirty() }
    pub fn clear_dirty(&mut self) { self.controller.clear_dirty(); }
}
impl Default for BrowserDisplay { fn default() -> Self { Self::new() } }

/// Structured inspection of the actual uploaded flash image; deliberately does not boot it.
#[wasm_bindgen]
pub struct BrowserFirmwareInfo {
    magic: String, version: u32, load_config: u32, encrypted: bool,
    table_count: u32, user_address: u32, user_length: u32,
}
#[wasm_bindgen]
impl BrowserFirmwareInfo {
    #[wasm_bindgen(constructor)]
    pub fn inspect(bytes: &[u8]) -> Result<BrowserFirmwareInfo, JsValue> {
        if bytes.len() != 16 * 1024 * 1024 { return Err(JsValue::from_str("Expected a 16 MiB flash dump.")); }
        let tables = find_tables(bytes);
        let t = tables.first().ok_or_else(|| JsValue::from_str("No valid Sonix load table found."))?;
        Ok(Self { magic:t.magic.clone(), version:t.version, load_config:t.load_config,
            encrypted:t.encrypted, table_count:tables.len() as u32,
            user_address:t.user_code.address, user_length:t.user_code.length })
    }
    pub fn magic(&self) -> String { self.magic.clone() }
    pub fn version(&self) -> u32 { self.version }
    pub fn load_config(&self) -> u32 { self.load_config }
    pub fn encrypted(&self) -> bool { self.encrypted }
    pub fn table_count(&self) -> u32 { self.table_count }
    pub fn user_address(&self) -> u32 { self.user_address }
    pub fn user_length(&self) -> u32 { self.user_length }
}
