//! Browser-safe parser for Sonix SNC73xx flash load tables.
//! Adapted from infinition/capybara (GPL-3.0-only); no firmware or device key is bundled.
//! This inspects metadata only. It does not decrypt or execute firmware.

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
