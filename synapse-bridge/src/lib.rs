//! # synapse-bridge
//!
//! Cross-platform FFI bridge exposing synapse-store and synapse-search to C/Go/Python.
//!
//! Compiled as a static library (.a / .lib) or shared library (.so / .dylib / .dll).
//! All functions use C-compatible types: raw pointers, lengths, and null-terminated strings.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::Mutex;
use std::sync::OnceLock;

use synapse_store::SynapseStore;
use synapse_search::SynapseSearch;

// ──────────────────────────────────────────────
//  Global singleton instances (thread-safe)
// ──────────────────────────────────────────────

static STORE: OnceLock<Mutex<Option<SynapseStore>>> = OnceLock::new();
static SEARCH: OnceLock<Mutex<SynapseSearch>> = OnceLock::new();

fn get_store() -> &'static Mutex<Option<SynapseStore>> {
    STORE.get_or_init(|| Mutex::new(None))
}

fn get_search() -> &'static Mutex<SynapseSearch> {
    SEARCH.get_or_init(|| Mutex::new(SynapseSearch::new()))
}

// ──────────────────────────────────────────────
//  Helper: Convert C string to Rust &str
// ──────────────────────────────────────────────

unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

// ──────────────────────────────────────────────
//  Memory management
// ──────────────────────────────────────────────

/// Free a string returned by any synapse_* function.
/// Must be called by the host language to prevent memory leaks.
#[no_mangle]
pub extern "C" fn synapse_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)); }
    }
}

// ──────────────────────────────────────────────
//  STORE API
// ──────────────────────────────────────────────

/// Open or create a database at the given directory path.
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn synapse_store_open(dir: *const c_char) -> i32 {
    let dir_str = match unsafe { cstr_to_str(dir) } {
        Some(s) => s,
        None => return -1,
    };

    match SynapseStore::open(dir_str) {
        Ok(store) => {
            let mutex = get_store();
            if let Ok(mut lock) = mutex.lock() {
                *lock = Some(store);
                0
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

/// Put a key-value pair into the store.
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn synapse_store_put(
    key_ptr: *const u8, key_len: u32,
    val_ptr: *const u8, val_len: u32,
) -> i32 {
    if key_ptr.is_null() || val_ptr.is_null() {
        return -1;
    }

    let key = unsafe { std::slice::from_raw_parts(key_ptr, key_len as usize) };
    let val = unsafe { std::slice::from_raw_parts(val_ptr, val_len as usize) };

    let mutex = get_store();
    if let Ok(lock) = mutex.lock() {
        if let Some(ref store) = *lock {
            return match store.put(key, val) {
                Ok(()) => 0,
                Err(_) => -1,
            };
        }
    }
    -1
}

/// Get a value by key. Returns a pointer to a C string (JSON: `{"found":true,"value":"..."}`)
/// Caller must free the returned pointer with `synapse_free_string`.
#[no_mangle]
pub extern "C" fn synapse_store_get(key_ptr: *const u8, key_len: u32) -> *mut c_char {
    if key_ptr.is_null() {
        return ptr::null_mut();
    }

    let key = unsafe { std::slice::from_raw_parts(key_ptr, key_len as usize) };

    let mutex = get_store();
    if let Ok(lock) = mutex.lock() {
        if let Some(ref store) = *lock {
            match store.get(key) {
                Ok(Some(val)) => {
                    // Return value as base64 to handle binary data safely
                    let encoded = base64_encode(&val);
                    let json = format!(r#"{{"found":true,"value":"{}"}}"#, encoded);
                    return to_c_string(&json);
                }
                Ok(None) => {
                    return to_c_string(r#"{"found":false,"value":""}"#);
                }
                Err(_) => return ptr::null_mut(),
            }
        }
    }
    ptr::null_mut()
}

/// Delete a key from the store. Returns 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn synapse_store_delete(key_ptr: *const u8, key_len: u32) -> i32 {
    if key_ptr.is_null() {
        return -1;
    }

    let key = unsafe { std::slice::from_raw_parts(key_ptr, key_len as usize) };

    let mutex = get_store();
    if let Ok(lock) = mutex.lock() {
        if let Some(ref store) = *lock {
            return match store.delete(key) {
                Ok(()) => 0,
                Err(_) => -1,
            };
        }
    }
    -1
}

/// Flush the store's MemTable to disk. Returns 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn synapse_store_flush() -> i32 {
    let mutex = get_store();
    if let Ok(lock) = mutex.lock() {
        if let Some(ref store) = *lock {
            return match store.flush() {
                Ok(()) => 0,
                Err(_) => -1,
            };
        }
    }
    -1
}

// ──────────────────────────────────────────────
//  SEARCH API
// ──────────────────────────────────────────────

/// Add a document to the search engine.
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn synapse_search_add(id: u64, text: *const c_char) -> i32 {
    let text_str = match unsafe { cstr_to_str(text) } {
        Some(s) => s,
        None => return -1,
    };

    if let Ok(mut engine) = get_search().lock() {
        engine.add(id, text_str);
        0
    } else {
        -1
    }
}

/// Get autocomplete suggestions for a prefix.
/// Returns a JSON array string. Caller must free with `synapse_free_string`.
#[no_mangle]
pub extern "C" fn synapse_search_autocomplete(prefix: *const c_char, limit: u32) -> *mut c_char {
    let prefix_str = match unsafe { cstr_to_str(prefix) } {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    if let Ok(engine) = get_search().lock() {
        let results = engine.autocomplete(prefix_str, limit as usize);
        let json = format!("[{}]", results.iter()
            .map(|s| format!("\"{}\"", s.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(","));
        to_c_string(&json)
    } else {
        ptr::null_mut()
    }
}

/// Perform fuzzy search. Returns a JSON array of {id, text} objects.
/// Caller must free with `synapse_free_string`.
#[no_mangle]
pub extern "C" fn synapse_search_fuzzy(
    query: *const c_char,
    max_distance: u32,
    limit: u32,
) -> *mut c_char {
    let query_str = match unsafe { cstr_to_str(query) } {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    if let Ok(engine) = get_search().lock() {
        let results = engine.fuzzy_search(query_str, max_distance as usize, limit as usize);
        let json = format!("[{}]", results.iter()
            .map(|(id, text)| format!(r#"{{"id":{},"text":"{}"}}"#, id, text.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(","));
        to_c_string(&json)
    } else {
        ptr::null_mut()
    }
}

/// Perform full-text ranked search. Returns a JSON array of {id, score} objects.
/// Caller must free with `synapse_free_string`.
#[no_mangle]
pub extern "C" fn synapse_search_query(query: *const c_char, limit: u32) -> *mut c_char {
    let query_str = match unsafe { cstr_to_str(query) } {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    if let Ok(engine) = get_search().lock() {
        let results = engine.search(query_str, limit as usize);
        let json = format!("[{}]", results.iter()
            .map(|(id, score)| format!(r#"{{"id":{},"score":{:.4}}}"#, id, score))
            .collect::<Vec<_>>()
            .join(","));
        to_c_string(&json)
    } else {
        ptr::null_mut()
    }
}

// ──────────────────────────────────────────────
//  Minimal base64 encoder (avoids external dep)
// ──────────────────────────────────────────────

fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    let chunks = data.chunks(3);

    for chunk in chunks {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };

        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARSET[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARSET[((triple >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARSET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARSET[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}
