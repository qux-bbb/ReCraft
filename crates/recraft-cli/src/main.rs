// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! ReCraft command line interface (M0: import / inspect / disassemble / edit / undo).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};
use recraft_analysis::{analyze, extract_strings, persist, persist_strings};
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
    name = "recraft-cli",
    version,
    about = "ReCraft — AI-native reverse engineering (pre-alpha)",
    long_about = "ReCraft — AI-native reverse engineering (pre-alpha)\n\nA project is a `.rcraft` file that `import` creates from a PE image. A typical session is\n`import <exe> -o <project>`, `analyze <project>`, then `decompile <project> --l2 --addr <addr>`\n(`--addr` accepts `0x…`, a symbol name or `entry`). `metrics`, `lint` and `golden` judge the\noutput without opening the GUI."
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
        /// Ask the configured LLM provider for the names (evidence-only by default).
        #[arg(long)]
        llm: bool,
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
    /// Answer a natural-language question offline (DESIGN §9.2).
    Query {
        project: PathBuf,
        /// Question, e.g. "find every function that references a URL".
        text: String,
        #[arg(long)]
        json: bool,
    },
    /// AI-polish one function's L1 pseudo-code (suggestion only, never stored).
    Polish {
        project: PathBuf,
        /// Function address/name (defaults to the entry point).
        #[arg(long)]
        addr: Option<String>,
        #[arg(long)]
        json: bool,
        /// Bypass the persistent response cache.
        #[arg(long)]
        no_cache: bool,
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
    /// Ask a question about the project; the model calls read-only tools to answer.
    Ask {
        project: PathBuf,
        /// Question, e.g. "which functions write to the registry?".
        question: String,
        #[arg(long)]
        json: bool,
        /// Maximum model turns (each may run several tools).
        #[arg(long, default_value_t = recraft_ai::AGENT_MAX_ROUNDS)]
        rounds: usize,
    },
    /// Score proposed names against the symbols the binary actually carries.
    ///
    /// Offline by default (the heuristic fallback); `--llm` runs the real naming pipeline
    /// with each candidate's symbol blinded, so the model cannot read the answer.
    EvalNames {
        project: PathBuf,
        #[arg(long)]
        json: bool,
        /// Use the configured LLM provider instead of the offline heuristic.
        #[arg(long)]
        llm: bool,
        /// Maximum number of named functions to score.
        #[arg(long, default_value_t = 40)]
        limit: usize,
        /// JSON manifest of acceptable aliases per real symbol
        /// (`{"open_config_file": ["read_config_file", ...]}`).
        #[arg(long)]
        aliases: Option<PathBuf>,
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
    /// Serve the project over the Model Context Protocol (read-only tools; `--write`
    /// adds the journaled write tools).
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
        /// Enable the journaled write tools (rename, set_comment, set_prototype,
        /// apply_type, define_function, undo).
        #[arg(long)]
        write: bool,
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
        /// Write the pseudo-C to this file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Write one L2 pseudo-C file per function into a directory.
    ExportPseudo {
        project: PathBuf,
        /// Directory to write into (created if missing).
        #[arg(long)]
        dir: PathBuf,
        /// Export at most this many functions (0 exports all of them).
        #[arg(long, default_value_t = 0)]
        limit: usize,
        /// Export only the functions that still contain a fallback jump.
        #[arg(long)]
        only_unstructured: bool,
    },
    /// Decompile the functions listed by a DecBench eval kit into a submission.
    Decbench {
        /// Unpacked eval kit directory (contains `functions.json` and `binaries/`).
        kit: PathBuf,
        /// Where to write `results.json` and the per-binary `.c` files.
        #[arg(long, short = 'o')]
        out: PathBuf,
    },
    /// Run automatic analysis (function discovery) and persist it into the project.
    Analyze {
        project: PathBuf,
        /// When `project` is a PE file, create/overwrite this project and analyze it.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Do not match the built-in CRT signatures after analysing.
        #[arg(long)]
        no_signatures: bool,
        #[arg(long)]
        json: bool,
    },
    Reanalyze {
        project: PathBuf,
        /// Range start: `0x...`, decimal, a symbol name, or `entry`.
        #[arg(long)]
        addr: String,
        #[arg(long, default_value_t = 0x100, value_parser = parse_u64_value)]
        len: u64,
        #[arg(long)]
        json: bool,
    },
    /// Measure decompiler quality gates (semantic coverage / structured rate); `--list` names the
    /// functions the structurer could not reduce.
    Metrics {
        project: PathBuf,
        #[arg(long)]
        json: bool,
        /// Measure at most this many functions (0 measures all of them).
        #[arg(long, default_value_t = 2000)]
        max_functions: usize,
        /// Fail (exit non-zero) when the structured rate is below this fraction.
        #[arg(long)]
        min_structured: Option<f64>,
        /// Fail (exit non-zero) when instruction coverage is below this fraction.
        #[arg(long)]
        min_coverage: Option<f64>,
        /// List the functions the structurer could not reduce, worst first.
        #[arg(long)]
        list: bool,
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
        /// Write/check at most this many functions (a sample of a large project).
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Check the decompiler output invariants over every function (non-zero exit on a
    /// violation).
    Lint {
        project: PathBuf,
        /// Check at most this many functions (default: all).
        #[arg(long)]
        limit: Option<usize>,
        /// Print the report as JSON.
        #[arg(long)]
        json: bool,
        /// Level to check: both (default), l1 or l2.
        #[arg(long)]
        level: Option<String>,
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
    /// Export the signature of every named function to a JSON file.
    SignatureExport {
        project: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Name functions from the built-in CRT signatures and/or signature files (or
    /// directories of `*.json`); a function that already has a name keeps it.
    SignatureImport {
        project: PathBuf,
        /// A signature JSON file, or a directory whose `*.json` files are all loaded.
        #[arg(long = "from")]
        from: Vec<PathBuf>,
        /// Also match the CRT signatures compiled into this binary.
        #[arg(long)]
        builtin: bool,
        /// Print the matches without renaming anything.
        #[arg(long)]
        dry_run: bool,
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
    restore_sigpipe();
    install_broken_pipe_hook();
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Rust ignores `SIGPIPE`, so `recraft-cli funcs … | head` used to panic with a
/// broken-pipe error instead of exiting quietly like every other Unix filter.
/// Restoring the default disposition gives the expected behaviour (the process dies
/// with `SIGPIPE`, exit status 141).
#[cfg(unix)]
fn restore_sigpipe() {
    // SAFETY: `signal` only changes this process's disposition; the constants are the
    // POSIX values on every Unix target we build for.
    unsafe {
        extern "C" {
            fn signal(signum: i32, handler: usize) -> usize;
        }
        const SIGPIPE: i32 = 13;
        const SIG_DFL: usize = 0;
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_sigpipe() {}

/// Windows has no `SIGPIPE`: the write fails with `ERROR_BROKEN_PIPE` and `println!`
/// panics. Turn that one panic into a quiet exit, so `… | more` / `| head` behave.
fn install_broken_pipe_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = info
            .payload()
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| info.payload().downcast_ref::<&str>().copied())
            .unwrap_or_default();
        if is_broken_pipe_message(message) {
            std::process::exit(0);
        }
        default_hook(info);
    }));
}

/// Whether a panic message is the "the consumer closed the pipe" case.
fn is_broken_pipe_message(message: &str) -> bool {
    // std panics with "failed printing to stdout: <io error>". The OS error code is the
    // stable part (the text is localized):
    //   32  EPIPE / ERROR_BROKEN_PIPE on Unix ("Broken pipe")
    //   109 ERROR_BROKEN_PIPE on Windows ("The pipe has been ended.")
    //   232 ERROR_NO_DATA on Windows ("管道正在被关闭。")
    message.starts_with("failed printing to stdout")
        && ["os error 32", "os error 109", "os error 232"]
            .iter()
            .any(|code| message.contains(code))
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
            write,
        } => {
            if http {
                let token = token.unwrap_or_else(generate_mcp_token);
                if token.is_empty() {
                    return Err(recraft_common::Error::internal("empty MCP token"));
                }
                eprintln!("[mcp] token: {token}");
                if write {
                    recraft_mcp::serve_http_writable(&project, &bind, &token)
                } else {
                    recraft_mcp::serve_http(&project, &bind, &token)
                }
            } else if write {
                recraft_mcp::McpServer::open_writable(&project)?.serve_stdio()
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
        } => {
            let materialized = materialize_project(&project)?;
            cmd_disasm(&materialized.path, addr.as_deref(), count, json)
        }
        Commands::Funcs { project, json } => {
            let materialized = materialize_project(&project)?;
            cmd_funcs(&materialized.path, json)
        }
        Commands::Analyze {
            project,
            out,
            json,
            no_signatures,
        } => cmd_analyze(&project, out.as_deref(), json, !no_signatures),
        Commands::Reanalyze {
            project,
            addr,
            len,
            json,
        } => {
            let materialized = materialize_project(&project)?;
            cmd_reanalyze(&materialized.path, &addr, len, json)
        }
        Commands::Metrics {
            project,
            json,
            max_functions,
            min_structured,
            min_coverage,
            list,
        } => {
            let materialized = materialize_project(&project)?;
            cmd_metrics(
                &materialized.path,
                json,
                max_functions,
                min_structured,
                min_coverage,
                list,
            )
        }
        Commands::Golden {
            project,
            dir,
            check,
            limit,
        } => {
            let materialized = materialize_project(&project)?;
            cmd_golden(&materialized.path, dir.as_deref(), check, limit)
        }
        Commands::Lint {
            project,
            limit,
            json,
            level,
        } => {
            let materialized = materialize_project(&project)?;
            cmd_lint(&materialized.path, limit, json, level.as_deref())
        }
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
        } => {
            let materialized = materialize_project(&project)?;
            cmd_strings(&materialized.path, max_text, json)
        }
        Commands::Search {
            project,
            bytes,
            string,
            limit,
            json,
        } => {
            let materialized = materialize_project(&project)?;
            cmd_search(
                &materialized.path,
                bytes.as_deref(),
                string.as_deref(),
                limit,
                json,
            )
        }
        Commands::Callgraph {
            project,
            addr,
            limit,
            json,
        } => {
            let materialized = materialize_project(&project)?;
            cmd_callgraph(&materialized.path, addr.as_deref(), limit, json)
        }
        Commands::Xrefs {
            project,
            addr,
            direction,
            json,
        } => {
            let materialized = materialize_project(&project)?;
            cmd_xrefs(&materialized.path, &addr, direction, json)
        }
        Commands::Imports { project, json } => {
            let materialized = materialize_project(&project)?;
            cmd_imports(&materialized.path, json)
        }
        Commands::Cfg {
            project,
            addr,
            json,
        } => {
            let materialized = materialize_project(&project)?;
            cmd_cfg(&materialized.path, addr.as_deref(), json)
        }
        Commands::Decompile {
            project,
            addr,
            max_insns,
            json,
            addresses,
            structured,
            l2,
            out,
        } => {
            let materialized = materialize_project(&project)?;
            cmd_decompile(
                &materialized.path,
                DecompileOptions {
                    addr: addr.as_deref(),
                    max_insns,
                    as_json: json,
                    with_addresses: addresses,
                    structured,
                    l2,
                    out_file: out,
                },
            )
        }
        Commands::ExportPseudo {
            project,
            dir,
            limit,
            only_unstructured,
        } => {
            let materialized = materialize_project(&project)?;
            let written = recraft_decompile::export_pseudocode(
                &mut Session::open(&materialized.path)?,
                &dir,
                limit,
                only_unstructured,
            )?;
            println!("wrote {written} files to {}", dir.display());
            Ok(())
        }
        Commands::Decbench { kit, out } => cmd_decbench(&kit, &out),
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
        Commands::SignatureExport { project, out } => cmd_signature_export(&project, &out),
        Commands::SignatureImport {
            project,
            from,
            builtin,
            dry_run,
        } => cmd_signature_import(&project, &from, builtin, dry_run),
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

fn is_project_file(path: &Path) -> bool {
    let mut magic = [0u8; 16];
    use std::io::Read;
    match std::fs::File::open(path).and_then(|mut file| file.read_exact(&mut magic)) {
        Ok(()) => magic.starts_with(b"SQLite format 3\0"),
        Err(_) => false,
    }
}

/// Create a project from a PE file and run the full analysis pass (functions, xrefs,
/// strings) — the one-shot form used by `analyze <pe> --out`.
fn import_and_analyze_pe(pe: &Path, out: &Path) -> Result<()> {
    let bytes = std::fs::read(pe)
        .map_err(|error| Error::internal(format!("cannot read {}: {error}", pe.display())))?;
    let mut session = Session::create(out, &bytes, &mut |_, _| {}, &|| false)?;
    let result = analyze(&mut session)?;
    persist(&mut session, &result)?;
    let strings = recraft_analysis::extract_strings(&mut session, 4, 200_000)?;
    recraft_analysis::persist_strings(&mut session, &strings)?;
    Ok(())
}

/// A project path, plus the temporary directory backing an auto-created project when the
/// user pointed the command at a PE file instead of an existing `.rcraft` project.
struct MaterializedProject {
    path: PathBuf,
    _temp: Option<tempfile::TempDir>,
}

/// DESIGN §10 lets read-only commands take a PE file directly. When the argument is not
/// an existing SQLite project but is a valid PE, import + analyze it into a temporary
/// project (strings included) so every read command works without a manual `import`.
fn materialize_project(input: &Path) -> Result<MaterializedProject> {
    if !input.is_file() {
        return Ok(MaterializedProject {
            path: input.to_path_buf(),
            _temp: None,
        });
    }
    let mut magic = [0u8; 16];
    let is_sqlite = {
        use std::io::Read;
        match std::fs::File::open(input).and_then(|mut file| file.read_exact(&mut magic)) {
            Ok(()) => magic.starts_with(b"SQLite format 3\0"),
            Err(_) => false,
        }
    };
    if is_sqlite {
        return Ok(MaterializedProject {
            path: input.to_path_buf(),
            _temp: None,
        });
    }
    let bytes = std::fs::read(input)
        .map_err(|error| Error::internal(format!("cannot read {}: {error}", input.display())))?;
    if recraft_format::load(&bytes).is_err() {
        // Let the normal project-open path produce the error message.
        return Ok(MaterializedProject {
            path: input.to_path_buf(),
            _temp: None,
        });
    }
    let dir = tempfile::tempdir()
        .map_err(|error| Error::internal(format!("cannot create temp project: {error}")))?;
    let project = dir.path().join("adhoc.rcraft");
    {
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false)?;
        let result = analyze(&mut session)?;
        persist(&mut session, &result)?;
        let strings = extract_strings(&mut session, 4, 4_000_000)?;
        persist_strings(&mut session, &strings)?;
    }
    Ok(MaterializedProject {
        path: project,
        _temp: Some(dir),
    })
}

fn parse_u64_value(raw: &str) -> std::result::Result<u64, String> {
    let trimmed = raw.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).map_err(|error| error.to_string())
    } else {
        trimmed.parse::<u64>().map_err(|error| error.to_string())
    }
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
        .map_err(|_| Error::invalid_argument(format!("invalid address or unknown symbol: {raw}")))
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
    let loaded = recraft_format::load(&bytes)?;
    let image = loaded.image;
    let arch = loaded.arch;
    let format = loaded.format;

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
        match (format, arch) {
            (recraft_format::Format::Pe, recraft_format::Arch::X64) => "PE32+",
            (recraft_format::Format::Pe, _) => "PE32",
            (recraft_format::Format::Elf, _) => "ELF64",
        },
        arch.name()
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

// --------------------------------------------------------------- decbench ----

/// Decompile the functions a DecBench eval kit asks for, in the kit's submission
/// layout: one whole-binary `.c` per binary plus a `results.json` mapping each
/// function identifier to the address the kit listed.
fn cmd_decbench(kit: &Path, out: &Path) -> Result<()> {
    let manifest_path = kit.join("functions.json");
    let manifest = std::fs::read_to_string(&manifest_path).map_err(|error| {
        Error::invalid_argument(format!("cannot read {}: {error}", manifest_path.display()))
    })?;
    let manifest: Value = serde_json::from_str(&manifest).map_err(|error| {
        Error::invalid_argument(format!("cannot parse {}: {error}", manifest_path.display()))
    })?;
    let public = manifest
        .get("public")
        .and_then(Value::as_object)
        .ok_or_else(|| Error::invalid_argument("functions.json has no `public` map"))?;

    std::fs::create_dir_all(out)?;
    let mut results = serde_json::Map::new();
    let mut written_binaries = 0usize;
    let mut written_functions = 0usize;
    let mut skipped = 0usize;

    for (binary, addresses) in public {
        let path = kit.join("binaries").join(binary);
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skip {binary}: not in binaries/");
            skipped += 1;
            continue;
        };
        let dir = tempfile::tempdir()?;
        let project = dir.path().join("eval.rcraft");
        let mut session = match Session::create(&project, &bytes, &mut |_, _| {}, &|| false) {
            Ok(session) => session,
            Err(error) => {
                eprintln!("skip {binary}: {error}");
                skipped += 1;
                continue;
            }
        };
        let result = analyze(&mut session)?;
        persist(&mut session, &result)?;
        let strings = extract_strings(&mut session, 4, 4_000_000)?;
        persist_strings(&mut session, &strings)?;

        let mut bodies = Vec::new();
        let mut functions = serde_json::Map::new();
        if let Some(list) = addresses.as_array() {
            for address in list {
                let Some(text) = address.as_str() else {
                    continue;
                };
                let Some(addr) = parse_hex_address(text) else {
                    continue;
                };
                let Ok(pseudo) = recraft_decompile::pseudocode_l2(&session, Address::new(addr))
                else {
                    continue;
                };
                let body = pseudo
                    .lines
                    .iter()
                    .map(|line| line.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                let Some(identifier) = function_identifier(&body) else {
                    continue;
                };
                functions.insert(identifier, Value::String(text.to_owned()));
                bodies.push(body);
            }
        }
        if bodies.is_empty() {
            skipped += 1;
            continue;
        }

        let stem = Path::new(binary)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(binary);
        let file_name = format!("{stem}.c");
        let mut text = String::new();
        for body in bodies {
            if !text.is_empty() {
                text.push_str("\n\n");
            }
            text.push_str(&body);
            text.push('\n');
        }
        std::fs::write(out.join(&file_name), text)?;
        written_functions += functions.len();
        results.insert(
            file_name,
            json!({ "binary": binary, "functions": Value::Object(functions) }),
        );
        written_binaries += 1;
    }

    let submission = json!({
        "decompiler": { "name": "recraft", "version": env!("CARGO_PKG_VERSION") },
        "results": Value::Object(results),
    });
    let text = serde_json::to_string_pretty(&submission)
        .map_err(|error| Error::internal(format!("serialize results.json: {error}")))?;
    std::fs::write(out.join("results.json"), text)?;
    println!(
        "wrote {written_binaries} binaries / {written_functions} functions ({skipped} skipped) to {}",
        out.display()
    );
    Ok(())
}

/// Parse a kit address (`"0x1234"` or bare hex).
fn parse_hex_address(text: &str) -> Option<u64> {
    let text = text.trim();
    let text = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    u64::from_str_radix(text, 16).ok()
}

/// The identifier of the top-level function definition in a rendered chunk.
fn function_identifier(body: &str) -> Option<String> {
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with("//") || !line.ends_with('{') {
            continue;
        }
        let Some(open) = line.find('(') else {
            continue;
        };
        let head = line[..open].trim_end();
        let name = head
            .rsplit(|character: char| character.is_whitespace() || character == '*')
            .next()?;
        let valid = !name.is_empty()
            && !name.starts_with(|character: char| character.is_ascii_digit())
            && name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_');
        if valid {
            return Some(name.to_owned());
        }
    }
    None
}

fn cmd_import(file: &Path, out: &Path) -> Result<()> {
    // Name the file: the OS message alone ("the system cannot find the file") does not say which
    // path was missing, and it is localised.
    let bytes = std::fs::read(file).map_err(|error| {
        Error::invalid_argument(format!("cannot read {}: {error}", file.display()))
    })?;
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

fn cmd_reanalyze(project: &Path, raw: &str, len: u64, as_json: bool) -> Result<()> {
    let mut session = Session::open(project)?;
    let start = resolve_address(&session, raw)?;
    let range = AddressRange::new(start, Address::new(start.get().saturating_add(len.max(1))));
    let started = Instant::now();
    let result = recraft_analysis::reanalyze_range(&mut session, range)?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    if as_json {
        return json_result(&json!({
            "start": format!("{:#x}", range.start.get()),
            "end": format!("{:#x}", range.end.get()),
            "functions": result.functions.len(),
            "instructions": result.instructions,
            "seeds": result.seeds,
            "call_edges": result.calls.len(),
            "jump_edges": result.jumps.len(),
            "data_refs": result.data_refs.len(),
            "api_tags": result.api_refs.len(),
            "elapsed_ms": elapsed_ms,
        }));
    }
    println!(
        "reanalyzed {:#x}..{:#x}: {} function(s), {} instruction(s), {} xref(s) in {:.1} ms",
        range.start.get(),
        range.end.get(),
        result.functions.len(),
        result.instructions,
        result.calls.len() + result.jumps.len() + result.data_refs.len(),
        elapsed_ms
    );
    Ok(())
}

fn cmd_analyze(project: &Path, out: Option<&Path>, as_json: bool, signatures: bool) -> Result<()> {
    // DESIGN §10: `analyze <file> --out project.rcraft` imports and analyses in one step.
    let owned_project;
    let project = if project.is_file() && !is_project_file(project) {
        let out = out.ok_or_else(|| {
            Error::unsupported("analyze <pe> needs --out <project.rcraft> (or import first)")
        })?;
        import_and_analyze_pe(project, out)?;
        owned_project = out.to_path_buf();
        owned_project.as_path()
    } else {
        project
    };
    let mut session = Session::open(project)?;
    let started = Instant::now();
    let result = analyze(&mut session)?;
    persist(&mut session, &result)?;
    let strings = recraft_analysis::extract_strings(&mut session, 4, 200_000)?;
    recraft_analysis::persist_strings(&mut session, &strings)?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    // Names learned elsewhere reach a new project without an extra command: the built-in CRT
    // signatures are matched once, right after the analysis that discovers the functions.
    let named = if signatures {
        recraft_decompile::signature::apply_builtin(&mut session, false)?
            .renamed
            .len()
    } else {
        0
    };

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
            "signature_names": named,
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
    if named > 0 {
        println!("built-in signatures:  {named} function(s) named");
    }
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
    list_unstructured: bool,
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
            "switches": report.switches(),
            "max_nesting": report.max_nesting(),
            "deeply_nested": report.deeply_nested(),
            "skipped": report.skipped,
            "intrinsic_top": top,
            "unstructured": report
                .functions
                .iter()
                .filter(|function| !function.structured)
                .map(|function| json!({
                    "start": format!("{:#x}", function.start.get()),
                    "name": function.name,
                    "goto": function.fallbacks,
                    "lines": function.lines,
                }))
                .collect::<Vec<_>>(),
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
    let histogram: Vec<String> = report
        .fallback_histogram()
        .into_iter()
        .map(|(gotos, count)| format!("{gotos}:{count}"))
        .collect();
    println!("goto histogram:         {}", histogram.join(" "));
    println!("labels:                 {}", report.labels());
    println!("inline asm statements:  {}", report.inline_asm());
    println!("switch statements:      {}", report.switches());
    println!(
        "max nesting depth:      {} ({} functions at 8+ levels)",
        report.max_nesting(),
        report.deeply_nested()
    );
    if report.skipped > 0 {
        println!("skipped (limit):        {}", report.skipped);
    }
    if list_unstructured {
        // The functions the structurer could not reduce, worst first: this is the work list for
        // the multi-entry loop work, and it is what the `N goto` header counts per function.
        let mut loose: Vec<_> = report
            .functions
            .iter()
            .filter(|function| !function.structured)
            .collect();
        loose.sort_by_key(|function| std::cmp::Reverse(function.fallbacks));
        if loose.is_empty() {
            println!("unstructured functions: none");
        } else {
            println!("unstructured functions:");
            for function in loose {
                let name = function.name.as_deref().unwrap_or("?");
                println!(
                    "  {:>4} goto  {:>5} lines  {:>10}  {name}",
                    function.fallbacks,
                    function.lines,
                    format!("{:#x}", function.start.get())
                );
            }
        }
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

fn cmd_lint(project: &Path, limit: Option<usize>, json: bool, level: Option<&str>) -> Result<()> {
    let level = match level.unwrap_or("both") {
        "l1" => recraft_decompile::LintLevel::L1,
        "l2" => recraft_decompile::LintLevel::L2,
        "both" => recraft_decompile::LintLevel::Both,
        other => {
            return Err(Error::internal(format!(
                "unknown --level {other} (use both, l1 or l2)"
            )));
        }
    };
    let mut session = Session::open(project)?;
    let report = recraft_decompile::lint(&mut session, limit.unwrap_or(usize::MAX), level)?;
    let levels = match level {
        recraft_decompile::LintLevel::Both => "L1 + L2",
        recraft_decompile::LintLevel::L1 => "L1",
        recraft_decompile::LintLevel::L2 => "L2",
    };
    if json {
        let items: Vec<String> = report
            .findings
            .iter()
            .map(|finding| {
                let name = finding
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("sub_{:x}", finding.start.get()));
                format!(
                    "{{\"name\":{:?},\"kind\":{:?},\"detail\":{:?},\"address\":\"{:#x}\"}}",
                    name,
                    finding.kind.as_str(),
                    finding.detail,
                    finding.start.get()
                )
            })
            .collect();
        let counts: Vec<String> = [
            recraft_decompile::LintKind::GotoWithoutLabel,
            recraft_decompile::LintKind::DuplicateLabel,
            recraft_decompile::LintKind::UnbalancedBraces,
            recraft_decompile::LintKind::EmptyCondition,
            recraft_decompile::LintKind::UndeclaredArgument,
            recraft_decompile::LintKind::LabelAboveSignature,
        ]
        .into_iter()
        .map(|kind| format!("{:?}:{}", kind.as_str(), report.count(kind)))
        .collect();
        println!(
            "{{\"functions\":{},\"checked\":{},\"level\":{:?},\"findings\":{},\"affected\":{},\"rules\":{{{}}},\"items\":[{}]}}",
            report.functions,
            report.checked,
            levels,
            report.findings.len(),
            report.affected(),
            counts.join(","),
            items.join(",")
        );
        return if report.ok() {
            Ok(())
        } else {
            Err(Error::internal(format!(
                "{} output invariant violations",
                report.findings.len()
            )))
        };
    }
    match limit {
        Some(limit) if limit < report.functions => {
            println!("functions:  {} (checked first {limit})", report.functions);
        }
        _ => println!("functions:  {}", report.functions),
    }
    let levels = match level {
        recraft_decompile::LintLevel::Both => "L1 + L2",
        recraft_decompile::LintLevel::L1 => "L1",
        recraft_decompile::LintLevel::L2 => "L2",
    };
    println!("checked:    {} texts ({levels})", report.checked);
    println!(
        "findings:   {} in {} functions",
        report.findings.len(),
        report.affected()
    );
    for kind in [
        recraft_decompile::LintKind::GotoWithoutLabel,
        recraft_decompile::LintKind::DuplicateLabel,
        recraft_decompile::LintKind::UnbalancedBraces,
        recraft_decompile::LintKind::EmptyCondition,
        recraft_decompile::LintKind::UndeclaredArgument,
        recraft_decompile::LintKind::LabelAboveSignature,
    ] {
        let count = report.count(kind);
        if count > 0 {
            println!("  {:<24} {count}", kind.as_str());
        }
    }
    for finding in report.findings.iter().take(10) {
        println!(
            "  {} ({}) : {} — {}",
            finding
                .name
                .clone()
                .unwrap_or_else(|| format!("sub_{:x}", finding.start.get())),
            finding.kind.as_str(),
            finding.detail,
            finding.start
        );
    }
    if report.ok() {
        Ok(())
    } else {
        Err(Error::internal(format!(
            "{} output invariant violations",
            report.findings.len()
        )))
    }
}

fn cmd_golden(project: &Path, dir: Option<&Path>, check: bool, limit: Option<usize>) -> Result<()> {
    let dir = dir
        .map(Path::to_path_buf)
        .unwrap_or_else(recraft_decompile::default_golden_dir);
    let mut session = Session::open(project)?;
    if check {
        let report = recraft_decompile::check_golden_limited(
            &mut session,
            &dir,
            limit.unwrap_or(usize::MAX),
        )?;
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
        } else if report.mismatched.is_empty() && report.missing.len() == report.checked {
            // Every function is "missing": the directory holds no snapshots at all, which is a
            // setup mistake rather than a regression.
            Err(Error::invalid_argument(format!(
                "no golden snapshots in {} (run without --check to write them)",
                dir.display()
            )))
        } else {
            Err(Error::internal(format!(
                "golden regression: {} mismatched, {} missing in {}",
                report.mismatched.len(),
                report.missing.len(),
                dir.display()
            )))
        }
    } else {
        let written = recraft_decompile::write_golden_limited(
            &mut session,
            &dir,
            limit.unwrap_or(usize::MAX),
        )?;
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
            llm,
        } => cmd_ai_suggest(&project, json, limit, out.as_deref(), llm),
        AiCommand::Triage { project, json } => cmd_ai_triage(&project, json),
        AiCommand::Apply { project, file, yes } => cmd_ai_apply(&project, &file, yes),
        AiCommand::Query {
            project,
            text,
            json,
        } => cmd_ai_query(&project, &text, json),
        AiCommand::Polish {
            project,
            addr,
            json,
            no_cache,
        } => cmd_ai_polish(&project, addr.as_deref(), json, no_cache),
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
        AiCommand::Ask {
            project,
            question,
            json,
            rounds,
        } => cmd_ai_ask(&project, &question, json, rounds),
        AiCommand::Eval { project, json } => cmd_ai_eval(&project, json),
        AiCommand::EvalNames {
            project,
            json,
            llm,
            limit,
            aliases,
        } => cmd_ai_eval_names(&project, json, llm, limit, aliases.as_deref()),
    }
}

fn require_ai_provider(operation: &str) -> Result<recraft_ai::ProviderSettings> {
    let settings = recraft_ai::ProviderSettings::from_env();
    if !settings.enabled {
        return Err(recraft_common::Error::unsupported(format!(
            "{operation} needs an LLM provider; set RECRAFT_AI_PROVIDER, \
             RECRAFT_AI_ENDPOINT and the API key env var (see `recraft ai provider`)"
        )));
    }
    Ok(settings)
}

fn cmd_ai_query(project: &Path, text: &str, as_json: bool) -> Result<()> {
    let mut session = Session::open(project)?;
    let answer = recraft_ai::answer_query(&mut session, text)?;
    if as_json {
        let items: Vec<Value> = answer
            .items
            .iter()
            .map(|item| {
                json!({
                    "kind": item.kind,
                    "addr": item.addr,
                    "text": item.text,
                    "functions": item.functions,
                })
            })
            .collect();
        return json_result(&json!({
            "query": answer.query,
            "intent": answer.intent,
            "items": items,
            "count": items.len(),
            "truncated": answer.truncated,
        }));
    }
    println!(
        "query '{}' -> {} match(es) [intent: {}]",
        answer.query,
        answer.items.len(),
        answer.intent
    );
    for item in &answer.items {
        if item.functions.is_empty() {
            println!("  {:<8} {} {}", item.kind, item.addr, item.text);
        } else {
            println!(
                "  {:<8} {} {}  <- {}",
                item.kind,
                item.addr,
                item.text,
                item.functions.join(", ")
            );
        }
    }
    if answer.truncated {
        println!("  (truncated)");
    }
    Ok(())
}

fn cmd_ai_polish(project: &Path, addr: Option<&str>, as_json: bool, no_cache: bool) -> Result<()> {
    let mut session = Session::open(project)?;
    let addr = match addr {
        Some(raw) => resolve_address(&session, raw)?,
        None => session.entry_point(),
    };
    let mut settings = recraft_ai::ProviderSettings::from_env();
    // Reasoning models spend most of this budget on hidden reasoning before the answer.
    settings.max_tokens = settings.max_tokens.max(65536);
    if !settings.enabled {
        return Err(recraft_common::Error::unsupported(
            "pseudo-code polish needs an LLM provider; set RECRAFT_AI_PROVIDER, \
             RECRAFT_AI_ENDPOINT and the API key env var (see `recraft ai provider`)",
        ));
    }
    let mut provider = recraft_ai::provider_from_settings(&settings);
    let prompt = recraft_ai::build_polish_prompt(&mut session, addr, &settings)?;
    let template = recraft_ai::PROMPT_TEMPLATE_VERSION;
    let cache_key = recraft_ai::AiCache::key(&prompt, provider.name(), &settings.model, template);
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
            "addr": format!("{:#x}", addr.get()),
            "provider": provider.name(),
            "model": settings.model,
            "template": template,
            "cached": cached,
            "polished": response,
            "note": "suggestion only; the project is not modified",
        }));
    }
    println!(
        "// AI polish suggestion (provider={}, model={}, cached={cached})",
        provider.name(),
        settings.model
    );
    println!("// The project is not modified; L1 output stays authoritative.");
    println!("{response}");
    Ok(())
}

/// Project-level agent: the model answers by calling read-only tools.
fn cmd_ai_ask(project: &Path, question: &str, as_json: bool, rounds: usize) -> Result<()> {
    let settings = require_ai_provider("ai ask")?;
    let mut session = Session::open(project)?;
    let mut trace: Vec<serde_json::Value> = Vec::new();
    let mut answer = String::new();
    let mut turn_text = String::new();
    let result =
        recraft_ai::run_agent(
            &mut session,
            &settings,
            question,
            rounds,
            &mut |event| match event {
                recraft_ai::AgentEvent::Tool { name, arguments } => {
                    if !as_json {
                        eprintln!("  → {name} {arguments}");
                    }
                    trace.push(json!({"name": name, "arguments": arguments}));
                }
                recraft_ai::AgentEvent::TurnStart => turn_text.clear(),
                recraft_ai::AgentEvent::Delta(chunk) => turn_text.push_str(&chunk),
                recraft_ai::AgentEvent::Retract => {
                    // Narration before a tool call, not part of the answer: show it as trace
                    // context and drop it from the answer.
                    if !as_json && !turn_text.trim().is_empty() {
                        eprintln!("  … {}", recraft_ai::truncate_text(turn_text.trim(), 160));
                    }
                    turn_text.clear();
                }
                recraft_ai::AgentEvent::Answer(text) => answer = text,
            },
        );
    let answer = match result {
        Ok(text) => text,
        Err(error) => {
            if as_json {
                return json_result(&json!({
                    "provider": settings.describe(),
                    "question": question,
                    "tools": trace,
                    "error": error.to_string(),
                }));
            }
            return Err(error);
        }
    };
    if as_json {
        json_result(&json!({
            "provider": settings.describe(),
            "question": question,
            "answer": answer,
            "tools": trace,
        }))
    } else {
        println!();
        println!("{answer}");
        Ok(())
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

/// Load the curated alias manifest: symbol -> acceptable alternative names.
fn load_name_aliases(path: Option<&Path>) -> Result<recraft_ai::AliasTable> {
    let Some(path) = path else {
        return Ok(recraft_ai::AliasTable::new());
    };
    let text = std::fs::read_to_string(path).map_err(|error| {
        recraft_common::Error::invalid_argument(format!(
            "cannot read aliases {}: {error}",
            path.display()
        ))
    })?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|error| {
        recraft_common::Error::invalid_argument(format!("aliases JSON: {error}"))
    })?;
    let object = value.as_object().ok_or_else(|| {
        recraft_common::Error::invalid_argument("aliases JSON must be an object of arrays")
    })?;
    let mut aliases = recraft_ai::AliasTable::new();
    for (symbol, names) in object {
        let names: Vec<String> = names
            .as_array()
            .ok_or_else(|| {
                recraft_common::Error::invalid_argument(format!(
                    "aliases for {symbol:?} must be an array of strings"
                ))
            })?
            .iter()
            .filter_map(|name| name.as_str().map(str::to_owned))
            .collect();
        if !names.is_empty() {
            aliases.insert(symbol.clone(), names);
        }
    }
    Ok(aliases)
}

/// Naming-accuracy benchmark: score proposals against symbols the binary still carries.
fn cmd_ai_eval_names(
    project: &Path,
    as_json: bool,
    llm: bool,
    limit: usize,
    alias_path: Option<&Path>,
) -> Result<()> {
    let mut session = Session::open(project)?;
    let aliases = load_name_aliases(alias_path)?;
    let report = if llm {
        let settings = require_ai_provider("ai eval-names")?;
        recraft_ai::evaluate_names_with_provider(&mut session, &settings, limit, &aliases)?
    } else {
        recraft_ai::evaluate_names_heuristic(&mut session, limit, &aliases)?
    };
    if as_json {
        let rows: Vec<_> = report
            .cases
            .iter()
            .map(|case| {
                json!({
                    "addr": format!("{:#x}", case.addr.get()),
                    "truth": case.truth,
                    "proposal": case.proposal,
                    "source": case.source,
                    "exact": case.score.exact,
                    "accepted_by": case.accepted_by,
                    "partial": case.score.partial,
                    "overlap": case.score.overlap,
                    "apis": case.apis,
                    "strings": case.strings,
                })
            })
            .collect();
        return json_result(&json!({
            "eval": report.version,
            "mode": report.mode.as_str(),
            "template": report.template,
            "cases": report.total(),
            "exact": report.exact,
            "accepted": report.accepted,
            "partial": report.partial,
            "missing": report.missing,
            "mean_overlap": report.mean_overlap,
            "rows": rows,
        }));
    }
    println!("{}", report.summary());
    if report.cases.is_empty() {
        println!(
            "  (no function carries a real symbol; import a binary with exports or build \
             corpus/naming first)"
        );
        return Ok(());
    }
    for case in &report.cases {
        let mark = if case.score.exact {
            "=="
        } else if case.score.partial {
            "~ "
        } else {
            "  "
        };
        println!(
            "  {mark} {:#x}  {:<26} -> {:<32} overlap {:.2}  apis={} strings={}",
            case.addr.get(),
            case.truth,
            if case.proposal.is_empty() {
                "<missing>"
            } else {
                case.proposal.as_str()
            },
            case.score.overlap,
            case.apis,
            case.strings
        );
    }
    Ok(())
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
        // The offline report needs no provider; only the model path does.
        let settings = require_ai_provider("ai explain")?;
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

fn cmd_ai_suggest(
    project: &Path,
    as_json: bool,
    limit: usize,
    out: Option<&Path>,
    llm: bool,
) -> Result<()> {
    let mut session = Session::open(project)?;
    let mut proposals = recraft_ai::suggest_names(&mut session, limit)?;
    if llm && !proposals.is_empty() {
        // The evidence-based proposals need no provider; only the model path does.
        let settings = require_ai_provider("ai suggest")?;
        let mut provider = recraft_ai::provider_from_settings(&settings);
        let candidates: Vec<Address> = proposals.iter().map(|proposal| proposal.target).collect();
        let prompt = recraft_ai::build_rename_prompt(&mut session, &candidates, &settings)?;
        let reply = provider.complete(&prompt)?;
        let updated = recraft_ai::apply_rename_reply(&mut proposals, &reply);
        if updated == 0 {
            eprintln!(
                "warning: the model reply had no usable names; keeping the evidence-based proposals"
            );
        }
    }
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
        let command = recraft_core::parse_command(&session, &virtual_names, entry)
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
                    recraft_analysis::FunctionSource::UserRange => "range",
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
    let session = Session::open(project)?;
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
    let cfg = recraft_analysis::build_cfg(&session, function.start, function.end)?;

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

/// Rendering options for `decompile`, grouped so the function keeps a readable signature.
struct DecompileOptions<'a> {
    addr: Option<&'a str>,
    max_insns: usize,
    as_json: bool,
    with_addresses: bool,
    structured: bool,
    l2: bool,
    out_file: Option<PathBuf>,
}

fn cmd_decompile(project: &Path, options: DecompileOptions<'_>) -> Result<()> {
    let DecompileOptions {
        addr,
        max_insns,
        as_json,
        with_addresses,
        structured,
        l2,
        out_file,
    } = options;
    let session = Session::open(project)?;
    let start = match addr {
        Some(raw) => resolve_address(&session, raw)?,
        None => session.entry_point(),
    };
    let containing = session
        .functions()?
        .iter()
        .find(|function| start >= function.start && start < function.end)
        .map(|function| function.start);
    // Structured output is per function: asking for it at an address that no function covers is a
    // bad request, not something to render as a one-off listing.
    if containing.is_none() && (structured || l2) {
        return Err(Error::invalid_argument(format!(
            "no analysed function contains {:#x}",
            start.get()
        )));
    }
    let function_start = containing.unwrap_or(start);
    let code = if l2 {
        recraft_decompile::pseudocode_l2(&session, function_start)?
    } else if structured {
        recraft_decompile::pseudocode_l1(&session, function_start)?
    } else {
        pseudocode_l0(&session, start, max_insns)?
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
        // The jump count and the structured flag are what the L2 header prints; exposing them
        // lets a caller sort functions by "how far from structured" without re-parsing the text.
        let gotos = code
            .lines
            .iter()
            .filter(|line| line.text.contains("goto loc_"))
            .count();
        return json_result(&json!({
            "start": format!("{:#x}", code.func_start.get()),
            "gotos": gotos,
            "structured": gotos == 0,
            "lines": lines,
        }));
    }

    if let Some(path) = out_file {
        // Rendering to a file keeps the terminal free and lets scripts diff the output directly.
        let mut text = String::new();
        for line in &code.lines {
            if with_addresses {
                text.push_str(&format!("{:#012x}  {}\n", line.addr.get(), line.text));
            } else {
                text.push_str(&line.text);
                text.push('\n');
            }
        }
        std::fs::write(&path, text).map_err(|error| {
            Error::invalid_argument(format!("cannot write {}: {error}", path.display()))
        })?;
        eprintln!("wrote {} lines to {}", code.lines.len(), path.display());
        return Ok(());
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
    // DESIGN §6.4: a user code/data change triggers a range-scoped re-analysis.
    let stats = recraft_analysis::reanalyze_range(&mut session, range)?;
    println!(
        "marked {:#x}..{:#x} as {:?} (incremental: {} function(s), {} instruction(s))",
        range.start.get(),
        range.end.get(),
        code_data,
        stats.functions.len(),
        stats.instructions
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

/// The name a project gave a function, if it is more than the default `sub_…` spelling.
fn user_name(session: &Session, addr: Address) -> Result<Option<String>> {
    recraft_decompile::signature::user_name(session, addr)
}

/// Write the signature of every named function to a JSON file.
fn cmd_signature_export(project: &Path, out: &Path) -> Result<()> {
    use recraft_decompile::signature;

    let mut session = Session::open(project)?;
    let functions = session.functions()?;
    let mut entries: Vec<serde_json::Value> = Vec::new();
    for function in &functions {
        let Some(name) = user_name(&session, function.start)? else {
            continue;
        };
        let Some(signature) = signature::build(&mut session, function.start, function.end, &name)
        else {
            continue;
        };
        entries.push(serde_json::json!({
            "name": signature.name,
            "pattern": signature.render(),
        }));
    }
    let document = serde_json::json!({ "format": 1, "signatures": entries });
    let text = serde_json::to_string_pretty(&document)
        .map_err(|error| Error::internal(format!("signature JSON: {error}")))?;
    std::fs::write(out, text)?;
    println!("wrote {} signatures to {}", entries.len(), out.display());
    Ok(())
}

/// Name functions that match a signature file; a name the project already has wins.
fn cmd_signature_import(
    project: &Path,
    from: &[PathBuf],
    builtin: bool,
    dry_run: bool,
) -> Result<()> {
    use recraft_decompile::signature::Signature;

    // A library is usually several exports: accept directories and load their `*.json` files
    // in a stable order so the same set always produces the same names.
    let mut files: Vec<PathBuf> = Vec::new();
    for path in from {
        if path.is_dir() {
            let mut found: Vec<PathBuf> = std::fs::read_dir(path)?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
                .collect();
            found.sort();
            files.extend(found);
        } else {
            files.push(path.clone());
        }
    }
    if files.is_empty() && !builtin {
        return Err(Error::invalid_argument(
            "pass --from <file|dir> or --builtin".to_owned(),
        ));
    }
    let mut signatures: Vec<Signature> = Vec::new();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // The first source wins: a later file cannot rename a signature already in the set.
    let mut load = |text: &str, origin: &str| -> Result<()> {
        let document: serde_json::Value = serde_json::from_str(text)
            .map_err(|error| Error::invalid_argument(format!("{origin}: {error}")))?;
        let Some(entries) = document
            .get("signatures")
            .and_then(|value| value.as_array())
        else {
            return Err(Error::unsupported(format!(
                "{origin} has no `signatures` array"
            )));
        };
        for entry in entries {
            let (Some(name), Some(pattern)) = (
                entry.get("name").and_then(|value| value.as_str()),
                entry.get("pattern").and_then(|value| value.as_str()),
            ) else {
                continue;
            };
            if !seen.insert(name.to_owned()) {
                continue;
            }
            if let Some(signature) = Signature::parse(name, pattern) {
                signatures.push(signature);
            }
        }
        Ok(())
    };
    for file in &files {
        let text = std::fs::read_to_string(file)?;
        load(&text, &file.display().to_string())?;
    }
    if builtin {
        load(
            recraft_decompile::signature::BUILTIN_SIGNATURES,
            "built-in signatures",
        )?;
    }

    let mut session = Session::open(project)?;
    let report = recraft_decompile::signature::apply(&mut session, &signatures, dry_run)?;
    for (addr, name) in &report.renamed {
        println!("{:#x} -> {name}", addr.get());
    }
    println!(
        "matched {} function(s) from {} signature(s), {} name(s) already taken{}",
        report.renamed.len(),
        signatures.len(),
        report.skipped,
        if dry_run { " (dry run)" } else { "" }
    );
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
    fn recognizes_closed_pipe_panics() {
        assert!(is_broken_pipe_message(
            "failed printing to stdout: Broken pipe (os error 32)"
        ));
        assert!(is_broken_pipe_message(
            "failed printing to stdout: The pipe has been ended. (os error 109)"
        ));
        assert!(is_broken_pipe_message(
            "failed printing to stdout: 管道正在被关闭。 (os error 232)"
        ));
    }

    #[test]
    fn leaves_other_panics_alone() {
        assert!(!is_broken_pipe_message("index out of bounds"));
        assert!(!is_broken_pipe_message(
            "failed printing to stdout: No space left on device (os error 28)"
        ));
        // Only stdout print failures are treated as the pipe case.
        assert!(!is_broken_pipe_message("os error 32"));
    }

    #[test]
    fn decbench_helpers_read_kit_addresses_and_identifiers() {
        assert_eq!(parse_hex_address("0x401000"), Some(0x40_1000));
        assert_eq!(parse_hex_address("401000"), Some(0x40_1000));
        assert_eq!(parse_hex_address("0x"), None);

        let body =
            "// L2 structure recovery (1 blocks)\nint32 sub_1030(void) {\n    return 1;\n}\n";
        assert_eq!(function_identifier(body).as_deref(), Some("sub_1030"));
        assert_eq!(
            function_identifier("int32 *ptr_fn(void) {"),
            Some("ptr_fn".to_owned())
        );
        // Names that are not C identifiers are skipped rather than reported.
        assert_eq!(function_identifier("int32 recraft::square(void) {"), None);
    }

    #[test]
    fn the_builtin_signatures_parse() {
        let document: serde_json::Value =
            serde_json::from_str(recraft_decompile::signature::BUILTIN_SIGNATURES).unwrap();
        let entries = document
            .get("signatures")
            .and_then(|value| value.as_array())
            .unwrap();
        assert!(entries.len() >= 8, "{}", entries.len());
        let mut names = std::collections::BTreeSet::new();
        for entry in entries {
            let name = entry.get("name").and_then(|value| value.as_str()).unwrap();
            let pattern = entry
                .get("pattern")
                .and_then(|value| value.as_str())
                .unwrap();
            assert!(names.insert(name), "duplicate {name}");
            assert!(
                recraft_decompile::signature::Signature::parse(name, pattern).is_some(),
                "{name}: {pattern}"
            );
        }
    }

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
