/// helpers for loading files
/// release and wasm builds include them in the binary
/// while other builds just load them from disk
/// it may be possible to turn this into a fancy hotreloading wrapper

// debug build for desktop
#[cfg(all(not(target_arch = "wasm32"), debug_assertions))]
macro_rules! kf_include_bytes {
    ($e:expr) => {{
        let path = format!("{}{}", env!("CARGO_MANIFEST_DIR"), $e);
        &std::fs::read(path).unwrap()
    }};
}


// release build or wasm
#[cfg(any(target_arch = "wasm32", not(debug_assertions)))]
macro_rules! kf_include_bytes {
    ($e:expr) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), $e))
    };
}


// debug build for desktop
#[cfg(all(not(target_arch = "wasm32"), debug_assertions))]
macro_rules! kf_include_str {
    ($e:expr) => {{
        let path = format!("{}{}", env!("CARGO_MANIFEST_DIR"), $e);
        &std::fs::read_to_string(path).unwrap()
    }};
}


// release build or wasm
#[cfg(any(target_arch = "wasm32", not(debug_assertions)))]
macro_rules! kf_include_str {
    ($e:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $e))
    };
}


pub(crate) use kf_include_bytes;
pub(crate) use kf_include_str;
