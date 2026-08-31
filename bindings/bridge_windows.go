//go:build windows

package synapse

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"syscall"
	"unsafe"
)

var (
	mod = loadDLL()

	procStoreOpen     = mod.NewProc("synapse_store_open")
	procStorePut      = mod.NewProc("synapse_store_put")
	procStoreGet      = mod.NewProc("synapse_store_get")
	procStoreDelete   = mod.NewProc("synapse_store_delete")
	procStoreFlush    = mod.NewProc("synapse_store_flush")
	procSearchAdd     = mod.NewProc("synapse_search_add")
	procSearchAuto    = mod.NewProc("synapse_search_autocomplete")
	procSearchFuzzy   = mod.NewProc("synapse_search_fuzzy")
	procSearchQuery   = mod.NewProc("synapse_search_query")
	procFreeString    = mod.NewProc("synapse_free_string")
)

func loadDLL() *syscall.LazyDLL {
	candidates := []string{
		"synapse_bridge.dll",
		"../target/release/synapse_bridge.dll",
		"../../target/release/synapse_bridge.dll",
		"target/release/synapse_bridge.dll",
	}

	// Also check next to executable
	if exe, err := os.Executable(); err == nil {
		candidates = append(candidates, filepath.Join(filepath.Dir(exe), "synapse_bridge.dll"))
		candidates = append(candidates, filepath.Join(filepath.Dir(exe), "..", "target", "release", "synapse_bridge.dll"))
	}

	for _, path := range candidates {
		if _, err := os.Stat(path); err == nil {
			if abs, err := filepath.Abs(path); err == nil {
				return syscall.NewLazyDLL(abs)
			}
			return syscall.NewLazyDLL(path)
		}
	}

	return syscall.NewLazyDLL("synapse_bridge.dll")
}

func goStringToCString(s string) []byte {
	return append([]byte(s), 0)
}

func cStringToGoString(ptr uintptr) string {
	if ptr == 0 {
		return ""
	}
	var bytes []byte
	p := (*byte)(unsafe.Pointer(ptr))
	for *p != 0 {
		bytes = append(bytes, *p)
		p = (*byte)(unsafe.Pointer(uintptr(unsafe.Pointer(p)) + 1))
	}
	return string(bytes)
}

func freeCString(ptr uintptr) {
	if ptr != 0 {
		_, _, _ = procFreeString.Call(ptr)
	}
}

// ── Store API ──────────────────────────────────

func StoreOpen(dir string) error {
	cDir := goStringToCString(dir)
	r, _, _ := procStoreOpen.Call(uintptr(unsafe.Pointer(&cDir[0])))
	if int32(r) != 0 {
		return errors.New("synapse: failed to open store at " + dir)
	}
	return nil
}

func StorePut(key, val []byte) error {
	if len(key) == 0 {
		return errors.New("synapse: key cannot be empty")
	}
	var valPtr uintptr
	if len(val) > 0 {
		valPtr = uintptr(unsafe.Pointer(&val[0]))
	}
	r, _, _ := procStorePut.Call(
		uintptr(unsafe.Pointer(&key[0])),
		uintptr(uint32(len(key))),
		valPtr,
		uintptr(uint32(len(val))),
	)
	if int32(r) != 0 {
		return errors.New("synapse: store put failed")
	}
	return nil
}

func StoreGet(key []byte) ([]byte, error) {
	if len(key) == 0 {
		return nil, errors.New("synapse: key cannot be empty")
	}
	ptr, _, _ := procStoreGet.Call(
		uintptr(unsafe.Pointer(&key[0])),
		uintptr(uint32(len(key))),
	)
	if ptr == 0 {
		return nil, errors.New("synapse: store get failed")
	}
	defer freeCString(ptr)

	return parseGetResult(cStringToGoString(ptr))
}

func StoreDelete(key []byte) error {
	if len(key) == 0 {
		return errors.New("synapse: key cannot be empty")
	}
	r, _, _ := procStoreDelete.Call(
		uintptr(unsafe.Pointer(&key[0])),
		uintptr(uint32(len(key))),
	)
	if int32(r) != 0 {
		return errors.New("synapse: store delete failed")
	}
	return nil
}

func StoreFlush() error {
	r, _, _ := procStoreFlush.Call()
	if int32(r) != 0 {
		return errors.New("synapse: store flush failed")
	}
	return nil
}

// ── Search API ─────────────────────────────────

func SearchAdd(id uint64, text string) error {
	cText := goStringToCString(text)
	r, _, _ := procSearchAdd.Call(
		uintptr(id),
		uintptr(unsafe.Pointer(&cText[0])),
	)
	if int32(r) != 0 {
		return errors.New("synapse: search add failed")
	}
	return nil
}

func SearchAutocomplete(prefix string, limit int) ([]string, error) {
	cPrefix := goStringToCString(prefix)
	ptr, _, _ := procSearchAuto.Call(
		uintptr(unsafe.Pointer(&cPrefix[0])),
		uintptr(uint32(limit)),
	)
	if ptr == 0 {
		return nil, errors.New("synapse: autocomplete failed")
	}
	defer freeCString(ptr)

	var results []string
	if err := json.Unmarshal([]byte(cStringToGoString(ptr)), &results); err != nil {
		return nil, err
	}
	return results, nil
}

func SearchFuzzy(query string, maxDistance, limit int) ([]FuzzyResult, error) {
	cQuery := goStringToCString(query)
	ptr, _, _ := procSearchFuzzy.Call(
		uintptr(unsafe.Pointer(&cQuery[0])),
		uintptr(uint32(maxDistance)),
		uintptr(uint32(limit)),
	)
	if ptr == 0 {
		return nil, errors.New("synapse: fuzzy search failed")
	}
	defer freeCString(ptr)

	var results []FuzzyResult
	if err := json.Unmarshal([]byte(cStringToGoString(ptr)), &results); err != nil {
		return nil, err
	}
	return results, nil
}

func SearchQuery(query string, limit int) ([]SearchResult, error) {
	cQuery := goStringToCString(query)
	ptr, _, _ := procSearchQuery.Call(
		uintptr(unsafe.Pointer(&cQuery[0])),
		uintptr(uint32(limit)),
	)
	if ptr == 0 {
		return nil, errors.New("synapse: search query failed")
	}
	defer freeCString(ptr)

	var results []SearchResult
	if err := json.Unmarshal([]byte(cStringToGoString(ptr)), &results); err != nil {
		return nil, err
	}
	return results, nil
}
