// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! ReCraft command line interface (M0: import / inspect / disassemble / edit / undo).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};
use recraft_analysis::{analyze, persist};
use recraft_common::{Address, AddressRange, Error, Result};
use recraft_core::{CodeData, Command, CommentKind, Session};
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
    /// Render L0 pseudo-C for a function.
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
    },
    /// Run automatic analysis (function discovery) and persist it into the project.
    Analyze {
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Evaluate function discovery against exports + .pdata (ground truth).
    Eval {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
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
        Commands::Import { file, out } => cmd_import(&file, &out),
        Commands::Disasm {
            project,
            addr,
            count,
            json,
        } => cmd_disasm(&project, addr.as_deref(), count, json),
        Commands::Funcs { project, json } => cmd_funcs(&project, json),
        Commands::Analyze { project, json } => cmd_analyze(&project, json),
        Commands::Eval { file, json } => cmd_eval(&file, json),
        Commands::Strings {
            project,
            max_text,
            json,
        } => cmd_strings(&project, max_text, json),
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
        } => cmd_decompile(&project, addr.as_deref(), max_insns, json, addresses),
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
    println!("instructions decoded: {}", result.instructions);
    println!("direct call edges:    {}", result.calls.len());
    println!("branch edges:         {}", result.jumps.len());
    println!("data references:      {}", result.data_refs.len());
    println!("api tags:             {}", result.api_refs.len());
    println!("auto names:           {auto_names}");
    println!("strings:              {}", strings.len());
    println!("elapsed:              {elapsed_ms:.1} ms");
    println!("saved to project:     {}", project.display());
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

fn cmd_eval(file: &Path, as_json: bool) -> Result<()> {
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
        let detected: std::collections::BTreeSet<u64> = result
            .functions
            .iter()
            .map(|function| function.start.get())
            .collect();
        let matched = ground_truth.intersection(&detected).count();
        let recall = matched as f64 / ground_truth.len().max(1) as f64;
        let precision = matched as f64 / detected.len().max(1) as f64;
        let exact_boundaries = image
            .pdata_functions
            .iter()
            .filter(|range| {
                result
                    .functions
                    .iter()
                    .any(|function| function.start == range.start && function.end == range.end)
            })
            .count();
        let boundary_accuracy = exact_boundaries as f64 / image.pdata_functions.len().max(1) as f64;

        if as_json {
            return json_result(&json!({
                "file": file.display().to_string(),
                "ground_truth": ground_truth.len(),
                "detected": detected.len(),
                "matched": matched,
                "recall": recall,
                "precision": precision,
                "code_exports": code_exports,
                "excluded_exports": skipped_exports,
                "pdata_functions": image.pdata_functions.len(),
                "pdata_exact_boundaries": exact_boundaries,
                "boundary_accuracy": boundary_accuracy,
                "instructions": result.instructions,
                "elapsed_ms": elapsed_ms,
            }));
        }

        println!("File:              {}", file.display());
        println!(
            "Ground truth:      {} (entry 1 + code exports {code_exports} + .pdata {})",
            ground_truth.len(),
            image.pdata_functions.len()
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
        println!(
            ".pdata boundaries: {exact_boundaries}/{} ({boundary_accuracy:.3})",
            image.pdata_functions.len()
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
                })
            })
            .collect();
        return json_result(&json!({ "functions": functions }));
    }
    if functions.is_empty() {
        println!("no functions defined (M1 will add automatic analysis)");
        return Ok(());
    }
    for function in &functions {
        println!(
            "{:#012x} .. {:#012x}  ({} bytes)",
            function.start.get(),
            function.end.get(),
            function.end.get().saturating_sub(function.start.get())
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
) -> Result<()> {
    let mut session = Session::open(project)?;
    let start = match addr {
        Some(raw) => resolve_address(&session, raw)?,
        None => session.entry_point(),
    };
    let code = pseudocode_l0(&mut session, start, max_insns)?;

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
