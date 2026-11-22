// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod board;
// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod effect;
// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod stream;

pub mod store;

pub use board::soundboard;
pub use effect::effect;
pub use stream::stream;
