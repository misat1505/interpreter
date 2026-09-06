pub mod scope;
pub mod scope_manager;
#[allow(clippy::module_inception)]
pub mod stack;
pub mod stack_frame;

#[cfg(test)]
mod tests;
