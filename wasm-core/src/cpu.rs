//! Small Thumb-1 execution nucleus for browser integration.
//! This is an incremental interpreter, not yet a complete ARMv7-M/NVIC implementation.
//! Unsupported instructions stop with an explicit fault instead of being silently skipped.

use crate::memory::MemoryStorage;

fn thumb_expand_imm_c(imm12:u32,carry_in:bool)->(u32,bool) {
    if (imm12>>10)&3==0 {
        let imm8=imm12&0xFF;
        let value=match (imm12>>8)&3 {
            0=>imm8,
            1=>(imm8<<16)|imm8,
            2=>(imm8<<24)|(imm8<<8),
            _=>(imm8<<24)|(imm8<<16)|(imm8<<8)|imm8,
        };
        (value,carry_in)
    } else {
        let unrotated=0x80|(imm12&0x7F);
        let rotate=(imm12>>7)&0x1F;
        let value=unrotated.rotate_right(rotate);
        (value,value>>31!=0)
    }
}
fn shift_c(value:u32,amount:u32,kind:u32,carry_in:bool)->(u32,bool) {
    match kind {
        0=>if amount==0 {(value,carry_in)} else if amount<32 {(value<<amount,(value>>(32-amount))&1!=0)} else if amount==32 {(0,value&1!=0)} else {(0,false)},
        1=>{let n=if amount==0 {32}else{amount};if n<32 {(value>>n,(value>>(n-1))&1!=0)} else if n==32 {(0,value>>31!=0)} else {(0,false)}},
        2=>{let n=if amount==0 {32}else{amount};if n<32 {(((value as i32)>>n) as u32,(value>>(n-1))&1!=0)} else { (if (value as i32)<0 {u32::MAX}else{0},value>>31!=0) }},
        _=>if amount==0 {((carry_in as u32)<<31|(value>>1),value&1!=0)} else {let n=amount%32;if n==0 {(value,value>>31!=0)} else {let result=value.rotate_right(n);(result,result>>31!=0)}},
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuFault { UnsupportedInstruction(u16), InvalidExecutionAddress(u32), Halted }

pub struct ThumbCpu {
    pub r: [u32; 16],
    pub n: bool, pub z: bool, pub c: bool, pub v: bool,
    pub instructions: u64, pub halted: bool, pub last_fault: Option<CpuFault>, in_irq: bool, primask: bool, itstate: u8, ipsr: u32, exclusive_address: Option<u32>,
}
impl ThumbCpu {
    pub fn new() -> Self {
        Self { r:[0;16], n:false,z:false,c:false,v:false,instructions:0,halted:false,last_fault:None,in_irq:false,primask:false,itstate:0,ipsr:0,exclusive_address:None }
    }
    pub fn reset_from_vectors(&mut self, memory:&MemoryStorage, vector_base:u32) {
        self.r=[0;16]; self.r[13]=memory.read_u32(vector_base);
        self.r[15]=memory.read_u32(vector_base.wrapping_add(4)) & !1;
        // Reset returns APSR and execution-control state to architectural defaults too.
        self.n=false; self.z=false; self.c=false; self.v=false;
        self.halted=false; self.last_fault=None; self.instructions=0;
        self.in_irq=false; self.primask=false; self.itstate=0; self.ipsr=0;
        self.exclusive_address=None;
    }
    fn set_nz(&mut self, value:u32) { self.n=value>>31!=0; self.z=value==0; }
    fn set_add_flags(&mut self,a:u32,b:u32,result:u32) { self.set_nz(result); self.c=(a as u64+b as u64)>u32::MAX as u64; self.v=(!(a^b)&(a^result)&0x8000_0000)!=0; }
    fn set_sub_flags(&mut self,a:u32,b:u32,result:u32) { self.set_nz(result); self.c=a>=b; self.v=((a^b)&(a^result)&0x8000_0000)!=0; }
    fn condition(&self, cond:u16)->bool { match cond {
        0=>self.z,1=>!self.z,2=>self.c,3=>!self.c,4=>self.n,5=>!self.n,
        6=>self.v,7=>!self.v,8=>self.c&&!self.z,9=>!self.c||self.z,
        10=>self.n==self.v,11=>self.n!=self.v,12=>!self.z&&(self.n==self.v),
        13=>self.z||(self.n!=self.v),14=>true,_=>false } }
    fn step_wide(&mut self, m:&mut MemoryStorage, pc:u32, first:u16)->Result<(),CpuFault> {
        let second=m.read_u8(pc.wrapping_add(2)) as u16 | ((m.read_u8(pc.wrapping_add(3)) as u16)<<8);
        let op1=first; let op2=second;
        self.r[15]=pc.wrapping_add(4);
        // Table branch: TBB / TBH.
        if (op1&0xFFF0)==0xE8D0 && (op2&0xFFE0)==0xF000 {
            let rn=(op1&0xF) as usize;let rm=(op2&0xF) as usize;
            let is_tbh=op2&0x10!=0;
            let base=if rn==15 {self.r[15]}else{self.r[rn]};
            let table=base.wrapping_add(if is_tbh {self.r[rm].wrapping_mul(2)}else{self.r[rm]});
            let offset=if is_tbh {m.read_u8(table) as u32|((m.read_u8(table+1) as u32)<<8)}else{m.read_u8(table) as u32};
            self.r[15]=self.r[15].wrapping_add(offset<<1);
            self.instructions=self.instructions.wrapping_add(1);return Ok(());
        }
        // LDREX / STREX word exclusives. The single-core monitor is intentionally minimal.
        if (op1&0xFFF0)==0xE850 && (op2&0x0F00)==0x0F00 {
            let rn=(op1&0xF) as usize;let rt=((op2>>12)&0xF) as usize;
            let address=self.r[rn].wrapping_add(((op2&0xFF) as u32)<<2);
            self.r[rt]=m.read_u32(address);self.exclusive_address=Some(address);
            self.instructions=self.instructions.wrapping_add(1);return Ok(());
        }
        if (op1&0xFFF0)==0xE840 && (op2&0x0F00)!=0x0F00 {
            let rn=(op1&0xF) as usize;let rd=((op2>>8)&0xF) as usize;let rt=((op2>>12)&0xF) as usize;
            let address=self.r[rn].wrapping_add(((op2&0xFF) as u32)<<2);
            if self.exclusive_address==Some(address) {
                let value=self.r[rt];for i in 0..4 {m.write_u8(address+i,(value>>(i*8)) as u8);}
                self.r[rd]=0;
            } else {self.r[rd]=1;}
            self.exclusive_address=None;
            self.instructions=self.instructions.wrapping_add(1);return Ok(());
        }
        // LDRD / STRD immediate, including pre/post-index and optional writeback.
        if (op1&0xFE40)==0xE840 {
            let p=(op1>>8)&1;let u=(op1>>7)&1;let writeback=(op1>>5)&1;
            let is_load=op1&0x10!=0;
            if p!=0 || writeback!=0 {
                let rn=(op1&0xF) as usize;let rt=((op2>>12)&0xF) as usize;
                let rt2=((op2>>8)&0xF) as usize;let imm=((op2&0xFF) as u32)<<2;
                let base=self.r[rn];let offset=if u!=0 {base.wrapping_add(imm)}else{base.wrapping_sub(imm)};
                let address=if p!=0 {offset}else{base};
                if is_load {self.r[rt]=m.read_u32(address);self.r[rt2]=m.read_u32(address.wrapping_add(4));}
                else {let a=self.r[rt];let b=self.r[rt2];for i in 0..4 {m.write_u8(address+i,(a>>(i*8)) as u8);m.write_u8(address+4+i,(b>>(i*8)) as u8);}}
                if writeback!=0 {self.r[rn]=offset;}
                self.instructions=self.instructions.wrapping_add(1);return Ok(());
            }
        }
        // LDM/STM multiple-register transfers (increment-after and decrement-before).
        if (op1&0xFE40)==0xE800 && matches!((op1>>7)&3,1|2) {
            let decrement_before=((op1>>7)&3)==2;let is_load=op1&0x10!=0;let writeback=op1&0x20!=0;
            let rn=(op1&0xF) as usize;let list=op2;let count=list.count_ones();let base=self.r[rn];
            let mut address=if decrement_before {base.wrapping_sub(4*count)}else{base};
            for reg in 0..16 {
                if list&(1<<reg)==0 {continue;}
                if is_load {self.r[reg]=m.read_u32(address);}
                else {let value=self.r[reg];for i in 0..4 {m.write_u8(address+i,(value>>(i*8)) as u8);}}
                address=address.wrapping_add(4);
            }
            if writeback {self.r[rn]=if decrement_before {base.wrapping_sub(4*count)}else{base.wrapping_add(4*count)};}
            self.instructions=self.instructions.wrapping_add(1);return Ok(());
        }
        // Thumb-2 shifted-register and modified-immediate data-processing operations.
        let is_shifted=(op1&0xFE00)==0xEA00 || (op1&0xFE00)==0xEB00;
        let is_modified_imm=(op1&0xFA00)==0xF000 && (op2&0x8000)==0;
        if is_shifted || is_modified_imm {
            let rn=(op1&0xF) as usize;
            let rd=((op2>>8)&0xF) as usize;
            let operation=(op1>>5)&0xF;
            let set_flags=op1&0x10!=0;
            let value_n=if rn==15 {0}else{self.r[rn]};
            let (value_2,shifter_c)=if is_modified_imm {
                let imm12=(((op1>>10)&1) as u32)<<11|(((op2>>12)&7) as u32)<<8|(op2&0xFF) as u32;
                thumb_expand_imm_c(imm12,self.c)
            } else {
                let rm=(op2&0xF) as usize;
                let amount=((((op2>>12)&7)<<2)|((op2>>6)&3)) as u32;
                let kind=((op2>>4)&3) as u32;
                shift_c(self.r[rm],amount,kind,self.c)
            };
            let mut carry=shifter_c;let mut overflow=self.v;
            let result=match operation {
                0=>value_n&value_2, // AND / TST
                1=>value_n&!value_2, // BIC
                2=>value_n|value_2, // ORR / MOV
                3=>value_n|!value_2, // ORN / MVN
                4=>value_n^value_2, // EOR / TEQ
                8=>{let r=value_n.wrapping_add(value_2);carry=(value_n as u64+value_2 as u64)>u32::MAX as u64;overflow=(!(value_n^value_2)&(value_n^r)&0x8000_0000)!=0;r},
                10=>{let sum=value_n as u64+value_2 as u64+self.c as u64;let r=sum as u32;carry=sum>u32::MAX as u64;overflow=(!(value_n^value_2)&(value_n^r)&0x8000_0000)!=0;r},
                11=>{let borrow=(!self.c) as u32;let r=value_n.wrapping_sub(value_2).wrapping_sub(borrow);carry=(value_n as u64)>=(value_2 as u64+borrow as u64);overflow=((value_n^value_2)&(value_n^r)&0x8000_0000)!=0;r},
                13=>{let r=value_n.wrapping_sub(value_2);carry=value_n>=value_2;overflow=((value_n^value_2)&(value_n^r)&0x8000_0000)!=0;r},
                14=>{let r=value_2.wrapping_sub(value_n);carry=value_2>=value_n;overflow=((value_2^value_n)&(value_2^r)&0x8000_0000)!=0;r},
                _=>{let f=CpuFault::UnsupportedInstruction(op1);self.halted=true;self.last_fault=Some(f);return Err(f);}
            };
            let compare_only=rd==15 && matches!(operation,0|4|8|13);
            if !compare_only { if rd==15 {self.r[15]=result&!1;} else {self.r[rd]=result;} }
            if set_flags || compare_only {
                self.set_nz(result);self.c=carry;self.v=overflow;
            }
            self.instructions=self.instructions.wrapping_add(1);return Ok(());
        }
        // Multiply, multiply-accumulate, and divide forms used by compiled C code.
        if (op1&0xFF80)==0xFB00 || (op1&0xFF80)==0xFB80 {
            let rn=(op1&0xF) as usize;
            let rm=(op2&0xF) as usize;
            let rd_hi=((op2>>8)&0xF) as usize;
            let rd_lo=((op2>>12)&0xF) as usize;
            let kind=(op2>>4)&0xF;
            let a=self.r[rn]; let b=self.r[rm];
            match op1&0xFFF0 {
                0xFB00=>{
                    let product=a.wrapping_mul(b);
                    let result=if rd_lo==15 {product} else if kind==0 {self.r[rd_lo].wrapping_add(product)} else if kind==1 {self.r[rd_lo].wrapping_sub(product)} else {
                        let f=CpuFault::UnsupportedInstruction(op1);self.halted=true;self.last_fault=Some(f);return Err(f);
                    };
                    self.r[rd_hi]=result;
                },
                0xFB80 if kind==0=>{
                    let product=(a as i32 as i64).wrapping_mul(b as i32 as i64) as u64;
                    self.r[rd_lo]=product as u32;self.r[rd_hi]=(product>>32) as u32;
                },
                0xFB90 if kind==15=>{
                    self.r[rd_hi]=if b==0 {0}else{(a as i32).wrapping_div(b as i32) as u32};
                },
                0xFBA0 if kind==0=>{
                    let product=(a as u64).wrapping_mul(b as u64);
                    self.r[rd_lo]=product as u32;self.r[rd_hi]=(product>>32) as u32;
                },
                0xFBB0 if kind==15=>{
                    self.r[rd_hi]=if b==0 {0}else{a/b};
                },
                0xFBC0 if kind==0=>{
                    let acc=((self.r[rd_hi] as u64)<<32)|self.r[rd_lo] as u64;
                    let product=(a as i32 as i64).wrapping_mul(b as i32 as i64) as u64;
                    let result=acc.wrapping_add(product);
                    self.r[rd_lo]=result as u32;self.r[rd_hi]=(result>>32) as u32;
                },
                0xFBE0 if kind==0=>{
                    let acc=((self.r[rd_hi] as u64)<<32)|self.r[rd_lo] as u64;
                    let result=acc.wrapping_add((a as u64).wrapping_mul(b as u64));
                    self.r[rd_lo]=result as u32;self.r[rd_hi]=(result>>32) as u32;
                },
                _=>{let f=CpuFault::UnsupportedInstruction(op1);self.halted=true;self.last_fault=Some(f);return Err(f);}
            }
            self.instructions=self.instructions.wrapping_add(1);return Ok(());
        }
        // CLZ (count leading zero bits).
        if (op1&0xFFF0)==0xFAB0 && (op2&0xF0F0)==0xF080 {
            let rn=(op1&0xF) as usize;let rd=((op2>>8)&0xF) as usize;
            self.r[rd]=self.r[rn].leading_zeros();
            self.instructions=self.instructions.wrapping_add(1);return Ok(());
        }
        // MRS / MSR access to core special registers used by interrupt handlers.
        if op1==0xF3EF && (op2&0xF000)==0x8000 {
            let rd=((op2>>8)&0xF) as usize;let sysm=op2&0xFF;
            let value=match sysm {
                0=>((self.n as u32)<<31)|((self.z as u32)<<30)|((self.c as u32)<<29)|((self.v as u32)<<28),
                3=>((self.n as u32)<<31)|((self.z as u32)<<30)|((self.c as u32)<<29)|((self.v as u32)<<28)|(1<<24)|((self.itstate as u32&3)<<25)|(((self.itstate as u32>>2)&0x3F)<<10)|(self.ipsr&0x1FF),
                5=>self.ipsr,
                8|9=>self.r[13],
                16=>self.primask as u32,
                _=>0,
            };
            self.r[rd]=value;self.instructions=self.instructions.wrapping_add(1);return Ok(());
        }
        if (op1&0xFFF0)==0xF380 && (op2&0xF000)==0x8000 {
            let rn=(op1&0xF) as usize;let sysm=op2&0xFF;let value=self.r[rn];
            match sysm {
                0..=3=>{self.n=value&(1<<31)!=0;self.z=value&(1<<30)!=0;self.c=value&(1<<29)!=0;self.v=value&(1<<28)!=0;},
                8|9=>self.r[13]=value&!3,
                16=>self.primask=value&1!=0,
                _=>{},
            }
            self.instructions=self.instructions.wrapping_add(1);return Ok(());
        }
        // BFI/BFC/SBFX/UBFX bit-field operations.
        if (op1&0xFBF0)==0xF340 || (op1&0xFBF0)==0xF360 || (op1&0xFBF0)==0xF3C0 {
            let rn=(op1&0xF) as usize;
            let rd=((op2>>8)&0xF) as usize;
            let lsb=((((op2>>12)&7)<<2)|((op2>>6)&3)) as u32;
            if (op1&0x00F0)==0x0060 {
                let msb=(op2&0x1F) as u32;
                if msb<lsb { let f=CpuFault::UnsupportedInstruction(op1);self.halted=true;self.last_fault=Some(f);return Err(f); }
                let width=msb-lsb+1;
                let mask=((((1u64<<width)-1)<<lsb)&0xFFFF_FFFF) as u32;
                let current=self.r[rd];
                let inserted=if rn==15 {0}else{self.r[rn].wrapping_shl(lsb)};
                self.r[rd]=(current&!mask)|(inserted&mask);
            } else {
                let width=((op2&0x1F) as u32)+1;
                let mask=(((1u64<<width)-1)&0xFFFF_FFFF) as u32;
                let mut value=(self.r[rn]>>lsb)&mask;
                if (op1&0x00F0)==0x0040 && width<32 && value&(1<<(width-1))!=0 { value|=!mask; }
                self.r[rd]=value;
            }
            self.instructions=self.instructions.wrapping_add(1);return Ok(());
        }
        // LSL/LSR/ASR/ROR by register (no memory-system side effects).
        if (op1&0xFF80)==0xFA00 && (op2&0xF0F0)==0xF000 {
            let rn=(op1&0xF) as usize;let rm=(op2&0xF) as usize;
            let rd=((op2>>8)&0xF) as usize;let set_flags=op1&0x0010!=0;
            let value=self.r[rn];let amount=self.r[rm]&0xFF;
            let (result,carry)=match (op1>>5)&3 {
                0=>{let r=if amount>=32 {0}else{value.wrapping_shl(amount)};let c=if amount==0 {self.c}else if amount<=32 {(value>>(32-amount.min(32)))&1!=0}else{false};(r,c)},
                1=>{let r=if amount>=32 {0}else{value>>amount};let c=if amount==0 {self.c}else if amount<=32 {(value>>(amount-1).min(31))&1!=0}else{false};(r,c)},
                2=>{let r=if amount>=32 {if (value as i32)<0 {u32::MAX}else{0}}else{((value as i32)>>amount) as u32};let c=if amount==0 {self.c}else if amount>=32 {(value>>31)!=0}else{(value>>(amount-1))&1!=0};(r,c)},
                _=>{let r=if amount==0 {value}else{value.rotate_right(amount)};let c=if amount==0 {self.c}else{r>>31!=0};(r,c)},
            };
            self.r[rd]=result;if set_flags {self.set_nz(result);self.c=carry;}
            self.instructions=self.instructions.wrapping_add(1);return Ok(());
        }
        // B.W unconditional: reconstruct the signed 25-bit PC-relative offset.
        if (op1&0xF800)==0xF000 && (op2&0xD000)==0x9000 {
            let sign=((op1>>10)&1) as u32;
            let imm10=(op1&0x03FF) as u32;
            let j1=((op2>>13)&1) as u32;
            let j2=((op2>>11)&1) as u32;
            let imm11=(op2&0x07FF) as u32;
            let i1=(!(j1^sign))&1; let i2=(!(j2^sign))&1;
            let mut imm25=(sign<<24)|(i1<<23)|(i2<<22)|(imm10<<12)|(imm11<<1);
            if imm25&(1<<24)!=0 { imm25|=0xFE00_0000; }
            self.r[15]=self.r[15].wrapping_add(imm25);
            self.instructions=self.instructions.wrapping_add(1); return Ok(());
        }
        // B<cond>.W conditional long branch.
        if (op1&0xF800)==0xF000 && (op2&0xD000)==0x8000 {
            let cond=(op1>>6)&0xF;
            if cond<14 && self.condition(cond) {
                let sign=((op1>>10)&1) as u32;
                let imm6=(op1&0x3F) as u32;
                let j1=((op2>>13)&1) as u32;
                let j2=((op2>>11)&1) as u32;
                let imm11=(op2&0x07FF) as u32;
                let mut imm21=(sign<<20)|(j2<<19)|(j1<<18)|(imm6<<12)|(imm11<<1);
                if imm21&(1<<20)!=0 { imm21|=0xFFE0_0000; }
                self.r[15]=self.r[15].wrapping_add(imm21);
            }
            self.instructions=self.instructions.wrapping_add(1); return Ok(());
        }
        // Barriers and hints have no additional effect in this uncached model.
        if (op1==0xF3BF && (op2&0xFF00)==0x8F00)
            || (op1==0xF3AF && (op2&0xFF00)==0x8000) {
            self.instructions=self.instructions.wrapping_add(1); return Ok(());
        }
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
        if matches!(transfer, 0xF8D0 | 0xF8C0 | 0xF890 | 0xF880 | 0xF8B0 | 0xF8A0 | 0xF990 | 0xF9B0) {
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
                0xF990 => { self.r[rt]=m.read_u8(address) as i8 as i32 as u32; },
                0xF9B0 => { self.r[rt]=(m.read_u8(address) as u32|((m.read_u8(address+1) as u32)<<8)) as u16 as i16 as i32 as u32; },
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
            0x2800..=0x2FFF => { let a=self.r[((op>>8)&7) as usize];let b=(op&0xff) as u32;let val=a.wrapping_sub(b);self.set_sub_flags(a,b,val); },
            0x3000..=0x37FF => { let d=((op>>8)&7) as usize;let a=self.r[d];let b=(op&0xff) as u32;let val=a.wrapping_add(b);self.r[d]=val;self.set_add_flags(a,b,val); },
            0x3800..=0x3FFF => { let d=((op>>8)&7) as usize;let a=self.r[d];let b=(op&0xff) as u32;let val=a.wrapping_sub(b);self.r[d]=val;self.set_sub_flags(a,b,val); },
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
            0x1800..=0x1FFF => { let imm=op&0x0400!=0;let sub=op&0x0200!=0;let a=self.r[rn];let b=if imm {(op>>6&7) as u32}else{self.r[rm]};let val=if sub {a.wrapping_sub(b)}else{a.wrapping_add(b)};self.r[rd]=val;if sub {self.set_sub_flags(a,b,val)}else{self.set_add_flags(a,b,val)}; },
            0x4000..=0x43FF => {
                let alu=(op>>6)&15; let a=self.r[rd]; let b=self.r[rm];
                match alu {
                    0=>{let v=a&b;self.r[rd]=v;self.set_nz(v);},
                    1=>{let v=a^b;self.r[rd]=v;self.set_nz(v);},
                    2=>{
                        let shift=b&0xff;
                        let (v,carry)=if shift==0 {(a,self.c)}
                            else if shift<32 {(a<<shift,(a>>(32-shift))&1!=0)}
                            else if shift==32 {(0,a&1!=0)}
                            else {(0,false)};
                        self.r[rd]=v;self.set_nz(v);if shift!=0 {self.c=carry;}
                    },
                    3=>{
                        let shift=b&0xff;
                        let v=if shift>=32 {0}else{a>>shift};
                        self.r[rd]=v;self.set_nz(v);
                        if shift!=0 {self.c=if shift>32 {false}else{(a>>(shift-1))&1!=0};}
                    },
                    4=>{
                        let shift=b&0xff;
                        let v=if shift>=32 {if (a as i32)<0 {u32::MAX}else{0}}else{((a as i32)>>shift) as u32};
                        self.r[rd]=v;self.set_nz(v);
                        // Register ASR preserves carry for a zero shift; shifts of
                        // 32 or more carry the original sign bit.
                        if shift!=0 {self.c=if shift>=32 {(a>>31)!=0}else{((a>>(shift-1))&1)!=0};}
                    },
                    5=>{let sum=a as u64+b as u64+self.c as u64;let v=sum as u32;self.r[rd]=v;self.set_nz(v);self.c=sum>u32::MAX as u64;self.v=(!(a^b)&(a^v)&0x8000_0000)!=0;},
                    6=>{let borrow=(!self.c) as u32;let v=a.wrapping_sub(b).wrapping_sub(borrow);self.r[rd]=v;self.set_nz(v);self.c=(a as u64)>=(b as u64+borrow as u64);self.v=((a^b)&(a^v)&0x8000_0000)!=0;},
                    8=>{self.set_nz(a&b);},
                    9=>{let v=0u32.wrapping_sub(b);self.r[rd]=v;self.set_sub_flags(0,b,v);},
                    10=>{self.set_sub_flags(a,b,a.wrapping_sub(b));},
                    11=>{self.set_add_flags(a,b,a.wrapping_add(b));},
                    12=>{let v=a|b;self.r[rd]=v;self.set_nz(v);},
                    13=>{let v=a.wrapping_mul(b);self.r[rd]=v;self.set_nz(v);},
                    14=>{let v=a&!b;self.r[rd]=v;self.set_nz(v);},
                    7=>{
                        let shift=b&0xff;
                        let v=if shift==0 {a}else{a.rotate_right(shift)};
                        self.r[rd]=v;self.set_nz(v);
                        if shift!=0 {self.c=v>>31!=0;}
                    },
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
                    1=>{let a=self.r[rd];let b=self.r[rs];self.set_sub_flags(a,b,a.wrapping_sub(b));},
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
    fn enter_exception(&mut self,m:&mut MemoryStorage,exception_number:u32) {
        let mut sp=self.r[13];
        let it=self.itstate as u32;
        let xpsr=((self.n as u32)<<31)|((self.z as u32)<<30)|((self.c as u32)<<29)|((self.v as u32)<<28)|(1<<24)|((it&3)<<25)|(((it>>2)&0x3F)<<10)|(self.ipsr&0x1FF);
        let frame=[self.r[0],self.r[1],self.r[2],self.r[3],self.r[12],self.r[14],self.r[15],xpsr];
        for value in frame.iter().rev() {
            sp=sp.wrapping_sub(4);
            for i in 0..4 { m.write_u8(sp+i,(*value>>(i*8)) as u8); }
        }
        self.r[13]=sp;
        self.r[14]=0xFFFF_FFF9;
        self.r[15]=m.read_u32(exception_number.wrapping_mul(4))&!1;
        self.ipsr=exception_number;self.in_irq=true;
    }
    fn service_pending_interrupt(&mut self,m:&mut MemoryStorage) {
        if self.in_irq || self.primask { return; }
        if m.systick_pending && m.systick_ctrl&2!=0 {
            m.systick_pending=false;
            self.enter_exception(m,15); // SysTick exception vector
            return;
        }
        if m.dma_irq_pending && m.dma_irq_enabled() {
            m.dma_irq_pending=false;
            self.enter_exception(m,74); // DMA IRQ 58 + core exception offset 16
        }
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
        self.ipsr=xpsr&0x1FF;
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
        let saved_flags=(self.n,self.z,self.c,self.v);
        let preserve_flags=in_it && (self.itstate&0x7)!=0;
        let result=self.step_decoded(m);
        if result.is_ok() {
            if preserve_flags { (self.n,self.z,self.c,self.v)=saved_flags; }
            if in_it { self.advance_itstate(); }
        }
        result
    }
    pub fn step(&mut self,m:&mut MemoryStorage)->Result<(),CpuFault> {
        let result=self.step_inner(m);
        if result.is_ok() { m.tick_systick(); self.service_pending_interrupt(m); }
        result
    }
}
impl Default for ThumbCpu { fn default()->Self {Self::new()} }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reset_from_vectors_clears_stale_flags_and_execution_state() {
        let mut m=MemoryStorage::new();
        m.write_u8(0,0x00); m.write_u8(1,0xBF);
        m.write_u8(0x100,0x00); m.write_u8(0x101,0xBF);
        m.write_u8(0x200,0x00); m.write_u8(0x201,0xBF);
        m.write_u8(0x20,0x00); m.write_u8(0x21,0x01); // vector PC = 0x100 (Thumb)
        m.write_u8(0x1C,0x00); m.write_u8(0x1D,0x00); m.write_u8(0x1E,0x01); m.write_u8(0x1F,0x18); // SP = 0x18010000
        let mut c=ThumbCpu::new();
        c.n=true; c.z=true; c.c=true; c.v=true;
        c.halted=true; c.last_fault=Some(CpuFault::UnsupportedInstruction(0xFFFF));
        c.instructions=99; c.primask=true; c.itstate=0x08; c.ipsr=15;
        c.exclusive_address=Some(0x100);
        c.reset_from_vectors(&m,0x1C);
        assert_eq!(c.r[13],0x1801_0000);
        assert_eq!(c.r[15],0x100);
        assert!(!c.n && !c.z && !c.c && !c.v);
        assert!(!c.halted); assert_eq!(c.last_fault,None); assert_eq!(c.instructions,0);
        assert!(!c.primask); assert_eq!(c.itstate,0); assert_eq!(c.ipsr,0);
        assert_eq!(c.exclusive_address,None); assert!(!c.in_irq);
    }

    #[test]
    fn push_and_pop_preserve_register_order_and_restore_stack_pointer() {
        let mut m=MemoryStorage::new();
        // PUSH {r0, r2, lr}; POP {r0, r2, pc}
        m.pram[0..2].copy_from_slice(&0xB505u16.to_le_bytes());
        m.pram[2..4].copy_from_slice(&0xBD05u16.to_le_bytes());
        let initial_sp=crate::memory::SRAM_BASE+0x100;
        let mut c=ThumbCpu::new();
        c.r[13]=initial_sp; c.r[0]=0x1122_3344; c.r[2]=0x5566_7788; c.r[14]=0x101;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[13],initial_sp-12);
        assert_eq!(m.read_u32(initial_sp-12),0x1122_3344);
        assert_eq!(m.read_u32(initial_sp-8),0x5566_7788);
        assert_eq!(m.read_u32(initial_sp-4),0x101);
        c.r[0]=0; c.r[2]=0; c.r[15]=2;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0x1122_3344);
        assert_eq!(c.r[2],0x5566_7788);
        assert_eq!(c.r[13],initial_sp);
        assert_eq!(c.r[15],0x100);
    }

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
    fn register_lsl_handles_shift_counts_at_and_above_word_size() {
        let mut m=MemoryStorage::new();
        // LSLS r0, r1: register-controlled shift.
        m.pram[0..2].copy_from_slice(&0x4088u16.to_le_bytes());
        let mut c=ThumbCpu::new(); c.r[15]=0; c.r[1]=0x8000_0001; c.r[2]=32;
        // r1 is the shift count register for this encoding; set it per case below.
        c.r[1]=32;
        c.r[0]=0x8000_0001;
        // Use r0 as the destination and r1 as source/count per encoding semantics:
        // the encoded instruction reads r0 and shifts it by r1.
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0);
        assert!(c.c); // shift by 32 carries original bit 0.
        c.r[15]=0; c.r[1]=33; c.r[0]=0x8000_0001; c.halted=false; c.last_fault=None;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0);
        assert!(!c.c); // counts greater than 32 clear carry.
    }

    #[test]
    fn register_lsr_updates_carry_at_word_size_boundaries() {
        let mut m=MemoryStorage::new();
        // LSRS r0, r1 (register-controlled logical right shift).
        m.pram[0..2].copy_from_slice(&0x40C8u16.to_le_bytes());
        let mut c=ThumbCpu::new();

        c.r[15]=0; c.r[0]=0x8000_0001; c.r[1]=32; c.c=false;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0);
        assert!(c.c); // A shift of exactly 32 carries original bit 31.

        c.r[15]=0; c.r[0]=0x8000_0001; c.r[1]=33; c.c=true;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0);
        assert!(!c.c); // Larger shifts clear carry.

        c.r[15]=0; c.r[0]=0x1234_5678; c.r[1]=0; c.c=true;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0x1234_5678);
        assert!(c.c); // A zero shift leaves carry unchanged.
    }

    #[test]
    fn register_asr_updates_carry_for_large_and_zero_shifts() {
        let mut m=MemoryStorage::new();
        // ASRS r0, r1 (register-controlled arithmetic right shift).
        m.pram[0..2].copy_from_slice(&0x4108u16.to_le_bytes());
        let mut c=ThumbCpu::new();
        c.r[15]=0; c.r[0]=0x8000_0001; c.r[1]=32; c.c=false;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],u32::MAX);
        assert!(c.c); // At 32+, carry is the original sign bit.

        c.r[15]=0; c.r[0]=0x7fff_ffff; c.r[1]=33; c.c=true;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0);
        assert!(!c.c);

        c.r[15]=0; c.r[0]=0x1234_5678; c.r[1]=0; c.c=true;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0x1234_5678);
        assert!(c.c); // A zero register shift leaves carry unchanged.
    }

    #[test]
    fn register_ror_is_implemented_and_sets_carry_from_result() {
        let mut m=MemoryStorage::new();
        // RORS r0, r1
        m.pram[0..2].copy_from_slice(&0x41C8u16.to_le_bytes());
        let mut c=ThumbCpu::new(); c.r[15]=0; c.r[0]=0x8000_0001; c.r[1]=1;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0xC000_0000);
        assert!(c.n); assert!(!c.z); assert!(c.c);
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
    fn it_block_preserves_flags_until_last_instruction() {
        let mut m=MemoryStorage::new();
        m.pram[0..2].copy_from_slice(&0xBF04u16.to_le_bytes()); // ITT EQ
        m.pram[2..4].copy_from_slice(&0x2001u16.to_le_bytes()); // MOVS r0, #1
        m.pram[4..6].copy_from_slice(&0x2102u16.to_le_bytes()); // MOVS r1, #2
        let mut c=ThumbCpu::new(); c.r[15]=0; c.z=true;
        assert!(c.step(&mut m).is_ok());
        assert!(c.step(&mut m).is_ok());
        assert!(c.z); assert_eq!(c.r[0],1);
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[1],2); assert!(!c.z); assert_eq!(c.itstate,0);
    }

    #[test]
    fn it_eq_skips_instruction_when_condition_is_false() {
        let mut m=MemoryStorage::new();
        m.pram[0..2].copy_from_slice(&0xBF08u16.to_le_bytes()); // IT EQ
        m.pram[2..4].copy_from_slice(&0x2001u16.to_le_bytes()); // MOVS r0, #1
        let mut c=ThumbCpu::new(); c.r[15]=0; c.z=false;
        assert!(c.step(&mut m).is_ok()); assert_eq!(c.itstate,0x08);
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0); assert_eq!(c.r[15],4); assert_eq!(c.itstate,0);
    }

    #[test]
    fn it_eq_executes_instruction_when_condition_is_true() {
        let mut m=MemoryStorage::new();
        m.pram[0..2].copy_from_slice(&0xBF08u16.to_le_bytes()); // IT EQ
        m.pram[2..4].copy_from_slice(&0x2001u16.to_le_bytes()); // MOVS r0, #1
        let mut c=ThumbCpu::new(); c.r[15]=0; c.z=true;
        assert!(c.step(&mut m).is_ok());
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],1); assert_eq!(c.r[15],4); assert_eq!(c.itstate,0);
    }
    #[test]
    fn arithmetic_sets_signed_overflow_and_carry_flags() {
        let mut m=MemoryStorage::new();
        // ADDS r0, #1; CMP r0, #0x80
        m.write_u8(0,0x01); m.write_u8(1,0x30);
        m.write_u8(2,0x80); m.write_u8(3,0x28);
        let mut c=ThumbCpu::new(); c.r[15]=0; c.r[0]=0x7FFF_FFFF;
        assert!(c.step(&mut m).is_ok());
        assert_eq!(c.r[0],0x8000_0000); assert!(c.n); assert!(c.v);
        assert!(c.step(&mut m).is_ok()); assert!(!c.n); assert!(c.v); assert!(c.c);
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
    fn systick_interrupt_enters_handler_and_returns_to_thread() {
        let mut m=MemoryStorage::new();
        m.pram[0..6].copy_from_slice(&[0x00,0xBF,0x00,0xBF,0x00,0xBF]);
        m.pram[0x3C..0x40].copy_from_slice(&0x1000_0101u32.to_le_bytes());
        m.flash[0x100..0x102].copy_from_slice(&0x4770u16.to_le_bytes());
        m.write_u8(0xE000_E014,2);
        m.write_u8(0xE000_E010,3); // SysTick ENABLE | TICKINT
        let mut c=ThumbCpu::new(); c.r[13]=crate::memory::SRAM_BASE+crate::memory::SRAM_SIZE as u32;
        c.r[15]=0;
        assert!(c.step(&mut m).is_ok());
        assert!(c.step(&mut m).is_ok());
        assert!(c.step(&mut m).is_ok());
        assert!(c.in_irq); assert_eq!(c.r[15],0x1000_0100);
        assert!(c.step(&mut m).is_ok()); assert_eq!(c.r[15],0xFFFF_FFF8);
        assert!(c.step(&mut m).is_ok());
        assert!(!c.in_irq); assert_eq!(c.r[15],6);
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
    fn executes_thumb2_mov_immediate_and_orr_register() {
        let mut m=MemoryStorage::new();
        // MOV.W r0, #0xFF; ORR.W r2, r0, r1
        m.write_u8(0,0x4F);m.write_u8(1,0xF0);m.write_u8(2,0xFF);m.write_u8(3,0x00);
        m.write_u8(4,0x40);m.write_u8(5,0xEA);m.write_u8(6,0x01);m.write_u8(7,0x02);
        let mut c=ThumbCpu::new();c.r[15]=0;c.r[1]=0x100;
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[0],0xFF);
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[2],0x1FF);
    }
    #[test]
    fn executes_thumb2_stmia_and_ldmia_with_writeback() {
        let mut m=MemoryStorage::new();
        // STMIA r0!, {r1,r2}; LDMIA r3!, {r4,r5}
        m.write_u8(0,0xA0);m.write_u8(1,0xE8);m.write_u8(2,0x06);m.write_u8(3,0x00);
        m.write_u8(4,0xB3);m.write_u8(5,0xE8);m.write_u8(6,0x30);m.write_u8(7,0x00);
        let mut c=ThumbCpu::new();c.r[15]=0;c.r[0]=0x100;c.r[1]=0x1111;c.r[2]=0x2222;c.r[3]=0x100;
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[0],0x108);
        assert_eq!(m.read_u32(0x100),0x1111);assert_eq!(m.read_u32(0x104),0x2222);
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[3],0x108);
        assert_eq!(c.r[4],0x1111);assert_eq!(c.r[5],0x2222);
    }
    #[test]
    fn executes_ldrex_and_strex_with_single_core_monitor() {
        let mut m=MemoryStorage::new();
        // LDREX r0, [r1]; STREX r2, r3, [r1]
        m.write_u8(0,0x51);m.write_u8(1,0xE8);m.write_u8(2,0x00);m.write_u8(3,0x0F);
        m.write_u8(4,0x41);m.write_u8(5,0xE8);m.write_u8(6,0x00);m.write_u8(7,0x32);
        let mut c=ThumbCpu::new();c.r[15]=0;c.r[1]=0x100;c.r[3]=0xABCD_1234;
        m.write_u8(0x100,0x78);m.write_u8(0x101,0x56);m.write_u8(0x102,0x34);m.write_u8(0x103,0x12);
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[0],0x1234_5678);
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[2],0);
        assert_eq!(m.read_u32(0x100),0xABCD_1234);
    }
    #[test]
    fn executes_thumb2_ldrd_and_tbb() {
        let mut m=MemoryStorage::new();
        // LDRD r0, r1, [r2]; TBB [r3, r4]
        m.write_u8(0,0xD2);m.write_u8(1,0xE9);m.write_u8(2,0x00);m.write_u8(3,0x01);
        m.write_u8(4,0xD3);m.write_u8(5,0xE8);m.write_u8(6,0x04);m.write_u8(7,0xF0);
        m.write_u8(0x100,0x34);m.write_u8(0x101,0x12);m.write_u8(0x104,0x78);m.write_u8(0x105,0x56);
        m.write_u8(0x202,3);
        let mut c=ThumbCpu::new();c.r[15]=0;c.r[2]=0x100;c.r[3]=0x200;c.r[4]=2;
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[0],0x1234);assert_eq!(c.r[1],0x5678);
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[15],14);
    }
    #[test]
    fn executes_thumb2_clz() {
        let mut m=MemoryStorage::new();
        // CLZ r2, r1
        m.write_u8(0,0xB1);m.write_u8(1,0xFA);m.write_u8(2,0x80);m.write_u8(3,0xF2);
        let mut c=ThumbCpu::new();c.r[15]=0;c.r[1]=1;
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[2],31);
    }
    #[test]
    fn executes_mrs_and_msr_primask() {
        let mut m=MemoryStorage::new();
        // MRS r0, PRIMASK; MSR PRIMASK, r1
        m.write_u8(0,0xEF);m.write_u8(1,0xF3);m.write_u8(2,0x10);m.write_u8(3,0x80);
        m.write_u8(4,0x81);m.write_u8(5,0xF3);m.write_u8(6,0x10);m.write_u8(7,0x88);
        m.write_u8(8,0xEF);m.write_u8(9,0xF3);m.write_u8(10,0x05);m.write_u8(11,0x82);
        let mut c=ThumbCpu::new();c.r[15]=0;c.primask=true;c.r[1]=0;c.ipsr=15;
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[0],1);
        assert!(c.step(&mut m).is_ok());assert!(!c.primask);
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[2],15);
    }
    #[test]
    fn executes_thumb2_multiply_and_unsigned_divide() {
        let mut m=MemoryStorage::new();
        // MUL r2, r0, r3; UDIV r2, r0, r3
        m.write_u8(0,0x00);m.write_u8(1,0xFB);m.write_u8(2,0x03);m.write_u8(3,0xF2);
        m.write_u8(4,0xB0);m.write_u8(5,0xFB);m.write_u8(6,0xF3);m.write_u8(7,0xF2);
        let mut c=ThumbCpu::new();c.r[15]=0;c.r[0]=84;c.r[3]=7;
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[2],588);
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[2],12);
    }
    #[test]
    fn executes_thumb2_bfc_sbfx_and_lsl_register() {
        let mut m=MemoryStorage::new();
        // BFC r0, #0, #3; SBFX r2, r1, #8, #8; LSL.W r3, r1, r0
        m.write_u8(0,0x6F);m.write_u8(1,0xF3);m.write_u8(2,0x02);m.write_u8(3,0x00);
        m.write_u8(4,0x41);m.write_u8(5,0xF3);m.write_u8(6,0x07);m.write_u8(7,0x22);
        m.write_u8(8,0x01);m.write_u8(9,0xFA);m.write_u8(10,0x00);m.write_u8(11,0xF3);
        let mut c=ThumbCpu::new();c.r[15]=0;c.r[0]=0xFFFF_FFFF;c.r[1]=0x0000_8000;
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[0],0xFFFF_FFF8);
        assert!(c.step(&mut m).is_ok());assert_eq!(c.r[2],0xFFFF_FF80);
        c.r[0]=5;assert!(c.step(&mut m).is_ok());assert_eq!(c.r[3],0x0010_0000);
    }
    #[test]
    fn executes_thumb2_b_w_and_barrier_hints() {
        let mut m=MemoryStorage::new();
        // B.W +0 then DMB (barrier hint).
        m.write_u8(0,0x00); m.write_u8(1,0xF0); m.write_u8(2,0x00); m.write_u8(3,0xB8);
        m.write_u8(4,0xBF); m.write_u8(5,0xF3); m.write_u8(6,0x5F); m.write_u8(7,0x8F);
        let mut c=ThumbCpu::new(); c.r[15]=0;
        assert!(c.step(&mut m).is_ok()); assert_eq!(c.r[15],4);
        assert!(c.step(&mut m).is_ok()); assert_eq!(c.r[15],8);
    }
    #[test]
    fn executes_thumb2_signed_byte_and_halfword_loads() {
        let mut m=MemoryStorage::new();
        // LDRSB r0, [r1, #0]; LDRSH r2, [r1, #2]
        m.write_u8(0,0x91); m.write_u8(1,0xF9); m.write_u8(2,0x00); m.write_u8(3,0x00);
        m.write_u8(4,0xB1); m.write_u8(5,0xF9); m.write_u8(6,0x02); m.write_u8(7,0x20);
        m.write_u8(0x100,0x80); m.write_u8(0x102,0x00); m.write_u8(0x103,0x80);
        let mut c=ThumbCpu::new(); c.r[15]=0; c.r[1]=0x100;
        assert!(c.step(&mut m).is_ok()); assert_eq!(c.r[0],0xFFFF_FF80);
        assert!(c.step(&mut m).is_ok()); assert_eq!(c.r[2],0xFFFF_8000);
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
        m.write_u8(0,0xFF); m.write_u8(1,0xFF); m.write_u8(2,0xFF); m.write_u8(3,0xFF);
        let mut c=ThumbCpu::new(); c.r[15]=0;
        assert!(matches!(c.step(&mut m),Err(CpuFault::UnsupportedInstruction(_))));
        assert!(c.halted);
    }
}
