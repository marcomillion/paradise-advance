//! Portable browser-facing components for the real Paradise emulator.
//! Firmware inspection is real; CPU execution and decryption are not yet integrated.

mod aes;
mod cpu;
mod display;
mod sonix;
mod memory;
use wasm_bindgen::prelude::*;
pub use display::{DisplayController, LCD_HEIGHT, LCD_WIDTH};
pub use sonix::{find_tables, decrypt_dump_with_key, CipherMode, LoadTable, Region};
pub use memory::{MemoryStorage, RegionKind};
pub use cpu::{CpuFault, ThumbCpu};

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

/// Exposes the real chip memory regions for incremental CPU/bus integration.
/// It does not execute instructions or dispatch MMIO yet.
#[wasm_bindgen]
pub struct BrowserMemory {
    memory: MemoryStorage,
}
#[wasm_bindgen]
impl BrowserMemory {
    #[wasm_bindgen(constructor)]
    pub fn new() -> BrowserMemory { BrowserMemory { memory: MemoryStorage::new() } }
    pub fn load_flash(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        self.memory.load_flash(bytes).map_err(JsValue::from_str)
    }
    pub fn read_u8(&self, address: u32) -> u32 { self.memory.read_u8(address) as u32 }
    pub fn read_u32(&self, address: u32) -> u32 { self.memory.read_u32(address) }
    pub fn write_u8(&mut self, address: u32, value: u32) -> bool {
        self.memory.write_u8(address, value as u8)
    }
    pub fn reset_vector_sp(&self) -> u32 { self.memory.read_u32(0x0800_0000) }
    pub fn reset_vector_pc(&self) -> u32 { self.memory.read_u32(0x0800_0004) }
}
impl Default for BrowserMemory { fn default() -> Self { Self::new() } }

/// Decrypts the firmware's encrypted regions using a user-provided 32-bit device key.
/// The returned bytes are held in memory only; this function does not execute firmware.
#[wasm_bindgen]
pub fn decrypt_firmware_with_key(bytes: &[u8], device_key: u32) -> Result<Vec<u8>, JsValue> {
    if bytes.len() != 16 * 1024 * 1024 {
        return Err(JsValue::from_str("Expected a 16 MiB flash dump."));
    }
    decrypt_dump_with_key(bytes, device_key).map_err(JsValue::from_str)
}

/// Browser API for incremental Thumb execution against the portable memory model.
/// Unsupported opcodes halt and report an error; this is not yet a full SNC73410 machine.
#[wasm_bindgen]
pub struct BrowserCpu { cpu: ThumbCpu, memory: MemoryStorage }
#[wasm_bindgen]
impl BrowserCpu {
    #[wasm_bindgen(constructor)]
    pub fn new() -> BrowserCpu { BrowserCpu { cpu:ThumbCpu::new(), memory:MemoryStorage::new() } }
    pub fn load_flash(&mut self, bytes:&[u8]) -> Result<(),JsValue> {
        self.memory.load_flash(bytes).map_err(JsValue::from_str)
    }
    /// Load a Sonix image using the caller-supplied device key, then prepare its user-code
    /// vector table in PRAM. This does not yet emulate the full boot ROM or MMIO devices.
    pub fn load_sonix_firmware(&mut self, bytes:&[u8], device_key:u32) -> Result<(),JsValue> {
        if bytes.len()!=16*1024*1024 { return Err(JsValue::from_str("Expected a 16 MiB flash dump.")); }
        let decrypted=decrypt_dump_with_key(bytes,device_key).map_err(JsValue::from_str)?;
        let table=find_tables(&decrypted).into_iter().next()
            .ok_or_else(||JsValue::from_str("No valid Sonix load table found"))?;
        self.memory.load_flash(&decrypted).map_err(JsValue::from_str)?;
        let off=(table.user_code.address & 0x00FF_FFFF) as usize;
        let end=off.checked_add(table.user_code.length as usize)
            .ok_or_else(||JsValue::from_str("Firmware region address overflow"))?;
        let region=decrypted.get(off..end)
            .ok_or_else(||JsValue::from_str("Declared user-code region is outside the dump"))?;
        self.memory.load_pram(region);
        let reset_vector=self.memory.read_u32(4);
        if reset_vector&1==0 {
            return Err(JsValue::from_str("User-code reset vector is not Thumb; device key may be incorrect."));
        }
        self.cpu.reset_from_vectors(&self.memory,0);
        Ok(())
    }
    pub fn reset_from_vectors(&mut self, vector_base:u32) {
        self.cpu.reset_from_vectors(&self.memory,vector_base);
    }
    pub fn set_register(&mut self,index:u32,value:u32) -> Result<(),JsValue> {
        if index>=16 {return Err(JsValue::from_str("Register index must be 0..15"));}
        self.cpu.r[index as usize]=value; Ok(())
    }
    pub fn register(&self,index:u32)->Result<u32,JsValue> {
        if index>=16 {return Err(JsValue::from_str("Register index must be 0..15"));}
        Ok(self.cpu.r[index as usize])
    }
    pub fn step(&mut self)->Result<(),JsValue> {
        self.cpu.step(&mut self.memory).map_err(|e|JsValue::from_str(&format!("{:?}",e)))
    }
    pub fn run_steps(&mut self,count:u32)->u32 {
        let mut done=0;
        for _ in 0..count.min(1_000_000) { if self.cpu.step(&mut self.memory).is_err(){break;} done+=1; }
        done
    }
    pub fn instructions(&self)->u64 {self.cpu.instructions}
    pub fn halted(&self)->bool {self.cpu.halted}
    pub fn last_fault(&self)->String {format!("{:?}",self.cpu.last_fault)}
}
impl Default for BrowserCpu {fn default()->Self{Self::new()}}
