use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

const PROTECTED: &[&str] = &[
    "launcher",
    "hermes.lock.json",
    "src-tauri/src/runtime.rs",
    "src-tauri/src/policy.rs",
    "src-tauri/src/paths.rs",
    "src-tauri/src/bin",
    "src-tauri/capabilities",
];

pub fn serve<R: BufRead, W: Write>(reader: R, mut writer: W, staging: PathBuf) {
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(id) = request.get("id").cloned() else {
            continue;
        };
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(Value::Null);
        let response = match method {
            "initialize" => json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "papers-guarded-builder", "version": "0.2.0" }
            }),
            "tools/list" => json!({ "tools": tool_definitions() }),
            "tools/call" => match call_tool(&staging, &params) {
                Ok(result) => result,
                Err(error) => json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": error }]
                }),
            },
            "ping" => json!({}),
            _ => {
                let frame = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": "Method not found" }
                });
                let _ = writeln!(writer, "{frame}");
                let _ = writer.flush();
                continue;
            }
        };
        let frame = json!({ "jsonrpc": "2.0", "id": id, "result": response });
        let _ = writeln!(writer, "{frame}");
        let _ = writer.flush();
    }
}

fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "papers_read_file",
            "Read a UTF-8 source file inside Papers' staging area.",
            json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"]
            }),
        ),
        tool(
            "papers_list_files",
            "List files under a directory inside Papers' staging area.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 1000 }
                },
                "required": ["path"]
            }),
        ),
        tool(
            "papers_search_files",
            "Search ordinary UTF-8 project files inside Papers' staging area.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200 }
                },
                "required": ["path", "query"]
            }),
        ),
        tool(
            "papers_write_file",
            "Create or replace an unprotected source file in a staged Papers copy.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"]
            }),
        ),
        tool(
            "papers_replace_in_file",
            "Replace one exact occurrence in an unprotected staged source file.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "old": { "type": "string" },
                    "new": { "type": "string" }
                },
                "required": ["path", "old", "new"]
            }),
        ),
        tool(
            "papers_git_diff",
            "Show the staged Papers copy's source diff without changing Git.",
            json!({
                "type": "object",
                "properties": { "worktree": { "type": "string" } },
                "required": ["worktree"]
            }),
        ),
        tool(
            "papers_run_check",
            "Run one allowlisted verification command in a staged Papers copy.",
            json!({
                "type": "object",
                "properties": {
                    "worktree": { "type": "string" },
                    "check": {
                        "type": "string",
                        "enum": ["frontend_check", "frontend_test", "frontend_build", "rust_check", "rust_test"]
                    }
                },
                "required": ["worktree", "check"]
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

fn call_tool(staging: &Path, params: &Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "A tool name is required.".to_string())?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let text = match name {
        "papers_read_file" => {
            let path = checked_existing(staging, field(&arguments, "path")?, false)?;
            fs::read_to_string(&path)
                .map_err(|error| format!("Could not read {}: {error}", path.display()))?
        }
        "papers_list_files" => {
            let root = checked_existing(staging, field(&arguments, "path")?, false)?;
            let limit = arguments
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(300)
                .min(1000) as usize;
            WalkDir::new(&root)
                .follow_links(false)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_file())
                .take(limit)
                .filter_map(|entry| {
                    entry
                        .path()
                        .strip_prefix(staging)
                        .ok()
                        .map(|path| path.to_string_lossy().into_owned())
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        "papers_search_files" => {
            let root = checked_existing(staging, field(&arguments, "path")?, false)?;
            let query = field(&arguments, "query")?;
            let limit = arguments
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(100)
                .min(200) as usize;
            let mut matches = Vec::new();
            for entry in WalkDir::new(&root)
                .follow_links(false)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_file())
            {
                if matches.len() >= limit {
                    break;
                }
                let Ok(metadata) = entry.metadata() else {
                    continue;
                };
                if metadata.len() > 2_000_000 {
                    continue;
                }
                let Ok(content) = fs::read_to_string(entry.path()) else {
                    continue;
                };
                for (line_number, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&query.to_lowercase()) {
                        let relative = entry
                            .path()
                            .strip_prefix(staging)
                            .unwrap_or(entry.path())
                            .to_string_lossy();
                        matches.push(format!("{relative}:{}: {}", line_number + 1, line.trim()));
                        if matches.len() >= limit {
                            break;
                        }
                    }
                }
            }
            matches.join("\n")
        }
        "papers_write_file" => {
            let path = checked_for_write(staging, field(&arguments, "path")?)?;
            reject_protected(staging, &path)?;
            let content = field(&arguments, "content")?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(&path, content)
                .map_err(|error| format!("Could not write {}: {error}", path.display()))?;
            format!("Wrote {}", path.display())
        }
        "papers_replace_in_file" => {
            let path = checked_existing(staging, field(&arguments, "path")?, true)?;
            reject_protected(staging, &path)?;
            let old = field(&arguments, "old")?;
            let new = field(&arguments, "new")?;
            let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
            let occurrences = content.matches(old).count();
            if occurrences != 1 {
                return Err(format!(
                    "Expected exactly one matching passage, found {occurrences}. No file was changed."
                ));
            }
            fs::write(&path, content.replacen(old, new, 1)).map_err(|error| error.to_string())?;
            format!("Updated {}", path.display())
        }
        "papers_git_diff" => {
            let worktree = checked_existing(staging, field(&arguments, "worktree")?, false)?;
            command_output("git", &["diff", "--no-ext-diff"], &worktree)?
        }
        "papers_run_check" => {
            let worktree = checked_existing(staging, field(&arguments, "worktree")?, false)?;
            let check = field(&arguments, "check")?;
            match check {
                "frontend_check" => command_output("npm.cmd", &["run", "check"], &worktree)?,
                "frontend_test" => command_output("npm.cmd", &["run", "test"], &worktree)?,
                "frontend_build" => command_output("npm.cmd", &["run", "build"], &worktree)?,
                "rust_check" => command_output(
                    "cargo.exe",
                    &["check", "--locked"],
                    &worktree.join("src-tauri"),
                )?,
                "rust_test" => command_output(
                    "cargo.exe",
                    &["test", "--locked"],
                    &worktree.join("src-tauri"),
                )?,
                _ => return Err("That verification command is not allowlisted.".to_string()),
            }
        }
        _ => return Err("That tool is not provided by Papers.".to_string()),
    };

    Ok(json!({ "content": [{ "type": "text", "text": text }] }))
}

fn field<'a>(arguments: &'a Value, name: &str) -> Result<&'a str, String> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("'{name}' is required."))
}

fn checked_existing(staging: &Path, requested: &str, file_only: bool) -> Result<PathBuf, String> {
    let staging = staging
        .canonicalize()
        .map_err(|error| format!("Could not verify staging: {error}"))?;
    let candidate = candidate_path(&staging, requested)
        .canonicalize()
        .map_err(|error| format!("Could not verify requested path: {error}"))?;
    if !candidate.starts_with(&staging) {
        return Err("Requested path is outside Papers staging.".to_string());
    }
    reject_reparse_chain(&staging, &candidate)?;
    if file_only && !candidate.is_file() {
        return Err("Requested path is not a file.".to_string());
    }
    Ok(candidate)
}

fn checked_for_write(staging: &Path, requested: &str) -> Result<PathBuf, String> {
    let staging = staging
        .canonicalize()
        .map_err(|error| format!("Could not verify staging: {error}"))?;
    let candidate = candidate_path(&staging, requested);
    let parent = candidate
        .parent()
        .ok_or_else(|| "Requested path has no parent.".to_string())?;
    let existing_parent = nearest_existing(parent)
        .canonicalize()
        .map_err(|error| format!("Could not verify requested path: {error}"))?;
    if !existing_parent.starts_with(&staging) {
        return Err("Requested path is outside Papers staging.".to_string());
    }
    reject_reparse_chain(&staging, &existing_parent)?;
    if candidate
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("Parent-directory traversal is not allowed.".to_string());
    }
    Ok(candidate)
}

fn candidate_path(staging: &Path, requested: &str) -> PathBuf {
    let path = PathBuf::from(requested);
    if path.is_absolute() {
        path
    } else {
        staging.join(path)
    }
}

fn nearest_existing(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    while !current.exists() {
        if !current.pop() {
            break;
        }
    }
    current
}

fn reject_protected(staging: &Path, path: &Path) -> Result<(), String> {
    let relative = path
        .strip_prefix(staging)
        .map_err(|_| "Requested path is outside Papers staging.".to_string())?
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    let within_change = relative
        .split_once('/')
        .map(|(_, rest)| rest)
        .unwrap_or(&relative);
    if PROTECTED.iter().any(|protected| {
        within_change == *protected || within_change.starts_with(&format!("{protected}/"))
    }) {
        return Err(
            "That file belongs to Papers' protected recovery and permission boundary. The builder cannot modify it."
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(windows)]
fn reject_reparse_chain(root: &Path, path: &Path) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    let mut current = root.to_path_buf();
    if current
        .symlink_metadata()
        .map_err(|error| error.to_string())?
        .file_attributes()
        & FILE_ATTRIBUTE_REPARSE_POINT
        != 0
    {
        return Err("The staging root cannot be a Windows reparse point.".to_string());
    }
    for component in path
        .strip_prefix(root)
        .map_err(|_| "Path escaped staging")?
        .components()
    {
        current.push(component);
        if current.exists()
            && current
                .symlink_metadata()
                .map_err(|error| error.to_string())?
                .file_attributes()
                & FILE_ATTRIBUTE_REPARSE_POINT
                != 0
        {
            return Err("Windows reparse points are not allowed inside staging.".to_string());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn reject_reparse_chain(root: &Path, path: &Path) -> Result<(), String> {
    let mut current = root.to_path_buf();
    for component in path
        .strip_prefix(root)
        .map_err(|_| "Path escaped staging")?
        .components()
    {
        current.push(component);
        if current.exists()
            && current
                .symlink_metadata()
                .map_err(|error| error.to_string())?
                .file_type()
                .is_symlink()
        {
            return Err("Symbolic links are not allowed inside staging.".to_string());
        }
    }
    Ok(())
}

fn command_output(program: &str, args: &[&str], cwd: &Path) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("Could not run {program}: {error}"))?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !output.status.success() {
        return Err(format!(
            "{program} {} stopped with {}.\n{}",
            args.join(" "),
            output.status,
            combined
        ));
    }
    Ok(combined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_boundary_matches_after_change_directory() {
        let root = PathBuf::from(r"C:\staging");
        let path = root.join("change-123").join("hermes.lock.json");
        assert!(reject_protected(&root, &path).is_err());
    }
}
