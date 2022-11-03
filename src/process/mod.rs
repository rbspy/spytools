/// Holds information about the process: memory map layout, parsed info
/// for the binary and/or library, etc.
pub mod process_info;
/// An abstraction over the different language runtimes (Python, Ruby, etc) that we support
pub mod process_type;
/// A trait implementation for Python processes
pub mod python_process_type;
/// A trait implementation for Ruby processes
pub mod ruby_process_type;

pub use process_type::ProcessType;
pub use python_process_type::PythonProcessType;
pub use ruby_process_type::RubyProcessType;
