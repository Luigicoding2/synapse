/// Prefix Trie for fast autocomplete suggestions.
/// Each node stores a character and whether it marks the end of a complete word.

#[derive(Debug, Clone)]
pub struct TrieNode {
    pub children: Vec<(u8, TrieNode)>,
    pub is_terminal: bool,
}

impl TrieNode {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            is_terminal: false,
        }
    }

    fn find_child(&self, byte: u8) -> Option<usize> {
        self.children.iter().position(|(b, _)| *b == byte)
    }

    fn find_or_create_child(&mut self, byte: u8) -> &mut TrieNode {
        if let Some(idx) = self.find_child(byte) {
            &mut self.children[idx].1
        } else {
            self.children.push((byte, TrieNode::new()));
            let last = self.children.len() - 1;
            &mut self.children[last].1
        }
    }
}

/// A prefix trie that indexes lowercased words for autocomplete.
pub struct Trie {
    root: TrieNode,
}

impl Trie {
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
        }
    }

    /// Insert a word into the trie (lowercased).
    pub fn insert(&mut self, word: &str) {
        let word = word.to_lowercase();
        let mut node = &mut self.root;
        for byte in word.bytes() {
            node = node.find_or_create_child(byte);
        }
        node.is_terminal = true;
    }

    /// Find all words that start with the given prefix.
    /// Returns up to `limit` results.
    pub fn autocomplete(&self, prefix: &str, limit: usize) -> Vec<String> {
        let prefix = prefix.to_lowercase();
        let mut node = &self.root;

        // Walk to the prefix node
        for byte in prefix.bytes() {
            match node.find_child(byte) {
                Some(idx) => node = &node.children[idx].1,
                None => return Vec::new(), // Prefix not found
            }
        }

        // Collect all terminal words below this node
        let mut results = Vec::new();
        let mut stack: Vec<(&TrieNode, String)> = vec![(node, prefix.clone())];

        while let Some((current, path)) = stack.pop() {
            if results.len() >= limit {
                break;
            }
            if current.is_terminal {
                results.push(path.clone());
            }
            for (byte, child) in current.children.iter().rev() {
                let mut new_path = path.clone();
                new_path.push(*byte as char);
                stack.push((child, new_path));
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trie_autocomplete() {
        let mut trie = Trie::new();
        trie.insert("apple");
        trie.insert("application");
        trie.insert("apply");
        trie.insert("banana");
        trie.insert("band");

        let results = trie.autocomplete("app", 10);
        assert!(results.contains(&"apple".to_string()));
        assert!(results.contains(&"application".to_string()));
        assert!(results.contains(&"apply".to_string()));
        assert!(!results.contains(&"banana".to_string()));

        let results = trie.autocomplete("ban", 10);
        assert!(results.contains(&"banana".to_string()));
        assert!(results.contains(&"band".to_string()));
        assert_eq!(results.len(), 2);

        let results = trie.autocomplete("xyz", 10);
        assert!(results.is_empty());
    }
}
