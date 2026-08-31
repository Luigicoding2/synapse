// synapse-bridge C header
// Auto-generated FFI interface for synapse-store and synapse-search
// Compatible with: Go (CGO), Python (ctypes/cffi), Node.js (ffi-napi), C/C++

#ifndef SYNAPSE_BRIDGE_H
#define SYNAPSE_BRIDGE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ── Memory Management ──────────────────────────
// Free any string returned by synapse_* functions.
void synapse_free_string(char* ptr);

// ── Store API ──────────────────────────────────
// Open or create a database at the given directory path.
// Returns 0 on success, -1 on failure.
int32_t synapse_store_open(const char* dir);

// Put a key-value pair. Returns 0 on success, -1 on failure.
int32_t synapse_store_put(
    const uint8_t* key_ptr, uint32_t key_len,
    const uint8_t* val_ptr, uint32_t val_len
);

// Get a value by key. Returns JSON string: {"found":bool,"value":"base64"}.
// Caller must free with synapse_free_string().
char* synapse_store_get(const uint8_t* key_ptr, uint32_t key_len);

// Delete a key. Returns 0 on success, -1 on failure.
int32_t synapse_store_delete(const uint8_t* key_ptr, uint32_t key_len);

// Flush MemTable to disk. Returns 0 on success, -1 on failure.
int32_t synapse_store_flush(void);

// ── Search API ─────────────────────────────────
// Add a document to the search engine. Returns 0 on success, -1 on failure.
int32_t synapse_search_add(uint64_t id, const char* text);

// Get autocomplete suggestions. Returns JSON array string.
// Caller must free with synapse_free_string().
char* synapse_search_autocomplete(const char* prefix, uint32_t limit);

// Fuzzy search with typo tolerance. Returns JSON array of {id, text}.
// Caller must free with synapse_free_string().
char* synapse_search_fuzzy(const char* query, uint32_t max_distance, uint32_t limit);

// Full-text ranked search. Returns JSON array of {id, score}.
// Caller must free with synapse_free_string().
char* synapse_search_query(const char* query, uint32_t limit);

#ifdef __cplusplus
}
#endif

#endif // SYNAPSE_BRIDGE_H
