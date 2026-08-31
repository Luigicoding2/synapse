#  Synapse: The High Performance Systems Trilogy

<p align="center">
  <a href="https://github.com"><img src="https://img.shields.io/badge/Rust-1.75%2B-orange.svg?style=flat-square&logo=rust" alt="Rust"></a>
  <a href="https://github.com"><img src="https://img.shields.io/badge/Go-1.21%2B-blue.svg?style=flat-square&logo=go" alt="Go"></a>
  <a href="https://github.com"><img src="https://img.shields.io/badge/License-MIT-green.svg?style=flat-square" alt="License"></a>
  <a href="https://github.com"><img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blueviolet.svg?style=flat-square" alt="Platform"></a>
  <a href="https://github.com"><img src="https://img.shields.io/badge/Tests-8%2F8%20Passing-brightgreen.svg?style=flat-square" alt="Tests"></a>
</p>

**Synapse** is a suite of three complementary, zero-dependency, ultra-fast systems libraries written in **Rust** with native bindings for **Go, C, and Python**.

It provides everything developers need to build high-speed storage, typo-tolerant search, and instant autocomplete with microsecond latencies.

---

##  Performance Benchmarks

All operations execute in **microseconds** (tested on Intel Core i5-11400 @ 4.40 GHz via Go bindings):

| Operation | Latency | Throughput | Description |
|---|---|---|---|
| **`StorePut`** | **2.9 µs** | **~343,000 ops/sec** | Append-only WAL write + in-memory MemTable update |
| **`StoreGet`** | **3.2 µs** | **~311,000 ops/sec** | Binary-searched SSTable + MemTable lookup |
| **`SearchAutocomplete`** | **2.5 µs** | **~392,000 queries/sec** | Prefix Trie traversal |
| **`SearchFuzzy`** | **12.3 µs** | **~81,000 queries/sec** | Levenshtein edit-distance typo correction |

---

##  The Trilogy Architecture

```
                       ┌────────────────────────┐
                       │   Host Application     │
                       │ (Go, Python, C++, Node)│
                       └───────────┬────────────┘
                                   │ Zero-Copy FFI Call
                                   ▼
        ┌────────────────────────────────────────────────────────┐
        │               synapse-bridge (C ABI)                   │
        └──────────────┬──────────────────────────┬──────────────┘
                       │                          │
                       ▼                          ▼
        ┌────────────────────────┐ ┌─────────────────────────────┐
        │     synapse-store      │ │       synapse-search        │
        │  Embedded LSM-Tree DB  │ │  Fuzzy & Autocomplete Engine│
        │                        │ │                             │
        │  • BTreeMap MemTable   │ │  • Prefix Trie (Autocomplete)│
        │  • Append-Only WAL     │ │  • Levenshtein Edit-Distance │
        │  • Indexed SSTables    │ │  • Inverted Index (TF-IDF)  │
        │  • Level Compaction    │ │                             │
        └────────────────────────┘ └─────────────────────────────┘
```

---

##  Quickstart

### 1. Using in Go

```go
package main

import (
	"fmt"
	"synapse"
)

func main() {
	// 1. Initialize Storage Engine
	_ = synapse.StoreOpen("./data/db")
	_ = synapse.StorePut([]byte("product:42"), []byte("Apple iPhone 15 Pro"))

	val, _ := synapse.StoreGet([]byte("product:42"))
	fmt.Printf("Retrieved from DB: %s\n", string(val))

	// 2. Initialize Search & Autocomplete Engine
	_ = synapse.SearchAdd(42, "Apple iPhone 15 Pro Max")
	_ = synapse.SearchAdd(43, "Samsung Galaxy S24 Ultra")

	// Autocomplete (sub-millisecond prefix suggestion)
	suggestions, _ := synapse.SearchAutocomplete("iph", 5)
	fmt.Printf("Autocomplete: %v\n", suggestions) // ["iphone"]

	// Typo-Tolerant Fuzzy Search ("iphne" -> matches iPhone)
	fuzzyMatches, _ := synapse.SearchFuzzy("iphne", 2, 5)
	fmt.Printf("Fuzzy Match: %+v\n", fuzzyMatches)

	// Ranked Full-Text Search (TF-IDF scoring)
	ranked, _ := synapse.SearchQuery("ultra", 5)
	fmt.Printf("Ranked Search: %+v\n", ranked)
}
```

### 2. Using in Rust

Add the dependencies to your `Cargo.toml`:

```toml
[dependencies]
synapse-store = { path = "synapse-store" }
synapse-search = { path = "synapse-search" }
```

```rust
use synapse_store::SynapseStore;
use synapse_search::SynapseSearch;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Storage Engine (LSM-Tree Key-Value)
    let store = SynapseStore::open("./db_data")?;
    store.put(b"session_key", b"active_payload")?;
    let val = store.get(b"session_key")?;

    // Search Engine (Fuzzy + Autocomplete)
    let mut search = SynapseSearch::new();
    search.add(1, "PostgreSQL High Availability");
    
    // Typo-tolerant query
    let results = search.fuzzy_search("postgre", 2, 5);
    println!("Found doc: {:?}", results);

    Ok(())
}
```

---

##  Testing & Verification

Run all workspace tests:

```bash
cargo test --workspace
```

Run Go test harness & benchmarks:

```bash
cd bindings
go test -v .
go test -bench=. -benchmem .
```

---

## 📄 License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.
