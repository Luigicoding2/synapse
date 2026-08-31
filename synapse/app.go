package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/wailsapp/wails/v2/pkg/runtime"
)

// App struct
type App struct {
	ctx        context.Context
	scannedDir string
}

// NewApp creates a new App application struct
func NewApp() *App {
	return &App{}
}

// startup is called when the app starts. The context is saved
// so we can call the runtime methods
func (a *App) startup(ctx context.Context) {
	a.ctx = ctx
}

type LLMConfig struct {
	Provider  string `json:"provider"`  // "gemini", "ollama", "custom"
	APIKey    string `json:"apiKey"`
	BaseURL   string `json:"baseUrl"`
	ModelName string `json:"modelName"`
}

func (a *App) getConfigPath() string {
	home, err := os.UserHomeDir()
	if err != nil {
		return ".synapse-config.json"
	}
	return filepath.Join(home, ".synapse-config.json")
}

// SaveConfig saves LLM configuration
func (a *App) SaveConfig(configJson string) error {
	configPath := a.getConfigPath()
	return os.WriteFile(configPath, []byte(configJson), 0644)
}

// LoadConfig loads LLM configuration
func (a *App) LoadConfig() (string, error) {
	configPath := a.getConfigPath()
	data, err := os.ReadFile(configPath)
	if err != nil {
		if os.IsNotExist(err) {
			// Return default configuration
			defaultConfig := LLMConfig{
				Provider:  "ollama",
				BaseURL:   "http://localhost:11434",
				ModelName: "llama3",
			}
			bytes, _ := json.Marshal(defaultConfig)
			return string(bytes), nil
		}
		return "", err
	}
	return string(data), nil
}

// SelectDirectory opens the OS folder picker
func (a *App) SelectDirectory() (string, error) {
	return runtime.OpenDirectoryDialog(a.ctx, runtime.OpenDialogOptions{
		Title: "Select Project Directory to Scan",
	})
}

// ScanDirectory calls the Rust sidecar binary and scans the codebase
func (a *App) ScanDirectory(dirPath string) (string, error) {
	a.scannedDir = dirPath

	// Find the sidecar / binary
	binPath := ""

	// Check sidecar packaging path (production next to exe or relative target paths)
	exePath, err := os.Executable()
	if err == nil {
		exeDir := filepath.Dir(exePath)
		
		// Candidate 1: Production sidecar folder (next to exe)
		candidate1 := filepath.Join(exeDir, "synapse-core.exe")
		if _, err := os.Stat(candidate1); err == nil {
			binPath = candidate1
		}
		
		// Candidate 2: Dev release path relative to build/bin/synapse.exe
		if binPath == "" {
			candidate2 := filepath.Clean(filepath.Join(exeDir, "..", "..", "core", "target", "release", "synapse-core.exe"))
			if _, err := os.Stat(candidate2); err == nil {
				binPath = candidate2
			}
		}
		
		// Candidate 3: Dev debug path relative to build/bin/synapse.exe
		if binPath == "" {
			candidate3 := filepath.Clean(filepath.Join(exeDir, "..", "..", "core", "target", "debug", "synapse-core.exe"))
			if _, err := os.Stat(candidate3); err == nil {
				binPath = candidate3
			}
		}
	}

	// Check relative paths from current working directory (in case running via CLI inside workspace root)
	if binPath == "" {
		candidate4 := filepath.Join("core", "target", "release", "synapse-core.exe")
		if _, err := os.Stat(candidate4); err == nil {
			binPath = candidate4
		}
	}

	if binPath == "" {
		candidate5 := filepath.Join("core", "target", "debug", "synapse-core.exe")
		if _, err := os.Stat(candidate5); err == nil {
			binPath = candidate5
		}
	}

	if binPath == "" {
		return "", fmt.Errorf("synapse-core parsing binary not found. Tested paths: next to exe, project release folder, and project debug folder. Please ensure the Rust core is built")
	}

	// Execute Rust scanner
	cmd := exec.Command(binPath, dirPath)
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	err = cmd.Run()
	if err != nil {
		return "", fmt.Errorf("failed scanning directory: %v, details: %s", err, stderr.String())
	}

	return stdout.String(), nil
}

// GetFileContent reads the local file contents
func (a *App) GetFileContent(relativePath string) (string, error) {
	if a.scannedDir == "" {
		return "", fmt.Errorf("no directory scanned yet")
	}

	// Resolve the full path and sanitize against directory traversal
	fullPath := filepath.Join(a.scannedDir, relativePath)
	rel, err := filepath.Rel(a.scannedDir, fullPath)
	if err != nil || strings.HasPrefix(rel, "..") {
		return "", fmt.Errorf("security error: path is outside scanned workspace")
	}

	content, err := os.ReadFile(fullPath)
	if err != nil {
		return "", err
	}

	return string(content), nil
}

// AskAI sends prompt along with code context to the configured LLM
func (a *App) AskAI(prompt string, filePath string, fileContent string) (string, error) {
	configStr, err := a.LoadConfig()
	if err != nil {
		return "", fmt.Errorf("failed to load configurations: %v", err)
	}

	var config LLMConfig
	if err := json.Unmarshal([]byte(configStr), &config); err != nil {
		return "", fmt.Errorf("failed to parse configurations: %v", err)
	}

	client := &http.Client{
		Timeout: 60 * time.Second,
	}

	systemInstruction := "You are Synapse, an expert developer AI assistant integrated into a codebase visualizer app. Help the developer understand their code. Provide concise, direct answers, and highlight critical parts. Format code snippets in markdown."
	contextMessage := fmt.Sprintf("Context: The user is viewing this file:\nFile: %s\n\n```\n%s\n```\n\nUser Question: %s", filePath, fileContent, prompt)

	if filePath == "" {
		contextMessage = fmt.Sprintf("Context: General workspace conversation. No active file selected.\n\nUser Question: %s", prompt)
	}

	switch config.Provider {
	case "gemini":
		model := config.ModelName
		if model == "" {
			model = "gemini-1.5-flash"
		}
		url := fmt.Sprintf("https://generativelanguage.googleapis.com/v1beta/models/%s:generateContent?key=%s", model, config.APIKey)

		reqBody := map[string]interface{}{
			"contents": []interface{}{
				map[string]interface{}{
					"parts": []interface{}{
						map[string]interface{}{
							"text": fmt.Sprintf("%s\n\n%s", systemInstruction, contextMessage),
						},
					},
				},
			},
		}

		jsonData, _ := json.Marshal(reqBody)
		req, err := http.NewRequest("POST", url, bytes.NewBuffer(jsonData))
		if err != nil {
			return "", err
		}
		req.Header.Set("Content-Type", "application/json")

		resp, err := client.Do(req)
		if err != nil {
			return "", err
		}
		defer resp.Body.Close()

		bodyBytes, _ := io.ReadAll(resp.Body)
		if resp.StatusCode != http.StatusOK {
			return "", fmt.Errorf("gemini API returned status %s: %s", resp.Status, string(bodyBytes))
		}

		var geminiResp struct {
			Candidates []struct {
				Content struct {
					Parts []struct {
						Text string `json:"text"`
					} `json:"parts"`
				} `json:"content"`
			} `json:"candidates"`
		}

		if err := json.Unmarshal(bodyBytes, &geminiResp); err != nil {
			return "", fmt.Errorf("failed parsing Gemini response: %v", err)
		}

		if len(geminiResp.Candidates) > 0 && len(geminiResp.Candidates[0].Content.Parts) > 0 {
			return geminiResp.Candidates[0].Content.Parts[0].Text, nil
		}
		return "", fmt.Errorf("no reply generated from Gemini API")

	case "ollama":
		baseUrl := config.BaseURL
		if baseUrl == "" {
			baseUrl = "http://localhost:11434"
		}
		url := fmt.Sprintf("%s/api/generate", strings.TrimSuffix(baseUrl, "/"))

		model := config.ModelName
		if model == "" {
			model = "llama3"
		}

		reqBody := map[string]interface{}{
			"model":  model,
			"prompt": fmt.Sprintf("%s\n\n%s", systemInstruction, contextMessage),
			"stream": false,
		}

		jsonData, _ := json.Marshal(reqBody)
		req, err := http.NewRequest("POST", url, bytes.NewBuffer(jsonData))
		if err != nil {
			return "", err
		}
		req.Header.Set("Content-Type", "application/json")

		resp, err := client.Do(req)
		if err != nil {
			return "", fmt.Errorf("failed to reach local Ollama. Is it running? %v", err)
		}
		defer resp.Body.Close()

		bodyBytes, _ := io.ReadAll(resp.Body)
		if resp.StatusCode != http.StatusOK {
			return "", fmt.Errorf("ollama API returned status %s: %s", resp.Status, string(bodyBytes))
		}

		var ollamaResp struct {
			Response string `json:"response"`
		}

		if err := json.Unmarshal(bodyBytes, &ollamaResp); err != nil {
			return "", fmt.Errorf("failed parsing Ollama response: %v", err)
		}

		return ollamaResp.Response, nil

	case "custom":
		baseUrl := config.BaseURL
		url := fmt.Sprintf("%s/chat/completions", strings.TrimSuffix(baseUrl, "/"))

		reqBody := map[string]interface{}{
			"model": config.ModelName,
			"messages": []interface{}{
				map[string]interface{}{
					"role":    "system",
					"content": systemInstruction,
				},
				map[string]interface{}{
					"role":    "user",
					"content": contextMessage,
				},
			},
		}

		jsonData, _ := json.Marshal(reqBody)
		req, err := http.NewRequest("POST", url, bytes.NewBuffer(jsonData))
		if err != nil {
			return "", err
		}
		req.Header.Set("Content-Type", "application/json")
		if config.APIKey != "" {
			req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", config.APIKey))
		}

		resp, err := client.Do(req)
		if err != nil {
			return "", err
		}
		defer resp.Body.Close()

		bodyBytes, _ := io.ReadAll(resp.Body)
		if resp.StatusCode != http.StatusOK {
			return "", fmt.Errorf("custom API returned status %s: %s", resp.Status, string(bodyBytes))
		}

		var customResp struct {
			Choices []struct {
				Message struct {
					Content string `json:"content"`
				} `json:"message"`
			} `json:"choices"`
		}

		if err := json.Unmarshal(bodyBytes, &customResp); err != nil {
			return "", fmt.Errorf("failed parsing Custom response: %v", err)
		}

		if len(customResp.Choices) > 0 {
			return customResp.Choices[0].Message.Content, nil
		}
		return "", fmt.Errorf("no reply generated from custom endpoint")

	default:
		return "", fmt.Errorf("unsupported provider: %s", config.Provider)
	}
}
