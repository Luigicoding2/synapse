/// Levenshtein distance calculator for typo-tolerant fuzzy matching.
/// Computes the minimum number of single-character edits (insertions, deletions,
/// or substitutions) required to change one word into another.

/// Calculate the Levenshtein edit distance between two byte slices.
pub fn levenshtein_distance(a: &[u8], b: &[u8]) -> usize {
    let len_a = a.len();
    let len_b = b.len();

    // Optimize: if one string is empty, distance = length of the other
    if len_a == 0 { return len_b; }
    if len_b == 0 { return len_a; }

    // Use a single-row DP approach for memory efficiency (O(min(m,n)) space)
    let mut prev_row: Vec<usize> = (0..=len_b).collect();
    let mut curr_row: Vec<usize> = vec![0; len_b + 1];

    for i in 1..=len_a {
        curr_row[0] = i;
        for j in 1..=len_b {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr_row[j] = (prev_row[j] + 1)               // deletion
                .min(curr_row[j - 1] + 1)                  // insertion
                .min(prev_row[j - 1] + cost);              // substitution
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[len_b]
}

/// Find all words in `candidates` within `max_distance` edits of `query`.
/// Returns matches sorted by distance (closest first), then alphabetically.
pub fn fuzzy_match(query: &str, candidates: &[String], max_distance: usize) -> Vec<(String, usize)> {
    let query_lower = query.to_lowercase();
    let query_bytes = query_lower.as_bytes();

    let mut matches: Vec<(String, usize)> = candidates
        .iter()
        .filter_map(|candidate| {
            let candidate_lower = candidate.to_lowercase();
            let dist = levenshtein_distance(query_bytes, candidate_lower.as_bytes());
            if dist <= max_distance {
                Some((candidate.clone(), dist))
            } else {
                None
            }
        })
        .collect();

    // Sort by distance first, then alphabetically for stability
    matches.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_basic() {
        assert_eq!(levenshtein_distance(b"kitten", b"sitting"), 3);
        assert_eq!(levenshtein_distance(b"", b"abc"), 3);
        assert_eq!(levenshtein_distance(b"abc", b"abc"), 0);
        assert_eq!(levenshtein_distance(b"abc", b"ab"), 1);
        assert_eq!(levenshtein_distance(b"iphone", b"iphne"), 1);
    }

    #[test]
    fn test_fuzzy_match() {
        let candidates = vec![
            "iPhone".to_string(),
            "iPad".to_string(),
            "iPod".to_string(),
            "Android".to_string(),
            "Windows".to_string(),
        ];

        let results = fuzzy_match("iphne", &candidates, 2);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "iPhone"); // Closest match

        let results = fuzzy_match("androd", &candidates, 2);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "Android");

        let results = fuzzy_match("xyz123", &candidates, 1);
        assert!(results.is_empty());
    }
}
