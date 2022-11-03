#![warn(missing_docs)]

//! A crate for discovering and inspecting the memory layout of a binary.

/// For parsing a binary to get symbols, BSS address, etc.
pub mod binary_parser;
/// For inspecting a process's symbols and handling Python-and Ruby-specific details.
pub mod process;

pub use process::process_info::ProcessInfo;
