// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Tool schemas advertised to MCP clients (read tools come from `recraft-query`).

use recraft_query::Budget;
use serde_json::{Value, json};

// ---------------------------------------------------------------- tools ----

pub(crate) fn tools(write: bool) -> Vec<Value> {
    // The read surface lives in `recraft-query`, shared with the in-app agent: one schema,
    // one executor, no per-front-end wording or caps to drift.
    let mut tools = recraft_query::specs(&Budget::mcp());
    tools.extend(instance_tools());
    if write {
        tools.extend(write_tools());
    }
    tools
}

/// Registry tools: which projects are open, how to add one, and how to save one.
///
/// They are always advertised, read-only server or not: opening a sample is how a client
/// gets something to read, and it never mutates an existing project.
fn instance_tools() -> Vec<Value> {
    let mut tools = vec![
        json!({
            "name": "open_program",
            "description": "Import a PE/ELF binary into a new project, or open an existing .rcraft project, and make it the active instance. A fresh import runs function discovery, xrefs and string extraction. Without `project`, the database goes to the per-user ReCraft folder (`%LOCALAPPDATA%\\ReCraft\\projects` on Windows), named after the binary so reopening the same program reuses the user's names and comments. `analyze: true` re-runs discovery on a project that is already open on disk. Calling it again for an already-open project just switches to that instance. stdio transport only.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Executable to import, or an existing .rcraft project."},
                    "project": {"type": "string", "description": "Database path to import into (must not exist); defaults to the per-user ReCraft folder."},
                    "analyze": {"type": "boolean", "description": "Re-run discovery when opening an existing project (default: only fresh imports are analysed)."},
                },
                "required": ["path"],
            },
        }),
        json!({
            "name": "list_instances",
            "description": "List the open projects: id, database path, format/arch, entry point, function and string counts, and which one is active. Every other tool accepts an optional `instance` argument with one of these ids.",
            "inputSchema": {"type": "object", "properties": {}},
        }),
        json!({
            "name": "use_instance",
            "description": "Make one open project the active instance, so later calls without an `instance` argument address it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {"type": "string", "description": "Instance id from list_instances."},
                },
                "required": ["id"],
            },
        }),
        json!({
            "name": "save_project",
            "description": "Checkpoint the project database (flush the WAL) and, when `path` names a different file, write a consistent copy of the project there (SQLite VACUUM INTO). Defaults to the per-user ReCraft folder. The path must not already exist, and the open project is never modified by the copy.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Destination .rcraft file; defaults to the per-user ReCraft folder."},
                    "instance": {"type": "string", "description": "Which open project to save (default: the active one)."},
                },
            },
        }),
        json!({
            "name": "close_instance",
            "description": "Close an open project and release its database handle. The project file stays on disk; a later open_program picks it up again. stdio transport only.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {"type": "string", "description": "Instance to close; defaults to the active one."},
                },
            },
        }),
    ];
    for tool in &mut tools {
        let name = tool["name"].as_str().unwrap_or_default().to_owned();
        let read_only = name == "list_instances";
        tool["title"] = json!(match name.as_str() {
            "open_program" => "Open program",
            "list_instances" => "List instances",
            "use_instance" => "Use instance",
            "save_project" => "Save project",
            "close_instance" => "Close instance",
            _ => "ReCraft instance tool",
        });
        tool["annotations"] = json!({
            "readOnlyHint": read_only,
            "destructiveHint": false,
            "idempotentHint": matches!(
                name.as_str(),
                "list_instances" | "use_instance" | "close_instance"
            ),
            "openWorldHint": name == "open_program",
        });
    }
    tools
}

fn write_tools() -> Vec<Value> {
    let mut tools = vec![
        json!({
            "name": "rename",
            "description": "Rename the address (function, data, import) through the journaled command bus. `name: null` clears the user name. Fails on duplicate or reserved names. Undoable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"], "description": "Address, hex string, name, or 'entry'."},
                    "name": {"type": ["string", "null"], "description": "New name; null clears it."},
                },
                "required": ["addr", "name"],
            },
        }),
        json!({
            "name": "set_comment",
            "description": "Set or clear a comment at an address. `text: null` deletes it. Undoable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"]},
                    "text": {"type": ["string", "null"]},
                    "kind": {"type": "string", "enum": ["regular", "repeatable", "anterior"], "description": "Comment slot; default regular."},
                },
                "required": ["addr", "text"],
            },
        }),
        json!({
            "name": "set_prototype",
            "description": "Set or clear the C prototype of a function. `text: null` deletes it. Undoable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"]},
                    "text": {"type": ["string", "null"], "description": "C declaration, e.g. 'int __cdecl sub_140001008(int a, char *b)'."},
                },
                "required": ["addr", "text"],
            },
        }),
        json!({
            "name": "apply_type",
            "description": "Bind a user-defined struct layout to a function argument slot (0 = first argument). `struct: null` clears the binding; define the layout first with `define_struct`. Undoable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "func": {"type": ["integer", "string"]},
                    "slot": {"type": "integer", "minimum": 0, "maximum": 255},
                    "struct": {"type": ["string", "null"]},
                },
                "required": ["func", "slot", "struct"],
            },
        }),
        json!({
            "name": "define_function",
            "description": "Define (or resize) a function at `start` with the given exclusive `end`. Undoable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "start": {"type": ["integer", "string"]},
                    "end": {"type": ["integer", "string"]},
                },
                "required": ["start", "end"],
            },
        }),
        json!({
            "name": "define_struct",
            "description": "Define or replace a user struct layout (the other half of `apply_type`). Fields are either \"offset:name:kind\" strings or {offset,name,kind} objects; offsets and size are decimal or 0x hex. Undoable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Struct name, e.g. 'HDR'."},
                    "size": {"type": "integer", "minimum": 1, "description": "Total size in bytes."},
                    "fields": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "oneOf": [
                                {"type": "string", "description": "offset:name:kind, e.g. '0:hdr:u32' (offset may be 0x hex)."},
                                {
                                    "type": "object",
                                    "properties": {
                                        "offset": {"type": "integer", "minimum": 0},
                                        "name": {"type": "string"},
                                        "kind": {"type": "string"},
                                    },
                                    "required": ["offset", "name", "kind"],
                                },
                            ],
                        },
                    },
                },
                "required": ["name", "size", "fields"],
            },
        }),
        json!({
            "name": "batch",
            "description": "Apply several commands as one journal batch: either all of them land or none does, and a single `undo` reverts the whole set. Each entry is either a batch-script command ({\"type\": \"rename\", \"addr\": ..., \"name\": ...}, same grammar as `recraft-cli batch`) or {\"tool\": ..., \"arguments\": {...}}.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "commands": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": {"type": "string", "enum": recraft_core::COMMAND_TYPES},
                                "tool": {"type": "string", "enum": ["rename", "set_comment", "set_prototype", "apply_type", "define_function", "define_struct"]},
                                "arguments": {"type": "object"},
                            },
                        },
                    },
                },
                "required": ["commands"],
            },
        }),
        json!({
            "name": "undo",
            "description": "Revert the newest journaled command (or batch) and report which records were reverted. `dryRun` shows what would be undone; `seq` requires that record to be the newest applied one.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "seq": {"type": "integer", "description": "Only undo if this is the newest applied record."},
                    "dryRun": {"type": "boolean", "description": "Report what would be undone without changing anything."},
                },
            },
        }),
    ];
    for tool in &mut tools {
        let name = tool["name"].as_str().unwrap_or_default().to_owned();
        tool["title"] = json!(write_title_of(&name));
        // Writes overwrite user data, so a client should confirm them; `batch` and `undo`
        // are not idempotent (a second undo reverts the previous command).
        tool["annotations"] = json!({
            "readOnlyHint": false,
            "destructiveHint": true,
            "idempotentHint": !matches!(name.as_str(), "batch" | "undo"),
            "openWorldHint": false,
        });
        tool["outputSchema"] = write_output_schema(&name);
    }
    tools
}

/// Human-readable label for a write tool.
fn write_title_of(name: &str) -> &'static str {
    match name {
        "rename" => "Rename",
        "set_comment" => "Set comment",
        "set_prototype" => "Set prototype",
        "apply_type" => "Apply struct type",
        "define_function" => "Define function",
        "define_struct" => "Define struct",
        "batch" => "Batch edits",
        "undo" => "Undo",
        _ => "ReCraft write tool",
    }
}

fn out_string() -> Value {
    json!({"type": "string"})
}

fn out_integer() -> Value {
    json!({"type": "integer"})
}

fn out_boolean() -> Value {
    json!({"type": "boolean"})
}

fn out_nullable_string() -> Value {
    json!({"type": ["string", "null"]})
}

fn out_string_list() -> Value {
    json!({"type": "array", "items": {"type": "string"}})
}

/// JSON Schema for one write tool's `structuredContent`.
fn write_output_schema(name: &str) -> Value {
    let with_change = |extra: Value, required: Value| {
        let mut properties = json!({
            "tool": out_string(),
            "seq": out_integer(),
            "addr": out_string(),
            "dirty": out_string_list(),
            "undo": out_boolean(),
            "note": out_string(),
        });
        for (key, value) in extra.as_object().cloned().unwrap_or_default() {
            properties[key] = value;
        }
        json!({
            "type": "object",
            "properties": properties,
            "required": required,
        })
    };
    match name {
        // Shared `change_payload` shape.
        "rename" | "set_comment" | "set_prototype" | "apply_type" => with_change(
            json!({
                "previous": out_nullable_string(),
                "value": out_nullable_string(),
            }),
            json!(["tool", "seq", "addr", "previous", "value", "dirty", "undo"]),
        ),
        "define_function" => with_change(
            json!({
                "previous_end": out_nullable_string(),
                "end": out_string(),
            }),
            json!([
                "tool",
                "seq",
                "addr",
                "previous_end",
                "end",
                "dirty",
                "undo"
            ]),
        ),
        "define_struct" => with_change(
            json!({"name": out_nullable_string()}),
            json!(["tool", "seq", "name", "dirty", "undo"]),
        ),
        "batch" => with_change(
            json!({
                "count": out_integer(),
                "commands": out_string_list(),
            }),
            json!(["tool", "seq", "count", "commands", "dirty", "undo"]),
        ),
        "undo" => {
            let record = json!({
                "seq": out_integer(),
                "cmd": out_string(),
                "origin": out_string(),
                "ts": out_integer(),
                "batch": json!({"type": ["integer", "null"]}),
            });
            json!({
                "type": "object",
                "properties": {
                    "undone": out_boolean(),
                    "records": json!({"type": "array", "items": {"type": "object", "properties": record, "required": ["seq", "cmd", "origin"]}}),
                    "dirty": out_string_list(),
                    "dry_run": out_boolean(),
                    "would_undo": json!({"type": "array", "items": {"type": "object", "properties": record.clone()}}),
                    "message": out_string(),
                },
            })
        }
        _ => json!({"type": "object", "additionalProperties": true}),
    }
}
