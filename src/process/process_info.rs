use anyhow::{format_err, Context, Error};
use log::*;
use proc_maps::{get_process_maps, MapRange};
#[cfg(target_os = "windows")]
use std::collections::HashMap;
use std::path::PathBuf;

use crate::binary_parser::{parse_binary, BinaryInfo};
use crate::process::ProcessType;

/// Holds information about the process: memory map layout, parsed info
/// for the binary and/or library, etc.
pub struct ProcessInfo {
    /// Metadata about the binary, if any
    pub binary: Option<BinaryInfo>,
    /// Metadata about the library, if any
    pub library: Option<BinaryInfo>,
    /// The binary or library's mapped memory ranges
    pub maps: Vec<MapRange>,
    /// The file path to the binary or library
    pub path: PathBuf,
    /// Whether the process is running in a Docker container
    #[cfg(target_os = "linux")]
    pub dockerized: bool,
}

impl ProcessInfo {
    /// Constructs a new `ProcessInfo` that can be used to get symbol information.
    pub fn new<T>(process: &remoteprocess::Process) -> Result<Self, Error>
    where
        T: crate::process::ProcessType,
    {
        let filename = process
            .exe()
            .context("Failed to get process executable name. Check that the process is running.")?;

        #[cfg(windows)]
        let filename = filename.to_lowercase();

        #[cfg(windows)]
        let is_bin = |pathname: &str| pathname.to_lowercase() == filename;

        #[cfg(not(windows))]
        let is_bin = |pathname: &str| pathname == filename;

        let maps = get_process_maps(process.pid)?;
        info!("Got virtual memory maps from pid {}:", process.pid);
        for map in &maps {
            debug!(
                "map: {:016x}-{:016x} {}{}{} {}",
                map.start(),
                map.start() + map.size(),
                if map.is_read() { 'r' } else { '-' },
                if map.is_write() { 'w' } else { '-' },
                if map.is_exec() { 'x' } else { '-' },
                map.filename()
                    .unwrap_or(&std::path::PathBuf::from(""))
                    .display()
            );
        }

        let (binary, filename) = {
            let map = maps.iter().find(|m| {
                if let Some(pathname) = m.filename() {
                    if let Some(pathname) = pathname.to_str() {
                        return is_bin(pathname) && m.is_exec();
                    }
                }
                false
            });

            let map = match map {
                Some(map) => map,
                None => {
                    // https://github.com/benfred/py-spy/issues/40
                    warn!("Failed to find '{}' in virtual memory maps, falling back to first map region", filename);
                    if maps.is_empty() {
                        return Err(format_err!("No memory map regions found for process"));
                    }
                    &maps[0]
                }
            };

            let filename = PathBuf::from(filename);

            // TODO: consistent types? u64 -> usize? for map.start etc
            #[allow(unused_mut)]
            let binary = parse_binary(
                process.pid,
                &filename,
                map.start() as u64,
                map.size() as u64,
                true,
            )
            .and_then(|mut pb| {
                // windows symbols are stored in separate files (.pdb), load
                #[cfg(windows)]
                {
                    get_windows_symbols::<T>(process.pid, &filename, map.start() as u64)
                        .map(|symbols| {
                            pb.symbols.extend(symbols);
                            pb
                        })
                        .map_err(|err| err.into())
                }

                // For macOS, need to adjust main binary symbols by subtracting _mh_execute_header
                // (which was added to map.start already, so undo that here)
                #[cfg(target_os = "macos")]
                {
                    let offset = pb.symbols["_mh_execute_header"] - map.start() as u64;
                    for address in pb.symbols.values_mut() {
                        *address -= offset;
                    }

                    if pb.bss_addr != 0 {
                        pb.bss_addr -= offset;
                    }
                }

                #[cfg(not(windows))]
                Ok(pb)
            });

            (binary, filename.clone())
        };

        // likewise handle library for versions compiled with --enabled-shared
        let library = {
            let libmap = maps.iter().find(|m| {
                if let Some(path) = m.filename() {
                    #[cfg(target_os = "windows")]
                    return is_lib::<T>(path) && m.is_read();
                    #[cfg(not(target_os = "windows"))]
                    return is_lib::<T>(path) && m.is_exec();
                }
                false
            });

            let mut library: Option<BinaryInfo> = None;
            if let Some(libmap) = libmap {
                if let Some(filename) = &libmap.filename() {
                    info!("Found library @ {}", filename.display());
                    #[allow(unused_mut)]
                    let mut parsed = parse_binary(
                        process.pid,
                        filename,
                        libmap.start() as u64,
                        libmap.size() as u64,
                        false,
                    )?;
                    #[cfg(windows)]
                    parsed.symbols.extend(get_windows_symbols::<T>(
                        process.pid,
                        filename,
                        libmap.start() as u64,
                    )?);
                    library = Some(parsed);
                }
            }

            // On macOS, it's possible that the library is a dylib loaded up from the system
            // framework (like /System/Library/Frameworks/<Python|Ruby>.framework).
            // In this case read in the dyld_info information and figure out the filename from there
            #[cfg(target_os = "macos")]
            {
                if library.is_none() {
                    use proc_maps::mac_maps::get_dyld_info;
                    let dyld_infos = get_dyld_info(process.pid)?;

                    for dyld in &dyld_infos {
                        let segname =
                            unsafe { std::ffi::CStr::from_ptr(dyld.segment.segname.as_ptr()) };
                        debug!(
                            "dyld: {:016x}-{:016x} {:10} {}",
                            dyld.segment.vmaddr,
                            dyld.segment.vmaddr + dyld.segment.vmsize,
                            segname.to_string_lossy(),
                            dyld.filename.display()
                        );
                    }

                    let dyld_data = dyld_infos.iter().find(|m| {
                        return T::is_framework(&m.filename)
                            && m.segment.segname[0..7] == [95, 95, 68, 65, 84, 65, 0];
                    });

                    if let Some(dyld_data) = dyld_data {
                        info!("Found library from dyld @ {}", dyld_data.filename.display());

                        let mut binary = parse_binary(
                            process.pid,
                            &dyld_data.filename,
                            dyld_data.segment.vmaddr,
                            dyld_data.segment.vmsize,
                            false,
                        )?;

                        // TODO: bss addr offsets returned from parsing binary are wrong
                        // (assumes data section isn't split from text section like done here).
                        // BSS occurs somewhere in the data section, just scan that
                        // (could later tighten this up to look at segment sections too)
                        binary.bss_addr = dyld_data.segment.vmaddr;
                        binary.bss_size = dyld_data.segment.vmsize;
                        library = Some(binary);
                    }
                }
            }

            library
        };

        // If we have a library - we can tolerate failures on parsing the main binary.
        let binary = match library {
            None => Some(binary.context("Failed to parse ruby binary")?),
            _ => binary.ok(),
        };

        #[cfg(target_os = "linux")]
        let dockerized = is_dockerized(process.pid).unwrap_or(false);

        Ok(Self {
            binary,
            library,
            maps,
            path: filename,
            #[cfg(target_os = "linux")]
            dockerized,
        })
    }

    /// Gets the memory address of the named symbol, if it exists.
    pub fn get_symbol(&self, symbol: &str) -> Option<&u64> {
        if let Some(ref pb) = self.binary {
            if let Some(addr) = pb.symbols.get(symbol) {
                info!("got symbol {} (0x{:016x}) from binary", symbol, addr);
                return Some(addr);
            }
        }

        if let Some(ref binary) = self.library {
            if let Some(addr) = binary.symbols.get(symbol) {
                info!("got symbol {} (0x{:016x}) from library", symbol, addr);
                return Some(addr);
            }
        }
        None
    }
}

#[cfg(target_os = "linux")]
fn is_dockerized(pid: remoteprocess::Pid) -> Result<bool, Error> {
    let self_mnt = std::fs::read_link("/proc/self/ns/mnt")?;
    let target_mnt = std::fs::read_link(&format!("/proc/{}/ns/mnt", pid))?;
    Ok(self_mnt != target_mnt)
}

#[cfg(target_os = "windows")]
/// Gets all symbols for the binary represented by the PID and file path.
pub fn get_windows_symbols<T>(
    pid: remoteprocess::Pid,
    filename: &std::path::Path,
    offset: u64,
) -> std::io::Result<HashMap<String, u64>>
where
    T: ProcessType,
{
    use proc_maps::win_maps::SymbolLoader;

    let handler = SymbolLoader::new(pid)?;
    let module = handler.load_module(filename)?; // need to keep this module in scope

    let mut ret = HashMap::new();

    // currently we only need a subset of symbols, and enumerating the symbols is
    // expensive (via SymEnumSymbolsW), so rather than load up all symbols like we
    // do for goblin, just load the the couple we need directly.
    for symbol in T::windows_symbols().iter() {
        if let Ok((base, addr)) = handler.address_from_name(symbol) {
            // If we have a module base (ie from PDB), need to adjust by the offset
            // otherwise seems like we can take address directly
            let addr = if base == 0 {
                offset + addr - module.base
            } else {
                offset + addr - base
            };
            ret.insert(String::from(symbol), addr);
        }
    }

    Ok(ret)
}

/// Returns `true` if the file at `path` looks like a library, and false otherwise.
pub fn is_lib<T>(path: &std::path::Path) -> bool
where
    T: ProcessType,
{
    T::library_regex().is_match(&path.to_string_lossy())
}
