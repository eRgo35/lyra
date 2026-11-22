// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod choose;
pub mod coinflip;
// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod eightball;
// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod mock;
pub mod rps;
// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod urban;

pub use choose::choose;
pub use coinflip::coinflip;
pub use eightball::eightball;
pub use mock::mock;
pub use rps::rps;
pub use urban::urban;
