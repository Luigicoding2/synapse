package main

import (
	"fmt"
	"os"
	"synapse"
)

func main() {
	fmt.Println("🔷 Running Synapse Go Demo...\n")

	// ── 1. Embedded Key-Value Storage Demo ──
	dbPath := "./go_demo_db"
	defer os.RemoveAll(dbPath)

	if err := synapse.StoreOpen(dbPath); err != nil {
		panic(fmt.Sprintf("Failed to open store: %v", err))
	}

	fmt.Println("1. Storing data in LSM-Tree Key-Value Engine via Go...")
	_ = synapse.StorePut([]byte("server:config"), []byte("{\"port\": 8080, \"ssl\": true, \"workers\": 16}"))

	val, err := synapse.StoreGet([]byte("server:config"))
	if err != nil {
		panic(err)
	}
	fmt.Printf("   [DB Read] server:config => %s\n", string(val))

	_ = synapse.StoreFlush()
	fmt.Println("   [DB] MemTable flushed to on-disk SSTable.\n")

	// ── 2. In-Memory Search & Autocomplete Demo ──
	fmt.Println("2. Indexing documents in Search Engine via Go...")
	_ = synapse.SearchAdd(101, "Google Antigravity Developer Suite")
	_ = synapse.SearchAdd(102, "Anthropic Claude Large Language Models")
	_ = synapse.SearchAdd(103, "OpenAI GPT Artificial General Intelligence")

	// Autocomplete
	suggestions, _ := synapse.SearchAutocomplete("ant", 5)
	fmt.Printf("   [Autocomplete] Prefix 'ant' => %v\n", suggestions)

	// Typo-Tolerant Fuzzy Search ("anthrpic" -> Anthropic)
	fuzzyMatches, _ := synapse.SearchFuzzy("anthrpic", 2, 5)
	fmt.Printf("   [Fuzzy Search] Query 'anthrpic' (with typo) => %+v\n", fuzzyMatches)

	// Ranked Search ("Intelligence")
	ranked, _ := synapse.SearchQuery("intelligence", 5)
	fmt.Printf("   [Ranked Search] Query 'intelligence' => %+v\n", ranked)

	fmt.Println("\n✅ Go demo completed successfully!")
}
