// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Instance registry tools: open a program, list what is open, pick one, save it, close it.
//!
//! An *instance* is one open project, addressed by a short id (the project file stem, made
//! unique). The server keeps one `Session` per instance, so a client can keep several samples
//! open and switch between them instead of restarting the server per binary.
//!
//! Where the database lives: `open_program` imports a binary into the per-user ReCraft folder
//! (`%LOCALAPPDATA%\ReCraft\projects` on Windows, `$XDG_DATA_HOME/recraft/projects` elsewhere,
//! `~/.local/share/recraft/projects` as the Unix fallback) unless the caller passes an explicit
//! `project` path. `save_project` checkpoints the WAL and can write a consistent copy of the
//! project file anywhere — by default into the same per-user folder.

use std::io::Read;
use std::path::{Path, PathBuf};

use recraft_common::{Error, Result};
use recraft_core::Session;
use recraft_query::hex;
use serde_json::{Value, json};

use crate::{Instance, McpServer, ToolResult};

/// Tools served by this module, in the order they are advertised.
pub(crate) const INSTANCE_TOOLS: [&str; 5] = [
    "open_program",
    "list_instances",
    "use_instance",
    "save_project",
    "close_instance",
];

/// Whether `name` is one of the registry tools.
pub(crate) fn is_instance_tool(name: &str) -> bool {
    INSTANCE_TOOLS.contains(&name)
}

impl Instance {
    /// Open a project from disk as an instance; the id comes from the file name.
    pub(crate) fn open(path: &Path) -> Result<Instance> {
        let session = Session::open(path)?;
        Ok(Instance {
            id: id_from_path(path),
            session,
        })
    }
}

impl McpServer {
    // ------------------------------------------------------- registry tools ----

    pub(crate) fn call_instance_tool(&mut self, name: &str, arguments: &Value) -> ToolResult {
        match name {
            "open_program" => self.tool_open_program(arguments),
            "list_instances" => Ok(self.tool_list_instances()),
            "use_instance" => self.tool_use_instance(arguments),
            "save_project" => self.tool_save_project(arguments),
            "close_instance" => self.tool_close_instance(arguments),
            other => Err(format!("unknown instance tool: {other}")),
        }
    }

    /// Import a binary or open an existing project and make it the active instance.
    ///
    /// Repeating the call for a project that is already open returns that instance instead of
    /// opening a second handle: a client can retry `open_program` safely.
    fn tool_open_program(&mut self, arguments: &Value) -> ToolResult {
        let Some(raw) = arguments.get("path").and_then(Value::as_str) else {
            return Err(
                "open_program needs a `path`: a PE/ELF binary or a .rcraft project".to_owned(),
            );
        };
        let source = PathBuf::from(raw);
        if !source.is_file() {
            return Err(format!("no such file: {}", source.display()));
        }
        let is_project = looks_like_project(&source);
        let project = match arguments.get("project").and_then(Value::as_str) {
            Some(path) => PathBuf::from(path),
            None if is_project => source.clone(),
            None => default_project_path(&source, self.projects_dir()),
        };
        if let Some(index) = self
            .instances
            .iter()
            .position(|instance| same_path(instance.session.path(), &project))
        {
            self.active = Some(index);
            return Ok(self.instance_payload(index, json!({"reopened": true})));
        }
        let created = !is_project && !project.exists();
        let analyze = arguments.get("analyze").and_then(Value::as_bool);
        let mut session = if created {
            import(&source, &project).map_err(|error| error.to_string())?
        } else {
            Session::open(&project)
                .map_err(|error| format!("cannot open project {}: {error}", project.display()))?
        };
        // A fresh import is always analysed; an existing project only when asked, because
        // re-running discovery on a project the user curated is not free.
        if created || analyze == Some(true) {
            analyze_session(&mut session).map_err(|error| error.to_string())?;
        }
        session.set_origin("mcp");
        // A binary is addressed by its own name (`notepad`), a project by the file it is.
        let base = if is_project {
            id_from_path(&project)
        } else {
            id_from_path(&source)
        };
        let id = self.unique_id(&base);
        self.instances.push(Instance { id, session });
        self.active = Some(self.instances.len() - 1);
        Ok(self.instance_payload(self.instances.len() - 1, json!({"imported": created})))
    }

    fn tool_list_instances(&self) -> Value {
        let instances: Vec<Value> = (0..self.instances.len())
            .map(|index| self.instance_payload(index, json!({})))
            .collect();
        json!({
            "tool": "list_instances",
            "count": instances.len(),
            "active": self.active.map(|index| self.instances[index].id.clone()),
            "instances": instances,
        })
    }

    fn tool_use_instance(&mut self, arguments: &Value) -> ToolResult {
        let Some(id) = arguments
            .get("id")
            .or_else(|| arguments.get("instance"))
            .and_then(Value::as_str)
        else {
            return Err("use_instance needs an `id` (see list_instances)".to_owned());
        };
        self.activate(id)?;
        Ok(json!({"tool": "use_instance", "active": id}))
    }

    /// Checkpoint the project database and, with a different `path`, write a copy of it.
    fn tool_save_project(&mut self, arguments: &Value) -> ToolResult {
        let index = self.active_index()?;
        let current = self.instances[index].session.path().to_path_buf();
        let target = match arguments.get("path").and_then(Value::as_str) {
            Some(path) => PathBuf::from(path),
            None => self
                .projects_dir()
                .join(current.file_name().unwrap_or_default()),
        };
        let session = &self.instances[index].session;
        session
            .project()
            .checkpoint()
            .map_err(|error| format!("cannot checkpoint {}: {error}", current.display()))?;
        let copied = !same_path(&current, &target);
        if copied {
            session
                .project()
                .copy_to(&target)
                .map_err(|error| format!("cannot save {}: {error}", target.display()))?;
        }
        let bytes = std::fs::metadata(&target)
            .map(|meta| meta.len())
            .unwrap_or(0);
        Ok(json!({
            "tool": "save_project",
            "id": self.instances[index].id,
            "path": target.display().to_string(),
            "saved": true,
            "copied": copied,
            "bytes": bytes,
            "note": if copied {
                "project checkpointed and copied; the source project is unchanged"
            } else {
                "project checkpointed (it already lives at this path)"
            },
        }))
    }

    fn tool_close_instance(&mut self, arguments: &Value) -> ToolResult {
        let index = match arguments
            .get("id")
            .or_else(|| arguments.get("instance"))
            .and_then(Value::as_str)
        {
            Some(id) => self.find_instance(id).ok_or_else(|| {
                format!("unknown instance `{id}`; call list_instances for the ids")
            })?,
            None => self.active_index()?,
        };
        let id = self.instances[index].id.clone();
        let project = self.instances[index].session.path().display().to_string();
        self.instances.remove(index);
        self.active = match self.active {
            None => None,
            Some(active) if active == index && self.instances.is_empty() => None,
            Some(active) if active == index => Some(index.min(self.instances.len() - 1)),
            Some(active) if active > index => Some(active - 1),
            Some(active) => Some(active),
        };
        Ok(json!({
            "tool": "close_instance",
            "id": id,
            "project": project,
            "closed": true,
            "remaining": self.instances.len(),
            "note": "the project file stays on disk",
        }))
    }

    // ----------------------------------------------------------- helpers ----

    /// One instance as clients see it in `list_instances` / `open_program` results.
    fn instance_payload(&self, index: usize, extra: Value) -> Value {
        let instance = &self.instances[index];
        let session = &instance.session;
        let image = session.image();
        let functions = session.functions().map(|rows| rows.len()).unwrap_or(0);
        let strings = session.strings().map(|rows| rows.len()).unwrap_or(0);
        let mut payload = json!({
            "id": instance.id,
            "active": self.active == Some(index),
            "project": session.path().display().to_string(),
            "format": session.format().meta_name(session.arch()),
            "arch": session.arch().name(),
            "bits": if session.mode().is_64() { 64 } else { 32 },
            "image_base": hex(image.image_base),
            "entry": hex(image.entry_point),
            "functions": functions,
            "strings": strings,
            "analyzed": functions > 0,
        });
        if let (Some(payload), Some(extra)) = (payload.as_object_mut(), extra.as_object()) {
            for (key, value) in extra {
                payload.insert(key.clone(), value.clone());
            }
        }
        payload
    }
}

/// Import `source` into a new project at `project` and run the analysis pass.
fn import(source: &Path, project: &Path) -> Result<Session> {
    let bytes = std::fs::read(source).map_err(|error| {
        Error::invalid_argument(format!("cannot read {}: {error}", source.display()))
    })?;
    Session::create(project, &bytes, &mut |_, _| {}, &|| false)
}

/// Function discovery, xrefs and strings — the same one-shot pass `recraft-cli analyze` runs.
fn analyze_session(session: &mut Session) -> Result<()> {
    let result = recraft_analysis::analyze(session)?;
    recraft_analysis::persist(session, &result)?;
    let strings = recraft_analysis::extract_strings(session, 4, 200_000)?;
    recraft_analysis::persist_strings(session, &strings)?;
    Ok(())
}

/// Whether `path` is a ReCraft project (SQLite) rather than an executable to import.
fn looks_like_project(path: &Path) -> bool {
    let mut magic = [0u8; 16];
    matches!(
        std::fs::File::open(path).and_then(|mut file| file.read_exact(&mut magic)),
        Ok(()) if magic.starts_with(b"SQLite format 3\0")
    )
}

/// Project file name for a binary: stable per path, so reopening the same program reuses the
/// project (and therefore the user's names and comments) instead of importing it again.
pub(crate) fn default_project_path(program: &Path, dir: &Path) -> PathBuf {
    let absolute = std::fs::canonicalize(program).unwrap_or_else(|_| program.to_path_buf());
    let stem = program
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("program");
    let stem = sanitize(stem);
    let key = path_key(&absolute);
    dir.join(format!("{stem}-{key}.rcraft"))
}

/// Per-user project folder: `%LOCALAPPDATA%\ReCraft\projects` on Windows,
/// `$XDG_DATA_HOME/recraft/projects` or `~/.local/share/recraft/projects` elsewhere.
pub(crate) fn user_project_dir() -> PathBuf {
    let base = if let Some(dir) = std::env::var_os("LOCALAPPDATA") {
        PathBuf::from(dir).join("ReCraft")
    } else if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(dir).join("recraft")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("recraft")
    } else {
        // No home directory to speak of (a service account): stay writable.
        std::env::temp_dir().join("ReCraft")
    };
    base.join("projects")
}

/// Keep the stem file-system friendly on every platform.
fn sanitize(stem: &str) -> String {
    let sanitized: String = stem
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "program".to_owned()
    } else {
        sanitized
    }
}

/// Short, stable digest of an absolute path (FNV-1a, so it does not change between runs).
fn path_key(path: &Path) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{:08x}", hash as u32)
}

/// Id for a project path: its file stem, or `project` when there is none.
fn id_from_path(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "project".to_owned())
}

fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_paths_are_stable_and_filesystem_friendly() {
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("my program (v2).exe");
        std::fs::write(&program, b"MZ").unwrap();
        let first = default_project_path(&program, dir.path());
        let second = default_project_path(&program, dir.path());
        assert_eq!(
            first, second,
            "the same binary must map to the same project"
        );
        let name = first.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("my_program__v2_"), "{name}");
        assert!(name.ends_with(".rcraft"), "{name}");

        let other = dir.path().join("other.exe");
        std::fs::write(&other, b"MZ").unwrap();
        assert_ne!(first, default_project_path(&other, dir.path()));
    }

    #[test]
    fn project_guesses_by_sqlite_magic() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("x.rcraft");
        std::fs::write(&project, b"SQLite format 3\0trailing").unwrap();
        assert!(looks_like_project(&project));
        let binary = dir.path().join("x.exe");
        std::fs::write(&binary, b"MZ\x90\0").unwrap();
        assert!(!looks_like_project(&binary));
    }
}
