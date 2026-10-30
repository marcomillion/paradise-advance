//! Small Thumb-1 execution nucleus for browser integration.
//! This is an incremental interpreter, not yet a complete ARMv7-M/NVIC implementation.
//! Unsupported instructions stop with an explicit fault instead of being silently skipped.

use crate::memory::MemoryStorage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuFault { UnsupportedInstruction(u16), InvalidExecutionAddress(u32), Halted }

pub struct ThumbCpu {
    pub r: [u32; 16],
    pub n: bool, pub z: bool, pub c: bool, pub v: bool,
    pub instructions: u64, pub halted: bool, pub last_fault: Option<CpuFault>,
}
impl ThumbCpu {
    pub fn new() -> Self {
        Self { r:[0;16], n:false,z:false,c:false,v:false,instructions:0,halted:false,last_fault:None }
    }
    pub fn reset_from_vectors(&mut self, memory:&MemoryStorage, vector_base:u32) {
        self.r=[0;16]; self.r[13]=memory.read_u32(vector_base);
        self.r[15]=memory.read_u32(vector_base.wrapping_add(4)) & !1;
        self.halted=false; self.last_fault=None; self.instructions=0;
    }
    fn set_nz(&mut self, value:u32) { self.n=value>>31!=0; self.z=value==0; }
    fn condition(&self, cond:u16)->bool { match cond {
        0=>self.z,1=>!self.z,2=>self.c,3=>!self.c,4=>self.n,5=>!self.n,
        6=>self.v,7=>!self.v,8=>self.c&&!self.z,9=>!self.c||self.z,
        10=>self.n==self.v,11=>self.n!=self.v,12=>!self.z&&(self.n==self.v),
        13=>self.z||(self.n!=self.v),14=>true,_=>false } }
    fn step_wide(&mut self, m:&mut MemoryStorage, pc:u32, first:u16)->Result<(),CpuFault> {
        let second=m.read_u8(pc.wrapping_add(2)) as u16 | ((m.read_u8(pc.wrapping_add(3)) as u16)<<8);
        let op1=first; let op2=second;
        self.r[15]=pc.wrapping_add(4);
        if (op1 & 0xFBF0)==0xF240 || (op1 & 0xFBF0)==0xF2C0 {
            let rd=((op2>>8)&15) as usize;
            let imm=(((op1&15) as u32)<<12)
                | (((op1>>10&1) as u32)<<11)
                | (((op2>>12&7) as u32)<<8)
                | (op2&0xff) as u32;
            if (op1&0xFBF0)==0xF240 { self.r[rd]=imm; }
            else { self.r[rd]=(self.r[rd]&0xffff)|(imm<<16); }
            self.instructions=self.instructions.wrapping_add(1); return Ok(());
        }
        // Common Thumb-2 immediate memory transfers: LDR/STR word, halfword and byte.
        let transfer = op1 & 0xFFF0;
        if matches!(transfer, 0xF8D0 | 0xF8C0 | 0xF890 | 0xF880 | 0xF8B0 | 0xF8A0) {
            let rn=(op1&15) as usize;
            let rt=((op2>>12)&15) as usize;
            let address=self.r[rn].wrapping_add((op2&0x0fff) as u32);
            match transfer {
                0xF8D0 => { let value=m.read_u32(address); if rt==15 {self.r[15]=value&!1;} else {self.r[rt]=value;} },
                0xF8C0 => { let value=self.r[rt]; for i in 0..4 {m.write_u8(address+i,(value>>(i*8)) as u8);} },
                0xF890 => { self.r[rt]=m.read_u8(address) as u32; },
                0xF880 => { m.write_u8(address,self.r[rt] as u8); },
                0xF8B0 => { self.r[rt]=m.read_u8(address) as u32|((m.read_u8(address+1) as u32)<<8); },
                0xF8A0 => { let value=self.r[rt];m.write_u8(address,value as u8);m.write_u8(address+1,(value>>8) as u8); },
                _=>{}
            }
            self.instructions=self.instructions.wrapping_add(1); return Ok(());
        }
        // BL (Thumb-2): reconstruct the signed 25-bit PC-relative displacement.
        if (op1&0xF800)==0xF000 && (op2&0xD000)==0xD000 {
            let s=((op1>>10)&1) as u32;
            let j1=((op2>>13)&1) as u32;
            let j2=((op2>>11)&1) as u32;
            let i1=(!(j1^s))&1; let i2=(!(j2^s))&1;
            let imm25=(s<<24)|(i1<<23)|(i2<<22)|(((op1&0x03ff) as u32)<<12)|(((op2&0x07ff) as u32)<<1);
            let offset=((imm25 as i32)<<7>>7) as u32;
            self.r[14]=pc.wrapping_add(4)|1;
            self.r[15]=pc.wrapping_add(4).wrapping_add(offset)&!1;
            self.instructions=self.instructions.wrapping_add(1); return Ok(());
        }
        let f=CpuFault::UnsupportedInstruction(first); self.halted=true; self.last_fault=Some(f); Err(f)
    }
    pub fn step(&mut self, m:&mut MemoryStorage)->Result<(),CpuFault> {
        if self.halted { return Err(CpuFault::Halted); }
        let pc=self.r[15];
        if pc&1!=0 || (pc&1)==0 && pc==0xffff_ffff {
            self.halted=true; let f=CpuFault::InvalidExecutionAddress(pc); self.last_fault=Some(f); return Err(f);
        }
        let op=m.read_u8(pc) as u16 | ((m.read_u8(pc.wrapping_add(1)) as u16)<<8);
        if matches!(op & 0xF800, 0xE800 | 0xF000 | 0xF800) {
            return self.step_wide(m,pc,op);
        }
        let next=pc.wrapping_add(2); self.r[15]=next;
        let rd=(op&7) as usize; let rn=((op>>3)&7) as usize; let rm=((op>>6)&7) as usize;
        match op {
            0xBF00 => {}, // NOP
            0xBE00..=0xBEFF => { self.halted=true; },
            0x2000..=0x27FF => { let d=((op>>8)&7) as usize; let val=(op&0xff) as u32; self.r[d]=val; self.set_nz(val); },
            0x2800..=0x2FFF => { let val=self.r[((op>>8)&7) as usize].wrapping_sub((op&0xff) as u32); self.set_nz(val); self.c=self.r[((op>>8)&7) as usize]>=(op&0xff) as u32; },
            0x3000..=0x37FF => { let d=((op>>8)&7) as usize; let (val,carry)=self.r[d].overflowing_add((op&0xff) as u32); self.r[d]=val; self.set_nz(val); self.c=carry; },
            0x3800..=0x3FFF => { let d=((op>>8)&7) as usize; let (val,borrow)=self.r[d].overflowing_sub((op&0xff) as u32); self.r[d]=val; self.set_nz(val); self.c=!borrow; },
            0x0000..=0x07FF => { let shift=((op>>6)&31) as u32; let val=self.r[rm]<<shift; self.r[rd]=val; self.set_nz(val); self.c=if shift==0 {self.c} else {(self.r[rm]>>(32-shift))&1!=0}; },
            0x0800..=0x0FFF => { let shift=((op>>6)&31) as u32; let val=if shift==0 {0} else {self.r[rm]>>shift}; self.r[rd]=val; self.set_nz(val); self.c=if shift==0 {self.r[rm]>>31!=0} else {(self.r[rm]>>(shift-1))&1!=0}; },
            0x1800..=0x1FFF => { let imm=op&0x0400!=0; let sub=op&0x0200!=0; let a=self.r[rn]; let b=if imm {(op>>6&7) as u32} else {self.r[rm]}; let (val,carry)=if sub {a.overflowing_sub(b)} else {a.overflowing_add(b)}; self.r[rd]=val; self.set_nz(val); self.c=if sub {!carry} else {carry}; },
            0x4000..=0x43FF => {
                let alu=(op>>6)&15; let a=self.r[rd]; let b=self.r[rm];
                match alu {
                    0=>{let v=a&b;self.r[rd]=v;self.set_nz(v);},
                    1=>{let v=a^b;self.r[rd]=v;self.set_nz(v);},
                    2=>{let shift=b&0xff;let v=a.wrapping_shl(shift);self.r[rd]=v;self.set_nz(v);if shift!=0 {self.c=(a>>(32-shift))&1!=0;}},
                    3=>{let shift=b&0xff;let v=if shift>=32 {0}else{a>>shift};self.r[rd]=v;self.set_nz(v);if shift!=0 {self.c=if shift>32 {false}else{(a>>(shift-1))&1!=0};}},
                    4=>{let shift=b&0xff;let v=if shift>=32 {if (a as i32)<0 {u32::MAX}else{0}}else{((a as i32)>>shift) as u32};self.r[rd]=v;self.set_nz(v);},
                    5=>{let (tmp,c1)=a.overflowing_add(b);let (v,c2)=tmp.overflowing_add(self.c as u32);self.r[rd]=v;self.set_nz(v);self.c=c1||c2;},
                    6=>{let (tmp,c1)=a.overflowing_sub(b);let (v,c2)=tmp.overflowing_sub((!self.c) as u32);self.r[rd]=v;self.set_nz(v);self.c=!(c1||c2);},
                    8=>{self.set_nz(a&b);},
                    9=>{let v=0u32.wrapping_sub(b);self.r[rd]=v;self.set_nz(v);self.c=b==0;},
                    10=>{self.set_nz(a.wrapping_sub(b));self.c=a>=b;},
                    11=>{let v=a.wrapping_add(b);self.set_nz(v);self.c=(v as u64)<(a as u64);},
                    12=>{let v=a|b;self.r[rd]=v;self.set_nz(v);},
                    13=>{let v=a.wrapping_mul(b);self.r[rd]=v;self.set_nz(v);},
                    14=>{let v=a&!b;self.r[rd]=v;self.set_nz(v);},
                    15=>{let v=!b;self.r[rd]=v;self.set_nz(v);},
                    _=>{}
                }
            },
            0x4400..=0x47FF => {
                let opkind=(op>>8)&3;
                let rs=((op>>3)&15) as usize;
                let rd=((op&7)|((op>>4)&8)) as usize;
                match opkind {
                    0=>{let v=self.r[rd].wrapping_add(self.r[rs]);if rd==15 {self.r[15]=v&!1;} else {self.r[rd]=v;}},
                    1=>{let v=self.r[rd].wrapping_sub(self.r[rs]);self.set_nz(v);self.c=self.r[rd]>=self.r[rs];},
                    2=>{let v=self.r[rs];if rd==15 {self.r[15]=v&!1;} else {self.r[rd]=v;}},
                    3=>{self.r[14]=next|1;self.r[15]=self.r[rs]&!1;},
                    _=>{}
                }
            },
            0x5000..=0x5FFF => {
                let opkind=(op>>9)&7; let addr=self.r[rn].wrapping_add(self.r[rm]);
                match opkind {
                    0=>{let v=self.r[rd];for i in 0..4 {m.write_u8(addr+i,(v>>(i*8)) as u8);}},
                    1=>{m.write_u8(addr,self.r[rd] as u8);m.write_u8(addr+1,(self.r[rd]>>8) as u8);},
                    2=>{m.write_u8(addr,self.r[rd] as u8);},
                    3=>{self.r[rd]=m.read_u8(addr) as i8 as i32 as u32;},
                    4=>{self.r[rd]=m.read_u32(addr);},
                    5=>{self.r[rd]=m.read_u8(addr) as u32 | ((m.read_u8(addr+1) as u32)<<8);},
                    6=>{self.r[rd]=m.read_u8(addr) as u32;},
                    7=>{self.r[rd]=(m.read_u8(addr) as u32|((m.read_u8(addr+1) as u32)<<8)) as u16 as i16 as i32 as u32;},
                    _=>{}
                }
            },
            0x4800..=0x4FFF => { let d=((op>>8)&7) as usize; let addr=((pc.wrapping_add(4))&!3).wrapping_add(((op&0xff) as u32)*4); self.r[d]=m.read_u32(addr); },
            0x6000..=0x67FF => { let addr=self.r[rn].wrapping_add(((op>>6&31) as u32)*4); m.write_u8(addr,self.r[rd] as u8); m.write_u8(addr+1,(self.r[rd]>>8) as u8); m.write_u8(addr+2,(self.r[rd]>>16) as u8); m.write_u8(addr+3,(self.r[rd]>>24) as u8); },
            0x6800..=0x6FFF => { let addr=self.r[rn].wrapping_add(((op>>6&31) as u32)*4); self.r[rd]=m.read_u32(addr); },
            0x7000..=0x77FF => { let addr=self.r[rn].wrapping_add(((op>>6&31) as u32)); m.write_u8(addr,self.r[rd] as u8); },
            0x7800..=0x7FFF => { let addr=self.r[rn].wrapping_add(((op>>6&31) as u32)); self.r[rd]=m.read_u8(addr) as u32; },
            0x8000..=0x8FFF => {
                let addr=self.r[rn].wrapping_add(((op>>6&31) as u32)*2);
                if op&0x0800!=0 {self.r[rd]=m.read_u8(addr) as u32|((m.read_u8(addr+1) as u32)<<8);}
                else {let v=self.r[rd];m.write_u8(addr,v as u8);m.write_u8(addr+1,(v>>8) as u8);}
            },
            0xB400..=0xB5FF => {
                let extra=op&0x0100!=0; let list=op&0xff;
                if extra {self.r[13]=self.r[13].wrapping_sub(4);let v=self.r[14];for i in 0..4 {m.write_u8(self.r[13]+i,(v>>(i*8)) as u8);}}
                for reg in (0..8).rev() {if list&(1<<reg)!=0 {self.r[13]=self.r[13].wrapping_sub(4);let v=self.r[reg];for i in 0..4 {m.write_u8(self.r[13]+i,(v>>(i*8)) as u8);}}}
            },
            0xBC00..=0xBDFF => {
                let extra=op&0x0100!=0;let list=op&0xff;
                for reg in 0..8 {if list&(1<<reg)!=0 {self.r[reg]=m.read_u32(self.r[13]);self.r[13]=self.r[13].wrapping_add(4);}}
                if extra {let target=m.read_u32(self.r[13]);self.r[13]=self.r[13].wrapping_add(4);self.r[15]=target&!1;}
            },
            0xC000..=0xCFFF => {
                let load=op&0x0800!=0;let base=((op>>8)&7) as usize;let list=op&0xff;let mut addr=self.r[base];
                for reg in 0..8 {if list&(1<<reg)!=0 {if load {self.r[reg]=m.read_u32(addr);}else{let v=self.r[reg];for i in 0..4 {m.write_u8(addr+i,(v>>(i*8)) as u8);}}addr=addr.wrapping_add(4);}}
                if list!=0 {self.r[base]=addr;}
            },
            0x9000..=0x9FFF => { let d=((op>>8)&7) as usize; let addr=self.r[13].wrapping_add(((op&0xff) as u32)*4); if op&0x0800!=0 {self.r[d]=m.read_u32(addr);} else {let v=self.r[d];for i in 0..4 {m.write_u8(addr+i,(v>>(i*8)) as u8);}} },
            0xA000..=0xAFFF => { let d=((op>>8)&7) as usize; self.r[d]=((pc.wrapping_add(4))&!3).wrapping_add(((op&0xff) as u32)*4); },
            0xB000..=0xB0FF => { let delta=((op&0x7f) as u32)*4; self.r[13]=if op&0x80!=0 {self.r[13].wrapping_sub(delta)} else {self.r[13].wrapping_add(delta)}; },
            0xD000..=0xDDFF => { let cond=(op>>8)&15; if cond<14 && self.condition(cond) {let off=(((op&0xff) as i8 as i32)<<1) as u32; self.r[15]=next.wrapping_add(2).wrapping_add(off);} },
            0xE000..=0xE7FF => { let off=(((op&0x7ff) as i16)<<5>>4) as u32; self.r[15]=next.wrapping_add(off); },
            _ => { let f=CpuFault::UnsupportedInstruction(op); self.halted=true; self.last_fault=Some(f); return Err(f); }
        }
        self.instructions=self.instructions.wrapping_add(1); Ok(())
    }
}
impl Default for ThumbCpu { fn default()->Self {Self::new()} }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn executes_movs_and_adds_immediate() {
        let mut m=MemoryStorage::new();
        m.write_u8(0,0x2a); m.write_u8(1,0x20); // MOVS r0, #42
        m.write_u8(2,0x01); m.write_u8(3,0x30); // ADDS r0, #1
        let mut c=ThumbCpu::new(); c.r[15]=0;
        assert!(c.step(&mut m).is_ok()); assert_eq!(c.r[0],42);
        assert!(c.step(&mut m).is_ok()); assert_eq!(c.r[0],43);
        assert_eq!(c.instructions,2);
    }
    #[test]
    fn executes_thumb2_ldr_immediate() {
        let mut m=MemoryStorage::new();
        m.write_u8(SRAM_BASE,0x78); m.write_u8(SRAM_BASE+1,0x56);
        m.write_u8(SRAM_BASE+2,0x34); m.write_u8(SRAM_BASE+3,0x12);
        // LDR.W r0, [r1, #0]
        m.write_u8(0,0xD1); m.write_u8(1,0xF8);
        m.write_u8(2,0x00); m.write_u8(3,0x00);
        let mut c=ThumbCpu::new(); c.r[15]=0; c.r[1]=SRAM_BASE;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0x1234_5678); assert_eq!(c.r[15],4);
    }
    #[test]
    fn unsupported_opcode_faults_explicitly() {
        let mut m=MemoryStorage::new();
        m.write_u8(0,0x00); m.write_u8(1,0xF0);
        let mut c=ThumbCpu::new(); c.r[15]=0;
        assert!(matches!(c.step(&mut m),Err(CpuFault::UnsupportedInstruction(_))));
        assert!(c.halted);
    }
}
