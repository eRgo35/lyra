// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod ai;
// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod dice;
// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod dictionary;
pub mod help;
pub mod ip;
pub mod lookup;
pub mod mcplayer;
pub mod mcstatus;
pub mod metar;
// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod owoify;
pub mod ping;
pub mod posix;
// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod qr;
pub mod register;
pub mod taf;
pub mod uptime;
// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod verse;
// poise's #[rest] codegen trips unreachable_code on Rust 1.100+
#[allow(unreachable_code)]
pub mod weather;
pub mod whois;

pub use ai::ai;
pub use dice::dice;
pub use dictionary::dictionary;
pub use help::help;
pub use ip::ip;
pub use mcplayer::mcplayer;
pub use mcstatus::mcstatus;
pub use metar::metar;
pub use owoify::owoify;
pub use ping::ping;
pub use posix::posix;
pub use qr::qr;
pub use register::register;
pub use taf::taf;
pub use uptime::uptime;
pub use verse::verse;
pub use weather::weather;
pub use whois::whois;
