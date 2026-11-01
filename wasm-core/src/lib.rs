//! Portable browser-facing components for the real Paradise emulator.
//! Firmware inspection is real; CPU execution and decryption are not yet integrated.

mod aes;
mod cpu;
mod display;
mod sonix;
mod memory;
use wasm_bindgen::prelude::*;
pub use display::{DisplayController, LCD_HEIGHT, LCD_WIDTH};
pub use sonix::{find_tables, decrypt_dump_with_key, search_device_key_chunk as search_key_chunk, CipherMode, LoadTable, Region};
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

/// Search a bounded block of the 32-bit device-key space. Keep chunks small in browsers.
#[wasm_bindgen]
pub fn search_device_key_candidates(bytes:&[u8],start:u32,count:u32)->Result<Option<u32>,JsValue> {
    search_key_chunk(bytes,start,count).map_err(JsValue::from_str)
}

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
pub struct BrowserCpu { cpu: ThumbCpu, memory: MemoryStorage, display: DisplayController }
#[wasm_bindgen]
impl BrowserCpu {
    #[wasm_bindgen(constructor)]
    pub fn new() -> BrowserCpu { BrowserCpu { cpu:ThumbCpu::new(), memory:MemoryStorage::new(), display:DisplayController::new() } }
    pub fn load_flash(&mut self, bytes:&[u8]) -> Result<(),JsValue> {
        self.memory.load_flash(bytes).map_err(JsValue::from_str)
    }
    /// Load a Sonix image using the caller-supplied device key, then prepare its user-code
    /// vector table in PRAM. This does not yet emulate the full boot ROM or MMIO devices.
    pub fn load_sonix_firmware(&mut self, bytes:&[u8], device_key:u32) -> Result<(),JsValue> {
        load_sonix_firmware_inner(self, bytes, device_key).map_err(|error| JsValue::from_str(&error))
    }
    pub fn reset_from_vectors(&mut self, vector_base:u32) {
        self.cpu.reset_from_vectors(&self.memory,vector_base);
    }
    pub fn set_button(&mut self,index:u32,pressed:bool) -> Result<(),JsValue> {
        if !self.memory.set_button(index as usize,pressed) { return Err(JsValue::from_str("Button index must be 0..3 (A, B, C, or dial)")); }
        Ok(())
    }
    pub fn rotate_dial(&mut self,delta:i32) { self.memory.rotate_dial(delta); }
    pub fn set_register(&mut self,index:u32,value:u32) -> Result<(),JsValue> {
        if index>=16 {return Err(JsValue::from_str("Register index must be 0..15"));}
        self.cpu.r[index as usize]=value; Ok(())
    }
    pub fn register(&self,index:u32)->Result<u32,JsValue> {
        if index>=16 {return Err(JsValue::from_str("Register index must be 0..15"));}
        Ok(self.cpu.r[index as usize])
    }
    /// Snapshot core registers and the next Thumb instruction for browser-side diagnosis.
    /// Return nearby Thumb halfwords as address/opcode pairs around the current PC.
    /// This is a raw code window, not a disassembler: 32-bit Thumb instructions occupy two entries.
    pub fn execution_window(&self)->Vec<u32> {
        let pc=self.cpu.r[15]&!1;
        // Keep a wider window before the PC so table-branch setup and
        // the instructions that lead into a wait loop can be inspected.
        let start=pc.saturating_sub(64)&!1;
        // Show enough instructions ahead of the current PC to inspect
        // nearby branch targets and jump-table destinations.
        let mut out=Vec::with_capacity(130);
        for index in 0..65u32 {
            let address=start.wrapping_add(index*2);
            let opcode=self.memory.read_u8(address) as u32
                | ((self.memory.read_u8(address.wrapping_add(1)) as u32)<<8);
            out.push(address);
            out.push(opcode);
        }
        out
    }
    pub fn execution_snapshot(&self)->Vec<u32> {
        let pc=self.cpu.r[15];
        let opcode=self.memory.read_u8(pc) as u32 | ((self.memory.read_u8(pc.wrapping_add(1)) as u32)<<8);
        let previous_pc=pc.wrapping_sub(2);
        let previous_opcode=self.memory.read_u8(previous_pc) as u32 | ((self.memory.read_u8(previous_pc.wrapping_add(1)) as u32)<<8);
        let mut out=self.cpu.r.to_vec();
        out.push(opcode);
        out.push(previous_opcode);
        out.push(self.cpu.n as u32 | ((self.cpu.z as u32)<<1) | ((self.cpu.c as u32)<<2) | ((self.cpu.v as u32)<<3));
        out
    }
    pub fn step(&mut self)->Result<(),JsValue> {
        let result=self.cpu.step(&mut self.memory).map_err(|e|JsValue::from_str(&format!("{:?}",e)));
        if let Some(frame)=self.memory.panel_frame.take() { self.display.receive_frame(&frame); }
        result
    }
    pub fn run_steps(&mut self,count:u32)->u32 {
        let mut done=0;
        for _ in 0..count.min(1_000_000) {
            let before=self.cpu.instructions;
            if self.cpu.step(&mut self.memory).is_err(){break;}
            if self.cpu.instructions!=before { done+=1; }
        }
        if let Some(frame)=self.memory.panel_frame.take() { self.display.receive_frame(&frame); }
        done
    }
    /// Execute until the PC reaches an address, stopping before that instruction.
    /// Returns [executed_instructions, breakpoint_reached, final_pc].
    pub fn run_until_pc(&mut self,address:u32,max_steps:u32)->Vec<u32> {
        let target=address&!1;
        let mut done=0u32;
        let mut reached=(self.cpu.r[15]&!1)==target;
        for _ in 0..max_steps.min(1_000_000) {
            if reached || self.cpu.halted { break; }
            let before=self.cpu.instructions;
            if self.cpu.step(&mut self.memory).is_err(){break;}
            if self.cpu.instructions!=before { done=done.saturating_add(1); }
            reached=(self.cpu.r[15]&!1)==target;
        }
        if let Some(frame)=self.memory.panel_frame.take() { self.display.receive_frame(&frame); }
        vec![done,reached as u32,self.cpu.r[15]]
    }
    pub fn display_width(&self)->u32 {self.display.width() as u32}
    pub fn display_height(&self)->u32 {self.display.height() as u32}
    pub fn display_rgba_framebuffer(&self)->Vec<u8> {self.display.rgba_bytes()}
    pub fn display_enabled(&self)->bool {self.display.enabled()}
    pub fn display_frames_received(&self)->u32 {self.display.frames_received() as u32}
    pub fn display_nonzero_pixels(&self)->u32 {self.display.nonzero_pixels()}
    pub fn display_pixel_checksum(&self)->u32 {self.display.pixel_checksum()}
    pub fn dma_diagnostics(&self)->Vec<u32> { let (w,a,v,t,s,d,n)=self.memory.dma_diagnostics(); vec![w as u32,a,v,t as u32,s,d,n] }
    pub fn peripheral_diagnostics(&self)->Vec<u32> { let (w,a,v,spi,dma,r,ra,rv)=self.memory.peripheral_diagnostics(); vec![w as u32,a,v,spi as u32,dma as u32,r as u32,ra,rv] }
    pub fn peripheral_trace(&self)->Vec<u32> { self.memory.peripheral_trace() }
    pub fn instructions(&self)->u64 {self.cpu.instructions}
    pub fn halted(&self)->bool {self.cpu.halted}
    pub fn sleeping(&self)->bool {self.cpu.is_sleeping()}
    pub fn last_fault(&self)->String {format!("{:?}",self.cpu.last_fault)}
}
fn load_sonix_firmware_inner(cpu:&mut BrowserCpu, bytes:&[u8], device_key:u32) -> Result<(),String> {
    if bytes.len()!=16*1024*1024 { return Err("Expected a 16 MiB flash dump.".to_string()); }
    let decrypted=decrypt_dump_with_key(bytes,device_key)?;
    let table=find_tables(&decrypted).into_iter().next()
        .ok_or_else(||"No valid Sonix load table found".to_string())?;
    // Validate the decrypted user region before mutating emulator state. A wrong key
    // or malformed vector table must not leave a half-loaded flash/PRAM image behind.
    let off=(table.user_code.address & 0x00FF_FFFF) as usize;
    let end=off.checked_add(table.user_code.length as usize)
        .ok_or_else(||"Firmware region address overflow".to_string())?;
    let region=decrypted.get(off..end)
        .ok_or_else(||"Declared user-code region is outside the dump".to_string())?;
    if region.len()<8 {
        return Err("User-code region is too short to contain its initial vectors.".to_string());
    }
    let stack_pointer=u32::from_le_bytes(region[0..4].try_into().unwrap());
    let reset_vector=u32::from_le_bytes(region[4..8].try_into().unwrap());
    if reset_vector&1==0 {
        return Err("User-code reset vector is not Thumb; device key may be incorrect.".to_string());
    }
    if (reset_vector&!1)>=table.user_code.length || (reset_vector&!1)>=crate::memory::PRAM_SIZE as u32 {
        return Err("Reset vector points outside the loaded user-code region; device key may be incorrect.".to_string());
    }
    let sram_start=crate::memory::SRAM_BASE;
    let sram_end=sram_start+crate::memory::SRAM_SIZE as u32;
    let mailbox_start=crate::memory::MAILBOX_BASE;
    let mailbox_end=mailbox_start+crate::memory::MAILBOX_SIZE as u32;
    let stack_in_ram=(sram_start..=sram_end).contains(&stack_pointer)
        || (mailbox_start..=mailbox_end).contains(&stack_pointer);
    if !stack_in_ram || stack_pointer&3!=0 {
        return Err("Initial stack pointer is outside supported RAM or is not 4-byte aligned; device key may be incorrect.".to_string());
    }

    // Commit the validated image only after every structural check succeeds.
    cpu.memory.load_flash(&decrypted)?;
    cpu.memory.load_pram(region);
    cpu.memory.set_device_key(device_key);

    // Reproduce the two boot-info fields the retail firmware reads after the
    // boot ROM has copied user code into PRAM.
    let xip_base=if table.sram_code.address!=0 {
        table.sram_code.address
    } else {
        table.user_code.address.wrapping_add(table.user_code.length)
    };
    write_mailbox_word(&mut cpu.memory,crate::memory::MAILBOX_BASE+0xF60,crate::memory::MAILBOX_BASE);
    write_mailbox_word(&mut cpu.memory,crate::memory::MAILBOX_BASE+0x818,xip_base);

    cpu.cpu.reset_from_vectors(&cpu.memory,0);
    cpu.display=DisplayController::new();
    Ok(())
}
fn write_mailbox_word(memory:&mut MemoryStorage,address:u32,value:u32) {
    for i in 0..4 { let _=memory.write_u8(address+i,(value>>(i*8)) as u8); }
}

impl Default for BrowserCpu {fn default()->Self{Self::new()}}

#[cfg(test)]
mod browser_integration_tests {
    use super::*;
    #[test]
    fn panel_does_not_treat_raw_sram_as_a_display_frame() {
        let mut cpu = BrowserCpu::new();
        // SRAM is only a candidate source buffer. The panel changes when the
        // firmware performs the observed DMA transfer to the SPI data register.
        cpu.memory.sram[0x8000..0x8002].copy_from_slice(&0x07E0u16.to_le_bytes());
        cpu.memory.pram[0..2].copy_from_slice(&0xBF00u16.to_le_bytes()); // NOP
        cpu.cpu.r[15] = 0;
        cpu.step().unwrap();
        let frame = cpu.display_rgba_framebuffer();
        assert_eq!(&frame[0..4], &[0, 0, 0, 255]);
        assert_eq!(cpu.display_frames_received(), 0);
        assert_eq!(cpu.instructions(), 1);
    }

    #[test]
    fn dma_panel_transfer_reaches_browser_framebuffer() {
        let mut cpu=BrowserCpu::new();
        cpu.memory.sram[0x100..0x104].copy_from_slice(&[0xE0,0x07,0x1F,0x00]);
        fn wr32(m:&mut MemoryStorage,a:u32,v:u32){for i in 0..4 {m.write_u8(a+i,(v>>(8*i)) as u8);}}
        wr32(&mut cpu.memory,0x4000_F108,crate::memory::SRAM_BASE+0x100);
        wr32(&mut cpu.memory,0x4000_F10C,0x4000_E01C);
        wr32(&mut cpu.memory,0x4000_F114,2);
        wr32(&mut cpu.memory,0x4000_F100,1);
        cpu.memory.pram[0..2].copy_from_slice(&0xBF00u16.to_le_bytes());
        cpu.cpu.r[15]=0;
        cpu.step().unwrap();
        let frame=cpu.display_rgba_framebuffer();
        assert_eq!(&frame[0..4],&[0,255,0,255]);
        assert_eq!(&frame[4..8],&[0,0,255,255]);
        // A later CPU step without a new DMA frame must not replace the panel
        // contents with an unverified SRAM snapshot.
        cpu.memory.pram[2..4].copy_from_slice(&0xBF00u16.to_le_bytes());
        cpu.step().unwrap();
        let persistent=cpu.display_rgba_framebuffer();
        assert_eq!(&persistent[0..4],&[0,255,0,255]);
        assert_eq!(&persistent[4..8],&[0,0,255,255]);
        assert_eq!(cpu.display_frames_received(),1);
    }

    #[test]
    fn firmware_cpu_programmed_dma_transfer_reaches_display_framebuffer() {
        let mut cpu = BrowserCpu::new();
        // A tiny Thumb-2 firmware routine programs DMA channel 0 exactly as
        // the retail driver does: source, SPI DATA destination, halfword count,
        // then CTRL.START. Four STR.W instructions perform the MMIO writes.
        cpu.memory.sram[0x100..0x104].copy_from_slice(&[
            0xE0, 0x07, 0x1F, 0x00, // RGB565 green, blue
        ]);
        cpu.memory.pram[0..16].copy_from_slice(&[
            0xC1, 0xF8, 0x08, 0x20, // STR.W r2, [r1, #8]  SOURCE
            0xC1, 0xF8, 0x0C, 0x30, // STR.W r3, [r1, #12] DESTINATION
            0xC1, 0xF8, 0x14, 0x40, // STR.W r4, [r1, #20] COUNT
            0xC1, 0xF8, 0x00, 0x50, // STR.W r5, [r1, #0]  START
        ]);
        cpu.cpu.r[1] = 0x4000_F100;
        cpu.cpu.r[2] = crate::memory::SRAM_BASE + 0x100;
        cpu.cpu.r[3] = 0x4000_E01C;
        cpu.cpu.r[4] = 2; // two RGB565 pixels
        cpu.cpu.r[5] = 1; // DMA START
        cpu.cpu.r[15] = 0;

        assert_eq!(cpu.run_steps(4), 4);
        let frame = cpu.display_rgba_framebuffer();
        assert_eq!(&frame[0..4], &[0, 255, 0, 255]);
        assert_eq!(&frame[4..8], &[0, 0, 255, 255]);
        assert_eq!(cpu.display_frames_received(), 1);
        let diagnostics = cpu.dma_diagnostics();
        assert_eq!(diagnostics[3], 1); // one actual DMA transfer
        assert_eq!(diagnostics[4], crate::memory::SRAM_BASE + 0x100);
        assert_eq!(diagnostics[5], 0x4000_E01C);
        assert_eq!(diagnostics[6], 2);
    }

    #[test]
    fn run_until_pc_stops_before_breakpoint_instruction() {
        let mut cpu=BrowserCpu::new();
        cpu.memory.pram[0..4].copy_from_slice(&[0x2A,0x20,0xFE,0xE7]); // MOVS r0,#42; B .
        cpu.cpu.r[15]=0;
        let result=cpu.run_until_pc(2,10);
        assert_eq!(result,vec![1,1,2]);
        assert_eq!(cpu.register(0).unwrap(),42);
        assert_eq!(cpu.instructions(),1);
    }

    #[test]
    fn run_steps_counts_instructions_not_idle_sleep_iterations() {
        let mut cpu=BrowserCpu::new();
        cpu.memory.pram[0..2].copy_from_slice(&0xBF30u16.to_le_bytes()); // WFI
        cpu.memory.pram[2..4].copy_from_slice(&0x202Au16.to_le_bytes()); // MOVS r0, #42
        cpu.cpu.r[15]=0;
        assert_eq!(cpu.run_steps(5),1);
        assert_eq!(cpu.instructions(),1);
        assert_eq!(cpu.register(15).unwrap(),2);
        assert_eq!(cpu.register(0).unwrap(),0);
    }

    #[test]
    fn framebuffer_dimensions_are_exposed_through_cpu() {
        let cpu = BrowserCpu::new();
        assert_eq!(cpu.display_width(), 128);
        assert_eq!(cpu.display_height(), 128);
        assert_eq!(cpu.display_rgba_framebuffer().len(), 128 * 128 * 4);
    }

    #[test]
    fn browser_cpu_exposes_all_four_input_lines() {
        let mut cpu = BrowserCpu::new();
        assert!(cpu.set_button(0, true).is_ok());
        assert!(cpu.set_button(1, true).is_ok());
        assert!(cpu.set_button(2, true).is_ok());
        assert!(cpu.set_button(3, true).is_ok());
        assert!(cpu.set_button(3, false).is_ok());
    }

    #[test]
    fn invalid_button_index_is_rejected_by_portable_input_layer() {
        let mut cpu = BrowserCpu::new();
        assert!(!cpu.memory.set_button(4, true));
        assert_eq!(cpu.memory.gpio_buttons, [false; 4]);
    }

    #[test]
    fn unencrypted_sonix_image_loads_and_executes_its_user_code() {
        let mut image = vec![0xFFu8; crate::memory::FLASH_SIZE];
        image[..8].copy_from_slice(b"SONIXDEV");
        // load_config bit zero is clear, so this synthetic image is unencrypted.
        image[8..12].copy_from_slice(&0x12u32.to_le_bytes());
        image[0x10..0x14].copy_from_slice(&0x6000_1000u32.to_le_bytes());
        image[0x14..0x18].copy_from_slice(&0x1_0000u32.to_le_bytes());
        image[0x90..0xA0].fill(0); // no separate SRAM or DPD image in this fixture
        image[0x1F8..0x1FC].copy_from_slice(&0x5A5A_0033u32.to_le_bytes());

        let region = 0x1000usize;
        image[region..region + 4].copy_from_slice(
            &(crate::memory::SRAM_BASE + crate::memory::SRAM_SIZE as u32 - 16).to_le_bytes()
        );
        image[region + 4..region + 8].copy_from_slice(&0x0000_0101u32.to_le_bytes());
        // Reset handler: NOP, then branch to itself.
        image[region + 0x100..region + 0x104].copy_from_slice(&[0x00, 0xBF, 0xFE, 0xE7]);

        let mut cpu = BrowserCpu::new();
        cpu.load_sonix_firmware(&image, 0).expect("synthetic image should load");
        assert_eq!(cpu.memory.read_u32(0x4500_003C),0);
        assert_eq!(cpu.register(13).unwrap(), crate::memory::SRAM_BASE + crate::memory::SRAM_SIZE as u32 - 16);
        assert_eq!(cpu.register(15).unwrap(), 0x100);
        assert_eq!(cpu.memory.read_u32(crate::memory::MAILBOX_BASE+0xF60),crate::memory::MAILBOX_BASE);
        assert_eq!(cpu.memory.read_u32(crate::memory::MAILBOX_BASE+0x818),0x6001_1000);
        assert_eq!(cpu.run_steps(3), 3);
        assert_eq!(cpu.instructions(), 3);
        assert!(!cpu.halted());
        assert_eq!(cpu.register(15).unwrap(), 0x102);

        // If the load table declares an SRAM image, its address becomes the XIP base.
        image[0x90..0x94].copy_from_slice(&0x6002_0000u32.to_le_bytes());
        image[0x94..0x98].copy_from_slice(&0x1000u32.to_le_bytes());
        let mut cpu_with_sram_region=BrowserCpu::new();
        cpu_with_sram_region.load_sonix_firmware(&image,0).unwrap();
        assert_eq!(cpu_with_sram_region.memory.read_u32(crate::memory::MAILBOX_BASE+0x818),0x6002_0000);

        // The firmware reads the physical device key back from FEUSE3.
        let mut cpu_with_key=BrowserCpu::new();
        cpu_with_key.load_sonix_firmware(&image,0x1234_5678).unwrap();
        assert_eq!(cpu_with_key.memory.read_u32(0x4500_0038),0x1234_0000);
        assert_eq!(cpu_with_key.memory.read_u32(0x4500_003C),0x1234_5678);
    }

    #[test]
    fn observed_early_firmware_sequence_can_read_modify_write_sn_sys0() {
        let mut cpu=BrowserCpu::new();
        // Captured sequence from the retail reset path:
        // MOV.W r1, #0x45000000; LDR r0, [r1]; ORR.W r0, r0, #0x10; STR r0, [r1].
        let code=[0x4F,0xF0,0x8A,0x41,0x08,0x68,0x40,0xF0,0x10,0x00,0x08,0x60];
        cpu.memory.pram[..code.len()].copy_from_slice(&code);
        cpu.cpu.r[15]=0;
        assert_eq!(cpu.run_steps(4),4);
        assert_eq!(cpu.register(1).unwrap(),0x4500_0000);
        assert_eq!(cpu.register(0).unwrap(),0x10);
        assert_eq!(cpu.memory.read_u32(0x4500_0000),0x10);
        assert!(!cpu.halted());
    }

    #[test]
    fn rejects_sonix_image_with_reset_vector_outside_user_code() {
        let mut image = vec![0xFFu8; crate::memory::FLASH_SIZE];
        image[..8].copy_from_slice(b"SONIXDEV");
        image[8..12].copy_from_slice(&0x12u32.to_le_bytes());
        image[0x10..0x14].copy_from_slice(&0x6000_1000u32.to_le_bytes());
        image[0x14..0x18].copy_from_slice(&0x100u32.to_le_bytes());
        image[0x1F8..0x1FC].copy_from_slice(&0x5A5A_0033u32.to_le_bytes());
        image[0x1000..0x1004].copy_from_slice(&0x1801_FFF0u32.to_le_bytes());
        image[0x1004..0x1008].copy_from_slice(&0x0000_0201u32.to_le_bytes());

        let mut cpu = BrowserCpu::new();
        let error = load_sonix_firmware_inner(&mut cpu, &image, 0).unwrap_err();
        assert!(error.contains("outside the loaded user-code region"));
    }

    #[test]
    fn rejected_firmware_does_not_partially_replace_emulator_memory() {
        let mut cpu = BrowserCpu::new();
        cpu.memory.pram[0] = 0xA5;
        cpu.memory.flash[0] = 0x5A;

        let mut image = vec![0xFFu8; crate::memory::FLASH_SIZE];
        image[..8].copy_from_slice(b"SONIXDEV");
        image[8..12].copy_from_slice(&0x12u32.to_le_bytes());
        image[0x10..0x14].copy_from_slice(&0x6000_1000u32.to_le_bytes());
        image[0x14..0x18].copy_from_slice(&0x100u32.to_le_bytes());
        image[0x1F8..0x1FC].copy_from_slice(&0x5A5A_0033u32.to_le_bytes());
        // Keep the reset vector valid so the misaligned stack pointer is the failing condition.
        image[0x1000..0x1004].copy_from_slice(&0x1800_0002u32.to_le_bytes());
        image[0x1004..0x1008].copy_from_slice(&0x0000_0081u32.to_le_bytes());

        let error = load_sonix_firmware_inner(&mut cpu, &image, 0).unwrap_err();
        assert!(error.contains("Initial stack pointer"));
        assert_eq!(cpu.memory.pram[0], 0xA5);
        assert_eq!(cpu.memory.flash[0], 0x5A);
    }
}
