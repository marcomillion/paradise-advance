//! Browser-portable Capybara emulation core.
//! Desktop UI, filesystem persistence, audio output, and native serial transport
//! are intentionally excluded. The browser adapter owns input, scheduling,
//! firmware bytes, and the rendered canvas.
pub mod aes;
pub mod cpu;
pub mod mmu;
pub mod peripherals;
pub mod sonix;

/// Flash page size shared with Capybara save-state bookkeeping.
pub mod etat { pub const PAGE_FLASH: usize = 0x1000; }

pub use cpu::{Cpu, StepResult};
pub use mmu::MemoryBus;
pub use peripherals::Peripherals;
pub use sonix::{SonixImage, SonixError};
