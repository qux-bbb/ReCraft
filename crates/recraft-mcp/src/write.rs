// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Journaled write tools: one `Edit` parsed from tool arguments and applied through the same
//! command bus as the GUI, so every mutation is undoable and attributed to `mcp`.

use recraft_common::{Address, Result};
use recraft_core::{Command, CommentKind, Effect, Session, UndoRecord};
use recraft_query::hex;
use serde_json::{Value, json};

use crate::{McpServer, ToolResult};

impl McpServer {
    // ------------------------------------------------------- write tools ----
    //
    // Registered only with `--write`. Each tool resolves its arguments, builds one
    // `Command`, and applies it through `Session::execute`, which journals the inverse
    // in the same transaction as the change. The result reports the previous value and
    // the dirty ranges, so the caller can verify the change or call `undo`.

    pub(crate) fn tool_rename(&mut self, arguments: &Value) -> ToolResult {
        let edit = Edit::parse(self, "rename", arguments)?;
        self.apply_edit(edit)
    }

    pub(crate) fn tool_set_comment(&mut self, arguments: &Value) -> ToolResult {
        let edit = Edit::parse(self, "set_comment", arguments)?;
        self.apply_edit(edit)
    }

    pub(crate) fn tool_set_prototype(&mut self, arguments: &Value) -> ToolResult {
        let edit = Edit::parse(self, "set_prototype", arguments)?;
        self.apply_edit(edit)
    }

    pub(crate) fn tool_apply_type(&mut self, arguments: &Value) -> ToolResult {
        let edit = Edit::parse(self, "apply_type", arguments)?;
        self.apply_edit(edit)
    }

    pub(crate) fn tool_define_function(&mut self, arguments: &Value) -> ToolResult {
        let edit = Edit::parse(self, "define_function", arguments)?;
        self.apply_edit(edit)
    }

    /// Apply one parsed edit and report the previous value, the new value and dirty ranges.
    pub(crate) fn apply_edit(&mut self, edit: Edit) -> ToolResult {
        let previous = edit.previous(&self.session)?;
        let effect = self
            .session
            .execute(edit.command.clone())
            .map_err(|e| e.to_string())?;
        let seq = self.session.last_journal_seq().map_err(|e| e.to_string())?;
        if let Command::DefineFunc { start, end } = &edit.command {
            return Ok(json!({
                "tool": edit.tool,
                "seq": seq,
                "addr": hex(*start),
                "previous_end": previous,
                "end": hex(*end),
                "dirty": dirty_ranges(&effect),
                "undo": true,
                "note": "applied to the project journal; call undo to revert",
            }));
        }
        Ok(change_payload(
            edit.tool,
            seq,
            edit.addr,
            previous,
            edit.value(),
            &effect,
        ))
    }

    /// Apply several edits as one journal batch (all or nothing, reverted by one `undo`).
    pub(crate) fn tool_batch(&mut self, arguments: &Value) -> ToolResult {
        let Some(entries) = arguments.get("commands").and_then(Value::as_array) else {
            return Err("batch needs a `commands` array".to_owned());
        };
        if entries.is_empty() {
            return Err("batch has no commands".to_owned());
        }
        let mut edits = Vec::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            let tool = entry
                .get("tool")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("command #{}: needs a string `tool`", index + 1))?;
            let arguments = entry.get("arguments").cloned().unwrap_or_else(|| json!({}));
            let edit = Edit::parse(self, tool, &arguments)
                .map_err(|error| format!("command #{} ({tool}): {error}", index + 1))?;
            edits.push(edit);
        }
        let commands: Vec<Command> = edits.iter().map(|edit| edit.command.clone()).collect();
        let effects = self
            .session
            .execute_batch(&commands)
            .map_err(|e| e.to_string())?;
        let mut dirty = Vec::new();
        for effect in &effects {
            dirty.extend(dirty_ranges(effect));
        }
        Ok(json!({
            "tool": "batch",
            "seq": self.session.last_journal_seq().map_err(|e| e.to_string())?,
            "count": edits.len(),
            "commands": edits.iter().map(|edit| edit.tool).collect::<Vec<_>>(),
            "dirty": dirty,
            "undo": true,
            "note": "applied as one journal batch; a single undo reverts all of it",
        }))
    }

    pub(crate) fn tool_undo(&mut self, arguments: &Value) -> ToolResult {
        let seq = arguments.get("seq").and_then(Value::as_i64);
        let dry_run = arguments
            .get("dryRun")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if dry_run {
            let Some(records) = self.session.undo_preview().map_err(|e| e.to_string())? else {
                return Ok(json!({"undone": false, "dry_run": true, "message": "nothing to undo"}));
            };
            return Ok(json!({
                "dry_run": true,
                "would_undo": undo_records(&records),
            }));
        }
        let report = match seq {
            Some(seq) => Some(self.session.undo_seq(seq).map_err(|e| e.to_string())?),
            None => self.session.undo_report().map_err(|e| e.to_string())?,
        };
        Ok(match report {
            Some(report) => json!({
                "undone": true,
                "records": undo_records(&report.records),
                "dirty": dirty_ranges(&report.effect),
            }),
            None => json!({
                "undone": false,
                "message": "nothing to undo",
            }),
        })
    }

    // ----------------------------------------------------------- helpers ----

    fn resolve_address(&self, value: Option<&Value>) -> Result<Address> {
        // Address syntax is part of the shared query contract, not of the transport.
        recraft_query::resolve_address(&self.session, value)
    }
}

pub(crate) struct Edit {
    tool: &'static str,
    addr: Address,
    command: Command,
}

impl Edit {
    fn parse(
        server: &McpServer,
        tool: &str,
        arguments: &Value,
    ) -> std::result::Result<Edit, String> {
        let resolve =
            |value: Option<&Value>| server.resolve_address(value).map_err(|e| e.to_string());
        Ok(match tool {
            "rename" => {
                let addr = resolve(arguments.get("addr"))?;
                let name = optional_text(arguments, "name")?;
                Edit {
                    tool: "rename",
                    addr,
                    command: Command::Rename { addr, name },
                }
            }
            "set_comment" => {
                let addr = resolve(arguments.get("addr"))?;
                let kind = comment_kind(arguments)?;
                let text = optional_text(arguments, "text")?;
                Edit {
                    tool: "set_comment",
                    addr,
                    command: Command::SetComment { addr, kind, text },
                }
            }
            "set_prototype" => {
                let addr = resolve(arguments.get("addr"))?;
                let text = optional_text(arguments, "text")?;
                Edit {
                    tool: "set_prototype",
                    addr,
                    command: Command::SetPrototype { addr, text },
                }
            }
            "apply_type" => {
                let func = resolve(arguments.get("func"))?;
                let slot = arguments
                    .get("slot")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| "apply_type requires `slot` (0 = first argument)".to_owned())?;
                let slot = u8::try_from(slot)
                    .map_err(|_| format!("slot {slot} is out of range (0-255)"))?;
                let struct_name = optional_text(arguments, "struct")?;
                Edit {
                    tool: "apply_type",
                    addr: func,
                    command: Command::ApplyType {
                        func,
                        slot,
                        struct_name,
                    },
                }
            }
            "define_function" => {
                let start = resolve(arguments.get("start"))?;
                let end = resolve(arguments.get("end"))?;
                if end <= start {
                    return Err(format!(
                        "define_function needs end > start ({} >= {})",
                        hex(end),
                        hex(start)
                    ));
                }
                Edit {
                    tool: "define_function",
                    addr: start,
                    command: Command::DefineFunc { start, end },
                }
            }
            other => return Err(format!("unknown write tool: {other}")),
        })
    }

    /// Value of the field this edit overwrites, formatted for the result payload.
    fn previous(&self, session: &Session) -> std::result::Result<Option<String>, String> {
        let value = match &self.command {
            Command::Rename { addr, .. } => session.name_at(*addr),
            Command::SetComment { addr, kind, .. } => session.comment_at(*addr, *kind),
            Command::SetPrototype { addr, .. } => session.prototype_at(*addr),
            Command::ApplyType { func, slot, .. } => session.arg_type(*func, *slot),
            Command::DefineFunc { start, .. } => session.func_at(*start).map(|end| end.map(hex)),
            _ => return Err("unsupported edit".to_owned()),
        };
        value.map_err(|error| error.to_string())
    }

    /// Value the edit writes, formatted for the result payload.
    fn value(&self) -> Option<String> {
        match &self.command {
            Command::Rename { name, .. } => name.clone(),
            Command::SetComment { text, .. } => text.clone(),
            Command::SetPrototype { text, .. } => text.clone(),
            Command::ApplyType { struct_name, .. } => struct_name.clone(),
            _ => None,
        }
    }
}

/// Journal records reverted by one undo, as they appear in a tool result.
fn undo_records(records: &[UndoRecord]) -> Vec<Value> {
    records
        .iter()
        .map(|record| {
            json!({
                "seq": record.seq,
                "cmd": record.cmd,
                "origin": record.origin,
                "ts": record.ts,
                "batch": record.batch,
            })
        })
        .collect()
}

pub(crate) fn is_write_tool(name: &str) -> bool {
    matches!(
        name,
        "rename"
            | "set_comment"
            | "set_prototype"
            | "apply_type"
            | "define_function"
            | "batch"
            | "undo"
    )
}

/// Read an optional string argument: missing is an error, `null` means "clear".
fn optional_text(arguments: &Value, key: &str) -> std::result::Result<Option<String>, String> {
    match arguments.get(key) {
        None => Err(format!("missing `{key}` (a string, or null to clear)")),
        Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(format!("`{key}` must be a string or null")),
    }
}

fn comment_kind(arguments: &Value) -> std::result::Result<CommentKind, String> {
    match arguments
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("regular")
    {
        "regular" => Ok(CommentKind::Regular),
        "repeatable" => Ok(CommentKind::Repeatable),
        "anterior" => Ok(CommentKind::Anterior),
        other => Err(format!(
            "unknown comment kind {other:?}; use regular, repeatable or anterior"
        )),
    }
}

/// Format the ranges a command invalidated, e.g. `["0x140001008..+1"]`.
fn dirty_ranges(effect: &Effect) -> Vec<String> {
    effect
        .dirty
        .iter()
        .map(|range| format!("{}..+{}", hex(range.start), range.len()))
        .collect()
}

/// Shared shape of a single write result: previous value, new value, dirty ranges.
fn change_payload(
    tool: &str,
    seq: i64,
    addr: Address,
    previous: Option<String>,
    value: Option<String>,
    effect: &Effect,
) -> Value {
    json!({
        "tool": tool,
        "seq": seq,
        "addr": hex(addr),
        "previous": previous,
        "value": value,
        "dirty": dirty_ranges(effect),
        "undo": true,
        "note": "applied to the project journal; call undo to revert",
    })
}
