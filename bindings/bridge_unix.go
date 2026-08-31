//go:build !windows

package synapse

/*
#cgo CFLAGS: -I${SRCDIR}
#cgo linux LDFLAGS: -L${SRCDIR}/../target/release -lsynapse_bridge -lm -ldl -lpthread
#cgo darwin LDFLAGS: -L${SRCDIR}/../target/release -lsynapse_bridge -lm -ldl -lpthread -framework Security -framework CoreFoundation
#include "bridge.h"
#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"errors"
	"unsafe"
)

// ── Store API ──────────────────────────────────

func StoreOpen(dir string) error {
	cDir := C.CString(dir)
	defer C.free(unsafe.Pointer(cDir))

	if C.synapse_store_open(cDir) != 0 {
		return errors.New("synapse: failed to open store at " + dir)
	}
	return nil
}

func StorePut(key, val []byte) error {
	if len(key) == 0 {
		return errors.New("synapse: key cannot be empty")
	}
	var valPtr *C.uint8_t
	if len(val) > 0 {
		valPtr = (*C.uint8_t)(unsafe.Pointer(&val[0]))
	}
	res := C.synapse_store_put(
		(*C.uint8_t)(unsafe.Pointer(&key[0])), C.uint32_t(len(key)),
		valPtr, C.uint32_t(len(val)),
	)
	if res != 0 {
		return errors.New("synapse: store put failed")
	}
	return nil
}

func StoreGet(key []byte) ([]byte, error) {
	if len(key) == 0 {
		return nil, errors.New("synapse: key cannot be empty")
	}
	cResult := C.synapse_store_get(
		(*C.uint8_t)(unsafe.Pointer(&key[0])), C.uint32_t(len(key)),
	)
	if cResult == nil {
		return nil, errors.New("synapse: store get failed")
	}
	defer C.synapse_free_string(cResult)

	return parseGetResult(C.GoString(cResult))
}

func StoreDelete(key []byte) error {
	if len(key) == 0 {
		return errors.New("synapse: key cannot be empty")
	}
	res := C.synapse_store_delete(
		(*C.uint8_t)(unsafe.Pointer(&key[0])), C.uint32_t(len(key)),
	)
	if res != 0 {
		return errors.New("synapse: store delete failed")
	}
	return nil
}

func StoreFlush() error {
	if C.synapse_store_flush() != 0 {
		return errors.New("synapse: store flush failed")
	}
	return nil
}

// ── Search API ─────────────────────────────────

func SearchAdd(id uint64, text string) error {
	cText := C.CString(text)
	defer C.free(unsafe.Pointer(cText))

	if C.synapse_search_add(C.uint64_t(id), cText) != 0 {
		return errors.New("synapse: search add failed")
	}
	return nil
}

func SearchAutocomplete(prefix string, limit int) ([]string, error) {
	cPrefix := C.CString(prefix)
	defer C.free(unsafe.Pointer(cPrefix))

	cResult := C.synapse_search_autocomplete(cPrefix, C.uint32_t(limit))
	if cResult == nil {
		return nil, errors.New("synapse: autocomplete failed")
	}
	defer C.synapse_free_string(cResult)

	var results []string
	if err := json.Unmarshal([]byte(C.GoString(cResult)), &results); err != nil {
		return nil, err
	}
	return results, nil
}

func SearchFuzzy(query string, maxDistance, limit int) ([]FuzzyResult, error) {
	cQuery := C.CString(query)
	defer C.free(unsafe.Pointer(cQuery))

	cResult := C.synapse_search_fuzzy(cQuery, C.uint32_t(maxDistance), C.uint32_t(limit))
	if cResult == nil {
		return nil, errors.New("synapse: fuzzy search failed")
	}
	defer C.synapse_free_string(cResult)

	var results []FuzzyResult
	if err := json.Unmarshal([]byte(C.GoString(cResult)), &results); err != nil {
		return nil, err
	}
	return results, nil
}

func SearchQuery(query string, limit int) ([]SearchResult, error) {
	cQuery := C.CString(query)
	defer C.free(unsafe.Pointer(cQuery))

	cResult := C.synapse_search_query(cQuery, C.uint32_t(limit))
	if cResult == nil {
		return nil, errors.New("synapse: search query failed")
	}
	defer C.synapse_free_string(cResult)

	var results []SearchResult
	if err := json.Unmarshal([]byte(C.GoString(cResult)), &results); err != nil {
		return nil, err
	}
	return results, nil
}
