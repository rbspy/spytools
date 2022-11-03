/// An abstraction over the different language runtimes (Python, Ruby, etc) that we support
pub trait ProcessType {
    #[cfg(target_os = "windows")]
    /// Returns all symbols available in the process
    fn windows_symbols() -> Vec<String>;
    /// A regular expression that matches library filenames for this process type
    fn library_regex() -> regex::Regex;
    /// Returns `true` if the given filename looks like a macOS framework, and `false` otherwise
    #[cfg(target_os = "macos")]
    fn is_framework(path: &std::path::Path) -> bool;
}
