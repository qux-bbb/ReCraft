// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! ReCraft command line interface (M0: import / inspect / disassemble / edit / undo).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};
use recraft_analysis::{analyze, persist};
use recraft_common::{Address, AddressRange, Error, Result, demangle};
use recraft_core::{CodeData, Command, CommentKind, Session, StructDef, StructField};
use recraft_decompile::pseudocode_l0;
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum XrefDirection {
    To,
    From,
    Both,
}

#[derive(Parser)]
#[command(
    name = "recraft",
    version,
    about = "ReCraft — AI-native reverse engineering (pre-alpha)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum AiCommand {
    /// Explain one function from API tags, referenced strings and calls.
    Explain {
        project: PathBuf,
        /// Function address/name (defaults to the entry point).
        #[arg(long)]
        addr: Option<String>,
        #[arg(long)]
        json: bool,
        /// Send the redacted prompt to the configured LLM provider (disabled by default).
        #[arg(long)]
        llm: bool,
        /// Bypass the persistent response cache (cache lives in %LOCALAPPDATA%\ReCraft).
        #[arg(long)]
        no_cache: bool,
    },
    /// Propose names for `sub_*` functions (evidence + confidence); never applies.
    Suggest {
        project: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Write the proposals JSON to this file for review/apply.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Sample triage: API categories, suspicious functions, URLs.
    Triage {
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Apply reviewed proposals through the command bus (journaled/undoable).
    Apply {
        project: PathBuf,
        /// Proposals JSON produced by `ai suggest --out`.
        file: PathBuf,
        /// Required confirmation: proposals are never applied silently.
        #[arg(long)]
        yes: bool,
    },
    /// Print the redacted provider prompt for one function (no network I/O).
    Prompt {
        project: PathBuf,
        #[arg(long)]
        addr: Option<String>,
        /// Token budget used for truncation.
        #[arg(long, default_value_t = 1024)]
        max_tokens: usize,
        #[arg(long)]
        json: bool,
    },
    /// Show the resolved LLM provider settings (never prints secrets).
    Provider,
    /// Run the offline prompt-quality baseline against a corpus project.
    Eval {
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum Commands {
    /// Show headers, sections, imports and exports of a PE file (no project needed).
    Info {
        file: PathBuf,
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
        /// Print every imported function and module.
        #[arg(long)]
        verbose: bool,
    },
    /// Serve the project over the Model Context Protocol (read-only tools).
    ServeMcp {
        project: PathBuf,
        /// Serve JSON-RPC over HTTP POST /mcp instead of stdio.
        #[arg(long)]
        http: bool,
        /// HTTP bind address (with --http).
        #[arg(long, default_value = "127.0.0.1:8765")]
        bind: String,
        /// Bearer token for HTTP; generated and printed when omitted.
        #[arg(long)]
        token: Option<String>,
    },
    /// Execute a JSON batch script of edit commands.
    Batch {
        project: PathBuf,
        /// Script file: `{"commands": [{"type": "rename", ...}, ...]}`.
        #[arg(long)]
        script: PathBuf,
        /// Validate every command without applying it.
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    /// AI assist: offline explanation, rename proposals, triage and proposal apply.
    Ai {
        #[command(subcommand)]
        command: AiCommand,
    },
    /// Import a PE file into a new project.
    Import {
        file: PathBuf,
        #[arg(long, short = 'o')]
        out: PathBuf,
    },
    /// Disassemble a project.
    Disasm {
        project: PathBuf,
        /// Address to start at: `0x...`, decimal, a symbol name, or `entry`.
        #[arg(long)]
        addr: Option<String>,
        #[arg(long, default_value_t = 40)]
        count: usize,
        #[arg(long)]
        json: bool,
    },
    /// Render pseudo-C for a function (L0 default; `--structured` L1, `--l2` L2).
    Decompile {
        project: PathBuf,
        /// Function start: `0x...`, decimal, a symbol name, or `entry`.
        #[arg(long)]
        addr: Option<String>,
        #[arg(long, default_value_t = 200)]
        max_insns: usize,
        #[arg(long)]
        json: bool,
        /// Prefix each line with its source address.
        #[arg(long)]
        addresses: bool,
        /// Emit L1 structured pseudo-C (requires `analyze` first).
        #[arg(long)]
        structured: bool,
        /// Emit L2 structured pseudo-C with argument/stack variable names.
        #[arg(long)]
        l2: bool,
    },
    /// Run automatic analysis (function discovery) and persist it into the project.
    Analyze {
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Measure decompiler quality gates (semantic coverage / structured rate).
    Metrics {
        project: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 2000)]
        max_functions: usize,
        /// Fail (exit non-zero) when the structured rate is below this fraction.
        #[arg(long)]
        min_structured: Option<f64>,
        /// Fail (exit non-zero) when instruction coverage is below this fraction.
        #[arg(long)]
        min_coverage: Option<f64>,
    },
    /// Write or check golden pseudo-C snapshots (normalised L1 text).
    Golden {
        project: PathBuf,
        /// Snapshot directory (default: tests/golden/corpus).
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Compare instead of writing.
        #[arg(long)]
        check: bool,
    },
    /// Evaluate function discovery against exports + .pdata (ground truth).
    Eval {
        file: PathBuf,
        /// Optional MSVC linker map file: uses every function symbol as ground truth
        /// (including non-exported helpers), which makes heuristic-only functions
        /// measurable.
        #[arg(long)]
        map: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Demangle a symbol name (MSVC C++, Rust legacy and v0).
    Demangle { name: String },
    /// List strings extracted by analysis (run `analyze` first).
    Strings {
        project: PathBuf,
        /// Truncate displayed text to this many characters.
        #[arg(long, default_value_t = 80)]
        max_text: usize,
        #[arg(long)]
        json: bool,
    },
    /// Show cross-references to/from an address or symbol.
    Xrefs {
        project: PathBuf,
        #[arg(long)]
        addr: String,
        #[arg(long, value_enum, default_value_t = XrefDirection::Both)]
        direction: XrefDirection,
        #[arg(long)]
        json: bool,
    },
    /// Search mapped segments for a byte pattern or ASCII string.
    Search {
        project: PathBuf,
        /// Hex byte pattern, e.g. `4d5a` or `4d 5a`.
        #[arg(long)]
        bytes: Option<String>,
        /// ASCII string to search for.
        #[arg(long)]
        string: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Show the call graph between defined functions.
    Callgraph {
        project: PathBuf,
        /// Only show edges touching this address or symbol.
        #[arg(long)]
        addr: Option<String>,
        #[arg(long, default_value_t = 2000)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// List imported APIs with categories and reference counts.
    Imports {
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Print the control-flow graph of the function containing an address.
    Cfg {
        project: PathBuf,
        #[arg(long)]
        addr: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List defined functions.
    Funcs {
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Rename an address.
    Rename {
        project: PathBuf,
        #[arg(long)]
        addr: String,
        #[arg(long)]
        name: String,
    },
    /// Delete the name at an address.
    Unname {
        project: PathBuf,
        #[arg(long)]
        addr: String,
    },
    /// Set a comment.
    Comment {
        project: PathBuf,
        #[arg(long)]
        addr: String,
        #[arg(long)]
        text: String,
    },
    /// Delete the comment at an address.
    Uncomment {
        project: PathBuf,
        #[arg(long)]
        addr: String,
    },
    /// Mark a range as code, data or unknown.
    Mark {
        project: PathBuf,
        #[arg(long)]
        addr: String,
        #[arg(long, default_value_t = 16)]
        len: u64,
        #[arg(long = "as", value_enum)]
        kind: MarkKind,
    },
    /// Set a user prototype (C declaration) for a function; shown in pseudocode.
    Prototype {
        project: PathBuf,
        #[arg(long)]
        addr: String,
        #[arg(long)]
        text: String,
    },
    /// Delete the user prototype at a function.
    Unprototype {
        project: PathBuf,
        #[arg(long)]
        addr: String,
    },
    /// Define or replace a user struct layout (repeatable `--field offset:name:kind`).
    DefineStruct {
        project: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        size: u64,
        /// e.g. `0:count:u32`; kinds: u8/i8/u16/i16/u32/i32/u64/i64/f32/f64/ptr/char/wchar.
        #[arg(long = "field")]
        fields: Vec<String>,
    },
    /// Bind a struct layout to a function argument slot (`--slot 0` = first argument).
    ApplyType {
        project: PathBuf,
        #[arg(long)]
        addr: String,
        #[arg(long)]
        slot: u8,
        /// Struct name; omitted clears the binding.
        #[arg(long = "type")]
        type_name: Option<String>,
    },
    /// List user-defined struct layouts.
    Structs { project: PathBuf },
    /// Undo the last command.
    Undo { project: PathBuf },
    /// Redo the last undone command.
    Redo { project: PathBuf },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum MarkKind {
    Code,
    Data,
    Unknown,
}

impl MarkKind {
    fn to_code_data(self) -> CodeData {
        match self {
            MarkKind::Code => CodeData::Code,
            MarkKind::Data => CodeData::Data,
            MarkKind::Unknown => CodeData::Unknown,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Info {
            file,
            json,
            verbose,
        } => cmd_info(&file, json, verbose),
        Commands::ServeMcp {
            project,
            http,
            bind,
            token,
        } => {
            if http {
                let token = token.unwrap_or_else(generate_mcp_token);
                if token.is_empty() {
                    return Err(recraft_common::Error::internal("empty MCP token"));
                }
                eprintln!("[mcp] token: {token}");
                recraft_mcp::serve_http(&project, &bind, &token)
            } else {
                recraft_mcp::McpServer::open(&project)?.serve_stdio()
            }
        }
        Commands::Batch {
            project,
            script,
            dry_run,
            json,
        } => cmd_batch(&project, &script, dry_run, json),
        Commands::Ai { command } => cmd_ai(command),
        Commands::Import { file, out } => cmd_import(&file, &out),
        Commands::Disasm {
            project,
            addr,
            count,
            json,
        } => cmd_disasm(&project, addr.as_deref(), count, json),
        Commands::Funcs { project, json } => cmd_funcs(&project, json),
        Commands::Analyze { project, json } => cmd_analyze(&project, json),
        Commands::Metrics {
            project,
            json,
            max_functions,
            min_structured,
            min_coverage,
        } => cmd_metrics(&project, json, max_functions, min_structured, min_coverage),
        Commands::Golden {
            project,
            dir,
            check,
        } => cmd_golden(&project, dir.as_deref(), check),
        Commands::Eval { file, map, json } => cmd_eval(&file, map.as_deref(), json),
        Commands::Demangle { name } => {
            match demangle(&name) {
                Some(demangled) => println!("{demangled}"),
                None => println!("{name}"),
            }
            Ok(())
        }
        Commands::Strings {
            project,
            max_text,
            json,
        } => cmd_strings(&project, max_text, json),
        Commands::Search {
            project,
            bytes,
            string,
            limit,
            json,
        } => cmd_search(&project, bytes.as_deref(), string.as_deref(), limit, json),
        Commands::Callgraph {
            project,
            addr,
            limit,
            json,
        } => cmd_callgraph(&project, addr.as_deref(), limit, json),
        Commands::Xrefs {
            project,
            addr,
            direction,
            json,
        } => cmd_xrefs(&project, &addr, direction, json),
        Commands::Imports { project, json } => cmd_imports(&project, json),
        Commands::Cfg {
            project,
            addr,
            json,
        } => cmd_cfg(&project, addr.as_deref(), json),
        Commands::Decompile {
            project,
            addr,
            max_insns,
            json,
            addresses,
            structured,
            l2,
        } => cmd_decompile(
            &project,
            addr.as_deref(),
            max_insns,
            json,
            addresses,
            structured,
            l2,
        ),
        Commands::Rename {
            project,
            addr,
            name,
        } => cmd_rename(&project, &addr, &name),
        Commands::Unname { project, addr } => cmd_rename(&project, &addr, ""),
        Commands::Comment {
            project,
            addr,
            text,
        } => cmd_comment(&project, &addr, &text),
        Commands::Uncomment { project, addr } => cmd_comment(&project, &addr, ""),
        Commands::Mark {
            project,
            addr,
            len,
            kind,
        } => cmd_mark(&project, &addr, len, kind),
        Commands::Prototype {
            project,
            addr,
            text,
        } => cmd_prototype(&project, &addr, Some(&text)),
        Commands::Unprototype { project, addr } => cmd_prototype(&project, &addr, None),
        Commands::DefineStruct {
            project,
            name,
            size,
            fields,
        } => cmd_define_struct(&project, &name, size, &fields),
        Commands::ApplyType {
            project,
            addr,
            slot,
            type_name,
        } => cmd_apply_type(&project, &addr, slot, type_name.as_deref()),
        Commands::Structs { project } => cmd_structs(&project),
        Commands::Undo { project } => cmd_undo(&project),
        Commands::Redo { project } => cmd_redo(&project),
    }
}

// ------------------------------------------------------------------ helpers --

fn json_result(value: &Value) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|error| Error::internal(format!("serialize JSON: {error}")))?
    );
    Ok(())
}

fn resolve_address(session: &Session, raw: &str) -> Result<Address> {
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
        .map_err(|_| Error::internal(format!("invalid address or unknown symbol: {raw}")))
}

fn permission_string(section: &recraft_format::Section) -> String {
    let read = if section.is_readable() { 'R' } else { '-' };
    let write = if section.is_writable() { 'W' } else { '-' };
    let execute = if section.is_executable() { 'X' } else { '-' };
    format!("{read}{write}{execute}")
}

fn import_symbol_name(function: &recraft_format::ImportFunction) -> String {
    match (&function.name, function.ordinal) {
        (Some(name), _) => name.clone(),
        (None, Some(ordinal)) => format!("#{ordinal}"),
        (None, None) => "<unnamed>".to_owned(),
    }
}

// ------------------------------------------------------------------- info ----

fn cmd_info(file: &Path, as_json: bool, verbose: bool) -> Result<()> {
    let bytes = std::fs::read(file)?;
    let image = recraft_format::load(&bytes)?;

    if as_json {
        let sections: Vec<Value> = image
            .sections
            .iter()
            .map(|section| {
                json!({
                    "name": section.name,
                    "va": format!("{:#x}", section.va.get()),
                    "virtual_size": section.virtual_size,
                    "raw_offset": section.raw_offset,
                    "raw_size": section.raw_size,
                    "permissions": permission_string(section),
                })
            })
            .collect();
        let imports: Vec<Value> = image
            .imports
            .iter()
            .map(|module| {
                json!({
                    "name": module.name,
                    "iat": format!("{:#x}", module.iat_va.get()),
                    "delayed": module.delayed,
                    "functions": module.functions.iter().map(import_symbol_name).collect::<Vec<_>>(),
                })
            })
            .collect();
        let exports: Vec<Value> = image
            .exports
            .iter()
            .map(|export| {
                json!({
                    "name": export.name,
                    "ordinal": export.ordinal,
                    "rva": format!("{:#x}", export.rva),
                    "forwarder": export.forwarder,
                })
            })
            .collect();
        return json_result(&json!({
            "file": file.display().to_string(),
            "size": image.file_size,
            "is_64": image.is_64,
            "image_base": format!("{:#x}", image.image_base.get()),
            "entry": format!("{:#x}", image.entry_point.get()),
            "size_of_image": image.size_of_image,
            "subsystem": image.subsystem,
            "timestamp": image.timestamp,
            "overlay": image.overlay_file_offset,
            "sections": sections,
            "imports": imports,
            "exports": exports,
            "tls_callbacks": image.tls_callbacks.iter().map(|a| format!("{:#x}", a.get())).collect::<Vec<_>>(),
            "pdata_ranges": image.pdata_functions.len(),
            "warnings": image.warnings,
        }));
    }

    println!("File:    {} ({} bytes)", file.display(), image.file_size);
    println!(
        "Format:  {} ({})",
        if image.is_64 { "PE32+" } else { "PE32" },
        if image.is_64 { "x64" } else { "x86" }
    );
    println!(
        "Image:   {:#x} .. {:#x}",
        image.image_base.get(),
        image.image_base.get().saturating_add(image.size_of_image)
    );
    println!("Entry:   {:#x}", image.entry_point.get());
    if let Some(overlay) = image.overlay_file_offset {
        println!("Overlay: {overlay:#x}");
    }

    println!("Sections ({}):", image.sections.len());
    for section in &image.sections {
        println!(
            "  {:#012x} {:<10} {:>8}/{:<8} {}",
            section.va.get(),
            section.name,
            section.virtual_size,
            section.raw_size,
            permission_string(section)
        );
    }

    let function_count: usize = image
        .imports
        .iter()
        .map(|module| module.functions.len())
        .sum();
    println!(
        "Imports ({} modules, {function_count} functions):",
        image.imports.len()
    );
    let module_limit = if verbose { usize::MAX } else { 8 };
    for module in image.imports.iter().take(module_limit) {
        println!(
            "  {}{} (IAT {:#x})",
            module.name,
            if module.delayed { " [delayed]" } else { "" },
            module.iat_va.get()
        );
        let per_module_limit = if verbose { usize::MAX } else { 4 };
        for function in module.functions.iter().take(per_module_limit) {
            println!("    {}", import_symbol_name(function));
        }
        if module.functions.len() > per_module_limit {
            println!("    ... {} more", module.functions.len() - per_module_limit);
        }
    }
    if image.imports.len() > module_limit {
        println!("  ... {} more modules", image.imports.len() - module_limit);
    }

    println!("Exports: {}", image.exports.len());
    let export_limit = if verbose { usize::MAX } else { 8 };
    for export in image.exports.iter().take(export_limit) {
        println!(
            "  {:#012x} {}",
            image.image_base.get().saturating_add(u64::from(export.rva)),
            export
                .name
                .clone()
                .unwrap_or_else(|| format!("#{}", export.ordinal))
        );
    }
    if image.exports.len() > export_limit {
        println!("  ... {} more", image.exports.len() - export_limit);
    }

    println!("TLS callbacks: {}", image.tls_callbacks.len());
    println!(".pdata ranges: {}", image.pdata_functions.len());
    if !image.warnings.is_empty() {
        println!("Warnings ({}):", image.warnings.len());
        for warning in &image.warnings {
            println!("  {warning}");
        }
    }
    Ok(())
}

// ----------------------------------------------------------------- import ----

fn cmd_import(file: &Path, out: &Path) -> Result<()> {
    let bytes = std::fs::read(file)?;
    let total_bytes = bytes.len();
    let mut last_percent = u64::MAX;
    let mut progress = |done: u64, total: u64| {
        let percent = (done * 100).checked_div(total).unwrap_or(100);
        if percent != last_percent {
            last_percent = percent;
            eprint!("\rimporting {total} bytes... {percent}%");
            let _ = std::io::stderr().flush();
        }
    };
    let session = Session::create(out, &bytes, &mut progress, &|| false)?;
    eprintln!("\rimported {total_bytes} bytes");
    println!(
        "OK: {} ({} segments, entry {:#x}, x{})",
        out.display(),
        session.segments().len(),
        session.entry_point().get(),
        if session.mode().is_64() { 64 } else { 32 }
    );
    Ok(())
}

// ----------------------------------------------------------------- disasm ----

fn cmd_disasm(project: &Path, addr: Option<&str>, count: usize, as_json: bool) -> Result<()> {
    let mut session = Session::open(project)?;
    let start = match addr {
        Some(raw) => resolve_address(&session, raw)?,
        None => session.entry_point(),
    };
    let lines = session.disasm(start, count)?;

    if as_json {
        let lines: Vec<Value> = lines
            .iter()
            .map(|line| {
                json!({
                    "addr": format!("{:#x}", line.addr.get()),
                    "len": line.len,
                    "bytes": line.bytes().iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>(),
                    "mnemonic": line.mnemonic,
                    "operands": line.operands,
                    "name": line.name,
                    "comment": line.comment,
                    "code": line.flags.is_code(),
                    "data": line.flags.is_data(),
                })
            })
            .collect();
        return json_result(&json!({ "start": format!("{:#x}", start.get()), "lines": lines }));
    }

    for line in &lines {
        let bytes = line
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        let mut text = format!(
            "{:#012x}  {:<47}  {:<8} {}",
            line.addr.get(),
            bytes,
            line.mnemonic,
            line.operands
        );
        if let Some(name) = &line.name {
            text.push_str("   ; ");
            text.push_str(name);
        }
        if let Some(comment) = &line.comment {
            text.push_str("   ; ");
            text.push_str(comment);
        }
        println!("{text}");
    }
    Ok(())
}

// ---------------------------------------------------------------- analysis ----

fn cmd_analyze(project: &Path, as_json: bool) -> Result<()> {
    let mut session = Session::open(project)?;
    let started = Instant::now();
    let result = analyze(&mut session)?;
    persist(&mut session, &result)?;
    let strings = recraft_analysis::extract_strings(&mut session, 4, 200_000)?;
    recraft_analysis::persist_strings(&mut session, &strings)?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    let xref_count = result.calls.len() + result.jumps.len() + result.data_refs.len();
    let auto_names = session.auto_name_count()?;
    if as_json {
        return json_result(&json!({
            "functions": result.functions.len(),
            "auto_names": auto_names,
            "instructions": result.instructions,
            "seeds": result.seeds,
            "discovered_by_call": result.discovered_by_call,
            "call_edges": result.calls.len(),
            "jump_edges": result.jumps.len(),
            "data_refs": result.data_refs.len(),
            "api_tags": result.api_refs.len(),
            "go_functions": result.go_functions,
            "xrefs": xref_count,
            "strings": strings.len(),
            "elapsed_ms": elapsed_ms,
        }));
    }

    println!("functions:            {}", result.functions.len());
    println!(
        "  from .pdata:        {}",
        result
            .functions
            .iter()
            .filter(|f| f.source == recraft_analysis::FunctionSource::Pdata)
            .count()
    );
    println!(
        "  from exports/entry: {}",
        result
            .functions
            .iter()
            .filter(|f| matches!(
                f.source,
                recraft_analysis::FunctionSource::Export
                    | recraft_analysis::FunctionSource::Entry
                    | recraft_analysis::FunctionSource::Tls
            ))
            .count()
    );
    println!(
        "  from call targets:  {}",
        result
            .functions
            .iter()
            .filter(|f| f.source == recraft_analysis::FunctionSource::CallTarget)
            .count()
    );
    println!(
        "  from data pointers: {}",
        result
            .functions
            .iter()
            .filter(|f| f.source == recraft_analysis::FunctionSource::DataPointer)
            .count()
    );
    println!("instructions decoded: {}", result.instructions);
    println!("direct call edges:    {}", result.calls.len());
    println!("branch edges:         {}", result.jumps.len());
    println!("data references:      {}", result.data_refs.len());
    println!("api tags:             {}", result.api_refs.len());
    println!("auto names:           {auto_names}");
    if result.go_functions > 0 {
        println!("go pclntab functions: {}", result.go_functions);
    }
    println!("strings:              {}", strings.len());
    println!("elapsed:              {elapsed_ms:.1} ms");
    println!("saved to project:     {}", project.display());
    Ok(())
}

fn cmd_metrics(
    project: &Path,
    as_json: bool,
    max_functions: usize,
    min_structured: Option<f64>,
    min_coverage: Option<f64>,
) -> Result<()> {
    let mut session = Session::open(project)?;
    let report = recraft_decompile::measure(&mut session, max_functions)?;
    if as_json {
        let top: Vec<Value> = report
            .top_intrinsics(10)
            .into_iter()
            .map(|(mnemonic, count)| json!({"mnemonic": mnemonic, "count": count}))
            .collect();
        return json_result(&json!({
            "functions": report.total(),
            "structured": report.structured(),
            "structured_rate": report.structured_rate(),
            "instructions": report.instructions(),
            "intrinsic_instructions": report.intrinsic_instructions(),
            "coverage": report.coverage(),
            "skipped": report.skipped,
            "intrinsic_top": top,
        }));
    }
    println!("functions:              {}", report.total());
    println!(
        "structured:             {} ({:.1}%)",
        report.structured(),
        report.structured_rate() * 100.0
    );
    println!("instructions:           {}", report.instructions());
    println!(
        "intrinsic instructions: {} (coverage {:.1}%)",
        report.intrinsic_instructions(),
        report.coverage() * 100.0
    );
    println!(
        "goto fallback functions: {}",
        report.total().saturating_sub(report.structured())
    );
    if report.skipped > 0 {
        println!("skipped (limit):        {}", report.skipped);
    }
    let mut failures = Vec::new();
    if let Some(min) = min_structured
        && report.structured_rate() < min
    {
        failures.push(format!(
            "structured rate {:.3} < {min:.3}",
            report.structured_rate()
        ));
    }
    if let Some(min) = min_coverage
        && report.coverage() < min
    {
        failures.push(format!("coverage {:.3} < {min:.3}", report.coverage()));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::internal(format!(
            "quality gate failed: {}",
            failures.join("; ")
        )))
    }
}

fn cmd_golden(project: &Path, dir: Option<&Path>, check: bool) -> Result<()> {
    let dir = dir
        .map(Path::to_path_buf)
        .unwrap_or_else(recraft_decompile::default_golden_dir);
    let mut session = Session::open(project)?;
    if check {
        let report = recraft_decompile::check_golden(&mut session, &dir)?;
        println!("checked:    {}", report.checked);
        println!("missing:    {}", report.missing.len());
        println!("mismatched: {}", report.mismatched.len());
        for (file, _, _) in report.mismatched.iter().take(10) {
            println!("  diff: {file}");
        }
        for file in report.missing.iter().take(10) {
            println!("  missing: {file}");
        }
        if report.ok() {
            Ok(())
        } else {
            Err(Error::internal(format!(
                "golden regression: {} mismatched, {} missing in {}",
                report.mismatched.len(),
                report.missing.len(),
                dir.display()
            )))
        }
    } else {
        let written = recraft_decompile::write_golden(&mut session, &dir)?;
        println!("wrote {written} snapshots to {}", dir.display());
        Ok(())
    }
}

fn cmd_ai(command: AiCommand) -> Result<()> {
    match command {
        AiCommand::Explain {
            project,
            addr,
            json,
            llm,
            no_cache,
        } => cmd_ai_explain(&project, addr.as_deref(), json, llm, no_cache),
        AiCommand::Suggest {
            project,
            json,
            limit,
            out,
        } => cmd_ai_suggest(&project, json, limit, out.as_deref()),
        AiCommand::Triage { project, json } => cmd_ai_triage(&project, json),
        AiCommand::Apply { project, file, yes } => cmd_ai_apply(&project, &file, yes),
        AiCommand::Prompt {
            project,
            addr,
            max_tokens,
            json,
        } => cmd_ai_prompt(&project, addr.as_deref(), max_tokens, json),
        AiCommand::Provider => {
            let settings = recraft_ai::ProviderSettings::from_env();
            println!("{}", settings.describe());
            Ok(())
        }
        AiCommand::Eval { project, json } => cmd_ai_eval(&project, json),
    }
}

fn cmd_ai_eval(project: &Path, as_json: bool) -> Result<()> {
    let mut session = Session::open(project)?;
    let report = recraft_ai::evaluate_explain(&mut session, recraft_ai::CORPUS_CASES)?;
    if as_json {
        json_result(&json!({
            "eval": recraft_ai::EVAL_VERSION,
            "template": recraft_ai::PROMPT_TEMPLATE_VERSION,
            "cases": report.cases,
            "passed": report.passed,
            "failures": report.failures,
        }))?;
    } else {
        println!(
            "prompt eval {} (template {}): {}/{} passed",
            recraft_ai::EVAL_VERSION,
            recraft_ai::PROMPT_TEMPLATE_VERSION,
            report.passed,
            report.cases
        );
        for failure in &report.failures {
            println!("  FAIL {failure}");
        }
    }
    recraft_ai::ensure_eval(&report)
}

fn cmd_ai_prompt(
    project: &Path,
    addr: Option<&str>,
    max_tokens: usize,
    as_json: bool,
) -> Result<()> {
    let mut session = Session::open(project)?;
    let addr = match addr {
        Some(raw) => resolve_address(&session, raw)?,
        None => session.entry_point(),
    };
    let mut settings = recraft_ai::ProviderSettings::from_env();
    settings.max_tokens = max_tokens;
    let prompt = recraft_ai::build_explain_prompt(&mut session, addr, &settings)?;
    if as_json {
        return json_result(&json!({
            "system": prompt.system,
            "user": prompt.user,
            "estimated_tokens": prompt.estimated_tokens,
            "provider": settings.describe(),
        }));
    }
    println!("# {}", settings.describe());
    println!("# estimated tokens: {}", prompt.estimated_tokens);
    println!("--- system ---");
    println!("{}", prompt.system);
    println!("--- user ---");
    println!("{}", prompt.user);
    Ok(())
}

fn cmd_ai_explain(
    project: &Path,
    addr: Option<&str>,
    as_json: bool,
    llm: bool,
    no_cache: bool,
) -> Result<()> {
    let mut session = Session::open(project)?;
    let addr = match addr {
        Some(raw) => resolve_address(&session, raw)?,
        None => session.entry_point(),
    };
    if llm {
        let settings = recraft_ai::ProviderSettings::from_env();
        let mut provider = recraft_ai::provider_from_settings(&settings);
        let prompt = recraft_ai::build_explain_prompt(&mut session, addr, &settings)?;
        let template = recraft_ai::PROMPT_TEMPLATE_VERSION;
        let cache_key =
            recraft_ai::AiCache::key(&prompt, provider.name(), &settings.model, template);
        let mut cache = match recraft_ai::AiCache::open_default() {
            Ok(cache) => Some(cache),
            Err(error) => {
                eprintln!("warning: ai cache unavailable: {error}");
                None
            }
        };
        let cached_entry = if no_cache {
            None
        } else {
            cache
                .as_ref()
                .and_then(|cache| cache.get(&cache_key))
                .cloned()
        };
        let (response, cached) = match cached_entry {
            Some(entry) => (entry.response, true),
            None => {
                let response = provider.complete(&prompt)?;
                if let Some(cache) = cache.as_mut() {
                    cache.insert(
                        cache_key,
                        recraft_ai::CacheEntry {
                            response: response.clone(),
                            provider: provider.name().to_owned(),
                            model: settings.model.clone(),
                            template: template.to_owned(),
                            created_unix: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|duration| duration.as_secs())
                                .unwrap_or(0),
                        },
                    );
                    if let Err(error) = cache.save() {
                        eprintln!("warning: could not save ai cache: {error}");
                    }
                }
                (response, false)
            }
        };
        let audit = recraft_ai::AuditRecord {
            provider: provider.name().to_owned(),
            model: settings.model.clone(),
            template: template.to_owned(),
            function: Some(addr),
            prompt_tokens: prompt.estimated_tokens,
            response_tokens: recraft_ai::TokenBudget::estimate(&response),
            cached,
        };
        if let Err(error) = recraft_ai::append_default_audit(&audit) {
            eprintln!("warning: could not append ai audit: {error}");
        }
        if as_json {
            return json_result(&json!({
                "provider": provider.name(),
                "model": settings.model,
                "template": template,
                "estimated_tokens": prompt.estimated_tokens,
                "cached": cached,
                "response": response,
            }));
        }
        println!(
            "# provider={} model={} template={} cached={} tokens≈{}",
            provider.name(),
            settings.model,
            template,
            if cached { "yes" } else { "no" },
            prompt.estimated_tokens
        );
        println!("{response}");
        return Ok(());
    }
    let explanation = recraft_ai::explain(&mut session, addr)?;
    if as_json {
        return json_result(&json!({
            "name": explanation.name,
            "start": format!("{:#x}", explanation.start.get()),
            "end": format!("{:#x}", explanation.end.get()),
            "instructions": explanation.instructions,
            "summary": explanation.summary,
            "apis": explanation.apis.iter().map(|(site, rule, category)| json!({
                "site": format!("{:#x}", site.get()),
                "rule": rule,
                "category": category,
            })).collect::<Vec<_>>(),
            "strings": explanation.strings.iter().map(|(addr, text)| json!({
                "addr": format!("{:#x}", addr.get()),
                "text": text,
            })).collect::<Vec<_>>(),
            "calls": explanation.calls.iter().map(|(site, name)| json!({
                "site": format!("{:#x}", site.get()),
                "name": name,
            })).collect::<Vec<_>>(),
        }));
    }
    println!("{}", explanation.summary);
    for (site, rule, category) in &explanation.apis {
        println!("  api  {:#012x}  {rule} [{category}]", site.get());
    }
    for (addr, text) in &explanation.strings {
        println!("  str  {:#012x}  {text}", addr.get());
    }
    for (site, name) in &explanation.calls {
        println!("  call {:#012x}  {name}", site.get());
    }
    Ok(())
}

fn cmd_ai_suggest(project: &Path, as_json: bool, limit: usize, out: Option<&Path>) -> Result<()> {
    let mut session = Session::open(project)?;
    let proposals = recraft_ai::suggest_names(&mut session, limit)?;
    let values: Vec<Value> = proposals
        .iter()
        .map(|proposal| proposal.to_json())
        .collect();
    if let Some(out) = out {
        let text = serde_json::to_string_pretty(&values)
            .map_err(|error| Error::internal(format!("proposal JSON: {error}")))?;
        std::fs::write(out, text)?;
        println!("wrote {} proposals to {}", values.len(), out.display());
        return Ok(());
    }
    if as_json {
        return json_result(&json!({ "proposals": values, "count": values.len() }));
    }
    println!("{} proposal(s); review before applying:", values.len());
    for proposal in &proposals {
        println!(
            "  {:.2}  {:#012x} -> {}",
            proposal.confidence,
            proposal.target.get(),
            proposal.value
        );
        println!("        {}", proposal.rationale);
        for evidence in proposal.evidence.iter().take(3) {
            println!(
                "        evidence {:#012x} {}: {}",
                evidence.addr.get(),
                evidence.kind,
                evidence.text
            );
        }
    }
    Ok(())
}

fn cmd_ai_triage(project: &Path, as_json: bool) -> Result<()> {
    let mut session = Session::open(project)?;
    let triage = recraft_ai::triage(&mut session)?;
    if as_json {
        return json_result(&json!({
            "functions": triage.functions,
            "instructions": triage.instructions,
            "strings": triage.strings,
            "apis_by_category": triage.apis_by_category,
            "suspicious": triage.suspicious.iter().map(|item| json!({
                "start": format!("{:#x}", item.start.get()),
                "name": item.name,
                "categories": item.categories,
                "score": item.score,
                "apis": item.apis,
            })).collect::<Vec<_>>(),
            "urls": triage.urls,
        }));
    }
    println!(
        "functions: {}  instructions: {}  strings: {}",
        triage.functions, triage.instructions, triage.strings
    );
    println!("api categories:");
    for (category, count) in &triage.apis_by_category {
        println!("  {category:16} {count}");
    }
    println!("suspicious functions:");
    for item in &triage.suspicious {
        println!(
            "  {:.1}  {:#012x}  {}  [{}]  {}",
            item.score,
            item.start.get(),
            item.name.as_deref().unwrap_or("?"),
            item.categories.join(","),
            item.apis.join(",")
        );
    }
    if !triage.urls.is_empty() {
        println!("urls:");
        for url in &triage.urls {
            println!("  {url}");
        }
    }
    Ok(())
}

fn cmd_ai_apply(project: &Path, file: &Path, yes: bool) -> Result<()> {
    if !yes {
        return Err(Error::unsupported(
            "refusing to apply proposals without --yes; review the JSON diff first",
        ));
    }
    let text = std::fs::read_to_string(file)?;
    let values: Value = serde_json::from_str(&text)
        .map_err(|error| Error::internal(format!("proposal JSON: {error}")))?;
    let items = values
        .as_array()
        .cloned()
        .or_else(|| values.get("proposals").and_then(Value::as_array).cloned())
        .ok_or_else(|| Error::internal("proposal file must be a JSON array"))?;
    let mut proposals = Vec::with_capacity(items.len());
    for item in &items {
        proposals.push(recraft_ai::Proposal::from_json(item)?);
    }
    let mut session = Session::open(project)?;
    let applied = recraft_ai::apply_proposals(&mut session, &proposals)?;
    println!("applied {applied} proposal(s) through the command bus (undoable)");
    for proposal in &proposals {
        println!("  {:#012x} -> {}", proposal.target.get(), proposal.value);
    }
    Ok(())
}

fn cmd_imports(project: &Path, as_json: bool) -> Result<()> {
    let session = Session::open(project)?;
    let image = session.image().clone();
    let mut rows: Vec<(String, String, &'static str, u64, Address)> = Vec::new();
    for module in &image.imports {
        for function in &module.functions {
            let api = match (&function.name, function.ordinal) {
                (Some(name), _) if !name.is_empty() => name.clone(),
                (_, Some(ordinal)) => format!("#{ordinal}"),
                _ => continue,
            };
            let category = recraft_analysis::categorize(&api);
            let refs = if function.iat_va.get() != 0 {
                session.xrefs_to(function.iat_va)?.len() as u64
            } else {
                0
            };
            rows.push((module.name.clone(), api, category, refs, function.iat_va));
        }
    }
    rows.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));

    if as_json {
        let values: Vec<Value> = rows
            .iter()
            .map(|(module, api, category, refs, iat)| {
                json!({
                    "module": module,
                    "api": api,
                    "category": category,
                    "refs": refs,
                    "iat": format!("{:#x}", iat.get()),
                })
            })
            .collect();
        return json_result(&Value::Array(values));
    }

    println!("{} imported APIs", rows.len());
    let mut roles = std::collections::BTreeMap::new();
    for (_, _, category, _, _) in &rows {
        *roles.entry(*category).or_insert(0usize) += 1;
    }
    for (category, count) in &roles {
        println!("  {category:<15} {count}");
    }
    for (module, api, category, refs, iat) in &rows {
        println!(
            "  {:#012x} {module:<20} {api:<32} {category:<15} refs {refs}",
            iat.get()
        );
    }
    Ok(())
}

fn cmd_strings(project: &Path, max_text: usize, as_json: bool) -> Result<()> {
    let mut session = Session::open(project)?;
    let strings = session.strings()?;
    if strings.is_empty() {
        if as_json {
            return json_result(&Value::Array(Vec::new()));
        }
        println!("no strings in this project - run `recraft analyze` first");
        return Ok(());
    }
    let previews: Vec<String> = strings
        .iter()
        .map(|string| truncate_string(&mut session, string.addr, string.len, max_text))
        .collect();
    if as_json {
        let rows: Vec<Value> = strings
            .iter()
            .zip(previews.iter())
            .map(|(string, text)| {
                json!({
                    "addr": format!("{:#x}", string.addr.get()),
                    "len": string.len,
                    "enc": string.enc,
                    "text": text,
                })
            })
            .collect();
        return json_result(&Value::Array(rows));
    }
    println!("{} strings", strings.len());
    for (string, text) in strings.iter().zip(previews.iter()) {
        let enc = if string.enc == recraft_analysis::STRING_ENC_UTF16LE {
            "utf16"
        } else {
            "ascii"
        };
        println!(
            "{:#012x}  {:<5} len {:<4} {}",
            string.addr.get(),
            enc,
            string.len,
            text
        );
    }
    Ok(())
}

fn truncate_string(session: &mut Session, addr: Address, len: u32, max_text: usize) -> String {
    let mut bytes = vec![0u8; len.min(4096) as usize];
    // Strings are file-backed; a read failure just yields an empty preview.
    let read = session.read_at(addr, &mut bytes).unwrap_or(0);
    bytes.truncate(read);
    let text = String::from_utf8_lossy(&bytes)
        .replace('\0', "")
        .trim()
        .to_owned();
    let mut chars: Vec<char> = text.chars().collect();
    if chars.len() > max_text {
        chars.truncate(max_text);
        chars.push('…');
    }
    chars.into_iter().collect()
}

fn cmd_search(
    project: &Path,
    bytes: Option<&str>,
    string: Option<&str>,
    limit: usize,
    as_json: bool,
) -> Result<()> {
    let needle = match (bytes, string) {
        (Some(raw), None) => parse_hex_bytes(raw)?,
        (None, Some(text)) => text.as_bytes().to_vec(),
        _ => {
            return Err(Error::internal(
                "pass exactly one of --bytes <hex> or --string <ascii>",
            ));
        }
    };
    let mut session = Session::open(project)?;
    let hits = session.search_bytes(&needle, limit)?;
    let functions = session.functions()?;
    let labelled: Vec<(Address, Option<String>)> = hits
        .iter()
        .map(|&hit| {
            let name = function_containing(&functions, hit)
                .and_then(|index| session.name_at(functions[index].start).ok().flatten());
            (hit, name)
        })
        .collect();
    if as_json {
        let rows: Vec<Value> = labelled
            .iter()
            .map(|(addr, name)| json!({"addr": format!("{:#x}", addr.get()), "function": name}))
            .collect();
        return json_result(&json!({
            "pattern_len": needle.len(),
            "hits": rows,
            "count": rows.len(),
            "truncated": rows.len() >= limit,
        }));
    }
    println!(
        "{} hit(s) for {} byte pattern",
        labelled.len(),
        needle.len()
    );
    for (addr, name) in &labelled {
        match name {
            Some(name) => println!("{:#012x}  {name}", addr.get()),
            None => println!("{:#012x}", addr.get()),
        }
    }
    Ok(())
}

fn cmd_callgraph(
    project: &Path,
    raw_addr: Option<&str>,
    limit: usize,
    as_json: bool,
) -> Result<()> {
    let session = Session::open(project)?;
    let functions = session.functions()?;
    let filter = match raw_addr {
        Some(raw) => Some(resolve_address(&session, raw)?),
        None => None,
    };
    let calls = session.all_xrefs(Some(recraft_analysis::XREF_CALL))?;
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for call in &calls {
        let Some(from) = function_containing(&functions, call.from) else {
            continue;
        };
        let Some(to) = function_containing(&functions, call.to) else {
            continue;
        };
        if from == to || !seen.insert((from, to)) {
            continue;
        }
        if let Some(filter) = filter {
            let touches_from = contains(&functions[from], filter);
            let touches_to = contains(&functions[to], filter);
            if !touches_from && !touches_to {
                continue;
            }
        }
        if edges.len() >= limit {
            break;
        }
        edges.push((from, to));
    }
    let name_of = |index: usize| {
        session
            .name_at(functions[index].start)
            .ok()
            .flatten()
            .unwrap_or_else(|| format!("{:#x}", functions[index].start.get()))
    };
    if as_json {
        let rows: Vec<Value> = edges
            .iter()
            .map(|(from, to)| {
                json!({
                    "from": format!("{:#x}", functions[*from].start.get()),
                    "from_name": name_of(*from),
                    "to": format!("{:#x}", functions[*to].start.get()),
                    "to_name": name_of(*to),
                })
            })
            .collect();
        return json_result(&json!({
            "nodes": functions.len(),
            "edges": rows,
            "count": rows.len(),
            "truncated": rows.len() >= limit,
        }));
    }
    println!(
        "call graph: {} function(s), {} edge(s)",
        functions.len(),
        edges.len()
    );
    for (from, to) in &edges {
        println!(
            "{:#012x} {:<28} -> {:#012x} {}",
            functions[*from].start.get(),
            name_of(*from),
            functions[*to].start.get(),
            name_of(*to)
        );
    }
    Ok(())
}

fn parse_hex_bytes(raw: &str) -> Result<Vec<u8>> {
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let digits = compact
        .strip_prefix("0x")
        .or_else(|| compact.strip_prefix("0X"))
        .unwrap_or(&compact);
    if digits.is_empty() || digits.len() % 2 != 0 {
        return Err(Error::internal(format!(
            "invalid hex pattern (need an even number of digits): {raw}"
        )));
    }
    let mut out = Vec::with_capacity(digits.len() / 2);
    for index in (0..digits.len()).step_by(2) {
        out.push(
            u8::from_str_radix(&digits[index..index + 2], 16).map_err(|_| {
                Error::internal(format!("invalid hex byte: {}", &digits[index..index + 2]))
            })?,
        );
    }
    Ok(out)
}

fn contains(function: &recraft_core::FunctionInfo, addr: Address) -> bool {
    function.start <= addr && addr < function.end
}

fn function_containing(functions: &[recraft_core::FunctionInfo], addr: Address) -> Option<usize> {
    // `functions` is ordered by start address.
    let mut low = 0usize;
    let mut high = functions.len();
    while low < high {
        let mid = (low + high) / 2;
        if functions[mid].start <= addr {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    let index = low.checked_sub(1)?;
    contains(&functions[index], addr).then_some(index)
}

fn cmd_batch(project: &Path, script: &Path, dry_run: bool, as_json: bool) -> Result<()> {
    let raw = std::fs::read_to_string(script)
        .map_err(|error| Error::internal(format!("cannot read {}: {error}", script.display())))?;
    // Tolerate a UTF-8 BOM (PowerShell 5.1 writes one with `-Encoding utf8`).
    let raw = raw.trim_start_matches('\u{feff}');
    let document: Value = serde_json::from_str(raw)
        .map_err(|error| Error::unsupported(format!("invalid batch JSON: {error}")))?;
    let entries = document
        .get("commands")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::unsupported("batch script needs a `commands` array"))?;
    if entries.is_empty() {
        return Err(Error::unsupported("batch script has no commands"));
    }
    let mut session = Session::open(project)?;
    let mut virtual_names: BTreeMap<String, Address> = BTreeMap::new();
    let mut results = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let command = parse_batch_command(&session, &virtual_names, entry)
            .map_err(|error| Error::unsupported(format!("command #{}: {error}", index + 1)))?;
        if dry_run {
            results.push(json!({
                "index": index + 1,
                "type": command.name(),
                "status": "validated",
            }));
        } else {
            session.execute(command.clone()).map_err(|error| {
                Error::unsupported(format!(
                    "command #{} ({}) failed: {error}",
                    index + 1,
                    command.name()
                ))
            })?;
            results.push(json!({
                "index": index + 1,
                "type": command.name(),
                "status": "applied",
            }));
        }
        if let Command::Rename {
            addr,
            name: Some(name),
        } = &command
        {
            virtual_names.insert(name.clone(), *addr);
        }
    }
    if as_json {
        return json_result(&json!({
            "dry_run": dry_run,
            "commands": results,
            "count": results.len(),
        }));
    }
    if dry_run {
        println!(
            "validated {} command(s) (dry run, nothing applied)",
            results.len()
        );
    } else {
        println!("applied {} command(s)", results.len());
    }
    Ok(())
}

fn parse_batch_command(
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
                Some(other) => {
                    return Err(Error::unsupported(format!(
                        "unknown comment kind {other:?} (regular/repeatable/anterior)"
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
                if let Some(text) = field.as_str() {
                    fields.push(parse_struct_field(tokenize_struct_field(text))?);
                } else {
                    let offset = field.get("offset").and_then(Value::as_u64).ok_or_else(|| {
                        Error::unsupported("struct field needs a numeric `offset`")
                    })?;
                    let field_name = field
                        .get("name")
                        .and_then(Value::as_str)
                        .ok_or_else(|| Error::unsupported("struct field needs `name`"))?;
                    let field_kind = field
                        .get("kind")
                        .and_then(Value::as_str)
                        .ok_or_else(|| Error::unsupported("struct field needs `kind`"))?;
                    fields.push(StructField {
                        offset,
                        name: field_name.to_owned(),
                        kind: field_kind.to_owned(),
                    });
                }
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

fn tokenize_struct_field(text: &str) -> (&str, &str, &str) {
    let mut parts = text.split(':');
    (
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default(),
    )
}

fn parse_struct_field((offset, name, kind): (&str, &str, &str)) -> Result<StructField> {
    if offset.is_empty() || name.is_empty() || kind.is_empty() {
        return Err(Error::unsupported(
            "struct field string must be offset:name:kind",
        ));
    }
    let offset = offset
        .trim_start_matches("0x")
        .parse::<u64>()
        .map_err(|_| Error::unsupported(format!("invalid field offset {offset:?}")))?;
    Ok(StructField {
        offset,
        name: name.to_owned(),
        kind: kind.to_owned(),
    })
}

fn generate_mcp_token() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    format!("{:032x}{:08x}", nanos, std::process::id())
}

fn cmd_xrefs(
    project: &Path,
    raw_addr: &str,
    direction: XrefDirection,
    as_json: bool,
) -> Result<()> {
    let session = Session::open(project)?;
    let addr = resolve_address(&session, raw_addr)?;
    let to = if matches!(direction, XrefDirection::To | XrefDirection::Both) {
        session.xrefs_to(addr)?
    } else {
        Vec::new()
    };
    let from = if matches!(direction, XrefDirection::From | XrefDirection::Both) {
        session.xrefs_from(addr)?
    } else {
        Vec::new()
    };
    if as_json {
        let rows: Vec<Value> = to
            .iter()
            .map(|xref| {
                json!({
                    "from": format!("{:#x}", xref.from.get()),
                    "to": format!("{:#x}", xref.to.get()),
                    "type": xref_kind_name(xref.kind),
                    "direction": "to",
                })
            })
            .chain(from.iter().map(|xref| {
                json!({
                    "from": format!("{:#x}", xref.from.get()),
                    "to": format!("{:#x}", xref.to.get()),
                    "type": xref_kind_name(xref.kind),
                    "direction": "from",
                })
            }))
            .collect();
        return json_result(&Value::Array(rows));
    }

    println!("xrefs at {:#x}:", addr.get());
    println!("  to this address:   {}", to.len());
    println!("  from this address: {}", from.len());
    for xref in &to {
        println!(
            "  <- {:#012x}  {:<4}",
            xref.from.get(),
            xref_kind_name(xref.kind)
        );
    }
    for xref in &from {
        println!(
            "  -> {:#012x}  {:<4}",
            xref.to.get(),
            xref_kind_name(xref.kind)
        );
    }
    Ok(())
}

fn xref_kind_name(kind: i64) -> &'static str {
    match kind {
        recraft_analysis::XREF_CALL => "call",
        recraft_analysis::XREF_JUMP => "jump",
        recraft_analysis::XREF_DATA => "data",
        _ => "other",
    }
}

fn cmd_eval(file: &Path, map: Option<&Path>, as_json: bool) -> Result<()> {
    let bytes = std::fs::read(file)?;
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("recraft-eval-{}-{unique}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let project = dir.join("eval.rcraft");

    let outcome = (|| -> Result<()> {
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false)?;
        let image = session.image().clone();
        let started = Instant::now();
        let result = analyze(&mut session)?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

        let mut ground_truth: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
        ground_truth.insert(session.entry_point().get());
        let segments = session.segments().to_vec();
        let in_code = |va: u64| {
            segments
                .iter()
                .any(|segment| segment.perm.is_executable() && segment.contains(Address::new(va)))
        };
        let mut code_exports = 0usize;
        let mut skipped_exports = 0usize;
        for export in &image.exports {
            if export.forwarder.is_some() {
                skipped_exports += 1;
                continue;
            }
            let va = image.image_base.get() + u64::from(export.rva);
            if !in_code(va) {
                skipped_exports += 1;
                continue;
            }
            ground_truth.insert(va);
            code_exports += 1;
        }
        for range in &image.pdata_functions {
            if in_code(range.start.get()) {
                ground_truth.insert(range.start.get());
            }
        }
        // Merge MSVC cold fragments so phantom function starts are not counted.
        let raw_pdata: Vec<recraft_common::AddressRange> = image
            .pdata_functions
            .iter()
            .map(|range| recraft_common::AddressRange::new(range.start, range.end))
            .collect();
        let merged_pdata = recraft_analysis::merge_pdata_functions(&mut session, &raw_pdata)?;
        ground_truth.retain(|start| {
            !image
                .pdata_functions
                .iter()
                .any(|range| range.start.get() == *start)
                || merged_pdata
                    .iter()
                    .any(|(start_merged, _)| start_merged.get() == *start)
        });
        for (start, _) in &merged_pdata {
            if in_code(start.get()) {
                ground_truth.insert(start.get());
            }
        }
        let mut ground_truth_source = "entry + code exports + .pdata";
        if let Some(map_path) = map {
            ground_truth.clear();
            ground_truth_source = "linker map (all function symbols)";
            let text = std::fs::read_to_string(map_path)
                .map_err(|error| Error::internal(format!("read map file: {error}")))?;
            for line in text.lines() {
                let tokens: Vec<&str> = line.split_whitespace().collect();
                if tokens.len() >= 4
                    && tokens[3] == "f"
                    && let Ok(va) = u64::from_str_radix(tokens[2], 16)
                    && in_code(va)
                {
                    ground_truth.insert(va);
                }
            }
            if ground_truth.is_empty() {
                return Err(Error::internal(
                    "no function symbols found in the linker map",
                ));
            }
        }
        let detected: std::collections::BTreeSet<u64> = result
            .functions
            .iter()
            .map(|function| function.start.get())
            .collect();
        let matched = ground_truth.intersection(&detected).count();
        let unmatched: Vec<(u64, &'static str)> = result
            .functions
            .iter()
            .filter(|function| !ground_truth.contains(&function.start.get()))
            .map(|function| {
                let source = match function.source {
                    recraft_analysis::FunctionSource::Entry => "entry",
                    recraft_analysis::FunctionSource::Export => "export",
                    recraft_analysis::FunctionSource::Pdata => "pdata",
                    recraft_analysis::FunctionSource::Tls => "tls",
                    recraft_analysis::FunctionSource::CallTarget => "call",
                    recraft_analysis::FunctionSource::GoPclntab => "go",
                    recraft_analysis::FunctionSource::DataPointer => "data",
                };
                (function.start.get(), source)
            })
            .collect();
        let recall = matched as f64 / ground_truth.len().max(1) as f64;
        let precision = matched as f64 / detected.len().max(1) as f64;
        let exact_boundaries = merged_pdata
            .iter()
            .filter(|(start, end)| {
                result
                    .functions
                    .iter()
                    .any(|function| function.start == *start && function.end == *end)
            })
            .count();
        let boundary_accuracy = exact_boundaries as f64 / merged_pdata.len().max(1) as f64;

        if as_json {
            return json_result(&json!({
                "file": file.display().to_string(),
                "ground_truth": ground_truth.len(),
                "ground_truth_source": ground_truth_source,
                "detected": detected.len(),
                "matched": matched,
                "recall": recall,
                "precision": precision,
                "code_exports": code_exports,
                "excluded_exports": skipped_exports,
                "pdata_functions": merged_pdata.len(),
                "pdata_exact_boundaries": exact_boundaries,
                "boundary_accuracy": boundary_accuracy,
                "instructions": result.instructions,
                "elapsed_ms": elapsed_ms,
            }));
        }

        println!("File:              {}", file.display());
        println!(
            "Ground truth:      {} ({})",
            ground_truth.len(),
            ground_truth_source
        );
        if skipped_exports > 0 {
            println!(
                "Excluded exports:  {skipped_exports} (forwarders / non-code, not analysable)"
            );
        }
        println!("Detected:          {}", detected.len());
        println!("Matched:           {matched}");
        println!("Recall:            {recall:.3}");
        println!("Precision:         {precision:.3}");
        if !unmatched.is_empty() {
            let sample: Vec<String> = unmatched
                .iter()
                .take(8)
                .map(|(addr, source)| format!("{addr:#x} ({source})"))
                .collect();
            println!(
                "Unmatched:         {} [{}]",
                unmatched.len(),
                sample.join(", ")
            );
        }
        println!(
            ".pdata boundaries: {exact_boundaries}/{} ({boundary_accuracy:.3})",
            merged_pdata.len()
        );
        println!("Instructions:      {}", result.instructions);
        println!("Elapsed:           {elapsed_ms:.1} ms");
        Ok(())
    })();

    let _ = std::fs::remove_dir_all(&dir);
    outcome
}

fn cmd_cfg(project: &Path, addr: Option<&str>, as_json: bool) -> Result<()> {
    let mut session = Session::open(project)?;
    let target = match addr {
        Some(raw) => resolve_address(&session, raw)?,
        None => session.entry_point(),
    };
    let functions = session.functions()?;
    let Some(function) = functions
        .iter()
        .find(|function| target >= function.start && target < function.end)
    else {
        return Err(Error::internal(
            "address is not inside a known function - run `recraft analyze` first",
        ));
    };
    let cfg = recraft_analysis::build_cfg(&mut session, function.start, function.end)?;

    if as_json {
        let blocks: Vec<Value> = cfg
            .blocks
            .iter()
            .map(|block| {
                json!({
                    "index": block.index,
                    "start": format!("{:#x}", block.start.get()),
                    "end": format!("{:#x}", block.end.get()),
                    "insns": block.insns.len(),
                    "edges": block
                        .edges
                        .iter()
                        .map(|(target, kind)| json!({"target": target, "kind": edge_kind_name(*kind)}))
                        .collect::<Vec<_>>(),
                })
            })
            .collect();
        return json_result(&json!({
            "function": format!("{:#x}", cfg.function.get()),
            "entry": cfg.entry,
            "blocks": blocks,
            "external_branches": cfg.external.len(),
        }));
    }

    println!(
        "function {:#x}: {} blocks, {} external branches",
        cfg.function.get(),
        cfg.blocks.len(),
        cfg.external.len()
    );
    for block in &cfg.blocks {
        let edges = block
            .edges
            .iter()
            .map(|(target, kind)| format!("{target}:{}", edge_kind_name(*kind)))
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "  [{:>2}] {:#012x} .. {:#012x}  {:>3} insns  -> {}",
            block.index,
            block.start.get(),
            block.end.get(),
            block.insns.len(),
            edges
        );
    }
    Ok(())
}

fn edge_kind_name(kind: recraft_analysis::EdgeKind) -> &'static str {
    match kind {
        recraft_analysis::EdgeKind::Fallthrough => "fallthrough",
        recraft_analysis::EdgeKind::Taken => "taken",
        recraft_analysis::EdgeKind::NotTaken => "not-taken",
        recraft_analysis::EdgeKind::Unconditional => "jump",
        recraft_analysis::EdgeKind::JumpTable => "table",
    }
}

// ------------------------------------------------------------------ funcs ----

fn cmd_funcs(project: &Path, as_json: bool) -> Result<()> {
    let session = Session::open(project)?;
    let functions = session.functions()?;
    if as_json {
        let functions: Vec<Value> = functions
            .iter()
            .map(|function| {
                json!({
                    "start": format!("{:#x}", function.start.get()),
                    "end": format!("{:#x}", function.end.get()),
                    "name": session.name_at(function.start).ok().flatten(),
                })
            })
            .collect();
        return json_result(&json!({ "functions": functions }));
    }
    if functions.is_empty() {
        println!("no functions defined - run `recraft analyze` first");
        return Ok(());
    }
    for function in &functions {
        let name = session
            .name_at(function.start)
            .ok()
            .flatten()
            .unwrap_or_default();
        println!(
            "{:#012x} .. {:#012x}  ({} bytes)  {}",
            function.start.get(),
            function.end.get(),
            function.end.get().saturating_sub(function.start.get()),
            name
        );
    }
    Ok(())
}

// ---------------------------------------------------------------- decompile ----

fn cmd_decompile(
    project: &Path,
    addr: Option<&str>,
    max_insns: usize,
    as_json: bool,
    with_addresses: bool,
    structured: bool,
    l2: bool,
) -> Result<()> {
    let mut session = Session::open(project)?;
    let start = match addr {
        Some(raw) => resolve_address(&session, raw)?,
        None => session.entry_point(),
    };
    let function_start = session
        .functions()?
        .iter()
        .find(|function| start >= function.start && start < function.end)
        .map(|function| function.start)
        .unwrap_or(start);
    let code = if l2 {
        recraft_decompile::pseudocode_l2(&mut session, function_start)?
    } else if structured {
        recraft_decompile::pseudocode_l1(&mut session, function_start)?
    } else {
        pseudocode_l0(&mut session, start, max_insns)?
    };

    if as_json {
        let lines: Vec<Value> = code
            .lines
            .iter()
            .map(|line| {
                json!({
                    "addr": format!("{:#x}", line.addr.get()),
                    "text": line.text,
                })
            })
            .collect();
        return json_result(&json!({
            "start": format!("{:#x}", code.func_start.get()),
            "lines": lines,
        }));
    }

    for line in &code.lines {
        if with_addresses {
            println!("{:#012x}  {}", line.addr.get(), line.text);
        } else {
            println!("{}", line.text);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- editing ----

fn cmd_rename(project: &Path, addr: &str, name: &str) -> Result<()> {
    let mut session = Session::open(project)?;
    let addr = resolve_address(&session, addr)?;
    let name = if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    };
    session.execute(Command::Rename {
        addr,
        name: name.clone(),
    })?;
    match name {
        Some(name) => println!("renamed {:#x} -> {name}", addr.get()),
        None => println!("removed name at {:#x}", addr.get()),
    }
    Ok(())
}

fn cmd_comment(project: &Path, addr: &str, text: &str) -> Result<()> {
    let mut session = Session::open(project)?;
    let addr = resolve_address(&session, addr)?;
    let text = if text.is_empty() {
        None
    } else {
        Some(text.to_owned())
    };
    session.execute(Command::SetComment {
        addr,
        kind: CommentKind::Regular,
        text: text.clone(),
    })?;
    match text {
        Some(text) => println!("commented {:#x}: {text}", addr.get()),
        None => println!("removed comment at {:#x}", addr.get()),
    }
    Ok(())
}

fn cmd_mark(project: &Path, addr: &str, len: u64, kind: MarkKind) -> Result<()> {
    let mut session = Session::open(project)?;
    let addr = resolve_address(&session, addr)?;
    let range = AddressRange::new(addr, Address::new(addr.get().saturating_add(len)));
    let code_data = kind.to_code_data();
    session.execute(Command::SetCodeData {
        range,
        kind: code_data,
    })?;
    println!(
        "marked {:#x}..{:#x} as {:?}",
        range.start.get(),
        range.end.get(),
        code_data
    );
    Ok(())
}

fn cmd_prototype(project: &Path, addr: &str, text: Option<&str>) -> Result<()> {
    let mut session = Session::open(project)?;
    let addr = resolve_address(&session, addr)?;
    session.execute(Command::SetPrototype {
        addr,
        text: text.map(str::to_owned),
    })?;
    match text {
        Some(text) => println!("prototype at {:#x}: {text}", addr.get()),
        None => println!("prototype at {:#x} removed", addr.get()),
    }
    Ok(())
}

fn cmd_define_struct(project: &Path, name: &str, size: u64, raw_fields: &[String]) -> Result<()> {
    let mut fields = Vec::with_capacity(raw_fields.len());
    for raw in raw_fields {
        let mut parts = raw.split(':');
        let (Some(offset), Some(field_name), Some(kind), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(Error::unsupported(format!(
                "invalid --field {raw:?}; expected offset:name:kind"
            )));
        };
        let offset = offset
            .trim_start_matches("0x")
            .parse::<u64>()
            .map_err(|_| Error::unsupported(format!("invalid field offset {offset:?}")))?;
        fields.push(StructField {
            offset,
            name: field_name.to_owned(),
            kind: kind.to_owned(),
        });
    }
    let def = StructDef {
        name: name.to_owned(),
        size,
        fields,
    }
    .validate()?;
    let mut session = Session::open(project)?;
    session.execute(Command::DefineStruct { def: def.clone() })?;
    println!(
        "defined struct {} (size {}, {} field(s))",
        def.name,
        def.size,
        def.fields.len()
    );
    Ok(())
}

fn cmd_apply_type(project: &Path, addr: &str, slot: u8, type_name: Option<&str>) -> Result<()> {
    let mut session = Session::open(project)?;
    let func = resolve_address(&session, addr)?;
    session.execute(Command::ApplyType {
        func,
        slot,
        struct_name: type_name.map(str::to_owned),
    })?;
    match type_name {
        Some(name) => println!("applied {name} to {:#x} arg{}", func.get(), slot + 1),
        None => println!("cleared type on {:#x} arg{}", func.get(), slot + 1),
    }
    Ok(())
}

fn cmd_structs(project: &Path) -> Result<()> {
    let session = Session::open(project)?;
    let structs = session.structs()?;
    if structs.is_empty() {
        println!("no user structs");
        return Ok(());
    }
    for def in structs {
        println!("struct {} (size {:#x}):", def.name, def.size);
        for field in &def.fields {
            println!("  {:#06x}  {:<24} {}", field.offset, field.name, field.kind);
        }
    }
    Ok(())
}

fn cmd_undo(project: &Path) -> Result<()> {
    let mut session = Session::open(project)?;
    match session.undo()? {
        Some(effect) => println!("undone ({} range(s) invalidated)", effect.dirty.len()),
        None => println!("nothing to undo"),
    }
    Ok(())
}

fn cmd_redo(project: &Path) -> Result<()> {
    let mut session = Session::open(project)?;
    match session.redo()? {
        Some(effect) => println!("redone ({} range(s) invalidated)", effect.dirty.len()),
        None => println!("nothing to redo"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_parsing_entry_and_hex() {
        let dir = tempfile::tempdir().unwrap();
        let Some(bytes) = [
            "C:/Windows/System32/notepad.exe",
            "C:/Windows/System32/kernel32.dll",
        ]
        .iter()
        .find_map(|path| std::fs::read(path).ok()) else {
            return;
        };
        let project = dir.path().join("cli.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        assert_eq!(
            resolve_address(&session, "entry").unwrap(),
            session.entry_point()
        );
        assert_eq!(
            resolve_address(&session, "0x10").unwrap(),
            Address::new(0x10)
        );
        assert_eq!(
            resolve_address(&session, "start").unwrap(),
            session.entry_point()
        );
        assert!(resolve_address(&session, "definitely_not_a_symbol").is_err());
    }
}
