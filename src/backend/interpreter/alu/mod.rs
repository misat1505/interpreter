pub mod arithmetic;
pub mod cast;
pub mod comparison;
pub mod logical;
pub mod unary;
pub mod value;

#[cfg(test)]
mod tests;

#[allow(clippy::upper_case_acronyms)]
pub(in crate::backend::interpreter) struct ALU;
