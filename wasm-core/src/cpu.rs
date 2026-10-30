//! Small Thumb-1 execution nucleus for browser integration.
//! This is an incremental interpreter, not yet a complete ARMv7-M/NVIC implementation.
//! Unsupported instructions stop with an explicit fault instead of being silently skipped.

use crate::memory::MemoryStorage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuFault { UnsupportedInstruction(u16), InvalidExecutionAddress(u32), Halted }

pub struct ThumbCpu {
    pub r: [u32; 16],
    pub n: bool, pub z: bool, pub c: bool, pub v: bool,
    pub instructions: u64, pub halted: bool, pub last_fault: Option<CpuFault>, in_irq: bool, primask: bool, itstate: u8,
}
impl ThumbCpu {
    pub fn new() -> Self {
        Self { r:[0;16], n:false,z:false,c:false,v:false,instructions:0,halted:false,last_fault:None,in_irq:false,primask:false,itstate:0 }
    }
    pub fn reset_from_vectors(&mut self, memory:&MemoryStorage, vector_base:u32) {
        self.r=[0;16]; self.r[13]=memory.read_u32(vector_base);
        self.r[15]=memory.read_u32(vector_base.wrapping_add(4)) & !1;
        self.halted=false; self.last_fault=None; self.instructions=0; self.in_irq=false; self.primask=false; self.itstate=0;
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
        // ADDW/SUBW immediate (flags are not changed by these encodings).
        if (op1 & 0xFBF0)==0xF200 || (op1 & 0xFBF0)==0xF2A0 {
            let rn=(op1&15) as usize;
            let rd=((op2>>8)&15) as usize;
            let imm=(((op1>>10&1) as u32)<<11)
                | (((op2>>12&7) as u32)<<8)
                | (op2&0xff) as u32;
            // ADDW/SUBW use i:imm3:imm8; Rn occupies the low nibble of op1.
            let value=if (op1&0xFBF0)==0xF200 {self.r[rn].wrapping_add(imm)} else {self.r[rn].wrapping_sub(imm)};
            self.r[rd]=value;
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
    fn step_decoded(&mut self, m:&mut MemoryStorage)->Result<(),CpuFault> {
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
            0xBF10 | 0xBF20 | 0xBF30 | 0xBF40 | 0xBF50 => {}, // YIELD/WFE/WFI/SEV/SEVL hints
            0xBF01..=0xBFFF if (op&0x000F)!=0 && ((op>>4)&0xF)<14 => { self.itstate=(op&0x00FF) as u8; }, // IT block
            0xB662 => { self.primask=false; }, // CPSIE i
            0xB672 => { self.primask=true; },  // CPSID i
            0xBE00..=0xBEFF => { self.halted=true; },
            0x2000..=0x27FF => { let d=((op>>8)&7) as usize; let val=(op&0xff) as u32; self.r[d]=val; self.set_nz(val); },
            0x2800..=0x2FFF => { let val=self.r[((op>>8)&7) as usize].wrapping_sub((op&0xff) as u32); self.set_nz(val); self.c=self.r[((op>>8)&7) as usize]>=(op&0xff) as u32; },
            0x3000..=0x37FF => { let d=((op>>8)&7) as usize; let (val,carry)=self.r[d].overflowing_add((op&0xff) as u32); self.r[d]=val; self.set_nz(val); self.c=carry; },
            0x3800..=0x3FFF => { let d=((op>>8)&7) as usize; let (val,borrow)=self.r[d].overflowing_sub((op&0xff) as u32); self.r[d]=val; self.set_nz(val); self.c=!borrow; },
            0x0000..=0x07FF => { let source=((op>>3)&7) as usize; let shift=((op>>6)&31) as u32; let val=self.r[source]<<shift; self.r[rd]=val; self.set_nz(val); self.c=if shift==0 {self.c} else {(self.r[source]>>(32-shift))&1!=0}; },
            0x0800..=0x0FFF => { let source=((op>>3)&7) as usize; let shift=((op>>6)&31) as u32; let val=if shift==0 {0} else {self.r[source]>>shift}; self.r[rd]=val; self.set_nz(val); self.c=if shift==0 {self.r[source]>>31!=0} else {(self.r[source]>>(shift-1))&1!=0}; },
            0x1000..=0x17FF => {
                // ASR (immediate); an encoded shift of zero means shift by 32.
                let shift=((op>>6)&31) as u32;
                let amount=if shift==0 {32} else {shift};
                let source_reg=((op>>3)&7) as usize;
                let source=self.r[source_reg] as i32;
                let value=(source >> amount) as u32;
                self.r[rd]=value; self.set_nz(value);
                self.c=if amount==32 {source<0} else {((self.r[source_reg]>>(amount-1))&1)!=0};
            },
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
                    3=>{let target=self.r[rs];if op&0x0080!=0 {self.r[14]=next|1;}self.r[15]=target&!1;},
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
            0xB100..=0xBFFF if (op&0xF500)==0xB100 => {
                // CBZ/CBNZ: forward-only branch based on a low register.
                let source=(op&7) as usize;
                let nonzero=op&0x0800!=0;
                let should_branch=if nonzero {self.r[source]!=0} else {self.r[source]==0};
                if should_branch {
                    let offset=(((op>>9)&1) as u32)<<6 | (((op>>3)&0x1F) as u32)<<1;
                    self.r[15]=pc.wrapping_add(4).wrapping_add(offset);
                }
            },
            0xB200..=0xB2FF => {
                let kind=op&0xFFC0;
                let source=((op>>3)&7) as usize;
                let value=self.r[source];
                let result=match kind {
                    0xB200 => (value as u16 as i16 as i32) as u32, // SXTH
                    0xB240 => (value as u8 as i8 as i32) as u32,  // SXTB
                    0xB280 => value as u16 as u32,                // UXTH
                    0xB2C0 => value as u8 as u32,                 // UXTB
                    _ => { let f=CpuFault::UnsupportedInstruction(op); self.halted=true; self.last_fault=Some(f); return Err(f); }
                };
                self.r[rd]=result;
            },
            0xBA00..=0xBAFF => {
                let kind=op&0xFFC0;
                let source=((op>>3)&7) as usize;
                let value=self.r[source];
                let result=match kind {
                    0xBA00 => value.swap_bytes(), // REV
                    0xBA40 => ((value&0x00FF_00FF)<<8)|((value&0xFF00_FF00)>>8), // REV16
                    0xBAC0 => ((value as u16).swap_bytes() as i16 as i32) as u32, // REVSH
                    _ => { let f=CpuFault::UnsupportedInstruction(op); self.halted=true; self.last_fault=Some(f); return Err(f); }
                };
                self.r[rd]=result;
            },
            0xB000..=0xB0FF => { let delta=((op&0x7f) as u32)*4; self.r[13]=if op&0x80!=0 {self.r[13].wrapping_sub(delta)} else {self.r[13].wrapping_add(delta)}; },
            0xD000..=0xDDFF => { let cond=(op>>8)&15; if cond<14 && self.condition(cond) {let off=(((op&0xff) as i8 as i32)<<1) as u32; self.r[15]=next.wrapping_add(2).wrapping_add(off);} },
            0xE000..=0xE7FF => { let off=(((op&0x7ff) as i16)<<5>>4) as u32; self.r[15]=pc.wrapping_add(4).wrapping_add(off); },
            _ => { let f=CpuFault::UnsupportedInstruction(op); self.halted=true; self.last_fault=Some(f); return Err(f); }
        }
        self.instructions=self.instructions.wrapping_add(1); Ok(())
    }
    fn service_dma_irq(&mut self, m:&mut MemoryStorage) {
        if !m.dma_irq_pending || !m.dma_irq_enabled() || self.in_irq || self.primask { return; }
        m.dma_irq_pending=false;
        let mut sp=self.r[13];
        let it=self.itstate as u32;
        let xpsr=((self.n as u32)<<31)|((self.z as u32)<<30)|((self.c as u32)<<29)|((self.v as u32)<<28)|(1<<24)|((it&3)<<25)|(((it>>2)&0x3F)<<10);
        let frame=[self.r[0],self.r[1],self.r[2],self.r[3],self.r[12],self.r[14],self.r[15],xpsr];
        for value in frame.iter().rev() {
            sp=sp.wrapping_sub(4);
            for i in 0..4 { m.write_u8(sp+i,(value>>(i*8)) as u8); }
        }
        self.r[13]=sp;
        self.r[14]=0xFFFF_FFF9;
        self.r[15]=m.read_u32(0x0000_0128)&!1; // IRQ 58 vector (exception 74)
        self.in_irq=true;
    }
    fn exception_return(&mut self,m:&MemoryStorage) {
        let mut sp=self.r[13];
        let mut pop=|| { let value=m.read_u32(sp); sp=sp.wrapping_add(4); value };
        self.r[0]=pop(); self.r[1]=pop(); self.r[2]=pop(); self.r[3]=pop();
        self.r[12]=pop(); self.r[14]=pop();
        let pc=pop(); let xpsr=pop();
        self.n=xpsr&(1<<31)!=0; self.z=xpsr&(1<<30)!=0;
        self.c=xpsr&(1<<29)!=0; self.v=xpsr&(1<<28)!=0;
        self.itstate=((((xpsr>>25)&3)|(((xpsr>>10)&0x3F)<<2))&0xFF) as u8;
        self.r[13]=sp; self.r[15]=pc&!1; self.in_irq=false;
    }
    fn advance_itstate(&mut self) {
        if self.itstate==0 { return; }
        if self.itstate&0x7==0 { self.itstate=0; }
        else { self.itstate=(self.itstate&0xE0)|((self.itstate<<1)&0x1F); }
    }
    fn step_inner(&mut self,m:&mut MemoryStorage)->Result<(),CpuFault> {
        if self.in_irq && self.r[15]>=0xFFFF_FFF0 { self.exception_return(m); return Ok(()); }
        if self.halted { return Err(CpuFault::Halted); }
        let in_it=self.itstate!=0;
        if in_it {
            let pc=self.r[15];
            let op=m.read_u8(pc) as u16|((m.read_u8(pc.wrapping_add(1)) as u16)<<8);
            let wide=matches!(op&0xF800,0xE800|0xF000|0xF800);
            if !self.condition((self.itstate>>4) as u16) {
                self.r[15]=pc.wrapping_add(if wide {4}else{2});
                self.instructions=self.instructions.wrapping_add(1);
                self.advance_itstate();
                return Ok(());
            }
        }
        let result=self.step_decoded(m);
        if in_it && result.is_ok() { self.advance_itstate(); }
        result
    }
    pub fn step(&mut self,m:&mut MemoryStorage)->Result<(),CpuFault> {
        let result=self.step_inner(m);
        if result.is_ok() { self.service_dma_irq(m); }
        result
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
    fn unconditional_branch_uses_pc_plus_four() {
        let mut m=MemoryStorage::new();
        // B +0 (branch target is PC + 4).
        m.write_u8(0,0x00); m.write_u8(1,0xE0);
        let mut c=ThumbCpu::new(); c.r[15]=0;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[15],4);
    }
    #[test]
    fn executes_asr_immediate_with_sign_extension() {
        let mut m=MemoryStorage::new();
        // ASRS r0, r1, #4
        m.write_u8(0,0x08); m.write_u8(1,0x11);
        let mut c=ThumbCpu::new(); c.r[15]=0; c.r[1]=0x8000_0000;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0xF800_0000);
        assert!(c.n); assert!(!c.z);
    }
    #[test]
    fn executes_cbz_and_cbnz_forward_branches() {
        let mut m=MemoryStorage::new();
        // CBZ r0, +2; CBNZ r1, +2
        m.write_u8(0,0x08); m.write_u8(1,0xB1);
        m.write_u8(6,0x09); m.write_u8(7,0xB9);
        let mut c=ThumbCpu::new(); c.r[15]=0; c.r[0]=0; c.r[1]=1;
        assert!(c.step(&mut m).is_ok()); assert_eq!(c.r[15],6);
        assert!(c.step(&mut m).is_ok()); assert_eq!(c.r[15],12);
    }
    #[test]
    fn executes_sxtb_and_rev_register_instructions() {
        let mut m=MemoryStorage::new();
        // SXTB r0, r1; REV r2, r3
        m.write_u8(0,0x48); m.write_u8(1,0xB2);
        m.write_u8(2,0x1A); m.write_u8(3,0xBA);
        let mut c=ThumbCpu::new(); c.r[15]=0;
        c.r[1]=0x1234_5680; c.r[3]=0x1234_5678;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0xFFFF_FF80);
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[2],0x7856_3412);
    }
    #[test]
    fn cpsid_and_cpsie_control_interrupt_mask() {
        let mut m=MemoryStorage::new();
        m.pram[0..2].copy_from_slice(&0xB672u16.to_le_bytes()); // CPSID i
        m.pram[2..4].copy_from_slice(&0xB662u16.to_le_bytes()); // CPSIE i
        let mut c=ThumbCpu::new(); c.r[15]=0;
        assert!(c.step(&mut m).is_ok()); assert!(c.primask);
        assert!(c.step(&mut m).is_ok()); assert!(!c.primask);
    }
    #[test]
    fn bx_lr_does_not_overwrite_exception_return_token() {
        let mut m=MemoryStorage::new();
        m.pram[0..2].copy_from_slice(&0x4770u16.to_le_bytes()); // BX LR
        let mut c=ThumbCpu::new(); c.r[15]=0; c.r[14]=0xFFFF_FFF9;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[15],0xFFFF_FFF8);
        assert_eq!(c.r[14],0xFFFF_FFF9);
    }
    #[test]
    fn dma_completion_irq_enters_handler_and_returns_to_thread() {
        let mut m=MemoryStorage::new();
        m.pram[0..2].copy_from_slice(&0xBF00u16.to_le_bytes()); // NOP
        m.pram[0x128..0x12C].copy_from_slice(&0x1000_0101u32.to_le_bytes());
        m.flash[0x100..0x102].copy_from_slice(&0x4770u16.to_le_bytes()); // BX LR
        let mut c=ThumbCpu::new(); c.r[13]=crate::memory::SRAM_BASE+crate::memory::SRAM_SIZE as u32;
        c.r[15]=0; m.write_u8(0xE000_E107,0x04); m.dma_irq_pending=true;
        assert!(c.step(&mut m).is_ok());
        assert!(c.in_irq); assert_eq!(c.r[15],0x1000_0100);
        assert_eq!(c.r[13],crate::memory::SRAM_BASE+crate::memory::SRAM_SIZE as u32-32);
        assert!(c.step(&mut m).is_ok()); assert_eq!(c.r[15],0xFFFF_FFF8);
        assert!(c.step(&mut m).is_ok());
        assert!(!c.in_irq); assert_eq!(c.r[15],2);
        assert_eq!(c.r[13],crate::memory::SRAM_BASE+crate::memory::SRAM_SIZE as u32);
    }
    #[test]
    fn executes_thumb2_addw_immediate() {
        let mut m=MemoryStorage::new();
        // ADDW r0, r1, #0x234
        m.write_u8(0,0x01); m.write_u8(1,0xF2);
        m.write_u8(2,0x34); m.write_u8(3,0x20);
        let mut c=ThumbCpu::new(); c.r[15]=0; c.r[1]=0x1000;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0x1234); assert_eq!(c.r[15],4);
    }
    #[test]
    fn executes_thumb2_subw_immediate() {
        let mut m=MemoryStorage::new();
        // SUBW r2, r1, #0x123
        m.write_u8(0,0xA1); m.write_u8(1,0xF2);
        m.write_u8(2,0x23); m.write_u8(3,0x12);
        let mut c=ThumbCpu::new(); c.r[15]=0; c.r[1]=0x2000;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[2],0x1EDD); assert_eq!(c.r[15],4);
    }
    #[test]
    fn executes_thumb2_ldr_immediate() {
        let mut m=MemoryStorage::new();
        m.write_u8(crate::memory::SRAM_BASE,0x78); m.write_u8(crate::memory::SRAM_BASE+1,0x56);
        m.write_u8(crate::memory::SRAM_BASE+2,0x34); m.write_u8(crate::memory::SRAM_BASE+3,0x12);
        // LDR.W r0, [r1, #0]
        m.write_u8(0,0xD1); m.write_u8(1,0xF8);
        m.write_u8(2,0x00); m.write_u8(3,0x00);
        let mut c=ThumbCpu::new(); c.r[15]=0; c.r[1]=crate::memory::SRAM_BASE;
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
