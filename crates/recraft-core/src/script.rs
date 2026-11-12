// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! The batch-script grammar: one JSON command form shared by `recraft-cli batch` and the
//! MCP `batch` tool.
//!
//! An entry is `{"type": "<command>", ...fields}`. Addresses may be `0x…`, decimal, `entry`,
//! an existing symbol, or a symbol created earlier in the same script (`virtual_names`).
//!
//! ```json
//! {"commands": [
//!   {"type": "rename", "addr": "sub_140001008", "name": "parse_args"},
//!   {"type": "comment", "addr": "parse_args", "text": "reads argv", "kind": "regular"},
//!   {"type": "define_struct", "name": "HDR", "size": 16,
//!    "fields": ["0:hdr:u32", "4:code:u32"]}
//! ]}
//! ```

use std::collections::BTreeMap;

use recraft_common::{Address, Error, Result};
use serde_json::Value;

use crate::command::{Command, CommentKind, StructDef, StructField};
use crate::session::Session;

/// Command types the grammar understands, in the order they are documented.
pub const COMMAND_TYPES: [&str; 6] = [
    "rename",
    "comment",
    "prototype",
    "apply_type",
    "define_function",
    "define_struct",
];

/// Resolve `0x…`, decimal, `entry` or a symbol name.
///
/// This is the one address syntax every front end shares: CLI arguments, batch scripts and
/// the MCP/agent tools all go through it.
pub fn resolve_address(session: &Session, raw: &str) -> Result<Address> {
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("entry") {
        return Ok(session.entry_point());
    }
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16)
            .map(Address::new)
            .map_err(|_| Error::internal(format!("invalid hex address: {raw}")));
    }
    if let Some(addr) = session.addr_of_name(raw)? {
        return Ok(addr);
    }
    raw.parse::<u64>()
        .map(Address::new)
        .map_err(|_| Error::invalid_argument(format!("invalid address or unknown symbol: {raw}")))
}

/// Parse one script entry into a [`Command`].
///
/// `virtual_names` holds names created earlier in the same script; resolution prefers the
/// project's own name table and falls back to that map.
pub fn parse_command(
    session: &Session,
    virtual_names: &BTreeMap<String, Address>,
    entry: &Value,
) -> Result<Command> {
    let kind = entry
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::unsupported("command needs a string `type`"))?;
    let resolve = |raw: &str| -> Result<Address> {
        resolve_address(session, raw).or_else(|_| {
            virtual_names
                .get(raw)
                .copied()
                .ok_or_else(|| Error::internal(format!("unknown symbol: {raw}")))
        })
    };
    let addr_of = |field: &str| -> Result<Address> {
        let raw = entry
            .get(field)
            .and_then(Value::as_str)
            .ok_or_else(|| Error::unsupported(format!("{kind} needs `{field}`")))?;
        resolve(raw)
    };
    let optional_text = |field: &str| -> Result<Option<String>> {
        match entry.get(field) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(text)) if text.is_empty() => Ok(None),
            Some(Value::String(text)) => Ok(Some(text.clone())),
            Some(_) => Err(Error::unsupported(format!(
                "{kind}.{field} must be a string"
            ))),
        }
    };
    match kind {
        "rename" => {
            let addr = addr_of("addr")?;
            let name = entry
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .map(str::to_owned);
            Ok(Command::Rename { addr, name })
        }
        "comment" => {
            let addr = addr_of("addr")?;
            let comment_kind = match entry.get("kind").and_then(Value::as_str) {
                None | Some("regular") => CommentKind::Regular,
                Some("repeatable") => CommentKind::Repeatable,
                Some("anterior") => CommentKind::Anterior,
                Some("pseudo") => CommentKind::Pseudo,
                Some(other) => {
                    return Err(Error::unsupported(format!(
                        "unknown comment kind {other:?} (regular/repeatable/anterior/pseudo)"
                    )));
                }
            };
            Ok(Command::SetComment {
                addr,
                kind: comment_kind,
                text: optional_text("text")?,
            })
        }
        "prototype" => Ok(Command::SetPrototype {
            addr: addr_of("addr")?,
            text: optional_text("text")?,
        }),
        "apply_type" => {
            let func = addr_of("addr")?;
            let slot = entry
                .get("slot")
                .and_then(Value::as_u64)
                .ok_or_else(|| Error::unsupported("apply_type needs a numeric `slot`"))?;
            let slot = u8::try_from(slot)
                .map_err(|_| Error::unsupported(format!("slot {slot} does not fit in u8")))?;
            Ok(Command::ApplyType {
                func,
                slot,
                struct_name: optional_text("struct")?,
            })
        }
        "define_function" => {
            let start = addr_of("start")?;
            let end = addr_of("end")?;
            if end <= start {
                return Err(Error::unsupported(format!(
                    "define_function needs end > start ({:#x} >= {:#x})",
                    start.get(),
                    end.get()
                )));
            }
            Ok(Command::DefineFunc { start, end })
        }
        "define_struct" => {
            let name = entry
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::unsupported("define_struct needs `name`"))?;
            let size = entry
                .get("size")
                .and_then(Value::as_u64)
                .ok_or_else(|| Error::unsupported("define_struct needs a numeric `size`"))?;
            let raw_fields = entry
                .get("fields")
                .and_then(Value::as_array)
                .ok_or_else(|| Error::unsupported("define_struct needs a `fields` array"))?;
            let mut fields = Vec::with_capacity(raw_fields.len());
            for field in raw_fields {
                fields.push(parse_struct_field(field)?);
            }
            let def = StructDef {
                name: name.to_owned(),
                size,
                fields,
            }
            .validate()?;
            Ok(Command::DefineStruct { def })
        }
        other => Err(Error::unsupported(format!(
            "unknown batch command type {other:?}"
        ))),
    }
}

/// One struct field: `"0x4:name:u32"` (string form) or `{"offset":4,"name":…,"kind":…}`.
fn parse_struct_field(field: &Value) -> Result<StructField> {
    if let Some(text) = field.as_str() {
        let mut parts = text.split(':');
        let (offset, name, kind) = (
            parts.next().unwrap_or_default().trim(),
            parts.next().unwrap_or_default().trim(),
            parts.next().unwrap_or_default().trim(),
        );
        if offset.is_empty() || name.is_empty() || kind.is_empty() {
            return Err(Error::unsupported(
                "struct field string must be offset:name:kind",
            ));
        }
        let offset = offset
            .trim_start_matches("0x")
            .parse::<u64>()
            .map_err(|_| Error::unsupported(format!("invalid field offset {offset:?}")))?;
        return Ok(StructField {
            offset,
            name: name.to_owned(),
            kind: kind.to_owned(),
        });
    }
    let offset = field
        .get("offset")
        .and_then(Value::as_u64)
        .ok_or_else(|| Error::unsupported("struct field needs a numeric `offset`"))?;
    let name = field
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::unsupported("struct field needs `name`"))?;
    let kind = field
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::unsupported("struct field needs `kind`"))?;
    Ok(StructField {
        offset,
        name: name.to_owned(),
        kind: kind.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn session(dir: &TempDir) -> Option<Session> {
        let bytes = std::fs::read("C:/Windows/System32/notepad.exe").ok()?;
        let path = dir.path().join("script.rcraft");
        Some(Session::create(&path, &bytes, &mut |_, _| {}, &|| false).expect("create"))
    }

    fn parse(
        session: &Session,
        entry: Value,
        virtual_names: &BTreeMap<String, Address>,
    ) -> Result<Command> {
        parse_command(session, virtual_names, &entry)
    }

    #[test]
    fn every_documented_type_is_parsed() {
        let dir = TempDir::new().unwrap();
        let Some(session) = session(&dir) else {
            eprintln!("skipping: no Windows sample on this platform");
            return;
        };
        let empty = BTreeMap::new();
        let cases = [
            json!({"type": "rename", "addr": "entry", "name": "main_entry"}),
            json!({"type": "comment", "addr": "entry", "text": "entry", "kind": "repeatable"}),
            json!({"type": "prototype", "addr": "entry", "text": "int main(void)"}),
            json!({"type": "apply_type", "addr": "entry", "slot": 0, "struct": "HDR"}),
            json!({"type": "define_function", "start": "entry", "end": "0x140001a00"}),
            json!({"type": "define_struct", "name": "HDR", "size": 16,
                   "fields": ["0:hdr:u32", {"offset": 4, "name": "code", "kind": "u32"}]}),
        ];
        assert_eq!(
            cases.len(),
            COMMAND_TYPES.len(),
            "one case per documented type"
        );
        for entry in cases {
            let kind = entry["type"].as_str().unwrap().to_owned();
            let parsed = parse(&session, entry, &empty);
            assert!(parsed.is_ok(), "{kind} failed: {parsed:?}");
            assert!(
                COMMAND_TYPES.contains(&kind.as_str()),
                "{kind} is not in COMMAND_TYPES"
            );
        }
        assert_eq!(
            parse(
                &session,
                json!({"type": "prototype", "addr": "entry"}),
                &empty
            )
            .unwrap()
            .name(),
            "prototype"
        );
        assert_eq!(
            parse(
                &session,
                json!({"type": "comment", "addr": "entry"}),
                &empty
            )
            .unwrap()
            .name(),
            "comment"
        );
        // `pseudo` is the Decompile view's indented-comment slot.
        match parse(
            &session,
            json!({"type": "comment", "addr": "entry", "text": "note", "kind": "pseudo"}),
            &empty,
        )
        .unwrap()
        {
            Command::SetComment { kind, .. } => assert_eq!(kind, CommentKind::Pseudo),
            other => panic!("unexpected comment command: {other:?}"),
        }
        assert_eq!(
            parse(
                &session,
                json!({"type": "define_function", "start": "entry", "end": "0x140001a00"}),
                &empty
            )
            .unwrap()
            .name(),
            "define_func"
        );
    }

    #[test]
    fn virtual_names_resolve_symbols_created_earlier_in_the_script() {
        let dir = TempDir::new().unwrap();
        let Some(session) = session(&dir) else {
            eprintln!("skipping: no Windows sample on this platform");
            return;
        };
        let mut virtual_names = BTreeMap::new();
        virtual_names.insert("renamed".to_owned(), Address::new(0x1400019C0));
        let command = parse(
            &session,
            json!({"type": "comment", "addr": "renamed", "text": "hi"}),
            &virtual_names,
        )
        .expect("virtual name");
        match command {
            Command::SetComment { addr, .. } => assert_eq!(addr, Address::new(0x1400019C0)),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn bad_entries_are_rejected_with_the_offending_field() {
        let dir = TempDir::new().unwrap();
        let Some(session) = session(&dir) else {
            eprintln!("skipping: no Windows sample on this platform");
            return;
        };
        let empty = BTreeMap::new();
        let error = parse(&session, json!({"type": "rename"}), &empty).unwrap_err();
        assert!(error.to_string().contains("addr"), "{error}");
        let error = parse(&session, json!({"type": "nope"}), &empty).unwrap_err();
        assert!(
            error.to_string().contains("unknown batch command type"),
            "{error}"
        );
        let error = parse(
            &session,
            json!({"type": "define_function", "start": "entry", "end": "entry"}),
            &empty,
        )
        .unwrap_err();
        assert!(error.to_string().contains("end > start"), "{error}");
        let error = parse(
            &session,
            json!({"type": "comment", "addr": "missing_symbol"}),
            &empty,
        )
        .unwrap_err();
        assert!(error.to_string().contains("missing_symbol"), "{error}");
    }
}
