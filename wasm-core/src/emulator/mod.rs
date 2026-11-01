//! Browser-portable Capybara emulation core.
//! Desktop UI, filesystem persistence, audio output, and native serial transport
//! are intentionally excluded. The browser adapter owns input, scheduling,
//! firmware bytes, and the rendered canvas.
pub mod aes;
pub mod cpu;
pub mod mmu;
pub mod peripherals;
pub mod sonix;

pub use cpu::{Cpu, StepResult};
pub use mmu::MemoryBus;
pub use peripherals::Peripherals;
pub use sonix::{SonixImage, SonixError};
