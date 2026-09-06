pub mod alu;
#[allow(clippy::module_inception)]
pub mod interpreter;
mod stack;

pub use alu::value::Value;
