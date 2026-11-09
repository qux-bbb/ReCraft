// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Journaled write tools: one `Edit` parsed from tool arguments and applied through the same
//! command bus as the GUI, so every mutation is undoable and attributed to `mcp`.

use std::collections::BTreeMap;

use recraft_common::Address;
use recraft_core::{Command, Effect, Session, UndoRecord};
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
        let previous = edit.previous(self.session_ref()?)?;
        let effect = self
            .session_mut()?
            .execute(edit.command.clone())
            .map_err(|e| e.to_string())?;
        self.notify_resources_updated();
        let seq = self
            .session_ref()?
            .last_journal_seq()
            .map_err(|e| e.to_string())?;
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
        let mut commands = Vec::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            let script = script_entry_or_structured(entry)
                .map_err(|error| format!("command #{}: {error}", index + 1))?;
            let command =
                recraft_core::parse_command(self.session_ref()?, &BTreeMap::new(), &script)
                    .map_err(|error| format!("command #{}: {error}", index + 1))?;
            commands.push(command);
        }
        let kinds: Vec<&str> = commands.iter().map(Command::name).collect();
        let effects = self
            .session_mut()?
            .execute_batch(&commands)
            .map_err(|e| e.to_string())?;
        self.notify_resources_updated();
        let mut dirty = Vec::new();
        for effect in &effects {
            dirty.extend(dirty_ranges(effect));
        }
        Ok(json!({
            "tool": "batch",
            "seq": self.session_ref()?.last_journal_seq().map_err(|e| e.to_string())?,
            "count": commands.len(),
            "commands": kinds,
            "dirty": dirty,
            "undo": true,
            "note": "applied as one journal batch; a single undo reverts all of it",
        }))
    }

    /// Define or replace a user struct layout (the missing half of `apply_type`).
    pub(crate) fn tool_define_struct(&mut self, arguments: &Value) -> ToolResult {
        let entry = script_entry("define_struct", arguments)?;
        let command = recraft_core::parse_command(self.session_ref()?, &BTreeMap::new(), &entry)
            .map_err(|error| error.to_string())?;
        let name = arguments
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let effect = self
            .session_mut()?
            .execute(command)
            .map_err(|error| error.to_string())?;
        self.notify_resources_updated();
        Ok(json!({
            "tool": "define_struct",
            "seq": self.session_ref()?.last_journal_seq().map_err(|e| e.to_string())?,
            "name": name,
            "dirty": dirty_ranges(&effect),
            "undo": true,
            "note": "applied to the project journal; call undo to revert",
        }))
    }

    pub(crate) fn tool_undo(&mut self, arguments: &Value) -> ToolResult {
        let seq = arguments.get("seq").and_then(Value::as_i64);
        let dry_run = arguments
            .get("dryRun")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if dry_run {
            let Some(records) = self
                .session_mut()?
                .undo_preview()
                .map_err(|e| e.to_string())?
            else {
                return Ok(json!({"undone": false, "dry_run": true, "message": "nothing to undo"}));
            };
            return Ok(json!({
                "dry_run": true,
                "would_undo": undo_records(&records),
            }));
        }
        let report = match seq {
            Some(seq) => Some(
                self.session_mut()?
                    .undo_seq(seq)
                    .map_err(|e| e.to_string())?,
            ),
            None => self
                .session_mut()?
                .undo_report()
                .map_err(|e| e.to_string())?,
        };
        if report.is_some() {
            self.notify_resources_updated();
        }
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
        // The single tools and `batch` share the batch-script grammar (`recraft-core`), so
        // there is exactly one place that knows an argument shape.
        let entry = script_entry(tool, arguments)?;
        let command = recraft_core::parse_command(server.session_ref()?, &BTreeMap::new(), &entry)
            .map_err(|error| error.to_string())?;
        let addr = edit_address(&command)?;
        Ok(Edit {
            tool: match tool {
                "rename" => "rename",
                "set_comment" => "set_comment",
                "set_prototype" => "set_prototype",
                "apply_type" => "apply_type",
                "define_function" => "define_function",
                other => return Err(format!("unknown write tool: {other}")),
            },
            addr,
            command,
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

/// Address a command edits, for the result payload.
fn edit_address(command: &Command) -> std::result::Result<Address, String> {
    Ok(match command {
        Command::Rename { addr, .. }
        | Command::SetComment { addr, .. }
        | Command::SetPrototype { addr, .. } => *addr,
        Command::ApplyType { func, .. } => *func,
        Command::DefineFunc { start, .. } => *start,
        _ => return Err("unsupported edit".to_owned()),
    })
}

/// Translate a tool call into its batch-script entry.
///
/// Numeric addresses are rendered as decimal strings so the script grammar (which accepts
/// `0x…`, decimal, `entry` and symbol names) sees one type.
fn script_entry(tool: &str, arguments: &Value) -> std::result::Result<Value, String> {
    // Numeric addresses become decimal strings so the script grammar (which accepts `0x…`,
    // decimal, `entry` and symbol names) sees one type; everything else stays as typed.
    let addr = |key: &str| -> Value {
        match arguments.get(key) {
            Some(Value::Number(number)) => Value::String(number.to_string()),
            Some(value) => value.clone(),
            None => Value::Null,
        }
    };
    let raw = |key: &str| arguments.get(key).cloned().unwrap_or(Value::Null);
    Ok(match tool {
        "rename" => json!({"type": "rename", "addr": addr("addr"), "name": raw("name")}),
        "set_comment" => json!({
            "type": "comment",
            "addr": addr("addr"),
            "text": raw("text"),
            "kind": raw("kind"),
        }),
        "set_prototype" => json!({
            "type": "prototype",
            "addr": addr("addr"),
            "text": raw("text"),
        }),
        "apply_type" => json!({
            "type": "apply_type",
            "addr": addr("func"),
            "slot": raw("slot"),
            "struct": raw("struct"),
        }),
        "define_function" => json!({
            "type": "define_function",
            "start": addr("start"),
            "end": addr("end"),
        }),
        "define_struct" => json!({
            "type": "define_struct",
            "name": raw("name"),
            "size": raw("size"),
            "fields": raw("fields"),
        }),
        other => return Err(format!("unknown write tool: {other}")),
    })
}

/// Accept either a batch-script entry (`type` + fields) or the MCP-style `{tool, arguments}`.
fn script_entry_or_structured(entry: &Value) -> std::result::Result<Value, String> {
    if entry.get("type").is_some() {
        return Ok(entry.clone());
    }
    if let Some(tool) = entry.get("tool").and_then(Value::as_str) {
        let arguments = entry.get("arguments").cloned().unwrap_or_else(|| json!({}));
        return script_entry(tool, &arguments);
    }
    Err("command needs a `type` (batch script) or `tool` (MCP style)".to_owned())
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
            | "define_struct"
            | "batch"
            | "undo"
    )
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
