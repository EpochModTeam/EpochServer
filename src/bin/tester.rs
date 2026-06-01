//! Standalone tester for the epochserver extension.
//!
//! ## Usage
//!
//! After `cargo build --release`:
//!   cargo run --bin tester --release -- "000" "510" "" "500"
//!
//! Multiple calls are supported in one invocation.
//!
//! ## Redis
//!
//! Most commands require a running Redis. Set the URL via environment variable:
//!   set EPOCH_REDIS_URL=redis://127.0.0.1:6379/0
//!   (or use the provided docker-compose.redis.yml)
//!
//! It loads the built cdylib and calls the classic RVExtension entry point
//! exactly the way Arma 3 does.

use std::env;
use std::ffi::{c_char, c_int, CString};
use std::path::{Path, PathBuf};

type RVExtensionFn =
    unsafe extern "system" fn(output: *mut c_char, output_size: c_int, function: *const c_char);

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        eprintln!("Usage: tester <call1> [call2] [call3] ...");
        eprintln!("Example: tester \"\" 000 510 500 810|1");
        std::process::exit(1);
    }

    let calls: Vec<String> = args[1..].to_vec();

    let dll_path = find_dll().expect("Could not locate epochserver_x64.dll / libepochserver_x64.so (we only ship the x64 variants)");

    println!("Loading extension from: {}", dll_path.display());

    unsafe {
        let lib =
            libloading::Library::new(&dll_path).expect("Failed to load the extension library");

        let rv_extension: libloading::Symbol<RVExtensionFn> = if cfg!(windows) {
            lib.get(b"_RVExtension@12")
                .or_else(|_| lib.get(b"RVExtension"))
                .expect("Could not find RVExtension symbol")
        } else {
            lib.get(b"RVExtension")
                .expect("Could not find RVExtension symbol")
        };

        for call in &calls {
            println!(
                "\n=== Calling: {:?}",
                if call.is_empty() { "<empty>" } else { call }
            );

            let mut output = vec![0u8; 8192]; // larger buffer for safety
            let input = CString::new(call.as_bytes()).unwrap();

            rv_extension(
                output.as_mut_ptr() as *mut c_char,
                output.len() as c_int,
                input.as_ptr(),
            );

            let len = output.iter().position(|&b| b == 0).unwrap_or(output.len());
            let result = String::from_utf8_lossy(&output[..len]);

            println!("Result: {}", result);
            println!("Length: {} bytes", result.len());
        }
    }
}

fn find_dll() -> Option<PathBuf> {
    // 1. Explicit env var override (most reliable during dev)
    if let Ok(p) = env::var("EPOCHSERVER_DLL") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }

    let candidates = [
        // We only support/ship the x64 variants (historical Arma convention).
        // Non-x64 names (epochserver.dll etc.) are ignored.
        "target/release/epochserver_x64.dll",
        "target/release/epochserver_x64.so",
        "target/release/libepochserver_x64.so",
        "epochserver_x64.dll",
        "epochserver_x64.so",
        "../target/release/epochserver_x64.dll",
        "../target/release/epochserver_x64.so",
    ];

    for c in &candidates {
        let p = Path::new(c);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }

    // Last resort: search recursively under target (slow but works)
    if let Ok(walk) = globwalk_for_dll() {
        if let Some(first) = walk.into_iter().next() {
            return Some(first);
        }
    }

    None
}

fn globwalk_for_dll() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    use std::fs;
    let mut results = Vec::new();

    fn visit(dir: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    visit(&p, out);
                } else if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    // Strictly only x64 variants (we do not support or ship non-x64 names)
                    if name == "epochserver_x64.dll"
                        || name == "epochserver_x64.so"
                        || name == "libepochserver_x64.so"
                    {
                        out.push(p);
                    }
                }
            }
        }
    }

    if let Ok(root) = env::current_dir() {
        let mut search_roots = vec![root.clone()];
        if let Some(parent) = root.parent() {
            search_roots.push(parent.to_path_buf());
        }
        for r in search_roots {
            visit(&r, &mut results);
            if !results.is_empty() {
                break;
            }
        }
    }

    Ok(results)
}
