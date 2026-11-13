// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Resources (`recraft://...`) and prompt templates.

use recraft_core::Session;
use recraft_query::{Budget, MAX_LIMIT, hex};
use serde_json::{Value, json};

use crate::McpServer;

/// Prompt body: sample triage.
const TRIAGE_PROMPT: &str = "Produce a triage report for this sample. Use get_metadata, \
list_imports, list_exports, find_api_callers, list_strings and function_summary before \
concluding. Cover: (1) what the program appears to do; (2) network, file, registry, process, \
/proc and crypto indicators, each with the 0x address that proves it; (3) the five most suspicious \
functions, ranked, with one line of evidence each; (4) what you could not verify. Cite \
addresses as 0x... and do not modify the project.";

/// Prompt body: explain one function.
const EXPLAIN_PROMPT: &str = "1) Call function_summary for it. 2) Call get_pseudocode and read \
the whole body, paging with startLine while `truncated` is true. 3) Call xrefs_to to see how it \
is reached. Then explain: purpose, inputs/outputs, side effects, notable constants and APIs, and \
how confident you are. Cite addresses as 0x...";

/// Prompt body: propose names without applying them.
const NAMES_PROMPT: &str = "1) Call list_functions with the filter, 200 rows at a time. 2) For \
each candidate read function_summary and get_pseudocode. 3) Propose a short, specific name with \
the evidence (strings, APIs, callers) and a confidence. Do not rename anything: if this server \
runs with --write, ask for confirmation first, then apply the accepted names with a single \
`batch` call.";

/// Prompt body: CTF solving outline.
const CTF_PROMPT: &str = "Work out how to solve this challenge. Start at the entry point and \
follow the input path: use find_api_callers for ReadFile/scanf/GetDlgItemText/strcmp and \
xrefs_to on the comparison, then read the functions that matter with get_pseudocode. Outline the \
steps to the key or flag with the addresses to read or patch, and state what is unverified.";

fn prompt_description(name: &str) -> &'static str {
    match name {
        "triage_sample" => {
            "Report what the binary does, its indicators, and the most suspicious functions."
        }
        "explain_function" => "Explain one function from its pseudocode, callers and API usage.",
        "propose_names" => {
            "Propose names for sub_* functions with evidence, without renaming anything."
        }
        "ctf_outline" => "Plan a crackme/CTF solve from the entry point and the validation path.",
        _ => "ReCraft prompt",
    }
}

impl McpServer {
    // --------------------------------------------------------- resources ----

    pub(crate) fn resources(&self) -> Vec<Value> {
        let mut resources = Vec::new();
        for instance in &self.instances {
            let base = format!("recraft://project/{}", instance.id);
            for (suffix, mime) in [
                ("functions", "application/json"),
                ("strings", "application/json"),
                ("imports", "application/json"),
            ] {
                resources.push(json!({
                    "uri": format!("{base}/{suffix}"),
                    "name": suffix,
                    "mimeType": mime,
                }));
            }
        }
        resources
    }

    pub(crate) fn resource_templates(&self) -> Vec<Value> {
        self.instances
            .iter()
            .map(|instance| {
                json!({
                    "uriTemplate": format!(
                        "recraft://project/{}/pseudocode/{{func}}",
                        instance.id
                    ),
                    "name": "pseudocode",
                    "mimeType": "text/plain",
                })
            })
            .collect()
    }

    /// `resources/subscribe` / `resources/unsubscribe`.
    ///
    /// Only stdio can deliver `notifications/resources/updated`; HTTP sessions answer each
    /// POST directly and have no push channel, so they advertise `subscribe: false` and
    /// reject the call instead of pretending.
    pub(crate) fn subscribe_resource(
        &mut self,
        params: &Value,
        subscribe: bool,
    ) -> std::result::Result<Value, (i64, String)> {
        if !self.subscribe {
            return Err((
                -32601,
                "resource subscriptions need a stateful transport; serve-mcp --http does not \
                 support them (use stdio)"
                    .to_owned(),
            ));
        }
        let Some(uri) = params.get("uri").and_then(Value::as_str) else {
            return Err((-32602, "resources/subscribe requires a uri".to_owned()));
        };
        if !self.known_resource(uri) {
            return Err((-32602, format!("unknown resource uri: {uri}")));
        }
        if subscribe {
            self.subscriptions.insert(uri.to_owned());
        } else {
            self.subscriptions.remove(uri);
        }
        Ok(json!({}))
    }

    /// Whether `uri` names one of the open projects' resources.
    fn known_resource(&self, uri: &str) -> bool {
        let Some((id, rest)) = split_resource_uri(uri) else {
            return false;
        };
        self.find_instance(id).is_some()
            && (matches!(rest, "functions" | "strings" | "imports")
                || rest.starts_with("pseudocode/"))
    }

    pub(crate) fn read_resource(
        &mut self,
        params: &Value,
    ) -> std::result::Result<Value, (i64, String)> {
        let Some(uri) = params.get("uri").and_then(Value::as_str) else {
            return Err((-32602, "resources/read requires a uri".to_owned()));
        };
        let Some((id, rest)) = split_resource_uri(uri) else {
            return Err((-32602, format!("unknown resource uri: {uri}")));
        };
        // A resource URI names its instance, so reading one switches to it: the id is part
        // of the address, and a client may interleave reads across open projects.
        self.activate(id).map_err(|error| (-32602, error))?;
        let session = self.session_mut().map_err(|error| (-32603, error))?;
        let read = |session: &mut Session, name: &str, arguments: Value| {
            recraft_query::call(session, name, &arguments, &Budget::mcp())
        };
        let (mime, result) = if rest == "functions" {
            (
                "application/json",
                read(session, "list_functions", json!({"limit": MAX_LIMIT})),
            )
        } else if rest == "strings" {
            (
                "application/json",
                read(session, "list_strings", json!({"limit": MAX_LIMIT})),
            )
        } else if rest == "imports" {
            ("application/json", read(session, "list_imports", json!({})))
        } else if let Some(target) = rest.strip_prefix("pseudocode/") {
            let value = json!({"addr": target, "level": "l2"});
            ("text/plain", read(session, "get_pseudocode", value))
        } else {
            return Err((-32602, format!("unknown resource uri: {uri}")));
        };
        match result {
            Ok(value) => {
                let text = if mime == "text/plain" {
                    let mut text = value
                        .get("text")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| value.to_string());
                    // A resource has no `truncated` field to read, so keep the cut visible as a
                    // trailing pseudo-C comment instead of serving a silently short document.
                    if value.get("truncated").and_then(Value::as_bool) == Some(true) {
                        let total = value
                            .get("total_lines")
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        let end = value.get("end_line").and_then(Value::as_u64).unwrap_or(0);
                        text.push_str(&format!(
                            "// truncated: lines 1-{end} of {total}; read the rest with get_pseudocode(startLine={})\n",
                            end + 1
                        ));
                    }
                    text
                } else {
                    value.to_string()
                };
                Ok(json!({
                    "contents": [{"uri": uri, "mimeType": mime, "text": text}],
                }))
            }
            Err(message) => Err((-32603, format!("resource error: {message}"))),
        }
    }

    // ----------------------------------------------------------- prompts ----

    /// Prompt templates a client can offer as slash commands or buttons.
    pub(crate) fn prompts(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "triage_sample",
                "title": "Triage sample",
                "description": prompt_description("triage_sample"),
                "arguments": [],
            }),
            json!({
                "name": "explain_function",
                "title": "Explain function",
                "description": prompt_description("explain_function"),
                "arguments": [{
                    "name": "addr",
                    "description": "Address, symbol name or 'entry'.",
                    "required": true,
                }],
            }),
            json!({
                "name": "propose_names",
                "title": "Propose function names",
                "description": prompt_description("propose_names"),
                "arguments": [{
                    "name": "filter",
                    "description": "Name substring to restrict candidates (default: sub_).",
                    "required": false,
                }],
            }),
            json!({
                "name": "ctf_outline",
                "title": "CTF solving outline",
                "description": prompt_description("ctf_outline"),
                "arguments": [],
            }),
        ]
    }

    pub(crate) fn read_prompt(
        &mut self,
        params: &Value,
    ) -> std::result::Result<Value, (i64, String)> {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return Err((-32602, "prompts/get requires a name".to_owned()));
        };
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let text = match name {
            "triage_sample" => format!("{}\n\n{TRIAGE_PROMPT}", self.image_facts()),
            "ctf_outline" => format!("{}\n\n{CTF_PROMPT}", self.image_facts()),
            "explain_function" => {
                let Some(addr) = arguments.get("addr").and_then(Value::as_str) else {
                    return Err((
                        -32602,
                        "explain_function needs the `addr` argument".to_owned(),
                    ));
                };
                format!(
                    "Explain the function at {addr} in this ReCraft project.\n\n{EXPLAIN_PROMPT}"
                )
            }
            "propose_names" => {
                let filter = arguments
                    .get("filter")
                    .and_then(Value::as_str)
                    .unwrap_or("sub_");
                format!(
                    "Propose names for the functions whose name contains `{filter}`.\n\n{NAMES_PROMPT}"
                )
            }
            other => return Err((-32602, format!("unknown prompt: {other}"))),
        };
        Ok(json!({
            "description": prompt_description(name),
            "messages": [{"role": "user", "content": {"type": "text", "text": text}}],
        }))
    }

    /// One-line image summary embedded in the prompts, so the model starts with facts
    /// instead of discovering the image format on its first tool call.
    pub(crate) fn image_facts(&self) -> String {
        let Ok(session) = self.session_ref() else {
            return "No project is open yet: call open_program with a binary or a .rcraft project."
                .to_owned();
        };
        let image = session.image();
        let functions = session.functions().map(|rows| rows.len()).unwrap_or(0);
        let strings = session.strings().map(|rows| rows.len()).unwrap_or(0);
        let imports: usize = image
            .imports
            .iter()
            .map(|module| module.functions.len())
            .sum();
        format!(
            "Image: {} ({}), base {}, entry {} ({}), {} functions, {} imports, {} exports, {} strings.",
            session.format().name(),
            session.arch().name(),
            hex(image.image_base),
            hex(image.entry_point),
            session
                .name_at(image.entry_point)
                .ok()
                .flatten()
                .unwrap_or_else(|| "-".to_owned()),
            functions,
            imports,
            image.exports.len(),
            strings,
        )
    }
}

/// Split `recraft://project/{id}/{rest}` into the instance id and the resource path.
fn split_resource_uri(uri: &str) -> Option<(&str, &str)> {
    let rest = uri.strip_prefix("recraft://project/")?;
    let (id, rest) = rest.split_once('/')?;
    (!id.is_empty() && !rest.is_empty()).then_some((id, rest))
}
