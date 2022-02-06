use regex::Regex;

use crate::process::ProcessType;

pub struct PythonProcessType {}

impl ProcessType for PythonProcessType {
    #[cfg(windows)]
    fn windows_symbols() -> Vec<String> {
        vec![
            "_PyThreadState_Current".to_string(),
            "interp_head".to_string(),
            "_PyRuntime".to_string(),
        ]
    }

    fn library_regex() -> Regex {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        return Regex::new(r"/libpython\d.\d\d?(m|d|u)?.so").unwrap();

        #[cfg(target_os = "macos")]
        return Regex::new(r"/libpython\d.\d\d?(m|d|u)?.(dylib|so)$").unwrap();

        #[cfg(windows)]
        return regex::RegexBuilder::new(r"\\python\d\d\d?(m|d|u)?.dll$")
            .case_insensitive(true)
            .build()
            .unwrap();
    }

    #[cfg(target_os = "macos")]
    fn is_framework(path: &std::path::Path) -> bool {
        path.ends_with("Python") && !path.to_string_lossy().contains("Python.app")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::process::process_info::is_lib;

    #[cfg(target_os = "macos")]
    #[test]
    fn test_python_is_lib() {
        assert!(is_lib::<PythonProcessType>(&PathBuf::from(
            "~/Anaconda2/lib/libpython2.7.dylib"
        )));

        // python lib configured with --with-pydebug (flag: d)
        assert!(is_lib::<PythonProcessType>(&PathBuf::from(
            "/lib/libpython3.4d.dylib"
        )));

        // configured --with-pymalloc (flag: m)
        assert!(is_lib::<PythonProcessType>(&PathBuf::from(
            "/usr/local/lib/libpython3.8m.dylib"
        )));

        // python2 configured with --with-wide-unicode (flag: u)
        assert!(is_lib::<PythonProcessType>(&PathBuf::from(
            "./libpython2.7u.dylib"
        )));

        assert!(!is_lib::<PythonProcessType>(&PathBuf::from(
            "/libboost_python.dylib"
        )));
        assert!(!is_lib::<PythonProcessType>(&PathBuf::from(
            "/lib/heapq.cpython-36m-darwin.dylib"
        )));
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    #[test]
    fn test_python_is_lib() {
        // libpython bundled by pyinstaller https://github.com/benfred/py-spy/issues/42
        assert!(is_lib::<PythonProcessType>(&PathBuf::from(
            "/tmp/_MEIOqzg01/libpython2.7.so.1.0"
        )));

        // test debug/malloc/unicode flags
        assert!(is_lib::<PythonProcessType>(&PathBuf::from(
            "./libpython2.7.so"
        )));
        assert!(is_lib::<PythonProcessType>(&PathBuf::from(
            "/usr/lib/libpython3.4d.so"
        )));
        assert!(is_lib::<PythonProcessType>(&PathBuf::from(
            "/usr/local/lib/libpython3.8m.so"
        )));
        assert!(is_lib::<PythonProcessType>(&PathBuf::from(
            "/usr/lib/libpython2.7u.so"
        )));

        // don't blindly match libraries with python in the name (boost_python etc)
        assert!(!is_lib::<PythonProcessType>(&PathBuf::from(
            "/usr/lib/libboost_python.so"
        )));
        assert!(!is_lib::<PythonProcessType>(&PathBuf::from(
            "/usr/lib/x86_64-linux-gnu/libboost_python-py27.so.1.58.0"
        )));
        assert!(!is_lib::<PythonProcessType>(&PathBuf::from(
            "/usr/lib/libboost_python-py35.so"
        )));
    }

    #[cfg(windows)]
    #[test]
    fn test_python_is_lib() {
        assert!(is_lib::<PythonProcessType>(&PathBuf::from(
            "C:\\Users\\test\\AppData\\Local\\Programs\\Python\\Python37\\python37.dll"
        )));
        // .NET host via https://github.com/pythonnet/pythonnet
        assert!(is_lib::<PythonProcessType>(&PathBuf::from(
            "C:\\Users\\test\\AppData\\Local\\Programs\\Python\\Python37\\python37.DLL"
        )));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_python_frameworks() {
        // homebrew v2
        assert!(!PythonProcessType::is_framework(&PathBuf::from("/usr/local/Cellar/python@2/2.7.15_1/Frameworks/Python.framework/Versions/2.7/Resources/Python.app/Contents/MacOS/Python")));
        assert!(PythonProcessType::is_framework(&PathBuf::from(
            "/usr/local/Cellar/python@2/2.7.15_1/Frameworks/Python.framework/Versions/2.7/Python"
        )));

        // System python from osx 10.13.6 (high sierra)
        assert!(!PythonProcessType::is_framework(&PathBuf::from("/System/Library/Frameworks/Python.framework/Versions/2.7/Resources/Python.app/Contents/MacOS/Python")));
        assert!(PythonProcessType::is_framework(&PathBuf::from(
            "/System/Library/Frameworks/Python.framework/Versions/2.7/Python"
        )));

        // pyenv 3.6.6 with OSX framework enabled (https://github.com/benfred/py-spy/issues/15)
        // env PYTHON_CONFIGURE_OPTS="--enable-framework" pyenv install 3.6.6
        assert!(PythonProcessType::is_framework(&PathBuf::from(
            "/Users/ben/.pyenv/versions/3.6.6/Python.framework/Versions/3.6/Python"
        )));
        assert!(!PythonProcessType::is_framework(&PathBuf::from("/Users/ben/.pyenv/versions/3.6.6/Python.framework/Versions/3.6/Resources/Python.app/Contents/MacOS/Python")));

        // single file pyinstaller
        assert!(PythonProcessType::is_framework(&PathBuf::from(
            "/private/var/folders/3x/qy479lpd1fb2q88lc9g4d3kr0000gn/T/_MEI2Akvi8/Python"
        )));
    }
}
