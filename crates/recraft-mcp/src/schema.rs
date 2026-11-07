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
    if write {
        tools.extend(write_tools());
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
        tool["outputSchema"] = json!({"type": "object", "additionalProperties": true});
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
