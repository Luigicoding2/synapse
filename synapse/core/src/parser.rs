use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use regex::Regex;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct FileInfo {
    pub path: String,       // Relative path from root
    pub name: String,       // Basename of the file
    pub size: u64,          // File size in bytes
    pub lines: usize,       // Total lines of code
    pub language: String,   // Detected programming language
    pub imports: Vec<String>, // Raw list of import/require statements
}

pub struct Parser {
    re_js_import_from: Regex,
    re_js_import_simple: Regex,
    re_js_require: Regex,
    re_js_dynamic_import: Regex,
    re_py_from_import: Regex,
    re_py_import: Regex,
    re_go_import: Regex,
    re_rust_mod: Regex,
    re_rust_use: Regex,
    re_cpp_include: Regex,
    re_java_import: Regex,
    re_cs_using: Regex,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            re_js_import_from: Regex::new(r#"import\s+.*\s+from\s+['"]([^'"]+)['"]"#).unwrap(),
            re_js_import_simple: Regex::new(r#"import\s+['"]([^'"]+)['"]"#).unwrap(),
            re_js_require: Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap(),
            re_js_dynamic_import: Regex::new(r#"import\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap(),
            re_py_from_import: Regex::new(r#"from\s+(\.?\.?[a-zA-Z0-9_.]+)\s+import"#).unwrap(),
            re_py_import: Regex::new(r#"import\s+([a-zA-Z0-9_., ]+)"#).unwrap(),
            re_go_import: Regex::new(r#"['"]([^'"]+)['"]"#).unwrap(), // Used inside import block or single import
            re_rust_mod: Regex::new(r#"mod\s+([a-zA-Z0-9_]+);"#).unwrap(),
            re_rust_use: Regex::new(r#"use\s+([a-zA-Z0-9_]+)(?:::|;)"#).unwrap(),
            re_cpp_include: Regex::new(r#"#include\s+["']([^"']+)["']"#).unwrap(),
            re_java_import: Regex::new(r#"import\s+([a-zA-Z0-9_.]+);"#).unwrap(),
            re_cs_using: Regex::new(r#"using\s+([a-zA-Z0-9_.]+);"#).unwrap(),
        }
    }

    pub fn parse_file(&self, path: &Path, root_path: &Path) -> Option<FileInfo> {
        let extension = path.extension()?.to_str()?.to_lowercase();
        let language = match extension.as_str() {
            "go" => "go",
            "rs" => "rust",
            "js" | "jsx" | "mjs" | "cjs" => "javascript",
            "ts" | "tsx" | "mts" | "cts" => "typescript",
            "py" => "python",
            "cpp" | "cc" | "cxx" | "c" => "cpp",
            "h" | "hpp" => "header",
            "java" => "java",
            "cs" => "csharp",
            _ => return None,
        };

        let file = File::open(path).ok()?;
        let metadata = file.metadata().ok()?;
        let size = metadata.len();
        
        let reader = BufReader::new(file);
        let mut lines_count = 0;
        let mut imports = Vec::new();
        let mut in_go_import_block = false;

        for line_res in reader.lines() {
            let line = match line_res {
                Ok(l) => l,
                Err(_) => continue,
            };
            lines_count += 1;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_matches(language) {
                continue;
            }

            match language {
                "javascript" | "typescript" => {
                    if let Some(caps) = self.re_js_import_from.captures(trimmed) {
                        imports.push(caps[1].to_string());
                    } else if let Some(caps) = self.re_js_import_simple.captures(trimmed) {
                        imports.push(caps[1].to_string());
                    } else if let Some(caps) = self.re_js_require.captures(trimmed) {
                        imports.push(caps[1].to_string());
                    } else if let Some(caps) = self.re_js_dynamic_import.captures(trimmed) {
                        imports.push(caps[1].to_string());
                    }
                }
                "python" => {
                    if let Some(caps) = self.re_py_from_import.captures(trimmed) {
                        imports.push(caps[1].to_string());
                    } else if let Some(caps) = self.re_py_import.captures(trimmed) {
                        // Split imports by comma, e.g. "import os, sys"
                        let parts: Vec<&str> = caps[1].split(',').collect();
                        for p in parts {
                            imports.push(p.trim().to_string());
                        }
                    }
                }
                "go" => {
                    if trimmed.starts_with("import (") {
                        in_go_import_block = true;
                        continue;
                    }
                    if in_go_import_block {
                        if trimmed == ")" {
                            in_go_import_block = false;
                            continue;
                        }
                        if let Some(caps) = self.re_go_import.captures(trimmed) {
                            imports.push(caps[1].to_string());
                        }
                    } else if trimmed.starts_with("import ") {
                        if let Some(caps) = self.re_go_import.captures(trimmed) {
                            imports.push(caps[1].to_string());
                        }
                    }
                }
                "rust" => {
                    if let Some(caps) = self.re_rust_mod.captures(trimmed) {
                        imports.push(caps[1].to_string());
                    } else if let Some(caps) = self.re_rust_use.captures(trimmed) {
                        // Only add use statements that target local components (crate, self, super) or crate name
                        imports.push(caps[1].to_string());
                    }
                }
                "cpp" | "header" => {
                    if let Some(caps) = self.re_cpp_include.captures(trimmed) {
                        imports.push(caps[1].to_string());
                    }
                }
                "java" => {
                    if let Some(caps) = self.re_java_import.captures(trimmed) {
                        imports.push(caps[1].to_string());
                    }
                }
                "csharp" => {
                    if let Some(caps) = self.re_cs_using.captures(trimmed) {
                        imports.push(caps[1].to_string());
                    }
                }
                _ => {}
            }
        }

        // Compute relative path
        let relative_path = path.strip_prefix(root_path)
            .ok()?
            .to_string_lossy()
            .replace('\\', "/");

        let name = path.file_name()?.to_string_lossy().to_string();

        Some(FileInfo {
            path: relative_path,
            name,
            size,
            lines: lines_count,
            language: language.to_string(),
            imports,
        })
    }
}

trait HelperExt {
    fn starts_matches(&self, language: &str) -> bool;
}

impl HelperExt for str {
    fn starts_matches(&self, language: &str) -> bool {
        match language {
            "javascript" | "typescript" | "go" | "rust" | "cpp" | "header" | "java" | "csharp" => {
                self.starts_with("//") || self.starts_with("/*") || self.starts_with("*")
            }
            "python" => self.starts_with('#'),
            _ => false,
        }
    }
}
