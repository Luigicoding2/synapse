use synapse_store::SynapseStore;
use std::fs;
use std::path::Path;

fn cleanup(dir: &str) {
    let path = Path::new(dir);
    if path.exists() {
        let _ = fs::remove_dir_all(path);
    }
}

#[test]
fn test_store_lifecycle() {
    let db_path = "./target/test_store_db";
    cleanup(db_path);

    // 1. Open database
    let store = SynapseStore::open(db_path).expect("Failed to open DB");

    // 2. Put and Get
    store.put(b"key_a", b"value_a").unwrap();
    store.put(b"key_b", b"value_b").unwrap();

    assert_eq!(store.get(b"key_a").unwrap(), Some(b"value_a".to_vec()));
    assert_eq!(store.get(b"key_b").unwrap(), Some(b"value_b".to_vec()));
    assert_eq!(store.get(b"key_c").unwrap(), None);

    // 3. Delete key
    store.delete(b"key_a").unwrap();
    assert_eq!(store.get(b"key_a").unwrap(), None);

    // 4. Test persist & flush to disk
    store.put(b"key_c", b"value_c").unwrap();
    store.flush().expect("Flush failed");

    // Verify key_c resides in SSTable now (and is still readable)
    assert_eq!(store.get(b"key_c").unwrap(), Some(b"value_c".to_vec()));
    assert_eq!(store.get(b"key_a").unwrap(), None); // Tombstone should still mask it

    // 5. Test recovery (re-opening the database loads disk files)
    drop(store); // Close db handles
    let store = SynapseStore::open(db_path).expect("Failed to reopen DB");
    assert_eq!(store.get(b"key_c").unwrap(), Some(b"value_c".to_vec()));
    assert_eq!(store.get(b"key_b").unwrap(), Some(b"value_b".to_vec()));
    assert_eq!(store.get(b"key_a").unwrap(), None);

    // 6. Test Compaction
    store.put(b"key_c", b"value_c_updated").unwrap();
    store.flush().unwrap(); // Create second sstable with override
    
    assert_eq!(store.get(b"key_c").unwrap(), Some(b"value_c_updated".to_vec()));

    store.compact().expect("Compaction failed");
    
    // Verify values remain intact after compilation merges sstables
    assert_eq!(store.get(b"key_c").unwrap(), Some(b"value_c_updated".to_vec()));
    assert_eq!(store.get(b"key_b").unwrap(), Some(b"value_b".to_vec()));
    assert_eq!(store.get(b"key_a").unwrap(), None);

    cleanup(db_path);
}
