// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Implementations of the shared read tools.
//!
//! Every function here is transport-agnostic: it takes JSON arguments and returns the JSON a
//! tool result carries. The MCP server and the in-app agent both call through
//! [`crate::call`], so a query is written once.

use recraft_common::Address;
use recraft_core::Session;
use serde_json::{Value, json};

use crate::{
    Budget, FunctionIndex, XREF_CALL, XREF_DATA, cap_of, decode_string, hex, limit_of, offset_of,
    page, parse_hex_bytes, resolve_address, xref_kind_name,
};

/// Hard ceiling on rows any caller can ask for.
pub const MAX_LIMIT: usize = 5000;
/// Hard ceiling on `get_pseudocode` pages, whatever the budget says.
pub const MAX_PSEUDOCODE_LINES: usize = 100_000;
/// Hard ceiling on `get_pseudocode` bytes, whatever the budget says.
pub const MAX_PSEUDOCODE_BYTES: usize = 8 * 1024 * 1024;

/// `list_functions`: address-ordered function table, filter → offset → limit.
pub fn list_functions(
    session: &Session,
    arguments: &Value,
    budget: Budget,
) -> Result<Value, String> {
    let filter = arguments
        .get("filter")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let limit = limit_of(arguments, 500, budget);
    let offset = offset_of(arguments);
    let functions = session.functions_named().map_err(|e| e.to_string())?;
    let total_available = functions.len();
    let mut items = Vec::new();
    let mut matched = 0usize;
    for (function, name) in &functions {
        // Without a filter the scan stops at the end of the page; with one it has to reach
        // the end, because the match may sit anywhere in the address-ordered table.
        if filter.is_none() && items.len() >= limit {
            break;
        }
        if let Some(filter) = &filter
            && !name
                .as_deref()
                .map(|name| name.to_ascii_lowercase().contains(filter))
                .unwrap_or(false)
        {
            continue;
        }
        matched += 1;
        if matched <= offset {
            continue;
        }
        if items.len() < limit {
            items.push(json!({
                "name": name,
                "start": hex(function.start),
                "end": hex(function.end),
                "size": function.end.get().saturating_sub(function.start.get()),
            }));
        }
    }
    let total = if filter.is_some() {
        matched
    } else {
        total_available
    };
    let mut payload = page(items.len(), total, offset, limit)?;
    payload["functions"] = json!(items);
    Ok(payload)
}

/// `list_strings`: extracted string table with decoded text.
pub fn strings(session: &mut Session, arguments: &Value, budget: Budget) -> Result<Value, String> {
    let filter = arguments
        .get("filter")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let limit = limit_of(arguments, 500, budget);
    let offset = offset_of(arguments);
    let entries = session.strings().map_err(|e| e.to_string())?;
    let total_available = entries.len();
    let mut items = Vec::new();
    let mut matched = 0usize;
    for entry in &entries {
        if filter.is_none() && items.len() >= limit {
            break;
        }
        let text = session
            .read_bytes(entry.addr, (entry.len as usize).max(2))
            .map(|bytes| decode_string(&bytes, entry.enc))
            .unwrap_or_default();
        if let Some(filter) = &filter
            && !text.to_ascii_lowercase().contains(filter)
        {
            continue;
        }
        matched += 1;
        if matched <= offset {
            continue;
        }
        if items.len() < limit {
            items.push(json!({
                "addr": hex(entry.addr),
                "length": entry.len,
                "encoding": if entry.enc == 2 { "utf-16le" } else { "ascii" },
                "text": text,
            }));
        }
    }
    let total = if filter.is_some() {
        matched
    } else {
        total_available
    };
    let mut payload = page(items.len(), total, offset, limit)?;
    payload["strings"] = json!(items);
    Ok(payload)
}

/// `xrefs_to` / `xrefs_from`: both directions share one shape.
pub fn xrefs(
    session: &Session,
    arguments: &Value,
    budget: Budget,
    incoming: bool,
) -> Result<Value, String> {
    let addr = resolve_address(session, arguments.get("addr")).map_err(|e| e.to_string())?;
    let limit = limit_of(arguments, MAX_LIMIT, budget);
    let offset = offset_of(arguments);
    let entries = if incoming {
        session.xrefs_to(addr)
    } else {
        session.xrefs_from(addr)
    }
    .map_err(|e| e.to_string())?;
    let functions = session.functions().map_err(|e| e.to_string())?;
    let index = FunctionIndex::new(&functions);
    let total = entries.len();
    let items: Vec<Value> = entries
        .iter()
        .skip(offset)
        .take(limit)
        .map(|entry| {
            let source = if incoming { entry.from } else { addr };
            let owner = index
                .containing(source)
                .map(|function| session.name_at(index.get(function).start).ok().flatten());
            json!({
                "from": hex(entry.from),
                "to": hex(entry.to),
                "kind": xref_kind_name(entry.kind),
                "function": owner.flatten(),
            })
        })
        .collect();
    let next = offset.saturating_add(items.len());
    Ok(json!({
        "addr": hex(addr),
        "xrefs": items,
        "count": items.len(),
        "total": total,
        "offset": offset,
        "truncated": next < total,
        "next_offset": if next < total { json!(next) } else { Value::Null },
    }))
}

/// `list_exports`: the export table, paged and filterable like the other lists.
pub fn exports(session: &Session, arguments: &Value, budget: Budget) -> Result<Value, String> {
    let image = session.image();
    let base = image.image_base.get();
    let filter = arguments
        .get("filter")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let limit = limit_of(arguments, 500, budget);
    let offset = offset_of(arguments);
    let mut items = Vec::new();
    let mut matched = 0usize;
    for entry in &image.exports {
        if let Some(filter) = &filter
            && !entry
                .name
                .as_deref()
                .map(|name| name.to_ascii_lowercase().contains(filter))
                .unwrap_or(false)
        {
            continue;
        }
        matched += 1;
        if matched <= offset {
            continue;
        }
        if items.len() < limit {
            let addr = Address::new(base + u64::from(entry.rva));
            items.push(json!({
                "name": entry.name,
                "ordinal": entry.ordinal,
                "addr": hex(addr),
                "rva": format!("{:#x}", entry.rva),
                "forwarder": entry.forwarder,
                "function": session.name_at(addr).ok().flatten(),
            }));
        }
    }
    Ok(json!({
        "exports": items,
        "count": items.len(),
        "total": matched,
        "offset": offset,
        "truncated": offset.saturating_add(items.len()) < matched,
        "next_offset": if offset.saturating_add(items.len()) < matched {
            json!(offset + items.len())
        } else {
            Value::Null
        },
    }))
}

/// `get_metadata`: image-level facts every investigation starts from.
pub fn metadata(session: &Session) -> Result<Value, String> {
    let image = session.image();
    let sections: Vec<Value> = session
        .segments()
        .iter()
        .map(|segment| {
            json!({
                "name": segment.name,
                "start": hex(segment.range.start),
                "end": hex(segment.range.end),
                "size": segment.len(),
                "perm": format!("{:?}", segment.perm),
                "kind": format!("{:?}", segment.kind),
            })
        })
        .collect();
    let functions = session.functions().map_err(|e| e.to_string())?.len();
    Ok(json!({
        "format": if image.is_64 { "pe32+" } else { "pe32" },
        "arch": if image.is_64 { "x86-64" } else { "x86" },
        "image_base": hex(image.image_base),
        "entry_point": hex(image.entry_point),
        "entry_name": session.name_at(image.entry_point).ok().flatten(),
        "size_of_image": image.size_of_image,
        "size_of_headers": image.size_of_headers,
        "file_size": image.file_size,
        "subsystem": image.subsystem,
        "timestamp": image.timestamp,
        "tls_callbacks": image.tls_callbacks.iter().map(|addr| hex(*addr)).collect::<Vec<_>>(),
        "sections": sections,
        "counts": {
            "functions": functions,
            "imports": image.imports.iter().map(|module| module.functions.len()).sum::<usize>(),
            "exports": image.exports.len(),
            "strings": session.strings().map_err(|e| e.to_string())?.len(),
        },
        "warnings": image.warnings,
    }))
}

/// `list_imports`: the IAT, grouped by module.
pub fn imports(session: &Session) -> Result<Value, String> {
    let image = session.image();
    let modules: Vec<Value> = image
        .imports
        .iter()
        .map(|module| {
            let functions: Vec<Value> = module
                .functions
                .iter()
                .map(|function| {
                    json!({
                        "name": function.name,
                        "ordinal": function.ordinal,
                        "hint": function.hint,
                        "iat": hex(function.iat_va),
                    })
                })
                .collect();
            json!({
                "module": module.name,
                "iat": hex(module.iat_va),
                "functions": functions,
            })
        })
        .collect();
    Ok(json!({"modules": modules}))
}

/// `callgraph`: function-level edges derived from call xrefs.
pub fn callgraph(session: &Session, arguments: &Value, budget: Budget) -> Result<Value, String> {
    let limit = limit_of(arguments, 2000, budget);
    let offset = offset_of(arguments);
    let functions = session.functions().map_err(|e| e.to_string())?;
    let index = FunctionIndex::new(&functions);
    let calls = session
        .all_xrefs(Some(XREF_CALL))
        .map_err(|e| e.to_string())?;
    let mut pairs = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for call in calls {
        let Some(from) = index.containing(call.from) else {
            continue;
        };
        let Some(to) = index.containing(call.to) else {
            continue;
        };
        if from == to || !seen.insert((from, to)) {
            continue;
        }
        pairs.push((from, to));
    }
    let edge_total = pairs.len();
    let edges: Vec<Value> = pairs
        .iter()
        .skip(offset)
        .take(limit)
        .map(|(from, to)| {
            json!({
                "from": hex(index.get(*from).start),
                "from_name": session.name_at(index.get(*from).start).ok().flatten(),
                "to": hex(index.get(*to).start),
                "to_name": session.name_at(index.get(*to).start).ok().flatten(),
            })
        })
        .collect();
    let nodes: Vec<Value> = functions
        .iter()
        .skip(offset)
        .take(limit)
        .map(|function| {
            json!({
                "start": hex(function.start),
                "name": session.name_at(function.start).ok().flatten(),
            })
        })
        .collect();
    let next_edges = offset.saturating_add(edges.len());
    let next_nodes = offset.saturating_add(nodes.len());
    Ok(json!({
        "nodes": nodes,
        "node_total": functions.len(),
        "edges": edges,
        "edge_total": edge_total,
        "offset": offset,
        "truncated": next_edges < edge_total || next_nodes < functions.len(),
        "next_offset": if next_edges < edge_total || next_nodes < functions.len() {
            json!(offset.saturating_add(limit))
        } else {
            Value::Null
        },
    }))
}

/// `search_bytes`: hex or ASCII pattern over the readable segments.
pub fn search_bytes(
    session: &mut Session,
    arguments: &Value,
    budget: Budget,
) -> Result<Value, String> {
    let limit = limit_of(arguments, 20, budget).min(1000);
    let offset = offset_of(arguments).min(1000);
    let needle = if let Some(hex) = arguments.get("hex").and_then(Value::as_str) {
        parse_hex_bytes(hex)?
    } else if let Some(ascii) = arguments.get("ascii").and_then(Value::as_str) {
        ascii.as_bytes().to_vec()
    } else {
        return Err("search_bytes needs `hex` or `ascii`".to_owned());
    };
    if needle.is_empty() {
        return Err("empty pattern".to_owned());
    }
    // Scan `offset + limit` hits, then drop the first `offset`: the session search has no
    // offset parameter of its own.
    let hits = session
        .search_bytes(&needle, offset.saturating_add(limit))
        .map_err(|e| e.to_string())?;
    let inspected = hits.len();
    let items: Vec<Value> = hits
        .iter()
        .skip(offset)
        .map(|addr| json!({"addr": hex(*addr)}))
        .collect();
    let next = offset.saturating_add(items.len());
    Ok(json!({
        "hits": items,
        "count": items.len(),
        "offset": offset,
        // The number of hits inspected, not the number that exist.
        "total": inspected,
        "truncated": inspected >= offset.saturating_add(limit),
        "next_offset": if inspected >= offset.saturating_add(limit) { json!(next) } else { Value::Null },
    }))
}

/// `get_disassembly`: linear sweep from an address.
pub fn disassembly(session: &mut Session, arguments: &Value) -> Result<Value, String> {
    let start = resolve_address(session, arguments.get("addr")).map_err(|e| e.to_string())?;
    let count = arguments
        .get("count")
        .and_then(Value::as_u64)
        .map(|count| (count as usize).clamp(1, 4096))
        .unwrap_or(64);
    let lines = session.disasm(start, count).map_err(|e| e.to_string())?;
    let items: Vec<Value> = lines
        .iter()
        .map(|line| {
            json!({
                "addr": hex(line.addr),
                "bytes": line.bytes().iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join(" "),
                "mnemonic": line.mnemonic,
                "operands": line.operands,
                "name": line.name,
                "comment": line.comment,
            })
        })
        .collect();
    Ok(json!({"start": hex(start), "lines": items, "count": items.len()}))
}

/// `get_pseudocode`: L0/L1/L2 pseudo-C, paged by line.
///
/// Rendered text is cached in the project (`pseudocode` table) under a fingerprint of
/// everything that can change it, so re-reading a 13 000-line function is instant instead of
/// paying the decompiler again.
pub fn pseudocode(
    session: &mut Session,
    arguments: &Value,
    budget: Budget,
) -> Result<Value, String> {
    let addr = resolve_address(session, arguments.get("addr")).map_err(|e| e.to_string())?;
    // L2 is what the GUI shows and the agent reads; defaulting MCP to L1 meant an external
    // client saw a strictly worse rendering than the user.
    let level = arguments
        .get("level")
        .and_then(Value::as_str)
        .unwrap_or("l2")
        .to_ascii_lowercase();
    let functions = session.functions().map_err(|e| e.to_string())?;
    let function = functions
        .iter()
        .find(|function| addr >= function.start && addr < function.end)
        .map(|function| function.start)
        .unwrap_or(addr);
    let max_lines = cap_of(
        arguments,
        "maxLines",
        budget.pseudo_lines,
        1,
        MAX_PSEUDOCODE_LINES,
    );
    let max_bytes = cap_of(
        arguments,
        "maxBytes",
        budget.pseudo_bytes,
        1024,
        MAX_PSEUDOCODE_BYTES,
    );
    // Page through a huge function by line number: the point of the cap is that lines 2001+
    // stay reachable without asking for the whole 13 000-line function.
    let start_line = arguments
        .get("startLine")
        .and_then(Value::as_u64)
        .map(|line| (line as usize).max(1))
        .unwrap_or(1);
    let include_lines = arguments
        .get("includeLines")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    // The cache keeps the text only, so a caller that wants per-line addresses is served
    // straight from the decompiler (that is a debugging view, not the hot path).
    let engine = format!("recraft-decompile {}", recraft_decompile::VERSION);
    let fingerprint = session
        .pseudocode_fingerprint(function, &level, &engine)
        .map_err(|e| e.to_string())?;
    let mut cached_hit = false;
    let mut lines: Vec<(Option<recraft_common::Address>, String)> = Vec::new();
    if !include_lines
        && let Some(text) = session
            .cached_pseudocode(function, &fingerprint)
            .map_err(|e| e.to_string())?
    {
        cached_hit = true;
        lines.extend(text.lines().map(|line| (None, line.to_owned())));
    }
    if !cached_hit {
        let code = match level.as_str() {
            "l0" => recraft_decompile::pseudocode_l0(session, function, 2000),
            "l1" => recraft_decompile::pseudocode_l1(session, function),
            _ => recraft_decompile::pseudocode_l2(session, function),
        }
        .map_err(|e| e.to_string())?;
        lines.extend(
            code.lines
                .iter()
                .map(|line| (Some(line.addr), line.text.clone())),
        );
        if !include_lines {
            let document: String = code
                .lines
                .iter()
                .map(|line| format!("{}\n", line.text))
                .collect();
            // Best effort: a cache write must never fail the tool call.
            let _ = session.store_pseudocode(function, &fingerprint, &document);
        }
    }
    let name = session.name_at(function).map_err(|e| e.to_string())?;
    let total_lines = lines.len();
    let first = start_line.saturating_sub(1).min(total_lines);
    let mut text = String::new();
    let mut rows = Vec::new();
    let mut kept = 0usize;
    for (line_addr, line) in lines.iter().skip(first) {
        if kept >= max_lines || (kept > 0 && text.len() + line.len() + 1 > max_bytes) {
            break;
        }
        text.push_str(line);
        text.push('\n');
        kept += 1;
        if include_lines {
            rows.push(json!({
                "addr": line_addr.map(hex),
                "text": line,
            }));
        }
    }
    let end_line = first + kept;
    let truncated = end_line < total_lines;
    let mut payload = json!({
        "function": hex(function),
        "name": name,
        "level": level,
        "text": text,
        "start_line": first + 1,
        "end_line": end_line,
        "returned_lines": kept,
        "total_lines": total_lines,
        "truncated": truncated,
        "cached": cached_hit,
        "next_start_line": if truncated { json!(end_line + 1) } else { Value::Null },
    });
    if include_lines {
        payload["lines"] = json!(rows);
    }
    if truncated {
        payload["hint"] = json!(format!(
            "truncated to lines {}-{} of {}; pass startLine={} (or raise maxLines/maxBytes), or export the full text with `recraft-cli decompile --out`",
            first + 1,
            end_line,
            total_lines,
            end_line + 1
        ));
    }
    Ok(payload)
}

/// `function_summary`: one function's range, callers, calls, strings and API tags.
pub fn summary(session: &mut Session, arguments: &Value) -> Result<Value, String> {
    let addr = resolve_address(session, arguments.get("addr")).map_err(|e| e.to_string())?;
    let functions = session.functions().map_err(|e| e.to_string())?;
    let index = FunctionIndex::new(&functions);
    let Some(slot) = index.containing(addr) else {
        return Err(format!("no function contains {}", hex(addr)));
    };
    let function = *index.get(slot);
    let name = session.name_at(function.start).map_err(|e| e.to_string())?;
    let prototype = session
        .prototype_at(function.start)
        .map_err(|e| e.to_string())?;

    let incoming = session
        .xrefs_to(function.start)
        .map_err(|e| e.to_string())?;
    let callers = incoming
        .iter()
        .filter(|entry| entry.kind == XREF_CALL)
        .count();
    let mut caller_functions = Vec::new();
    let mut seen_callers = std::collections::BTreeSet::new();
    for entry in incoming.iter().filter(|entry| entry.kind == XREF_CALL) {
        let Some(slot) = index.containing(entry.from) else {
            continue;
        };
        let start = index.get(slot).start;
        if !seen_callers.insert(start.get()) {
            continue;
        }
        caller_functions.push(json!({
            "addr": hex(start),
            "name": session.name_at(start).ok().flatten(),
            "site": hex(entry.from),
        }));
        if caller_functions.len() >= 16 {
            break;
        }
    }

    let strings = session.strings().map_err(|e| e.to_string())?;
    let string_addrs: std::collections::BTreeSet<u64> =
        strings.iter().map(|entry| entry.addr.get()).collect();

    let calls = session
        .xrefs_from_range(function.start, function.end, Some(XREF_CALL))
        .map_err(|e| e.to_string())?;
    let data_refs = session
        .xrefs_from_range(function.start, function.end, Some(XREF_DATA))
        .map_err(|e| e.to_string())?;
    let mut call_targets: Vec<Value> = Vec::new();
    let mut call_sites = Vec::new();
    let mut seen_targets = std::collections::BTreeSet::new();
    for call in &calls {
        call_sites.push(call.from);
        if seen_targets.insert(call.to.get()) {
            call_targets.push(json!({
                "addr": hex(call.to),
                "name": session.name_at(call.to).ok().flatten(),
            }));
        }
    }
    let mut referenced_strings = Vec::new();
    for data in &data_refs {
        if string_addrs.contains(&data.to.get()) {
            let text = session
                .read_bytes(data.to, 64)
                .map(|bytes| decode_string(&bytes, 1))
                .unwrap_or_default();
            referenced_strings.push(json!({"addr": hex(data.to), "text": text}));
        }
    }
    let mut api_tags = Vec::new();
    for site in call_sites {
        for (rule, category) in session.api_tags_at(site).map_err(|e| e.to_string())? {
            api_tags.push(json!({
                "addr": hex(site),
                "rule": rule,
                "category": category,
            }));
        }
    }

    let mut instruction_count = 0usize;
    let mut cursor = function.start;
    let mut truncated = false;
    while cursor < function.end {
        if instruction_count >= 20_000 {
            truncated = true;
            break;
        }
        let Ok(Some(insn)) = session.insn_at(cursor) else {
            break;
        };
        instruction_count += 1;
        cursor = insn.next_addr();
    }

    Ok(json!({
        "name": name,
        "start": hex(function.start),
        "end": hex(function.end),
        "size": function.end.get().saturating_sub(function.start.get()),
        "instructions": instruction_count,
        "truncated": truncated,
        "prototype": prototype,
        "callers": callers,
        "caller_functions": caller_functions,
        "calls": call_targets,
        "strings": referenced_strings,
        "api_tags": api_tags,
        "segment": session
            .segment_containing(function.start)
            .map(|segment| segment.name.clone()),
    }))
}

/// `find_api_callers`: functions whose call sites match an imported API name.
pub fn api_callers(session: &Session, arguments: &Value, budget: Budget) -> Result<Value, String> {
    let needle = arguments
        .get("api")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| "find_api_callers needs `api`".to_owned())?
        .to_ascii_lowercase();
    let limit = limit_of(arguments, 30, budget);
    let functions = session.functions().map_err(|e| e.to_string())?;
    let index = FunctionIndex::new(&functions);
    let mut seen = std::collections::BTreeSet::new();
    let mut items = Vec::new();
    let mut matched = 0usize;
    for (site, rule, category) in session.all_api_tags().map_err(|e| e.to_string())? {
        if !rule.to_ascii_lowercase().contains(&needle) {
            continue;
        }
        let Some(slot) = index.containing(site) else {
            continue;
        };
        let start = index.get(slot).start;
        if !seen.insert(start.get()) {
            continue;
        }
        matched += 1;
        if items.len() < limit {
            items.push(json!({
                "name": session.name_at(start).ok().flatten(),
                "addr": hex(start),
                "api": rule,
                "category": category,
                "site": hex(site),
            }));
        }
    }
    Ok(json!({
        "functions": items,
        "count": items.len(),
        "total": matched,
        "truncated": matched > items.len(),
    }))
}
