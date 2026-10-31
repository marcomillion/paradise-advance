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

/// Byte-addressable RAM/ROM/flash plus initial GPIO and display-DMA MMIO models.
/// This is a narrow peripheral slice, not a complete SNC73410 bus implementation.
pub struct MemoryStorage {
    pub pram: Vec<u8>, pub boot_rom: Vec<u8>, pub sram: Vec<u8>,
    pub mailbox: Vec<u8>, pub flash: Vec<u8>,
    pub gpio_buttons: [bool; 4], pub dial_counter: i32,
    pub gpio_port_data: [u32; 3], pub gpio_port_direction: [u32; 3], pub gpio_port_mode: [u32; 3],
    pub gpio_port_irq_enable: [u32; 3], pub gpio_port_irq_status: [u32; 3],
    gpio_port_cycles: u64, gpio_te_high: bool,
    pub spi_registers: [u32; 8], pub spi_last_tx: u16, pub spi_tx_words: u64,
    pub dma_registers: [u32; 78], pub panel_frame: Option<Vec<u16>>, pub dma_irq_pending: bool,
    pub dma_write_count: u64, pub dma_last_write_address: u32, pub dma_last_write_value: u32,
    pub dma_transfer_attempts: u64, pub dma_last_source: u32, pub dma_last_destination: u32, pub dma_last_units: u32,
    pub peripheral_write_count: u64, pub peripheral_last_write_address: u32, pub peripheral_last_write_value: u32,
    pub peripheral_read_count: u64, pub peripheral_last_read_address: u32, pub peripheral_last_read_value: u32,
    pub peripheral_trace: Vec<u32>,
    pub nvic_iser: [u32; 2],
    pub systick_ctrl: u32, pub systick_reload: u32, pub systick_current: u32, pub systick_pending: bool,
}
impl MemoryStorage {
    pub fn new() -> Self {
        let mut boot_rom = vec![0; ROM_SIZE];
        boot_rom[0..4].copy_from_slice(&0x2001_0000u32.to_le_bytes());
        boot_rom[4..8].copy_from_slice(&0x0800_0041u32.to_le_bytes());
        boot_rom[0x40..0x46].copy_from_slice(&[0x00,0xBF,0x01,0x20,0xFE,0xE7]);
        Self { pram:vec![0;PRAM_SIZE], boot_rom, sram:vec![0;SRAM_SIZE],
            mailbox:vec![0;MAILBOX_SIZE], flash:vec![0xFF;FLASH_SIZE],
            gpio_buttons:[false;4], dial_counter:0,
            gpio_port_data:[0;3], gpio_port_direction:[0;3], gpio_port_mode:[0;3],
            gpio_port_irq_enable:[0;3], gpio_port_irq_status:[0;3], gpio_port_cycles:0, gpio_te_high:true,
            spi_registers:[0,2,5,0,0,0,0,0], spi_last_tx:0, spi_tx_words:0,
            dma_registers:[0;78], panel_frame:None, dma_irq_pending:false,
            dma_write_count:0, dma_last_write_address:0, dma_last_write_value:0,
            dma_transfer_attempts:0, dma_last_source:0, dma_last_destination:0, dma_last_units:0,
            peripheral_write_count:0, peripheral_last_write_address:0, peripheral_last_write_value:0,
            peripheral_read_count:0, peripheral_last_read_address:0, peripheral_last_read_value:0, peripheral_trace:Vec::new(), nvic_iser:[0;2],
            systick_ctrl:0, systick_reload:0, systick_current:0, systick_pending:false }
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
    fn trace_peripheral(&mut self, operation:u32, address:u32, value:u32) {
        self.peripheral_trace.extend_from_slice(&[operation,address,value]);
        if self.peripheral_trace.len()>96 {
            let excess=self.peripheral_trace.len()-96;
            self.peripheral_trace.drain(0..excess);
        }
    }
    pub fn peripheral_trace(&self)->Vec<u32> { self.peripheral_trace.clone() }
    pub fn set_button(&mut self, index: usize, pressed: bool) -> bool {
        if index >= self.gpio_buttons.len() { return false; }
        self.gpio_buttons[index] = pressed; true
    }
    pub fn rotate_dial(&mut self, delta: i32) { self.dial_counter = self.dial_counter.wrapping_add(delta); }
    pub fn tick_systick(&mut self) {
        if self.systick_ctrl&1==0 { return; }
        if self.systick_current==0 { self.systick_current=self.systick_reload; }
        else { self.systick_current-=1; }
        if self.systick_current==0 {
            self.systick_ctrl|=1<<16;
            if self.systick_ctrl&2!=0 { self.systick_pending=true; }
        }
    }
    pub fn dma_irq_enabled(&self) -> bool { self.nvic_iser[1] & (1 << (58-32)) != 0 }
    /// Advance the LCD tearing-effect signal. The core has no cycle-accurate
    /// instruction timing yet, so one instruction step represents 1,600 chip cycles
    /// (an explicit speed-up approximation to make frame/IRQ behavior testable).
    pub fn tick_gpio(&mut self) {
        const HALF_PERIOD:u64=800_000;
        const PERIOD:u64=HALF_PERIOD*2;
        let was_high=self.gpio_te_high;
        self.gpio_port_cycles=(self.gpio_port_cycles+1_600)%PERIOD;
        self.gpio_te_high=self.gpio_port_cycles<HALF_PERIOD;
        if !was_high && self.gpio_te_high && self.gpio_port_irq_enable[1]&(1<<10)!=0 {
            self.gpio_port_irq_status[1]|=1<<10;
        }
    }
    pub fn gpio_te_irq_pending(&self)->bool {
        self.gpio_port_irq_status[1]&self.gpio_port_irq_enable[1]&(1<<10)!=0
    }
    pub fn gpio_te_irq_enabled(&self)->bool {
        self.nvic_iser[0]&(1<<27)!=0
    }
    fn spi_read_reg(&self,offset:u32)->u32 {
        match offset {
            0x00=>self.spi_registers[0],
            0x04=>self.spi_registers[1],
            0x08=>0x05, // TXEMPTY and RXEMPTY; transfers complete immediately in this slice
            0x10=>self.spi_registers[4],
            0x14=>self.spi_registers[5],
            0x1C=>0, // no SPI peripheral is connected to the RX FIFO in the minimal model
            _=>0,
        }
    }
    fn spi_write_reg(&mut self,offset:u32,value:u32) {
        match offset {
            0x00=>self.spi_registers[0]=value,
            0x04=>self.spi_registers[1]=value,
            0x10=>self.spi_registers[4]=value,
            0x18=>self.spi_registers[5]&=!value,
            0x1C=>{self.spi_last_tx=value as u16;self.spi_tx_words=self.spi_tx_words.wrapping_add(1);self.spi_registers[7]=value&0xFFFF;},
            _=>{},
        }
    }
    fn gpio_port_index(address: u32) -> Option<(usize,u32)> {
        let base=match address {
            0x4001_8000..=0x4001_8FFF => 0x4001_8000,
            0x4001_9000..=0x4001_9FFF => 0x4001_9000,
            0x4001_A000..=0x4001_AFFF => 0x4001_A000,
            _ => return None,
        };
        Some((((base-0x4001_8000)/0x1000) as usize,address-base))
    }
    fn gpio_port_inputs(&self, port:usize) -> u32 {
        let mut pins=u32::MAX;
        if port==0 {
            // Observed Paradise wiring: P0.8 dial push, P0.9 A, P0.10 C, P0.11 B.
            for (index,pin) in [(3usize,8u32),(0,9),(2,10),(1,11)] {
                if self.gpio_buttons[index] { pins &= !(1<<pin); }
            }
        } else if port==1 {
            // LCD tearing-effect input on P1.10; periodic transitions are generated by tick_gpio.
            if self.gpio_te_high { pins|=1<<10; } else { pins&=!(1<<10); }
        } else if port==2 {
            // Expose the dial as a two-bit Gray-code quadrature phase on P2.0/P2.1.
            let phase=self.dial_counter.rem_euclid(4) as usize;
            let gray=[0u32,1,3,2][phase];
            pins=(pins&!3)|gray;
        }
        pins
    }
    fn gpio_port_read_reg(&self,port:usize,offset:u32)->u32 {
        match offset {
            0x00 => (self.gpio_port_data[port]&self.gpio_port_direction[port]) |
                    (self.gpio_port_inputs(port)&!self.gpio_port_direction[port]),
            0x04 => self.gpio_port_direction[port],
            0x08 => self.gpio_port_mode[port],
            0x18 => self.gpio_port_irq_enable[port],
            0x1C => self.gpio_port_irq_status[port],
            _ => 0,
        }
    }
    fn gpio_port_write_reg(&mut self,port:usize,offset:u32,value:u32) {
        match offset {
            0x00 => self.gpio_port_data[port]=value,
            0x04 => self.gpio_port_direction[port]=value,
            0x08 => self.gpio_port_mode[port]=value,
            0x18 => self.gpio_port_irq_enable[port]=value,
            0x20 => self.gpio_port_irq_status[port]&=!value,
            _ => {},
        }
    }
    fn bitband_target(address:u32)->Option<(u32,u32)> {
        if (0x2200_0000..=0x23FF_FFFF).contains(&address) {
            let delta=address-0x2200_0000;
            return Some((0x2000_0000+delta/32,(delta%32)/4));
        }
        if (0x4200_0000..=0x43FF_FFFF).contains(&address) {
            let delta=address-0x4200_0000;
            return Some((0x4000_0000+delta/32,(delta%32)/4));
        }
        None
    }
    fn gpio_register(&self, offset: u32) -> u32 {
        match offset {
            0 => { let mut v=u32::MAX; for (i,pressed) in self.gpio_buttons.iter().enumerate() { if *pressed { v &= !(1<<i); } } v },
            4 => self.dial_counter as u32,
            _ => 0,
        }
    }
    pub fn read_u8(&self, address: u32) -> u8 {
        if (0x4000_E000..=0x4000_E01F).contains(&address) {
            let reg=self.spi_read_reg((address-0x4000_E000)&!3);
            return ((reg>>((address&3)*8))&0xFF) as u8;
        }
        if (0x2200_0000..=0x23FF_FFFF).contains(&address) || (0x4200_0000..=0x43FF_FFFF).contains(&address) {
            let word=self.read_u32(address & !3);
            return ((word >> ((address&3)*8)) & 0xFF) as u8;
        }
        if let Some((port,offset))=Self::gpio_port_index(address) {
            let reg=self.gpio_port_read_reg(port,offset&!3);
            return ((reg >> ((offset&3)*8)) & 0xFF) as u8;
        }
        if (0xE000_E010..0xE000_E020).contains(&address) {
            let offset=address-0xE000_E010;
            let reg=match offset/4 {0=>self.systick_ctrl,1=>self.systick_reload,2=>self.systick_current,3=>0x00FF_FFFF,_=>0};
            return ((reg >> ((offset&3)*8)) & 0xff) as u8;
        }
        if (0xE000_E100..0xE000_E108).contains(&address) {
            let offset=address-0xE000_E100; let reg=self.nvic_iser[(offset/4) as usize];
            return ((reg >> ((offset&3)*8)) & 0xff) as u8;
        }
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
    /// CPU-originated byte read with the architectural SysTick COUNTFLAG side effect.
    /// Debugger/UI peeks should keep using read_u8 so inspection does not mutate device state.
    pub fn read_cpu_u8(&mut self, address: u32) -> u8 {
        let value = self.read_u8(address);
        if (0x4000_0000..0x5000_0000).contains(&address) {
            self.peripheral_read_count=self.peripheral_read_count.wrapping_add(1);
            self.peripheral_last_read_address=address;
            self.peripheral_last_read_value=value as u32;
            self.trace_peripheral(0,address,value as u32);
        }
        if (0xE000_E010..0xE000_E014).contains(&address) {
            self.systick_ctrl &= !(1 << 16);
        }
        value
    }
    /// CPU-originated word read. Capture the register value before clearing COUNTFLAG.
    pub fn read_cpu_u32(&mut self, address: u32) -> u32 {
        let value = self.read_u32(address);
        if (0x4000_0000..0x5000_0000).contains(&address) {
            self.peripheral_read_count=self.peripheral_read_count.wrapping_add(1);
            self.peripheral_last_read_address=address;
            self.peripheral_last_read_value=value;
            self.trace_peripheral(0,address,value);
        }
        if address == 0xE000_E010 {
            self.systick_ctrl &= !(1 << 16);
        }
        value
    }
    pub fn read_u32(&self, address: u32) -> u32 {
        if (0x4000_E000..=0x4000_E01F).contains(&address) { return self.spi_read_reg((address-0x4000_E000)&!3); }
        if (0x2200_0000..=0x23FF_FFFF).contains(&address) || (0x4200_0000..=0x43FF_FFFF).contains(&address) {
            if let Some((target,bit))=Self::bitband_target(address) { return ((self.read_u8(target)>>bit)&1) as u32; }
        }
        u32::from_le_bytes([self.read_u8(address),self.read_u8(address.wrapping_add(1)),
            self.read_u8(address.wrapping_add(2)),self.read_u8(address.wrapping_add(3))])
    }
    pub fn write_u32(&mut self,address:u32,value:u32)->bool {
        if (0x4000_E000..=0x4000_E01F).contains(&address) {
            if address&3!=0 { return false; }
            // Keep the broad bus counters consistent for aligned word writes,
            // which bypass write_u8 for the SPI register block.
            self.peripheral_write_count=self.peripheral_write_count.wrapping_add(4);
            self.peripheral_last_write_address=address.wrapping_add(3);
            self.peripheral_last_write_value=(value>>24)&0xFF;
            for i in 0..4 { self.trace_peripheral(1,address.wrapping_add(i),(value>>(i*8))&0xFF); }
            self.spi_write_reg(address-0x4000_E000,value); return true;
        }
        if let Some((target,bit))=Self::bitband_target(address) {
            if address&3!=0 { return false; }
            let old=self.read_u8(target);
            let new=if value&1!=0 {old|(1u8<<bit)} else {old&!(1u8<<bit)};
            return self.write_u8(target,new);
        }
        let mut accepted=true;
        for i in 0..4 { accepted &= self.write_u8(address.wrapping_add(i),(value>>(i*8)) as u8); }
        accepted
    }
    pub fn write_u8(&mut self, address: u32, value: u8) -> bool {
        // Broad bus trace: identify the real peripheral block used by firmware before
        // assuming the LCD/DMA address map. Count each byte lane written.
        if (0x4000_0000..0x5000_0000).contains(&address) {
            self.peripheral_write_count=self.peripheral_write_count.wrapping_add(1);
            self.peripheral_last_write_address=address;
            self.peripheral_last_write_value=value as u32;
            self.trace_peripheral(1,address,value as u32);
        }
        if (0x4000_E000..=0x4000_E01F).contains(&address) {
            let shift=(address&3)*8;let offset=(address-0x4000_E000)&!3;
            if offset==0x18 { self.spi_write_reg(offset,(value as u32)<<shift); }
            else { let current=self.spi_read_reg(offset);let mask=!(0xFFu32<<shift);self.spi_write_reg(offset,(current&mask)|((value as u32)<<shift)); }
            return true;
        }
        if (0x2200_0000..=0x23FF_FFFF).contains(&address) || (0x4200_0000..=0x43FF_FFFF).contains(&address) {
            if address&3!=0 { return true; }
            if let Some((target,bit))=Self::bitband_target(address) {
                let old=self.read_u8(target);
                let new=if value&1!=0 {old|(1u8<<bit)} else {old&!(1u8<<bit)};
                return self.write_u8(target,new);
            }
        }
        if let Some((port,offset))=Self::gpio_port_index(address) {
            let shift=(offset&3)*8;
            if (offset&!3)==0x20 { self.gpio_port_write_reg(port,0x20,(value as u32)<<shift); }
            else {
                let aligned=offset&!3;
                let current=self.gpio_port_read_reg(port,aligned);
                let mask=!(0xFFu32<<shift);
                self.gpio_port_write_reg(port,aligned,(current&mask)|((value as u32)<<shift));
            }
            return true;
        }
        if (0xE000_E010..0xE000_E020).contains(&address) {
            let offset=address-0xE000_E010; let shift=(offset&3)*8; let mask=!(0xffu32<<shift);
            match offset/4 {
                0=>self.systick_ctrl=((self.systick_ctrl&mask)|((value as u32)<<shift))&0x7,
                1=>self.systick_reload=((self.systick_reload&mask)|((value as u32)<<shift))&0x00FF_FFFF,
                2=>{self.systick_current=0;self.systick_pending=false;self.systick_ctrl&=!(1<<16);},
                _=>{},
            }
            return true;
        }
        if (0xE000_E100..0xE000_E108).contains(&address) {
            let offset=address-0xE000_E100; let bank=(offset/4) as usize; let shift=(offset&3)*8;
            self.nvic_iser[bank] |= (value as u32) << shift; return true;
        }
        if (0xE000_E180..0xE000_E188).contains(&address) {
            let offset=address-0xE000_E180; let bank=(offset/4) as usize; let shift=(offset&3)*8;
            self.nvic_iser[bank] &= !((value as u32) << shift); return true;
        }
        if (0x4000_F000..0x4000_F138).contains(&address) {
            let offset=address-0x4000_F000;
            let word=(offset/4) as usize;
            let shift=(offset&3)*8;
            let mask=!(0xffu32<<shift);
            self.dma_write_count=self.dma_write_count.wrapping_add(1);
            self.dma_last_write_address=address;
            self.dma_last_write_value=value as u32;
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
                    let raw_units=self.dma_registers[ctrl_word+5]&0x003F_FFFF;
                    self.dma_transfer_attempts=self.dma_transfer_attempts.wrapping_add(1);
                    self.dma_last_source=source; self.dma_last_destination=destination; self.dma_last_units=raw_units;
                    let units=raw_units.min(16_384) as usize;
                    if destination==0x4000_E01C && units>0 {
                        let mut pixels=Vec::with_capacity(units);
                        for i in 0..units {
                            let addr=source.wrapping_add((i*2) as u32);
                            pixels.push(u16::from_le_bytes([self.read_u8(addr),self.read_u8(addr.wrapping_add(1))]));
                        }
                        self.panel_frame=Some(pixels);
                        self.dma_irq_pending=true;
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
    pub fn dma_diagnostics(&self) -> (u64,u32,u32,u64,u32,u32,u32) {
        (self.dma_write_count,self.dma_last_write_address,self.dma_last_write_value,self.dma_transfer_attempts,self.dma_last_source,self.dma_last_destination,self.dma_last_units)
    }
    pub fn peripheral_diagnostics(&self) -> (u64,u32,u32,u64,u64,u64,u32,u32) {
        (self.peripheral_write_count,self.peripheral_last_write_address,self.peripheral_last_write_value,self.spi_tx_words,self.dma_write_count,self.peripheral_read_count,self.peripheral_last_read_address,self.peripheral_last_read_value)
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
    fn systick_counts_down_and_sets_pending_interrupt() {
        let mut m=MemoryStorage::new();
        fn wr32(m:&mut MemoryStorage,a:u32,v:u32){for i in 0..4 {m.write_u8(a+i,(v>>(8*i)) as u8);}}
        wr32(&mut m,0xE000_E014,2);
        wr32(&mut m,0xE000_E010,3); // ENABLE | TICKINT
        m.tick_systick(); m.tick_systick(); m.tick_systick();
        assert!(m.systick_pending);
        assert_eq!(m.systick_current,0);
        assert_ne!(m.read_u32(0xE000_E010)&(1<<16),0);
    }
    #[test]
    fn systick_countflag_clears_only_after_cpu_reads_control_register() {
        let mut m=MemoryStorage::new();
        m.systick_ctrl=(1<<16)|3;
        // Debugger-style reads are observational and leave the flag untouched.
        assert_ne!(m.read_u32(0xE000_E010)&(1<<16),0);
        assert_ne!(m.systick_ctrl&(1<<16),0);
        // A CPU word read returns the set flag, then clears it in device state.
        assert_ne!(m.read_cpu_u32(0xE000_E010)&(1<<16),0);
        assert_eq!(m.systick_ctrl&(1<<16),0);

        m.systick_ctrl|=1<<16;
        assert_ne!(m.read_cpu_u8(0xE000_E010)&1,0);
        assert_eq!(m.systick_ctrl&(1<<16),0);
    }
    #[test]
    fn paradise_gpio_ports_expose_real_button_pins_and_dial_phase() {
        let mut m=MemoryStorage::new();
        assert_eq!(m.read_u32(0x4001_8000)&(1<<9),1<<9);
        assert!(m.set_button(0,true));
        assert_eq!(m.read_u32(0x4001_8000)&(1<<9),0);
        assert!(m.set_button(1,true));
        assert_eq!(m.read_u32(0x4001_8000)&(1<<11),0);
        assert!(m.set_button(2,true));
        assert_eq!(m.read_u32(0x4001_8000)&(1<<10),0);
        m.rotate_dial(1);
        assert_eq!(m.read_u32(0x4001_A000)&3,1);
    }
    #[test]
    fn sram_bitband_alias_reads_and_writes_mailbox_bits() {
        let mut m=MemoryStorage::new();
        let target=MAILBOX_BASE+0x10;
        let alias=0x2200_0000+(target-0x2000_0000)*32+5*4;
        assert!(m.write_u32(alias,1));
        assert_eq!(m.read_u8(target),1<<5);
        assert_eq!(m.read_u32(alias),1);
        assert!(m.write_u32(alias,0));
        assert_eq!(m.read_u8(target),0);
        assert_eq!(m.read_u32(alias),0);
    }
    #[test]
    fn cpu_peripheral_reads_and_word_writes_are_traced() {
        let mut m=MemoryStorage::new();
        assert!(m.write_u32(0x4000_E000,0x12345678));
        assert_eq!(m.peripheral_write_count,4);
        assert_eq!(m.peripheral_last_write_address,0x4000_E003);
        assert_eq!(m.peripheral_last_write_value,0x12);
        let _=m.read_cpu_u32(0x4000_E000);
        assert_eq!(m.peripheral_read_count,1);
        assert_eq!(m.peripheral_last_read_address,0x4000_E000);
        assert_eq!(m.peripheral_last_read_value,0x12345678);
        let trace=m.peripheral_trace();
        assert_eq!(trace.len(),15);
        assert_eq!(&trace[0..3],&[1,0x4000_E000,0x78]);
        assert_eq!(&trace[12..15],&[0,0x4000_E000,0x12345678]);
    }
    #[test]
    fn peripheral_trace_retains_only_the_latest_32_accesses() {
        let mut m=MemoryStorage::new();
        for i in 0..33u32 {
            let _=m.write_u8(0x4500_0000+i,i as u8);
        }
        let trace=m.peripheral_trace();
        assert_eq!(trace.len(),96);
        assert_eq!(&trace[0..3],&[1,0x4500_0001,1]);
        assert_eq!(&trace[93..96],&[1,0x4500_0020,32]);
    }
    #[test]
    fn spi0_control_status_and_data_registers_are_mapped() {
        let mut m=MemoryStorage::new();
        assert_eq!(m.read_u32(0x4000_E008),0x05); // TXEMPTY | RXEMPTY
        assert!(m.write_u32(0x4000_E000,0x103));
        assert_eq!(m.read_u32(0x4000_E000),0x103);
        assert!(m.write_u32(0x4000_E004,4));
        assert_eq!(m.read_u32(0x4000_E004),4);
        assert!(m.write_u32(0x4000_E010,0x20));
        assert_eq!(m.read_u32(0x4000_E010),0x20);
        m.spi_registers[5]=0x30;
        assert!(m.write_u32(0x4000_E018,0x10));
        assert_eq!(m.read_u32(0x4000_E014),0x20); // W1C interrupt acknowledgement
        assert!(m.write_u32(0x4000_E01C,0xA55A));
        assert_eq!(m.spi_last_tx,0xA55A);
        assert_eq!(m.spi_tx_words,1);
    }
    #[test]
    fn lcd_te_signal_toggles_and_latches_enabled_rising_edge() {
        let mut m=MemoryStorage::new();
        m.gpio_port_irq_enable[1]=1<<10;
        for _ in 0..499 { m.tick_gpio(); }
        assert!(m.gpio_te_high);
        assert_eq!(m.gpio_port_read_reg(1,0)&(1<<10),1<<10);
        m.tick_gpio();
        assert!(!m.gpio_te_high);
        assert_eq!(m.gpio_port_read_reg(1,0)&(1<<10),0);
        assert!(!m.gpio_te_irq_pending());
        for _ in 0..500 { m.tick_gpio(); }
        assert!(m.gpio_te_high);
        assert!(m.gpio_te_irq_pending());
        assert_eq!(m.gpio_port_read_reg(1,0)&(1<<10),1<<10);
        m.write_u32(0x4001_9020,1<<10);
        assert!(!m.gpio_te_irq_pending());
    }
    #[test]
    fn peripheral_bitband_alias_reads_and_writes_single_gpio_bits() {
        let mut m=MemoryStorage::new();
        let alias_for=|byte_address:u32,bit:u32| 0x4200_0000+(byte_address-0x4000_0000)*32+bit*4;
        m.write_u32(0x4001_8004,1<<2);
        let data_bit=alias_for(0x4001_8000,2);
        assert!(m.write_u32(data_bit,1));
        assert_eq!(m.read_u32(0x4001_8000)&(1<<2),1<<2);
        assert_eq!(m.read_u32(data_bit),1);
        assert!(m.write_u32(data_bit,0));
        assert_eq!(m.read_u32(0x4001_8000)&(1<<2),0);
        assert_eq!(m.read_u32(data_bit),0);
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
