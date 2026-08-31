//! # synapse-search
//!
//! A blazing-fast, embeddable, in-memory fuzzy search and autocomplete engine.
//!
//! ## Features
//! - **Prefix autocomplete** via a compressed Trie
//! - **Typo-tolerant fuzzy search** via Levenshtein distance
//! - **Relevance-ranked full-text search** via TF-IDF inverted index
//!
//! ## Example
//! ```rust
//! use synapse_search::SynapseSearch;
//!
//! let mut engine = SynapseSearch::new();
//! engine.add(1, "iPhone 15 Pro Max");
//! engine.add(2, "Samsung Galaxy S24");
//! engine.add(3, "Google Pixel 9 Pro");
//!
//! // Exact prefix autocomplete
//! let suggestions = engine.autocomplete("iph", 5);
//! // -> ["iphone"]
//!
//! // Typo-tolerant fuzzy search
//! let results = engine.fuzzy_search("iphne", 2, 10);
//! // -> [(1, "iPhone 15 Pro Max")]
//!
//! // Full-text ranked search
//! let results = engine.search("pro", 10);
//! // -> [(1, ...), (3, ...)]  — both contain "pro"
//! ```

pub mod trie;
pub mod lev;
pub mod index;

use trie::Trie;
use index::{InvertedIndex, Document};
use lev::fuzzy_match;

/// The unified search engine combining autocomplete, fuzzy matching, and full-text search.
pub struct SynapseSearch {
    trie: Trie,
    index: InvertedIndex,
    next_id: u64,
}

impl SynapseSearch {
    /// Create a new empty search engine.
    pub fn new() -> Self {
        Self {
            trie: Trie::new(),
            index: InvertedIndex::new(),
            next_id: 0,
        }
    }

    /// Add a document with an explicit ID.
    pub fn add(&mut self, id: u64, text: &str) {
        // Index each word in the trie for autocomplete
        let tokens = index::tokenize(text);
        for token in &tokens {
            self.trie.insert(token);
        }

        // Index the full document for ranked search
        self.index.add(Document {
            id,
            text: text.to_string(),
        });

        if id >= self.next_id {
            self.next_id = id + 1;
        }
    }

    /// Add a document with an auto-generated ID. Returns the assigned ID.
    pub fn add_auto(&mut self, text: &str) -> u64 {
        let id = self.next_id;
        self.add(id, text);
        id
    }

    /// Get prefix-based autocomplete suggestions.
    /// Returns matching tokens (lowercased words), up to `limit`.
    pub fn autocomplete(&self, prefix: &str, limit: usize) -> Vec<String> {
        self.trie.autocomplete(prefix, limit)
    }

    /// Perform typo-tolerant fuzzy search across all indexed tokens.
    /// Returns (doc_id, document_text) pairs for documents containing
    /// any token within `max_distance` edits of the query.
    pub fn fuzzy_search(&self, query: &str, max_distance: usize, limit: usize) -> Vec<(u64, String)> {
        // Find fuzzy-matched tokens
        let matches = fuzzy_match(query, &self.index.all_tokens, max_distance);

        if matches.is_empty() {
            return Vec::new();
        }

        // Search for documents containing any matched token, collect unique doc IDs
        let mut seen = std::collections::HashSet::new();
        let mut results = Vec::new();

        for (matched_token, _distance) in &matches {
            let search_results = self.index.search(matched_token, limit);
            for (doc_id, _score) in search_results {
                if seen.insert(doc_id) {
                    if let Some(text) = self.index.documents.get(&doc_id) {
                        results.push((doc_id, text.clone()));
                    }
                }
                if results.len() >= limit {
                    return results;
                }
            }
        }

        results
    }

    /// Perform full-text ranked search using TF-IDF scoring.
    /// Returns (doc_id, score) pairs sorted by relevance.
    pub fn search(&self, query: &str, limit: usize) -> Vec<(u64, f64)> {
        self.index.search(query, limit)
    }

    /// Get the original text of a document by ID.
    pub fn get_document(&self, id: u64) -> Option<&String> {
        self.index.documents.get(&id)
    }

    /// Total number of indexed documents.
    pub fn doc_count(&self) -> u64 {
        self.index.doc_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_engine() {
        let mut engine = SynapseSearch::new();
        engine.add(1, "iPhone 15 Pro Max");
        engine.add(2, "Samsung Galaxy S24 Ultra");
        engine.add(3, "Google Pixel 9 Pro");
        engine.add(4, "OnePlus 12");

        // Autocomplete
        let suggestions = engine.autocomplete("iph", 5);
        assert!(suggestions.contains(&"iphone".to_string()));

        let suggestions = engine.autocomplete("sam", 5);
        assert!(suggestions.contains(&"samsung".to_string()));

        // Fuzzy search with typo
        let results = engine.fuzzy_search("iphne", 2, 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, 1); // iPhone doc

        let results = engine.fuzzy_search("samung", 2, 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, 2); // Samsung doc

        // Full-text search
        let results = engine.search("pro", 10);
        let ids: Vec<u64> = results.iter().map(|r| r.0).collect();
        assert!(ids.contains(&1)); // iPhone 15 Pro Max
        assert!(ids.contains(&3)); // Google Pixel 9 Pro
        assert!(!ids.contains(&2)); // Samsung doesn't have "pro"

        assert_eq!(engine.doc_count(), 4);
    }
}
