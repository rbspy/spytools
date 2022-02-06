use regex::Regex;

use crate::process::ProcessType;

pub struct RubyProcessType {}

impl ProcessType for RubyProcessType {
    #[cfg(windows)]
    fn windows_symbols() -> Vec<String> {
        vec![
            "global_symbols".to_string(),
            "ruby_global_symbols".to_string(),
            "ruby_current_vm".to_string(),
            "ruby_current_vm_ptr".to_string(),
            "ruby_current_thread".to_string(),
            "ruby_current_execution_context_ptr".to_string(),
            "ruby_version".to_string(),
        ]
    }

    fn library_regex() -> Regex {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        return Regex::new(r"/libruby\.so(\.\d+\.\d+(\.\d+)?)?").unwrap();

        #[cfg(target_os = "macos")]
        return Regex::new(r"/libruby\.?\d\.\d\d?\.(dylib|so)$").unwrap();

        #[cfg(windows)]
        return regex::RegexBuilder::new(r"\\.*ruby\d\d\d?\.dll(\.a)?$")
            .case_insensitive(true)
            .build()
            .unwrap();
    }

    #[cfg(target_os = "macos")]
    fn is_framework(path: &std::path::Path) -> bool {
        path.ends_with("Ruby") && !path.to_string_lossy().contains("Ruby.app")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::process::process_info::is_lib;

    #[cfg(target_os = "macos")]
    #[test]
    fn test_is_lib() {
        assert!(is_lib::<RubyProcessType>(&PathBuf::from(
            "/System/Library/Frameworks/Ruby.framework/Versions/2.6/usr/lib/libruby.2.6.dylib"
        )));

        assert!(is_lib::<RubyProcessType>(&PathBuf::from(
            "/lib/libruby.2.6.dylib"
        )));

        assert!(!is_lib::<RubyProcessType>(&PathBuf::from(
            "/libboost_ruby.dylib"
        )));
        assert!(!is_lib::<RubyProcessType>(&PathBuf::from(
            "/lib/heapq.cruby-36m-darwin.dylib"
        )));
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    #[test]
    fn test_is_lib() {
        assert!(is_lib::<RubyProcessType>(&PathBuf::from("./libruby2.7.so")));
        assert!(is_lib::<RubyProcessType>(&PathBuf::from(
            "/usr/lib/libruby3.1.so"
        )));
        assert!(is_lib::<RubyProcessType>(&PathBuf::from(
            "/usr/local/lib/libruby3.1.so"
        )));
        assert!(is_lib::<RubyProcessType>(&PathBuf::from(
            "/usr/lib/libruby2.6.so"
        )));

        // don't blindly match libraries with ruby in the name
        assert!(!is_lib::<RubyProcessType>(&PathBuf::from(
            "/usr/lib/libfoo_ruby.so"
        )));
        assert!(!is_lib::<RubyProcessType>(&PathBuf::from(
            "/usr/lib/x86_64-linux-gnu/libfoo_ruby-27.so.1.58.0"
        )));
        assert!(!is_lib::<RubyProcessType>(&PathBuf::from(
            "/usr/lib/libfoo_ruby-31.so"
        )));
    }

    #[cfg(windows)]
    #[test]
    fn test_is_lib() {
        assert!(is_lib::<RubyProcessType>(&PathBuf::from(
            "C:\\Users\\test\\AppData\\Local\\Programs\\ruby\\ruby31\\ruby31.dll"
        )));
        assert!(is_lib::<RubyProcessType>(&PathBuf::from(
            "C:\\Users\\test\\AppData\\Local\\Programs\\ruby\\ruby31\\ruby31.DLL"
        )));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_is_framework() {
        // homebrew v2
        assert!(RubyProcessType::is_framework(&PathBuf::from(
            "/usr/local/Cellar/ruby@2/2.7.5_1/Frameworks/ruby.framework/Versions/2.7/Ruby"
        )));
        assert!(!RubyProcessType::is_framework(&PathBuf::from("/usr/local/Cellar/ruby@2/2.7.5_1/Frameworks/ruby.framework/Versions/2.7/Resources/Ruby.app/Contents/MacOS/Ruby")));
    }
}
