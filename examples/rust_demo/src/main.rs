use synapse_store::SynapseStore;
use synapse_search::SynapseSearch;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦀 Running Synapse Rust Demo...\n");

    // ── 1. Embedded Key-Value Storage Demo ──
    let db_path = "./demo_db";
    let store = SynapseStore::open(db_path)?;

    println!("1. Storing data in LSM-Tree Key-Value Engine...");
    store.put(b"user:001", b"{\"name\": \"Alice\", \"role\": \"Staff Engineer\"}")?;
    store.put(b"user:002", b"{\"name\": \"Bob\", \"role\": \"Security Architect\"}")?;

    if let Some(val) = store.get(b"user:001")? {
        println!("   [DB Read] user:001 => {}", String::from_utf8_lossy(&val));
    }

    store.flush()?;
    println!("   [DB] MemTable flushed to on-disk SSTable.\n");

    // ── 2. In-Memory Search & Autocomplete Demo ──
    println!("2. Indexing documents in Search Engine...");
    let mut search = SynapseSearch::new();
    search.add(1, "PostgreSQL High Availability and Replication Guide");
    search.add(2, "Kubernetes Cloud Native Container Orchestration");
    search.add(3, "Rust Systems Programming and Memory Safety");

    // Autocomplete
    let prefix = "post";
    let suggestions = search.autocomplete(prefix, 5);
    println!("   [Autocomplete] Prefix '{}' => {:?}", prefix, suggestions);

    // Typo-Tolerant Search
    let typo_query = "kubernets";
    let fuzzy_results = search.fuzzy_search(typo_query, 2, 5);
    println!("   [Fuzzy Search] Query '{}' (with typo) => {:?}", typo_query, fuzzy_results);

    // Full-Text Ranked Search
    let rank_query = "programming";
    let ranked_results = search.search(rank_query, 5);
    println!("   [Ranked Search] Query '{}' => {:?}", rank_query, ranked_results);

    // Clean up demo db
    let _ = std::fs::remove_dir_all(db_path);

    println!("\n✅ Rust demo completed successfully!");
    Ok(())
}
