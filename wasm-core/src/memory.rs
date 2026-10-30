//! Portable SNC73410 memory regions adapted from infinition/capybara (GPL-3.0-only).
//! This is the memory-storage/bus boundary only; CPU and MMIO dispatch are not yet wired.

pub const PRAM_BASE: u32 = 0x0000_0000;
pub const PRAM_SIZE: usize = 64 * 1024;
pub const ROM_BASE: u32 = 0x0800_0000;
pub const ROM_SIZE: usize = 64 * 1024;
pub const SRAM_BASE: u32 = 0x1800_0000;
pub const SRAM_SIZE: usize = 128 * 1024;
pub const MAILBOX_BASE: u32 = 0x2000_0000;
pub const MAILBOX_SIZE: usize = 4 * 1024;
pub const XIP_BASE: u32 = 0x1000_0000;
pub const XIP_SIZE: usize = 1024 * 1024;
pub const FLASH_BASE: u32 = 0x6000_0000;
pub const FLASH_SIZE: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionKind { Pram, BootRom, Sram, Mailbox, Xip, Flash, Unmapped }

pub fn region_for(address: u32) -> RegionKind {
    if (PRAM_BASE..PRAM_BASE + PRAM_SIZE as u32).contains(&address) { RegionKind::Pram }
    else if (ROM_BASE..ROM_BASE + ROM_SIZE as u32).contains(&address) { RegionKind::BootRom }
    else if (SRAM_BASE..SRAM_BASE + SRAM_SIZE as u32).contains(&address) { RegionKind::Sram }
    else if (MAILBOX_BASE..MAILBOX_BASE + MAILBOX_SIZE as u32).contains(&address) { RegionKind::Mailbox }
    else if (XIP_BASE..XIP_BASE + XIP_SIZE as u32).contains(&address) { RegionKind::Xip }
    else if (FLASH_BASE..FLASH_BASE + FLASH_SIZE as u32).contains(&address) { RegionKind::Flash }
    else { RegionKind::Unmapped }
}

/// Byte-addressable memory storage for the machine's mapped RAM/ROM/flash regions.
/// MMIO registers, aliases, and CPU access timing are intentionally not emulated here yet.
pub struct MemoryStorage {
    pub pram: Vec<u8>, pub boot_rom: Vec<u8>, pub sram: Vec<u8>,
    pub mailbox: Vec<u8>, pub flash: Vec<u8>,
    pub gpio_buttons: [bool; 3], pub dial_counter: i32,
    pub dma_registers: [u32; 78], pub panel_frame: Option<Vec<u16>>,
}
impl MemoryStorage {
    pub fn new() -> Self {
        let mut boot_rom = vec![0; ROM_SIZE];
        boot_rom[0..4].copy_from_slice(&0x2001_0000u32.to_le_bytes());
        boot_rom[4..8].copy_from_slice(&0x0800_0041u32.to_le_bytes());
        boot_rom[0x40..0x46].copy_from_slice(&[0x00,0xBF,0x01,0x20,0xFE,0xE7]);
        Self { pram:vec![0;PRAM_SIZE], boot_rom, sram:vec![0;SRAM_SIZE],
            mailbox:vec![0;MAILBOX_SIZE], flash:vec![0xFF;FLASH_SIZE],
            gpio_buttons:[false;3], dial_counter:0,
            dma_registers:[0;78], panel_frame:None }
    }
    pub fn load_pram(&mut self, bytes: &[u8]) {
        self.pram.fill(0);
        let len = bytes.len().min(self.pram.len());
        self.pram[..len].copy_from_slice(&bytes[..len]);
    }
    pub fn load_flash(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        if bytes.len() != FLASH_SIZE { return Err("flash dump must be exactly 16 MiB"); }
        self.flash.copy_from_slice(bytes); Ok(())
    }
    pub fn set_button(&mut self, index: usize, pressed: bool) -> bool {
        if index >= self.gpio_buttons.len() { return false; }
        self.gpio_buttons[index] = pressed; true
    }
    pub fn rotate_dial(&mut self, delta: i32) { self.dial_counter = self.dial_counter.wrapping_add(delta); }
    fn gpio_register(&self, offset: u32) -> u32 {
        match offset {
            0 => { let mut v=u32::MAX; for (i,pressed) in self.gpio_buttons.iter().enumerate() { if *pressed { v &= !(1<<i); } } v },
            4 => self.dial_counter as u32,
            _ => 0,
        }
    }
    pub fn read_u8(&self, address: u32) -> u8 {
        if (0x4000_F000..0x4000_F138).contains(&address) {
            let offset=address-0x4000_F000;
            let reg=self.dma_registers[(offset/4) as usize];
            return ((reg >> ((offset&3)*8)) & 0xff) as u8;
        }
        if (0x4003_1000..0x4003_1008).contains(&address) {
            let reg=self.gpio_register((address-0x4003_1000)&!3);
            return ((reg >> ((address&3)*8)) & 0xff) as u8;
        }
        let (slice, offset) = match region_for(address) {
            RegionKind::Pram => (&self.pram[..], (address-PRAM_BASE) as usize),
            RegionKind::BootRom => (&self.boot_rom[..], (address-ROM_BASE) as usize),
            RegionKind::Sram => (&self.sram[..], (address-SRAM_BASE) as usize),
            RegionKind::Mailbox => (&self.mailbox[..], (address-MAILBOX_BASE) as usize),
            RegionKind::Xip => (&self.flash[..], (address-XIP_BASE) as usize),
            RegionKind::Flash => (&self.flash[..], (address-FLASH_BASE) as usize),
            RegionKind::Unmapped => return 0,
        };
        slice.get(offset).copied().unwrap_or(0)
    }
    pub fn read_u32(&self, address: u32) -> u32 {
        u32::from_le_bytes([self.read_u8(address),self.read_u8(address.wrapping_add(1)),
            self.read_u8(address.wrapping_add(2)),self.read_u8(address.wrapping_add(3))])
    }
    pub fn write_u8(&mut self, address: u32, value: u8) -> bool {
        if (0x4000_F000..0x4000_F138).contains(&address) {
            let offset=address-0x4000_F000;
            let word=(offset/4) as usize;
            let shift=(offset&3)*8;
            let mask=!(0xffu32<<shift);
            self.dma_registers[word]=(self.dma_registers[word]&mask)|((value as u32)<<shift);
            if offset==0x0B {
                let ack=self.dma_registers[2];
                self.dma_registers[0]&=!ack;
                self.dma_registers[2]=0;
            }
            // Channel CTRL is acted on only after its complete word is written.
            if offset>=0x100 && (offset-0x100)%0x20==0x03 {
                let channel=((offset-0x100)/0x20) as usize;
                let ctrl_word=(0x100/4)+channel*8;
                if self.dma_registers[ctrl_word]&1!=0 {
                    let source=self.dma_registers[ctrl_word+2];
                    let destination=self.dma_registers[ctrl_word+3];
                    let units=(self.dma_registers[ctrl_word+5]&0x003F_FFFF).min(16_384) as usize;
                    if destination==0x4000_E01C && units>0 {
                        let mut pixels=Vec::with_capacity(units);
                        for i in 0..units {
                            let addr=source.wrapping_add((i*2) as u32);
                            pixels.push(u16::from_le_bytes([self.read_u8(addr),self.read_u8(addr.wrapping_add(1))]));
                        }
                        self.panel_frame=Some(pixels);
                    }
                    self.dma_registers[ctrl_word]&=!1;
                    self.dma_registers[0]|=1<<channel;
                }
            }
            return true;
        }
        if (0x4003_1004..0x4003_1008).contains(&address) {
            let shift=((address-0x4003_1004)*8) as u32;
            let mask=!(0xffu32<<shift);
            let current=self.dial_counter as u32;
            self.dial_counter=((current&mask)|((value as u32)<<shift)) as i32;
            return true;
        }
        let (slice, offset) = match region_for(address) {
            RegionKind::Pram => (&mut self.pram[..], (address-PRAM_BASE) as usize),
            RegionKind::Sram => (&mut self.sram[..], (address-SRAM_BASE) as usize),
            RegionKind::Mailbox => (&mut self.mailbox[..], (address-MAILBOX_BASE) as usize),
            // Boot ROM and external flash need controller-specific write semantics.
            _ => return false,
        };
        if let Some(slot)=slice.get_mut(offset) { *slot=value; true } else { false }
    }
}
impl Default for MemoryStorage { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn maps_known_hardware_regions() {
        assert_eq!(region_for(0),RegionKind::Pram);
        assert_eq!(region_for(ROM_BASE),RegionKind::BootRom);
        assert_eq!(region_for(SRAM_BASE),RegionKind::Sram);
        assert_eq!(region_for(MAILBOX_BASE),RegionKind::Mailbox);
        assert_eq!(region_for(XIP_BASE),RegionKind::Xip);
        assert_eq!(region_for(FLASH_BASE),RegionKind::Flash);
        assert_eq!(region_for(0x4000_0000),RegionKind::Unmapped);
    }
    #[test]
    fn dma_channel_copies_rgb565_words_to_panel_data_register() {
        let mut m=MemoryStorage::new();
        m.sram[0x100..0x104].copy_from_slice(&[0xE0,0x07,0x1F,0x00]);
        fn wr32(m:&mut MemoryStorage,a:u32,v:u32){for i in 0..4 {m.write_u8(a+i,(v>>(8*i)) as u8);}}
        wr32(&mut m,0x4000_F108,SRAM_BASE+0x100);
        wr32(&mut m,0x4000_F10C,0x4000_E01C);
        wr32(&mut m,0x4000_F114,2);
        wr32(&mut m,0x4000_F100,1);
        assert_eq!(m.panel_frame,Some(vec![0x07E0,0x001F]));
        assert_eq!(m.dma_registers[0x100/4]&1,0);
    }
    #[test]
    fn gpio_buttons_are_active_low_and_dial_is_readable() {
        let mut m=MemoryStorage::new();
        assert_eq!(m.read_u32(0x4003_1000)&7,7);
        assert!(m.set_button(1,true));
        assert_eq!(m.read_u32(0x4003_1000)&7,5);
        m.rotate_dial(3);
        assert_eq!(m.read_u32(0x4003_1004),3);
    }
    #[test] fn reset_vector_boot_rom_is_little_endian() {
        let m=MemoryStorage::new();
        assert_eq!(m.read_u32(ROM_BASE),0x2001_0000);
        assert_eq!(m.read_u32(ROM_BASE+4),0x0800_0041);
    }
    #[test] fn sram_is_writable_but_rom_is_not() {
        let mut m=MemoryStorage::new();
        assert!(m.write_u8(SRAM_BASE+3,0xA5));
        assert_eq!(m.read_u8(SRAM_BASE+3),0xA5);
        assert!(!m.write_u8(ROM_BASE,0));
    }
    #[test] fn flash_requires_full_dump() {
        let mut m=MemoryStorage::new();
        assert!(m.load_flash(&vec![0x12;FLASH_SIZE]).is_ok());
        assert_eq!(m.read_u8(FLASH_BASE),0x12);
        assert_eq!(m.read_u8(XIP_BASE),0x12);
        assert!(m.load_flash(&[1,2,3]).is_err());
    }
}
