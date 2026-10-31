//! Display controller boundary ported from Capybara's real hardware model.
//! CPU/DMA integration remains a later porting step.

pub const LCD_WIDTH: usize = 128;
pub const LCD_HEIGHT: usize = 128;

#[derive(Clone)]
pub struct DisplayController {
    ctrl: u32, fb_base_addr: u32, width: usize, height: usize,
    vram: Vec<u16>, enabled: bool, dirty: bool, frames_received: u64,
}
impl DisplayController {
    pub fn new() -> Self {
        Self { ctrl: 1, fb_base_addr: 0x1800_8000, width: LCD_WIDTH,
            height: LCD_HEIGHT, vram: vec![0x1084; LCD_WIDTH * LCD_HEIGHT],
            enabled: true, dirty: true, frames_received: 0 }
    }
    pub fn read_reg(&self, offset: u32) -> u32 {
        match offset { 0 => self.ctrl, 4 => self.fb_base_addr,
            8 => self.width as u32 | ((self.height as u32) << 16), _ => 0 }
    }
    pub fn write_reg(&mut self, offset: u32, value: u32) {
        match offset {
            0 => { self.ctrl = value; self.enabled = value & 1 != 0; self.dirty = true; }
            4 => self.fb_base_addr = value,
            8 => { self.width = ((value & 0xFFFF) as usize).min(LCD_WIDTH);
                self.height = ((value >> 16) as usize).min(LCD_HEIGHT); self.dirty = true; }
            _ => {}
        }
    }
    pub fn write_vram_pixel(&mut self, x: usize, y: usize, rgb565: u16) {
        if x >= self.width || y >= self.height { return; }
        if let Some(pixel) = self.vram.get_mut(y * self.width + x) {
            *pixel = rgb565; self.dirty = true;
        }
    }
    pub fn receive_frame(&mut self, pixels: &[u16]) {
        let count = pixels.len().min(self.vram.len());
        self.vram[..count].copy_from_slice(&pixels[..count]);
        self.dirty = true; self.frames_received += 1;
    }
    pub fn sync_from_sram(&mut self, sram: &[u8]) {
        let offset = self.fb_base_addr.saturating_sub(0x1800_0000) as usize;
        let count = self.width.saturating_mul(self.height);
        let byte_len = count.saturating_mul(2);
        if offset.checked_add(byte_len).is_some_and(|end| end <= sram.len()) {
            for i in 0..count {
                let p = offset + i * 2;
                self.vram[i] = u16::from_le_bytes([sram[p], sram[p + 1]]);
            }
            self.dirty = true;
        }
    }
    pub fn rgba_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.vram.len() * 4);
        for &pixel in &self.vram {
            let r5 = ((pixel >> 11) & 31) as u8;
            let g6 = ((pixel >> 5) & 63) as u8;
            let b5 = (pixel & 31) as u8;
            out.extend_from_slice(&[(r5 << 3) | (r5 >> 2),
                (g6 << 2) | (g6 >> 4), (b5 << 3) | (b5 >> 2), 255]);
        }
        out
    }
    pub fn width(&self) -> usize { self.width }
    pub fn height(&self) -> usize { self.height }
    pub fn enabled(&self) -> bool { self.enabled }
    pub fn dirty(&self) -> bool { self.dirty }
    pub fn clear_dirty(&mut self) { self.dirty = false; }
    pub fn frames_received(&self) -> u64 { self.frames_received }
    /// Number of non-black pixels in the current panel buffer.
    pub fn nonzero_pixels(&self) -> u32 { self.vram.iter().filter(|&&p| p != 0).count() as u32 }
    /// Cheap deterministic checksum for comparing panel output between runs.
    pub fn pixel_checksum(&self) -> u32 { self.vram.iter().fold(0u32, |sum, &p| sum.wrapping_add(p as u32)) }
}
impl Default for DisplayController { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn framebuffer_has_expected_rgba_size() {
        assert_eq!(DisplayController::new().rgba_bytes().len(), LCD_WIDTH * LCD_HEIGHT * 4);
    }
    #[test] fn rgb565_red_is_converted_to_rgba() {
        let mut d = DisplayController::new(); d.write_vram_pixel(0, 0, 0xF800);
        assert_eq!(&d.rgba_bytes()[0..4], &[255, 0, 0, 255]);
    }
    #[test] fn writes_outside_panel_are_ignored() {
        let mut d = DisplayController::new(); let before = d.rgba_bytes();
        d.write_vram_pixel(LCD_WIDTH, 0, 0xFFFF); assert_eq!(d.rgba_bytes(), before);
    }
    #[test] fn frame_transfer_updates_buffer_and_counter() {
        let mut d = DisplayController::new(); d.receive_frame(&[0x07E0]);
        assert_eq!(&d.rgba_bytes()[0..4], &[0, 255, 0, 255]);
        assert_eq!(d.frames_received(), 1);
    }
}
