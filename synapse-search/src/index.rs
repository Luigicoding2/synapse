/// Inverted Index — maps tokens to the document IDs that contain them.
/// Supports TF-IDF-style relevance scoring.

use std::collections::HashMap;

/// A single indexed document.
#[derive(Debug, Clone)]
pub struct Document {
    pub id: u64,
    pub text: String,
}

/// Tokenize text: lowercase, split on non-alphanumeric, filter short tokens.
pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 2)
        .map(|s| s.to_string())
        .collect()
}

/// The inverted index: token -> list of (doc_id, term_frequency).
pub struct InvertedIndex {
    /// token -> Vec<(doc_id, count)>
    postings: HashMap<String, Vec<(u64, u32)>>,
    /// Total number of documents indexed.
    pub doc_count: u64,
    /// All unique tokens in the index (used for fuzzy candidate matching).
    pub all_tokens: Vec<String>,
    /// doc_id -> original text (for returning results)
    pub documents: HashMap<u64, String>,
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            postings: HashMap::new(),
            doc_count: 0,
            all_tokens: Vec::new(),
            documents: HashMap::new(),
        }
    }

    /// Add a document to the index.
    pub fn add(&mut self, doc: Document) {
        let tokens = tokenize(&doc.text);
        self.documents.insert(doc.id, doc.text.clone());
        self.doc_count += 1;

        // Count term frequency per token in this document
        let mut tf_map: HashMap<String, u32> = HashMap::new();
        for token in &tokens {
            *tf_map.entry(token.clone()).or_insert(0) += 1;
        }

        for (token, count) in tf_map {
            let entry = self.postings.entry(token.clone()).or_insert_with(Vec::new);
            entry.push((doc.id, count));

            // Track unique tokens
            if entry.len() == 1 {
                self.all_tokens.push(token);
            }
        }
    }

    /// Search the index for documents matching a query.
    /// Returns (doc_id, score) pairs sorted by relevance (highest first).
    pub fn search(&self, query: &str, limit: usize) -> Vec<(u64, f64)> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() {
            return Vec::new();
        }

        let mut scores: HashMap<u64, f64> = HashMap::new();

        for token in &query_tokens {
            if let Some(postings) = self.postings.get(token) {
                // IDF = log(total_docs / docs_containing_token)
                let idf = (self.doc_count as f64 / postings.len() as f64).ln().max(0.1);

                for &(doc_id, tf) in postings {
                    // TF-IDF score
                    let tf_score = 1.0 + (tf as f64).ln();
                    *scores.entry(doc_id).or_insert(0.0) += tf_score * idf;
                }
            }
        }

        let mut results: Vec<(u64, f64)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello, World! This is a test-123.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        assert!(tokens.contains(&"123".to_string()));
        // Single char "a" should be filtered out
        assert!(!tokens.contains(&"a".to_string()));
    }

    #[test]
    fn test_index_search() {
        let mut idx = InvertedIndex::new();
        idx.add(Document { id: 1, text: "Rust programming language".to_string() });
        idx.add(Document { id: 2, text: "Go programming language".to_string() });
        idx.add(Document { id: 3, text: "Rust is fast and safe".to_string() });

        let results = idx.search("rust", 10);
        assert!(!results.is_empty());
        // Both doc 1 and 3 mention "rust"
        let ids: Vec<u64> = results.iter().map(|r| r.0).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&3));

        let results = idx.search("programming language", 10);
        let ids: Vec<u64> = results.iter().map(|r| r.0).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
    }
}
