//! Browser-safe parser for Sonix SNC73xx flash load tables.
//! Adapted from infinition/capybara (GPL-3.0-only); no firmware or device key is bundled.
//! Includes the Sonix AES decryption path when the user supplies a device key; CPU boot is separate.
use crate::aes::Aes;

pub const TABLE_STRIDE: usize = 0x200;
const MAGICS: [&[u8]; 6] = [b"SNC7320", b"SN323200", b"SNUR00", b"SN98300", b"SONIXDEV", b"SNCSPINF"];
const VERSION_MASK: u32 = 0x5a5a_0000;
const VERSION_V3: u32 = 0x5a5a_0033;
const ENCRYPTER_PENDING: u32 = 0x5f5f_4e45;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CipherMode { Ofb, Cbc }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region { pub address: u32, pub length: u32 }
impl Region { pub fn is_empty(&self) -> bool { self.length == 0 } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadTable {
    pub offset: usize, pub magic: String, pub version: u32, pub load_config: u32,
    pub encrypted: bool, pub mode: CipherMode, pub user_code: Region,
    pub sram_code: Region, pub dpd_code: Region,
}
fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let s = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes([s[0],s[1],s[2],s[3]]))
}
impl LoadTable {
    pub fn parse(bytes: &[u8], offset: usize) -> Option<Self> {
        let table = bytes.get(offset..offset.checked_add(TABLE_STRIDE)?)?;
        let magic = MAGICS.iter().find(|m| table.starts_with(m))?;
        let version = read_u32(table, 0x1f8)?;
        if version & 0xffff_0000 != VERSION_MASK { return None; }
        let load_config = read_u32(table, 8)?;
        let encrypted = (load_config & 1 != 0)
            && !(version >= VERSION_V3 && read_u32(table, 0x80)? == ENCRYPTER_PENDING);
        let region = |at| Some(Region { address: read_u32(table, at)?, length: read_u32(table, at+4)? });
        Some(Self {
            offset, magic: String::from_utf8_lossy(magic).into_owned(), version, load_config,
            encrypted, mode: if version < VERSION_V3 { CipherMode::Ofb } else { CipherMode::Cbc },
            user_code: region(0x10)?,
            sram_code: if version >= VERSION_V3 { region(0x90)? } else { Region{address:0,length:0} },
            dpd_code: if version >= VERSION_V3 { region(0x98)? } else { Region{address:0,length:0} },
        })
    }
    pub fn regions(&self) -> impl Iterator<Item=Region> {
        [self.user_code,self.sram_code,self.dpd_code].into_iter().filter(|r| !r.is_empty())
    }
}

pub fn find_tables(bytes: &[u8]) -> Vec<LoadTable> {
    let mut offsets = vec![0usize, 0x1000];
    let mut probe = 0x1000usize;
    while probe <= bytes.len()/2 {
        offsets.push(bytes.len()-probe);
        probe = match probe.checked_mul(2) { Some(n) => n, None => break };
    }
    let mut out = Vec::new();
    for offset in offsets {
        if let Some(t) = LoadTable::parse(bytes, offset) {
            if !out.iter().any(|existing: &LoadTable| existing.offset == t.offset) { out.push(t); }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Vec<u8> {
        let mut b=vec![0u8; TABLE_STRIDE]; b[..8].copy_from_slice(b"SONIXDEV");
        b[8..12].copy_from_slice(&0x13u32.to_le_bytes());
        b[0x10..0x14].copy_from_slice(&0x60001000u32.to_le_bytes());
        b[0x14..0x18].copy_from_slice(&0x10000u32.to_le_bytes());
        b[0x1f8..0x1fc].copy_from_slice(&VERSION_V3.to_le_bytes()); b
    }
    #[test] fn parses_v3_encrypted_sonix_table() {
        let t=LoadTable::parse(&sample(),0).unwrap();
        assert_eq!(t.magic,"SONIXDEV"); assert_eq!(t.version,VERSION_V3);
        assert!(t.encrypted); assert_eq!(t.mode,CipherMode::Cbc);
        assert_eq!(t.user_code,Region{address:0x60001000,length:0x10000});
    }
    #[test] fn rejects_unknown_and_truncated_tables() {
        assert!(LoadTable::parse(b"not sonix",0).is_none());
        assert!(LoadTable::parse(&sample(),1).is_none());
    }
    #[test] fn v3_encrypter_marker_means_already_decrypted() {
        let mut b=sample(); b[0x80..0x84].copy_from_slice(&ENCRYPTER_PENDING.to_le_bytes());
        assert!(!LoadTable::parse(&b,0).unwrap().encrypted);
    }
}

/// Decrypts encrypted load-table regions in a private copy of a flash dump.
/// Caller must supply the device key; this never modifies the user's original file.
pub fn decrypt_dump_with_key(bytes: &[u8], device_key: u32) -> Result<Vec<u8>, &'static str> {
    let mut flash=bytes.to_vec();
    let tables=find_tables(&flash);
    if tables.is_empty() { return Err("No valid Sonix load table found"); }
    let plan: Vec<_>=tables.iter().filter(|t| t.encrypted).cloned().collect();
    if plan.is_empty() { return Ok(flash); }
    for table in plan {
        for region in table.regions() {
            decrypt_region(&mut flash,&table,region,device_key);
        }
    }
    Ok(flash)
}
fn decrypt_region(flash:&mut [u8], table:&LoadTable, region:Region, device_key:u32) {
    let off=(region.address & 0x00FF_FFFF) as usize;
    let len=(region.length as usize) & !0xf;
    if len==0 || off.checked_add(len).is_none_or(|end|end>flash.len()) { return; }
    let mut key=[0u8;32]; // key material comes from the load table; retain byte order semantics
    // Parse the key directly from the load table at the exact table offset.
    key.copy_from_slice(&flash[table.offset+0x28..table.offset+0x48]);
    key.reverse();
    let mut iv=[0u8;16]; iv.copy_from_slice(&key[0x10..0x20]);
    let dk=device_key.to_be_bytes();
    for (i,b) in iv.iter_mut().enumerate(){*b^=dk[i%4];}
    let key128:[u8;16]=key[0..16].try_into().unwrap();
    let iv=Aes::new(&key128).encrypt_block(&iv);
    let aes=Aes::new(&key);
    match table.mode {
        CipherMode::Cbc => {
            const SLICE:usize=0x1000;
            for start in (0..len).step_by(SLICE) {
                let mut prev=iv; let end=(start+SLICE).min(len);
                for b in (start..end).step_by(16) {
                    let mut ct:[u8;16]=flash[off+b..off+b+16].try_into().unwrap();
                    ct.reverse(); let mut pt=aes.decrypt_block(&ct);
                    for i in 0..16 {pt[i]^=prev[i];}
                    prev=ct; pt.reverse(); flash[off+b..off+b+16].copy_from_slice(&pt);
                }
            }
        }
        CipherMode::Ofb => {
            let mut stream=iv;
            for b in (0..len).step_by(16) {
                stream=aes.encrypt_block(&stream);
                for w in 0..4 {
                    let s=u32::from_le_bytes(stream[(3-w)*4..(3-w)*4+4].try_into().unwrap());
                    let p=off+b+w*4;
                    let v=u32::from_le_bytes(flash[p..p+4].try_into().unwrap());
                    flash[p..p+4].copy_from_slice(&(v^s.swap_bytes()).to_le_bytes());
                }
            }
        }
    }
}
