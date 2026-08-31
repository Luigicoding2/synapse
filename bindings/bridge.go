package synapse

import (
	"encoding/json"
	"errors"
)

// ── Types ──────────────────────────────────────

type storeGetResult struct {
	Found bool   `json:"found"`
	Value string `json:"value"`
}

// FuzzyResult represents a fuzzy search match.
type FuzzyResult struct {
	ID   uint64 `json:"id"`
	Text string `json:"text"`
}

// SearchResult represents a ranked search match.
type SearchResult struct {
	ID    uint64  `json:"id"`
	Score float64 `json:"score"`
}

// ── Shared Base64 Decoder ──────────────────────

const base64Chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"

func base64Decode(s string) ([]byte, error) {
	var result []byte
	var buf uint32
	var bits int

	for _, c := range s {
		if c == '=' {
			break
		}
		idx := -1
		for i, bc := range base64Chars {
			if bc == c {
				idx = i
				break
			}
		}
		if idx < 0 {
			return nil, errors.New("invalid base64 character")
		}
		buf = (buf << 6) | uint32(idx)
		bits += 6
		if bits >= 8 {
			bits -= 8
			result = append(result, byte(buf>>bits))
			buf &= (1 << bits) - 1
		}
	}
	return result, nil
}

func parseGetResult(jsonStr string) ([]byte, error) {
	var result storeGetResult
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return nil, err
	}
	if !result.Found {
		return nil, nil
	}
	return base64Decode(result.Value)
}
