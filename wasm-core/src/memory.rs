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
pub const FLASH_BASE: u32 = 0x6000_0000;
pub const FLASH_SIZE: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionKind { Pram, BootRom, Sram, Mailbox, Flash, Unmapped }

pub fn region_for(address: u32) -> RegionKind {
    if (PRAM_BASE..PRAM_BASE + PRAM_SIZE as u32).contains(&address) { RegionKind::Pram }
    else if (ROM_BASE..ROM_BASE + ROM_SIZE as u32).contains(&address) { RegionKind::BootRom }
    else if (SRAM_BASE..SRAM_BASE + SRAM_SIZE as u32).contains(&address) { RegionKind::Sram }
    else if (MAILBOX_BASE..MAILBOX_BASE + MAILBOX_SIZE as u32).contains(&address) { RegionKind::Mailbox }
    else if (FLASH_BASE..FLASH_BASE + FLASH_SIZE as u32).contains(&address) { RegionKind::Flash }
    else { RegionKind::Unmapped }
}

/// Byte-addressable memory storage for the machine's mapped RAM/ROM/flash regions.
/// MMIO registers, aliases, and CPU access timing are intentionally not emulated here yet.
pub struct MemoryStorage {
    pub pram: Vec<u8>, pub boot_rom: Vec<u8>, pub sram: Vec<u8>,
    pub mailbox: Vec<u8>, pub flash: Vec<u8>,
}
impl MemoryStorage {
    pub fn new() -> Self {
        let mut boot_rom = vec![0; ROM_SIZE];
        boot_rom[0..4].copy_from_slice(&0x2001_0000u32.to_le_bytes());
        boot_rom[4..8].copy_from_slice(&0x0800_0041u32.to_le_bytes());
        boot_rom[0x40..0x46].copy_from_slice(&[0x00,0xBF,0x01,0x20,0xFE,0xE7]);
        Self { pram:vec![0;PRAM_SIZE], boot_rom, sram:vec![0;SRAM_SIZE],
            mailbox:vec![0;MAILBOX_SIZE], flash:vec![0xFF;FLASH_SIZE] }
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
    pub fn read_u8(&self, address: u32) -> u8 {
        let (slice, offset) = match region_for(address) {
            RegionKind::Pram => (&self.pram[..], (address-PRAM_BASE) as usize),
            RegionKind::BootRom => (&self.boot_rom[..], (address-ROM_BASE) as usize),
            RegionKind::Sram => (&self.sram[..], (address-SRAM_BASE) as usize),
            RegionKind::Mailbox => (&self.mailbox[..], (address-MAILBOX_BASE) as usize),
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
        assert_eq!(region_for(FLASH_BASE),RegionKind::Flash);
        assert_eq!(region_for(0x4000_0000),RegionKind::Unmapped);
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
        assert!(m.load_flash(&[1,2,3]).is_err());
    }
}
