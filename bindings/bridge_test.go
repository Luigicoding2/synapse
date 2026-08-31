package synapse

import (
	"os"
	"path/filepath"
	"testing"
)

func TestStoreOperations(t *testing.T) {
	testDir := filepath.Join(os.TempDir(), "synapse_go_store_test")
	_ = os.RemoveAll(testDir)
	defer os.RemoveAll(testDir)

	if err := StoreOpen(testDir); err != nil {
		t.Fatalf("StoreOpen failed: %v", err)
	}

	key := []byte("user:1001")
	val := []byte("Jane Doe, Lead Engineer")

	if err := StorePut(key, val); err != nil {
		t.Fatalf("StorePut failed: %v", err)
	}

	got, err := StoreGet(key)
	if err != nil {
		t.Fatalf("StoreGet failed: %v", err)
	}
	if string(got) != string(val) {
		t.Fatalf("StoreGet mismatch: got %s, want %s", string(got), string(val))
	}

	// Test non-existent key
	nonExistent, err := StoreGet([]byte("user:9999"))
	if err != nil {
		t.Fatalf("StoreGet non-existent failed: %v", err)
	}
	if nonExistent != nil {
		t.Fatalf("StoreGet expected nil for non-existent key, got: %s", string(nonExistent))
	}

	// Test Delete
	if err := StoreDelete(key); err != nil {
		t.Fatalf("StoreDelete failed: %v", err)
	}

	gotAfterDelete, err := StoreGet(key)
	if err != nil {
		t.Fatalf("StoreGet after delete failed: %v", err)
	}
	if gotAfterDelete != nil {
		t.Fatalf("StoreGet expected nil after delete, got: %s", string(gotAfterDelete))
	}

	// Test Flush
	if err := StoreFlush(); err != nil {
		t.Fatalf("StoreFlush failed: %v", err)
	}
}

func TestSearchOperations(t *testing.T) {
	docs := []struct {
		id   uint64
		text string
	}{
		{1, "Apple iPhone 15 Pro Titanium"},
		{2, "Samsung Galaxy S24 Ultra"},
		{3, "Google Pixel 9 Pro Fold"},
		{4, "Sony WH-1000XM5 Wireless Headphones"},
	}

	for _, doc := range docs {
		if err := SearchAdd(doc.id, doc.text); err != nil {
			t.Fatalf("SearchAdd failed: %v", err)
		}
	}

	// 1. Autocomplete test
	suggestions, err := SearchAutocomplete("iph", 5)
	if err != nil {
		t.Fatalf("SearchAutocomplete failed: %v", err)
	}
	found := false
	for _, s := range suggestions {
		if s == "iphone" {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("SearchAutocomplete expected 'iphone' in suggestions, got: %v", suggestions)
	}

	// 2. Fuzzy search test (with typos: "galxy" -> Galaxy)
	fuzzyResults, err := SearchFuzzy("galxy", 2, 5)
	if err != nil {
		t.Fatalf("SearchFuzzy failed: %v", err)
	}
	if len(fuzzyResults) == 0 || fuzzyResults[0].ID != 2 {
		t.Fatalf("SearchFuzzy expected doc ID 2 for 'galxy', got: %+v", fuzzyResults)
	}

	// 3. Ranked full-text search test ("Pro")
	searchResults, err := SearchQuery("pro", 5)
	if err != nil {
		t.Fatalf("SearchQuery failed: %v", err)
	}
	if len(searchResults) < 2 {
		t.Fatalf("SearchQuery expected at least 2 results containing 'pro', got: %+v", searchResults)
	}
}

// ── Benchmarks ──────────────────────────────────

func BenchmarkStorePut(b *testing.B) {
	testDir := filepath.Join(os.TempDir(), "synapse_bench_store")
	_ = os.RemoveAll(testDir)
	defer os.RemoveAll(testDir)

	_ = StoreOpen(testDir)

	key := []byte("bench_key")
	val := []byte("bench_val_payload_1234567890")

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = StorePut(key, val)
	}
}

func BenchmarkStoreGet(b *testing.B) {
	testDir := filepath.Join(os.TempDir(), "synapse_bench_store_get")
	_ = os.RemoveAll(testDir)
	defer os.RemoveAll(testDir)

	_ = StoreOpen(testDir)
	key := []byte("bench_key_get")
	val := []byte("bench_val_payload_1234567890")
	_ = StorePut(key, val)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = StoreGet(key)
	}
}

func BenchmarkSearchAutocomplete(b *testing.B) {
	_ = SearchAdd(100, "High Performance Distributed Database Engine")
	_ = SearchAdd(101, "High Availability Microservices Architecture")

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = SearchAutocomplete("dist", 5)
	}
}

func BenchmarkSearchFuzzy(b *testing.B) {
	_ = SearchAdd(200, "Kubernetes Container Orchestration Platform")

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = SearchFuzzy("kubernets", 2, 5)
	}
}
