pub trait ProcessType {
    #[cfg(target_os = "windows")]
    fn windows_symbols() -> Vec<String>;
    fn library_regex() -> regex::Regex;
    #[cfg(target_os = "macos")]
    fn is_framework(path: &std::path::Path) -> bool;
}
