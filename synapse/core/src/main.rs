use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use serde::Serialize;

mod parser;
use parser::{FileInfo, Parser};

#[derive(Serialize)]
struct GraphNode {
    id: String,
    name: String,
    r#type: String, // "file" or "dir"
    size: u64,
    lines: usize,
    language: String,
}

#[derive(Serialize, Hash, Eq, PartialEq, Clone)]
struct GraphLink {
    source: String,
    target: String,
}

#[derive(Serialize)]
struct GraphData {
    nodes: Vec<GraphNode>,
    links: Vec<GraphLink>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let target_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let target_dir = fs::canonicalize(&target_dir).unwrap_or(target_dir);

    let parser = Parser::new();
    let mut files = Vec::new();
    let mut file_map = HashSet::new(); // Tracks which files exist by relative path
    let mut folders = HashSet::new(); // Tracks directories

    // 1. Scan directory and collect FileInfo for supported files
    let walker = WalkDir::new(&target_dir).into_iter();
    for entry_res in walker {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        // Ignore common build / tool directories
        let path_str = path.to_string_lossy().replace('\\', "/");
        if path_str.contains("/.git/") 
            || path_str.contains("/node_modules/") 
            || path_str.contains("/target/") 
            || path_str.contains("/build/") 
            || path_str.contains("/dist/") 
            || path_str.contains("/wailsjs/") 
            || path_str.contains("/.wails/")
            || path_str.contains("/.vscode/") 
            || path_str.contains("/.idea/") 
        {
            continue;
        }

        if let Some(file_info) = parser.parse_file(path, &target_dir) {
            file_map.insert(file_info.path.clone());
            
            // Extract parent directories for directory nodes
            let mut parent = Path::new(&file_info.path).parent();
            while let Some(p) = parent {
                let p_str = p.to_string_lossy().replace('\\', "/");
                if !p_str.is_empty() {
                    folders.insert(p_str);
                }
                parent = p.parent();
            }

            files.push(file_info);
        }
    }

    // Attempt to read go.mod to extract Go module path
    let go_module_name = read_go_module_name(&target_dir);

    // 2. Build Links by resolving imports
    let mut links = HashSet::new();

    for f in &files {
        let f_path = Path::new(&f.path);
        let f_dir = f_path.parent().unwrap_or_else(|| Path::new(""));

        for imp in &f.imports {
            match f.language.as_str() {
                "javascript" | "typescript" => {
                    // Resolve relative node imports
                    if imp.starts_with('.') {
                        let resolved_rel = resolve_js_import(f_dir, imp);
                        if let Some(target_rel_path) = find_existing_file(&resolved_rel, &file_map) {
                            links.insert(GraphLink {
                                source: f.path.clone(),
                                target: target_rel_path,
                            });
                        }
                    }
                }
                "python" => {
                    // Resolve relative imports like from .utils import x
                    let resolved_rel = if imp.starts_with('.') {
                        resolve_py_import(f_dir, imp)
                    } else {
                        // Absolute import e.g. "app.database" -> "app/database"
                        imp.replace('.', "/")
                    };

                    if let Some(target_rel_path) = find_existing_py_file(&resolved_rel, &file_map) {
                        links.insert(GraphLink {
                            source: f.path.clone(),
                            target: target_rel_path,
                        });
                    }
                }
                "go" => {
                    if let Some(ref mod_name) = go_module_name {
                        if imp.starts_with(mod_name) {
                            // Strip package prefix: e.g. "github.com/foo/bar/pkg/db" -> "pkg/db"
                            let rel_pkg = &imp[mod_name.len()..];
                            let rel_pkg_trimmed = rel_pkg.trim_start_matches('/');
                            
                            // Go packages represent folders. Let's find any Go file in that folder and link to it, 
                            // or represent folders as links. Let's find go files that start with "pkg/db/"
                            for candidate in &file_map {
                                if candidate.starts_with(rel_pkg_trimmed) && candidate.ends_with(".go") {
                                    links.insert(GraphLink {
                                        source: f.path.clone(),
                                        target: candidate.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
                "rust" => {
                    // Rust imports: `mod parser;` or `use parser::FileInfo;`
                    // Let's resolve `mod sub;` or `use sub::...;`
                    let candidate1 = f_dir.join(imp).to_string_lossy().replace('\\', "/");
                    let candidate2 = f_dir.join(format!("{}/mod", imp)).to_string_lossy().replace('\\', "/");

                    if let Some(target_rel_path) = find_existing_file(&candidate1, &file_map) {
                        links.insert(GraphLink {
                            source: f.path.clone(),
                            target: target_rel_path,
                        });
                    } else if let Some(target_rel_path) = find_existing_file(&candidate2, &file_map) {
                        links.insert(GraphLink {
                            source: f.path.clone(),
                            target: target_rel_path,
                        });
                    }
                }
                "cpp" | "header" => {
                    // C/C++ includes, e.g. "parser.h" or "../utils/math.h"
                    let resolved = f_dir.join(imp).to_string_lossy().replace('\\', "/");
                    if file_map.contains(&resolved) {
                        links.insert(GraphLink {
                            source: f.path.clone(),
                            target: resolved,
                        });
                    } else {
                        // Fallback: search for header with same name in file_map
                        let imp_filename = Path::new(imp).file_name().and_then(|n| n.to_str());
                        if let Some(fname) = imp_filename {
                            for candidate in &file_map {
                                if candidate.ends_with(fname) {
                                    links.insert(GraphLink {
                                        source: f.path.clone(),
                                        target: candidate.clone(),
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // 3. Assemble Graph Nodes (directories and files)
    let mut nodes = Vec::new();

    // Directory Nodes
    for folder in folders {
        let name = Path::new(&folder)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new(""))
            .to_string_lossy()
            .to_string();
        nodes.push(GraphNode {
            id: folder,
            name,
            r#type: "dir".to_string(),
            size: 0,
            lines: 0,
            language: "folder".to_string(),
        });
    }

    // File Nodes
    for f in files {
        nodes.push(GraphNode {
            id: f.path,
            name: f.name,
            r#type: "file".to_string(),
            size: f.size,
            lines: f.lines,
            language: f.language,
        });
    }

    let graph_data = GraphData {
        nodes,
        links: links.into_iter().collect(),
    };

    // Print resulting graph to standard output
    if let Ok(json) = serde_json::to_string_pretty(&graph_data) {
        println!("{}", json);
    } else {
        eprintln!("Error generating graph JSON");
    }
}

// Read Go module path from go.mod file
fn read_go_module_name(root: &Path) -> Option<String> {
    let go_mod_path = root.join("go.mod");
    if go_mod_path.exists() {
        let content = fs::read_to_string(go_mod_path).ok()?;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("module ") {
                return Some(trimmed["module ".len()..].trim().to_string());
            }
        }
    }
    None
}

// JS Import resolution resolver
fn resolve_js_import(f_dir: &Path, imp: &str) -> String {
    let path = f_dir.join(imp);
    // Return relative string representation
    path.to_string_lossy().replace('\\', "/")
}

// Find existing file checking standard JS/TS/Rust/Go/Python/C++/Java/C# extensions
fn find_existing_file(path_str: &str, file_map: &HashSet<String>) -> Option<String> {
    let extensions = [
        "", ".ts", ".tsx", ".js", ".jsx", ".d.ts", 
        ".rs", ".go", ".py", ".cpp", ".cc", ".c", ".h", ".hpp", ".java", ".cs",
        "/index.ts", "/index.tsx", "/index.js"
    ];
    for ext in &extensions {
        let candidate = format!("{}{}", path_str, ext);
        // Clean up redundant "./" at start
        let cleaned = candidate.trim_start_matches("./");
        if file_map.contains(cleaned) {
            return Some(cleaned.to_string());
        }
    }
    None
}

// Python Relative Import resolution resolver
fn resolve_py_import(f_dir: &Path, imp: &str) -> String {
    // Count leading dots
    let dots = imp.chars().take_while(|&c| c == '.').count();
    let module_path = &imp[dots..].replace('.', "/");

    let mut dir = f_dir.to_path_buf();
    // For each dot after the first, navigate up
    for _ in 1..dots {
        if let Some(parent) = dir.parent() {
            dir = parent.to_path_buf();
        }
    }

    dir.join(module_path).to_string_lossy().replace('\\', "/")
}

// Find existing Python files (normal or __init__.py package index)
fn find_existing_py_file(path_str: &str, file_map: &HashSet<String>) -> Option<String> {
    let candidates = [
        format!("{}.py", path_str),
        format!("{}/__init__.py", path_str),
    ];
    for cand in &candidates {
        let cleaned = cand.trim_start_matches("./");
        if file_map.contains(cleaned) {
            return Some(cleaned.to_string());
        }
    }
    None
}
