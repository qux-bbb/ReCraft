// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Local variable recovery for the typed (L2) rendering.
//!
//! The emitter works on machine state, so its statements name registers (`eax = r8d`,
//! `*(u64*)(rcx)`). Source code names variables instead. This pass rewrites the finished
//! statement text: it recovers the value flow of every register over the function's
//! control-flow graph (including `goto`/labels, loops and calls), groups the definitions
//! that can observe one another into one local, and prints those locals with a recovered
//! width. A local is declared at the top of the body, so the reader follows `v3` rather
//! than `r8d` across a block boundary.
//!
//! The pass is deliberately conservative:
//!
//! * widths are tracked in bytes; a value read wider than it was written is widened, and a
//!   narrow read becomes an explicit cast, so `(uint32)v3` still says "low 32 bits";
//! * 8/16-bit writes (and the high-byte registers) preserve the rest of the register, so
//!   they are modelled as a read-modify-write of the previous value;
//! * calls return in `rax`; a call whose result is actually read becomes an assignment
//!   (`v4 = GetLastError();`), while a dead call stays a statement;
//! * an `__asm` statement has effects the lifter did not model. The values such an
//!   instruction writes keep their register names until a normal assignment overwrites them
//!   (the printed assembly and the following statements then agree); every other value in
//!   that function is still recovered;
//! * when the emitted text is not valid C (the structurer has known shapes that drop a
//!   brace), a straight-line fallback renames only definitions and uses that sit in the same
//!   basic run; it never carries a value across a branch, loop edge or label.
//!
//! Anything the analysis cannot prove is left byte-for-byte as the emitter wrote it, so the
//! pass can never make the output worse than the emitter's.

use std::collections::{HashMap, HashSet};

use recraft_ir::Reg;

use crate::PseudoLine;

/// One entry per `Reg` value (16 GPRs followed by 16 XMM registers).
const REGS: usize = 32;
const NONE: usize = usize::MAX;

/// Recover locals in the L2 body of `lines` (signature included).
///
/// Returns `true` when the body was rewritten. When the control-flow graph cannot be
/// recovered exactly (the emitter printed a shape the parser does not prove, or the text is
/// not valid C), a straight-line fallback still names values whose definition and use sit in
/// the same basic run; the text is never left worse than the emitter's.
pub(crate) fn recover_variables(lines: &mut Vec<PseudoLine>, is_64: bool) -> bool {
    if recover_structured(lines, is_64) || recover_linear(lines, is_64) {
        strip_parameter_registers(lines);
        return true;
    }
    false
}

/// The parameter comment names the argument registers; once the body has locals it is both
/// redundant and the last place a register name survives in the typed view.
fn strip_parameter_registers(lines: &mut [PseudoLine]) {
    for line in lines {
        let trimmed = line.text.trim_start();
        let Some(rest) = trimmed.strip_prefix("// L2 params:") else {
            continue;
        };
        let prefix = &line.text[..line.text.len() - trimmed.len()];
        let parts: Vec<String> = rest
            .trim_start()
            .split(", ")
            .map(|part| {
                if part.contains("+=") {
                    return part.to_owned();
                }
                match part.split_once('=') {
                    Some((name, value)) if contains_register_name(value) => name.to_owned(),
                    _ => part.to_owned(),
                }
            })
            .collect();
        line.text = format!("{prefix}// L2 params: {}", parts.join(", "));
    }
}

fn recover_structured(lines: &mut Vec<PseudoLine>, is_64: bool) -> bool {
    let Some(signature) = lines.iter().position(|line| signature_line(&line.text)) else {
        return false;
    };
    if lines.last().map(|line| line.text.trim()) != Some("}") {
        return false;
    }
    let body_end = lines.len() - 1;
    if body_end <= signature + 1 {
        return false;
    }
    // Parsing borrows the text; it ends before anything is written back.
    let items = {
        let mut parser = Parser {
            lines,
            pos: signature + 1,
            end: body_end,
        };
        let Some(items) = parser.parse_block(Stop::Top) else {
            return false;
        };
        if parser.pos != body_end {
            return false;
        }
        items
    };
    let effects = asm_effects_of(&lines[signature + 1..body_end], signature + 1);
    let mut builder = Builder::new();
    let exit = builder.push(NONE, 0, 0, NodeKind::Exit);
    let entry = builder.flatten(&items, exit, &mut Vec::new());
    if builder.broken || !builder.resolve_gotos() {
        return false;
    }
    builder.scan(lines, is_64, &effects);
    if !builder
        .nodes
        .iter()
        .any(|node| !node.reads.is_empty() || node.write.is_some())
    {
        return false;
    }
    let Some(program) = Program::analyze(builder.nodes, entry, is_64, lines) else {
        return false;
    };
    let Some(body) = program.render(lines) else {
        return false;
    };
    let mut out: Vec<PseudoLine> = Vec::with_capacity(lines.len() + program.declarations.len());
    out.extend_from_slice(&lines[..=signature]);
    for declaration in &program.declarations {
        out.push(PseudoLine {
            addr: lines[signature].addr,
            text: declaration.clone(),
        });
    }
    out.extend_from_slice(&body[signature + 1..]);
    *lines = out;
    true
}

// ---------------------------------------------------------------------------
// Straight-line fallback
// ---------------------------------------------------------------------------

/// One line of the fallback plan.
enum LinearKind {
    /// A read-only line (an `if`/`switch` condition, a `return`, …); the environment is
    /// cleared afterwards because control leaves the straight-line run here.
    Control(Vec<LinearRead>),
    Scalar {
        reads: Vec<LinearRead>,
        write: Option<LinearWrite>,
        call: Option<LinearCall>,
    },
}

struct LinearRead {
    start: usize,
    end: usize,
    var: usize,
    width: u8,
    signed: bool,
}

struct LinearWrite {
    write: WriteOcc,
    var: usize,
    prev: Option<usize>,
}

struct LinearCall {
    span: (usize, usize),
    var: usize,
}

#[derive(Clone, Copy)]
struct LinearVar {
    width: u8,
    signed: u32,
    unsigned: u32,
    reads: u32,
}

/// Recover locals for a function whose control flow could not be parsed exactly.
///
/// Only values whose definition and use sit in the same straight-line run are renamed; any
/// branch, loop header, label, `goto` or `return` ends the run and clears the environment.
/// That makes the rewrite exact without needing a correct CFG: no value crosses a join.
fn recover_linear(lines: &mut Vec<PseudoLine>, is_64: bool) -> bool {
    let Some(signature) = lines.iter().position(|line| signature_line(&line.text)) else {
        return false;
    };
    if lines.last().map(|line| line.text.trim()) != Some("}") {
        return false;
    }
    let body_end = lines.len() - 1;
    if body_end <= signature + 1 {
        return false;
    }
    let mut vars: Vec<LinearVar> = Vec::new();
    let mut names: Vec<Option<String>> = Vec::new();
    let mut pools: HashMap<(u8, bool), Vec<String>> = HashMap::new();
    let mut declarations: Vec<(usize, String)> = Vec::new();
    let mut next_name = 1usize;
    let mut env = [NONE; REGS];
    let mut run_start = 0usize;
    let mut plans: Vec<LinearKind> = Vec::with_capacity(body_end - signature - 1);
    let mut changed = false;

    for line in lines.iter().take(body_end).skip(signature + 1) {
        let text = line.text.as_str();
        let trimmed = text.trim();
        if trimmed.starts_with("//") {
            plans.push(LinearKind::Control(Vec::new()));
            continue;
        }
        if !is_control_line(trimmed) {
            if !trimmed.ends_with(';') {
                // An unknown line: end the run rather than guess.
                finish_linear_run(
                    &vars,
                    &mut names,
                    &mut pools,
                    &mut declarations,
                    run_start,
                    &mut next_name,
                );
                run_start = vars.len();
                plans.push(LinearKind::Control(Vec::new()));
                env = [NONE; REGS];
                continue;
            }
            let mut reads = Vec::new();
            let mut write = None;
            let mut call = None;
            let tokens: Vec<Token> = scan_tokens(text);
            let signed = text.contains("/* signed */");
            if let Some(assignment) = split_assignment(text) {
                let lhs = assignment.lhs;
                let rhs = assignment.rhs;
                let lhs_tokens: Vec<Token> = tokens
                    .iter()
                    .copied()
                    .filter(|token| token.start >= lhs.0 && token.end <= lhs.1)
                    .collect();
                let lhs_is_register = lhs_tokens.len() == 1
                    && lhs_tokens[0].start == lhs.0
                    && lhs_tokens[0].end == lhs.1;
                if lhs_is_register {
                    let token = lhs_tokens[0];
                    let full = full_width(token.reg, is_64);
                    let partial = is_partial(token.width, full);
                    let previous = env[token.reg as usize];
                    let compound = assignment.compound;
                    // A compound assignment reads and writes the same value, so it keeps the
                    // name it had. A fresh name would print an uninitialised `v2 -= x`.
                    let var = if compound.is_some() {
                        previous
                    } else {
                        let var = vars.len();
                        vars.push(LinearVar {
                            width: token.width,
                            signed: 0,
                            unsigned: 0,
                            reads: 0,
                        });
                        names.push(None);
                        var
                    };
                    if var != NONE {
                        if partial && previous != NONE {
                            let prev = &mut vars[previous];
                            prev.width = prev.width.max(full);
                            prev.unsigned += 1;
                            prev.reads += 1;
                        }
                        if compound.is_some() && previous != NONE {
                            reads.push(LinearRead {
                                start: token.start,
                                end: token.end,
                                var: previous,
                                width: token.width,
                                signed,
                            });
                            let var = &mut vars[previous];
                            var.width = var.width.max(token.width);
                            if signed {
                                var.signed += 1;
                            } else {
                                var.unsigned += 1;
                            }
                            var.reads += 1;
                        }
                        write = Some(LinearWrite {
                            write: WriteOcc {
                                start: token.start,
                                reg: token.reg,
                                width: token.width,
                                offset: token.offset,
                                partial,
                                compound,
                                lhs,
                                rhs,
                                signed,
                                prev: NONE,
                            },
                            var,
                            prev: if partial && previous != NONE {
                                Some(previous)
                            } else {
                                None
                            },
                        });
                        env[token.reg as usize] = var;
                    } else {
                        env[token.reg as usize] = NONE;
                    }
                }
                for token in tokens {
                    if lhs_is_register
                        && token.start == lhs_tokens[0].start
                        && token.end == lhs_tokens[0].end
                    {
                        continue;
                    }
                    if let Some(var) = read_var(&mut vars, &env, token, signed) {
                        reads.push(LinearRead {
                            start: token.start,
                            end: token.end,
                            var,
                            width: token.width,
                            signed,
                        });
                    }
                }
            } else {
                for token in tokens {
                    if let Some(var) = read_var(&mut vars, &env, token, signed) {
                        reads.push(LinearRead {
                            start: token.start,
                            end: token.end,
                            var,
                            width: token.width,
                            signed,
                        });
                    }
                }
                if let Some(span) = call_expression(text) {
                    let var = vars.len();
                    vars.push(LinearVar {
                        width: 0,
                        signed: 0,
                        unsigned: 0,
                        reads: 0,
                    });
                    names.push(None);
                    env[Reg::Rax as usize] = var;
                    call = Some(LinearCall { span, var });
                }
            }
            if !reads.is_empty() || write.is_some() || call.is_some() {
                changed = true;
            }
            plans.push(LinearKind::Scalar { reads, write, call });
            continue;
        }
        // A control line ends the run. Conditions that execute on the path into the run are
        // renamed with the current environment; loop conditions are left alone because they
        // run again after the body modified the registers.
        let mut reads = Vec::new();
        let renames_condition = (trimmed.starts_with("if (") && !trimmed.starts_with("} else"))
            || trimmed.starts_with("switch (")
            || trimmed.starts_with("return");
        if renames_condition {
            let signed = text.contains("/* signed */");
            for token in scan_tokens(text) {
                if let Some(var) = read_var(&mut vars, &env, token, signed) {
                    reads.push(LinearRead {
                        start: token.start,
                        end: token.end,
                        var,
                        width: token.width,
                        signed,
                    });
                }
            }
        }
        finish_linear_run(
            &vars,
            &mut names,
            &mut pools,
            &mut declarations,
            run_start,
            &mut next_name,
        );
        run_start = vars.len();
        env = [NONE; REGS];
        plans.push(LinearKind::Control(reads));
    }
    finish_linear_run(
        &vars,
        &mut names,
        &mut pools,
        &mut declarations,
        run_start,
        &mut next_name,
    );
    if !changed {
        return false;
    }
    declarations.sort_by_key(|(number, _)| *number);
    declarations.dedup_by(|left, right| left.1 == right.1);

    let mut out: Vec<PseudoLine> = Vec::with_capacity(lines.len() + declarations.len());
    out.extend_from_slice(&lines[..=signature]);
    for (_, declaration) in &declarations {
        out.push(PseudoLine {
            addr: lines[signature].addr,
            text: declaration.clone(),
        });
    }
    for (offset, plan) in plans.iter().enumerate() {
        let line_index = signature + 1 + offset;
        let text = lines[line_index].text.as_str();
        let rendered = match plan {
            LinearKind::Control(reads) => apply_linear_reads(text, reads, &vars, &names),
            LinearKind::Scalar { reads, write, call } => {
                let mut edits = linear_edits(text, reads, &vars, &names);
                let mut rendered = match write {
                    Some(write) => {
                        let name = names[write.var].clone();
                        match name {
                            Some(name) => {
                                let rhs = render_range(
                                    text,
                                    write.write.rhs.0,
                                    write.write.rhs.1,
                                    &mut edits,
                                );
                                let prev = write.prev.and_then(|prev| names[prev].clone());
                                render_write_line(
                                    text,
                                    &write.write,
                                    &name,
                                    vars[write.var].width,
                                    vars[write.var].signed > 0 && vars[write.var].unsigned == 0,
                                    prev.as_deref(),
                                    &rhs,
                                    is_64,
                                )
                            }
                            None => apply_edits(text, &mut edits),
                        }
                    }
                    None => apply_edits(text, &mut edits),
                };
                if let Some(call) = call {
                    if let Some(name) = names[call.var].clone() {
                        let at = call.span.0.min(rendered.len());
                        rendered.insert_str(at, &format!("{name} = "));
                    }
                }
                rendered
            }
        };
        out.push(PseudoLine {
            addr: lines[line_index].addr,
            text: rendered,
        });
    }
    out.push(lines[body_end].clone());
    *lines = out;
    true
}

fn finish_linear_run(
    vars: &[LinearVar],
    names: &mut [Option<String>],
    pools: &mut HashMap<(u8, bool), Vec<String>>,
    declarations: &mut Vec<(usize, String)>,
    run_start: usize,
    next_name: &mut usize,
) {
    let mut assigned: Vec<((u8, bool), String)> = Vec::new();
    for id in run_start..vars.len() {
        let var = vars[id];
        if var.reads == 0 {
            continue;
        }
        let signed = var.signed > 0 && var.unsigned == 0;
        let key = (var.width, signed);
        let name = match pools.get_mut(&key).and_then(|pool| pool.pop()) {
            Some(name) => name,
            None => {
                let name = format!("v{}", *next_name);
                *next_name += 1;
                name
            }
        };
        let number: usize = name[1..].parse().unwrap_or(0);
        declarations.push((
            number,
            format!("    {} {};", type_name(var.width, signed), name),
        ));
        names[id] = Some(name.clone());
        assigned.push((key, name));
    }
    // The run is over, so these names are free for a later run of the same type. Returning
    // them keeps the declaration list proportional to the live values, not to the whole
    // function.
    for (key, name) in assigned {
        pools.entry(key).or_default().push(name);
    }
}

fn is_control_line(trimmed: &str) -> bool {
    trimmed.starts_with("if (")
        || trimmed.starts_with("} else")
        || trimmed.starts_with("while (")
        || trimmed.starts_with("do {")
        || trimmed.starts_with("} while")
        || trimmed.starts_with("switch (")
        || trimmed.starts_with("case ")
        || trimmed.starts_with("default")
        || trimmed.starts_with("goto ")
        || trimmed == "break;"
        || trimmed == "continue;"
        || trimmed.starts_with("return")
        || trimmed.starts_with("__asm(")
        || trimmed.starts_with('}')
        || trimmed.ends_with(':')
}

/// Record a read of `token` if the register currently holds a named value.
fn read_var(
    vars: &mut [LinearVar],
    env: &[usize; REGS],
    token: Token,
    signed: bool,
) -> Option<usize> {
    let var = env[token.reg as usize];
    if var == NONE {
        return None;
    }
    let meta = &mut vars[var];
    meta.width = meta.width.max(token.width);
    if signed {
        meta.signed += 1;
    } else {
        meta.unsigned += 1;
    }
    meta.reads += 1;
    Some(var)
}

fn linear_edits(
    text: &str,
    reads: &[LinearRead],
    vars: &[LinearVar],
    names: &[Option<String>],
) -> Vec<Edit> {
    let mut edits = Vec::with_capacity(reads.len());
    for read in reads {
        let Some(name) = &names[read.var] else {
            continue;
        };
        let var = vars[read.var];
        let signed = var.signed > 0 && var.unsigned == 0;
        let replacement = if read.width >= var.width && (signed == read.signed || read.width == 16)
        {
            name.clone()
        } else {
            format!("({}){name}", type_name(read.width, read.signed))
        };
        let _ = text;
        edits.push(Edit {
            start: read.start,
            end: read.end,
            text: replacement,
            order: 1,
        });
    }
    edits
}

fn apply_linear_reads(
    text: &str,
    reads: &[LinearRead],
    vars: &[LinearVar],
    names: &[Option<String>],
) -> String {
    let mut edits = linear_edits(text, reads, vars, names);
    apply_edits(text, &mut edits)
}

/// The signature line of a rendered function (`… ) {`), ignoring comments.
fn signature_line(text: &str) -> bool {
    let trimmed = text.trim_start();
    !trimmed.starts_with("//") && text.trim_end().ends_with(") {")
}

fn is_xmm(reg: Reg) -> bool {
    (reg as usize) >= 16
}

fn contains_register_name(text: &str) -> bool {
    scan_tokens(text).iter().any(|token| !is_xmm(token.reg))
}

/// The register effects of every `__asm` statement, in source order.
///
/// The lifter prints an `__asm` when it cannot model an instruction, so the printed text is
/// the only description of what happened. A small table recovers the common cases; anything
/// it does not know is treated as clobbering every general-purpose register, which keeps the
/// following values register-named instead of inventing a stale local.
fn asm_effects_of(lines: &[PseudoLine], offset: usize) -> HashMap<usize, (Vec<Reg>, Vec<Reg>)> {
    let mut effects = HashMap::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.text.trim_start();
        if trimmed.starts_with("__asm(") {
            effects.insert(offset + index, asm_effects(trimmed));
        }
    }
    effects
}

fn asm_effects(text: &str) -> (Vec<Reg>, Vec<Reg>) {
    let body = text
        .strip_prefix("__asm(")
        .and_then(|rest| rest.split('"').nth(1))
        .unwrap_or("");
    let mut reads: Vec<Reg> = Vec::new();
    for token in scan_tokens(body) {
        if !reads.contains(&token.reg) {
            reads.push(token.reg);
        }
    }
    let lower = body.trim_start();
    let lower = lower.strip_prefix("lock ").unwrap_or(lower).trim_start();
    let mnemonic = lower
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .next()
        .unwrap_or("");
    let operands = lower
        .split_once(' ')
        .map(|(_, rest)| rest)
        .unwrap_or("")
        .trim();
    let first_register = scan_tokens(operands).first().map(|token| token.reg);
    let mut writes: Vec<Reg> = Vec::new();
    let mut push = |reg: Reg| {
        if !writes.contains(&reg) {
            writes.push(reg);
        }
    };
    match mnemonic {
        "cmpxchg" => push(Reg::Rax),
        "xadd" | "xchg" | "mov" | "movzx" | "movsx" | "movsxd" | "bswap" => {
            if let Some(reg) = first_register {
                push(reg);
            }
        }
        "rdtsc" => {
            push(Reg::Rax);
            push(Reg::Rdx);
        }
        "rdtscp" => {
            push(Reg::Rax);
            push(Reg::Rdx);
            push(Reg::Rcx);
        }
        "cpuid" => {
            push(Reg::Rax);
            push(Reg::Rbx);
            push(Reg::Rcx);
            push(Reg::Rdx);
        }
        "xgetbv" => {
            push(Reg::Rax);
            push(Reg::Rdx);
        }
        "mul" | "div" | "idiv" => {
            push(Reg::Rax);
            push(Reg::Rdx);
        }
        "imul" => {
            if operands.contains(',') {
                if let Some(reg) = first_register {
                    push(reg);
                }
            } else {
                push(Reg::Rax);
                push(Reg::Rdx);
            }
        }
        "stos" => {
            push(Reg::Rdi);
            push(Reg::Rcx);
        }
        "movs" | "lods" => {
            push(Reg::Rsi);
            push(Reg::Rdi);
            push(Reg::Rcx);
        }
        "scas" => {
            push(Reg::Rdi);
            push(Reg::Rcx);
        }
        "push" | "pop" => push(Reg::Rsp),
        "cmova" | "cmovae" | "cmovb" | "cmovbe" | "cmove" | "cmovg" | "cmovge" | "cmovl"
        | "cmovle" | "cmovne" | "cmovns" | "cmovs" => {
            // Conditional writes preserve the destination, which the merge model does not
            // express; keeping the destination register-named is exact.
            if let Some(reg) = first_register {
                push(reg);
            }
        }
        "if" | "jmp" | "call" | "ret" | "nop" | "int" | "int3" => {}
        "prefetchw" | "prefetch" | "prefetchnta" | "prefetcht0" | "prefetcht1" | "prefetcht2" => {}
        _ => {
            // Unknown instruction: assume every GPR changed.
            for reg in 0..16 {
                push(reg_from_index(reg));
            }
        }
    }
    (reads, writes)
}

fn reg_from_index(index: usize) -> Reg {
    match index {
        0 => Reg::Rax,
        1 => Reg::Rcx,
        2 => Reg::Rdx,
        3 => Reg::Rbx,
        4 => Reg::Rsp,
        5 => Reg::Rbp,
        6 => Reg::Rsi,
        7 => Reg::Rdi,
        8 => Reg::R8,
        9 => Reg::R9,
        10 => Reg::R10,
        11 => Reg::R11,
        12 => Reg::R12,
        13 => Reg::R13,
        14 => Reg::R14,
        _ => Reg::R15,
    }
}

// ---------------------------------------------------------------------------
// Registers and tokens
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Token {
    start: usize,
    end: usize,
    reg: Reg,
    /// Width in bytes (1, 2, 4, 8 or 16).
    width: u8,
    /// Bit offset inside the register (8 for `ah`/`ch`/`dh`/`bh`).
    offset: u8,
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Register tokens outside strings and comments.
fn scan_tokens(text: &str) -> Vec<Token> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                    } else if bytes[i] == b'"' {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => break,
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
            }
            byte if is_ident_start(byte) => {
                let start = i;
                i += 1;
                while i < bytes.len() && is_ident_char(bytes[i]) {
                    i += 1;
                }
                if let Some((reg, width, offset)) = register_token(&text[start..i]) {
                    out.push(Token {
                        start,
                        end: i,
                        reg,
                        width,
                        offset,
                    });
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// Map a printed register spelling (`r8d`, `esi`, `xmm3`, …) to its base register and view.
fn register_token(token: &str) -> Option<(Reg, u8, u8)> {
    let named = match token {
        "rax" => (Reg::Rax, 8, 0),
        "eax" => (Reg::Rax, 4, 0),
        "ax" => (Reg::Rax, 2, 0),
        "al" => (Reg::Rax, 1, 0),
        "ah" => (Reg::Rax, 1, 8),
        "rbx" => (Reg::Rbx, 8, 0),
        "ebx" => (Reg::Rbx, 4, 0),
        "bx" => (Reg::Rbx, 2, 0),
        "bl" => (Reg::Rbx, 1, 0),
        "bh" => (Reg::Rbx, 1, 8),
        "rcx" => (Reg::Rcx, 8, 0),
        "ecx" => (Reg::Rcx, 4, 0),
        "cx" => (Reg::Rcx, 2, 0),
        "cl" => (Reg::Rcx, 1, 0),
        "ch" => (Reg::Rcx, 1, 8),
        "rdx" => (Reg::Rdx, 8, 0),
        "edx" => (Reg::Rdx, 4, 0),
        "dx" => (Reg::Rdx, 2, 0),
        "dl" => (Reg::Rdx, 1, 0),
        "dh" => (Reg::Rdx, 1, 8),
        "rsp" => (Reg::Rsp, 8, 0),
        "esp" => (Reg::Rsp, 4, 0),
        "sp" => (Reg::Rsp, 2, 0),
        "spl" => (Reg::Rsp, 1, 0),
        "rbp" => (Reg::Rbp, 8, 0),
        "ebp" => (Reg::Rbp, 4, 0),
        "bp" => (Reg::Rbp, 2, 0),
        "bpl" => (Reg::Rbp, 1, 0),
        "rsi" => (Reg::Rsi, 8, 0),
        "esi" => (Reg::Rsi, 4, 0),
        "si" => (Reg::Rsi, 2, 0),
        "sil" => (Reg::Rsi, 1, 0),
        "rdi" => (Reg::Rdi, 8, 0),
        "edi" => (Reg::Rdi, 4, 0),
        "di" => (Reg::Rdi, 2, 0),
        "dil" => (Reg::Rdi, 1, 0),
        _ => {
            if let Some(rest) = token.strip_prefix("xmm") {
                let number: u8 = rest.parse().ok()?;
                let reg = match number {
                    0 => Reg::Xmm0,
                    1 => Reg::Xmm1,
                    2 => Reg::Xmm2,
                    3 => Reg::Xmm3,
                    4 => Reg::Xmm4,
                    5 => Reg::Xmm5,
                    6 => Reg::Xmm6,
                    7 => Reg::Xmm7,
                    8 => Reg::Xmm8,
                    9 => Reg::Xmm9,
                    10 => Reg::Xmm10,
                    11 => Reg::Xmm11,
                    12 => Reg::Xmm12,
                    13 => Reg::Xmm13,
                    14 => Reg::Xmm14,
                    15 => Reg::Xmm15,
                    _ => return None,
                };
                return Some((reg, 16, 0));
            }
            let rest = token.strip_prefix('r')?;
            let at = rest.find(|ch: char| ch.is_ascii_alphabetic());
            let (digits, suffix) = match at {
                Some(at) => (&rest[..at], &rest[at..]),
                None => (rest, ""),
            };
            let number: u8 = digits.parse().ok()?;
            let reg = match number {
                8 => Reg::R8,
                9 => Reg::R9,
                10 => Reg::R10,
                11 => Reg::R11,
                12 => Reg::R12,
                13 => Reg::R13,
                14 => Reg::R14,
                15 => Reg::R15,
                _ => return None,
            };
            let width = match suffix {
                "" => 8,
                "d" => 4,
                "w" => 2,
                "b" => 1,
                _ => return None,
            };
            (reg, width, 0)
        }
    };
    Some(named)
}

fn full_width(reg: Reg, is_64: bool) -> u8 {
    if (reg as usize) >= 16 {
        16
    } else if is_64 {
        8
    } else {
        4
    }
}

/// A write that leaves the rest of the register in place. A 32-bit write on x64 is a full
/// definition because the CPU zero-extends it into the 64-bit parent.
fn is_partial(width: u8, full: u8) -> bool {
    width < full && !(full == 8 && width == 4)
}

fn type_name(width: u8, signed: bool) -> &'static str {
    match (width, signed) {
        (16, _) => "__m128",
        (8, true) => "int64",
        (8, false) => "uint64",
        (4, true) => "int32",
        (4, false) => "uint32",
        (2, true) => "int16",
        (2, false) => "uint16",
        (1, true) => "int8",
        (1, false) => "uint8",
        _ => "uint64",
    }
}

// ---------------------------------------------------------------------------
// Line parsing
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    Top,
    IfBody,
    DoBody,
}

enum Item {
    Simple {
        line: usize,
        asm: bool,
    },
    If {
        line: usize,
        then: Vec<Item>,
        els: Option<Vec<Item>>,
    },
    While {
        line: usize,
        body: Vec<Item>,
    },
    DoWhile {
        body: Vec<Item>,
        line: usize,
    },
    InlineIf {
        line: usize,
        cond: (usize, usize),
        body: InlineBody,
        body_span: (usize, usize),
    },
    Switch {
        line: usize,
        cases: Vec<Vec<Item>>,
        has_default: bool,
    },
    Label {
        line: usize,
        name: String,
    },
    Goto {
        line: usize,
        target: String,
    },
    Break {
        line: usize,
    },
    Continue {
        line: usize,
    },
    Return {
        line: usize,
    },
    Comment,
}

enum InlineBody {
    Goto(String),
    Break,
    Continue,
    Return,
}

struct Parser<'a> {
    lines: &'a [PseudoLine],
    pos: usize,
    end: usize,
}

impl Parser<'_> {
    fn parse_block(&mut self, stop: Stop) -> Option<Vec<Item>> {
        let mut items = Vec::new();
        loop {
            if self.pos >= self.end {
                return match stop {
                    Stop::Top => Some(items),
                    _ => None,
                };
            }
            let trimmed = self.lines[self.pos].text.trim().to_owned();
            match stop {
                Stop::Top if trimmed == "}" => return Some(items),
                Stop::IfBody if trimmed == "}" || trimmed.starts_with("} else") => {
                    return Some(items);
                }
                Stop::DoBody if trimmed.starts_with("} while") => return Some(items),
                _ => {}
            }
            if trimmed == "}" || trimmed.starts_with("} ") {
                return None;
            }
            items.push(self.parse_item()?);
        }
    }

    fn parse_item(&mut self) -> Option<Item> {
        let line = self.pos;
        let trimmed = self.lines[line].text.trim().to_owned();
        if trimmed.starts_with("//") {
            self.pos += 1;
            return Some(Item::Comment);
        }
        if trimmed.starts_with("do {") {
            return self.parse_do();
        }
        if trimmed.starts_with("if (") {
            if trimmed.ends_with('{') {
                return self.parse_if();
            }
            if trimmed.ends_with(';') {
                return self.parse_inline_if();
            }
            return None;
        }
        if trimmed.starts_with("while (") && trimmed.ends_with('{') {
            return self.parse_while();
        }
        if trimmed.starts_with("switch (") && trimmed.ends_with('{') {
            return self.parse_switch();
        }
        if trimmed.starts_with("for (")
            || trimmed.starts_with("case ")
            || trimmed.starts_with("default")
        {
            return None;
        }
        if trimmed == "{" {
            return None;
        }
        if trimmed.starts_with("__asm(") {
            if !trimmed.ends_with(");") {
                return None;
            }
            self.pos += 1;
            return Some(Item::Simple { line, asm: true });
        }
        if trimmed == "break;" {
            self.pos += 1;
            return Some(Item::Break { line });
        }
        if trimmed == "continue;" {
            self.pos += 1;
            return Some(Item::Continue { line });
        }
        if trimmed == "return;" || trimmed.starts_with("return ") {
            if !trimmed.ends_with(';') {
                return None;
            }
            self.pos += 1;
            return Some(Item::Return { line });
        }
        if let Some(rest) = trimmed.strip_prefix("goto ") {
            let target = rest.strip_suffix(';')?;
            if !is_identifier(target) {
                return None;
            }
            self.pos += 1;
            return Some(Item::Goto {
                line,
                target: target.to_owned(),
            });
        }
        if let Some(name) = trimmed.strip_suffix(':')
            && is_identifier(name)
        {
            self.pos += 1;
            return Some(Item::Label {
                line,
                name: name.to_owned(),
            });
        }
        if trimmed.ends_with(';') {
            self.pos += 1;
            return Some(Item::Simple { line, asm: false });
        }
        None
    }

    fn parse_if(&mut self) -> Option<Item> {
        let line = self.pos;
        parenthesized(&self.lines[line].text, "if")?;
        self.pos += 1;
        let then = self.parse_block(Stop::IfBody)?;
        if self.pos >= self.end {
            return None;
        }
        let trimmed = self.lines[self.pos].text.trim().to_owned();
        if trimmed == "}" {
            self.pos += 1;
            return Some(Item::If {
                line,
                then,
                els: None,
            });
        }
        if trimmed == "} else {" {
            self.pos += 1;
            let els = self.parse_block(Stop::IfBody)?;
            if self.pos >= self.end || self.lines[self.pos].text.trim() != "}" {
                return None;
            }
            self.pos += 1;
            return Some(Item::If {
                line,
                then,
                els: Some(els),
            });
        }
        if trimmed.starts_with("} else if") {
            let nested = self.parse_if()?;
            return Some(Item::If {
                line,
                then,
                els: Some(vec![nested]),
            });
        }
        None
    }

    fn parse_while(&mut self) -> Option<Item> {
        let line = self.pos;
        parenthesized(&self.lines[line].text, "while")?;
        self.pos += 1;
        let body = self.parse_block(Stop::IfBody)?;
        if self.pos >= self.end || self.lines[self.pos].text.trim() != "}" {
            return None;
        }
        self.pos += 1;
        Some(Item::While { line, body })
    }

    fn parse_switch(&mut self) -> Option<Item> {
        let line = self.pos;
        parenthesized(&self.lines[line].text, "switch")?;
        self.pos += 1;
        let mut cases = Vec::new();
        let mut has_default = false;
        loop {
            if self.pos >= self.end {
                return None;
            }
            let trimmed = self.lines[self.pos].text.trim().to_owned();
            if trimmed == "}" {
                self.pos += 1;
                break;
            }
            let is_case = trimmed.starts_with("case ");
            let is_default = trimmed.starts_with("default");
            if (!is_case && !is_default) || !trimmed.ends_with('{') {
                return None;
            }
            has_default |= is_default;
            self.pos += 1;
            let body = self.parse_block(Stop::IfBody)?;
            if self.pos >= self.end || self.lines[self.pos].text.trim() != "}" {
                return None;
            }
            self.pos += 1;
            cases.push(body);
        }
        Some(Item::Switch {
            line,
            cases,
            has_default,
        })
    }

    fn parse_do(&mut self) -> Option<Item> {
        self.pos += 1;
        let body = self.parse_block(Stop::DoBody)?;
        if self.pos >= self.end {
            return None;
        }
        let text = &self.lines[self.pos].text;
        let trimmed = text.trim();
        if !trimmed.starts_with("} while (") || !trimmed.ends_with(");") {
            return None;
        }
        parenthesized(text, "while")?;
        let line = self.pos;
        self.pos += 1;
        Some(Item::DoWhile { body, line })
    }

    fn parse_inline_if(&mut self) -> Option<Item> {
        let line = self.pos;
        let text = &self.lines[line].text;
        let (open, close) = parenthesized(text, "if")?;
        let cond = (open + 1, close);
        let rest_start = close + 1;
        let rest = &text[rest_start..];
        let body_text = rest.trim_start();
        let body_start = rest_start + (rest.len() - body_text.len());
        let body = if body_text == "break;" {
            InlineBody::Break
        } else if body_text == "continue;" {
            InlineBody::Continue
        } else if body_text == "return;" || body_text.starts_with("return ") {
            InlineBody::Return
        } else {
            let target = body_text.strip_prefix("goto ")?.strip_suffix(';')?;
            if !is_identifier(target) {
                return None;
            }
            InlineBody::Goto(target.to_owned())
        };
        self.pos += 1;
        Some(Item::InlineIf {
            line,
            cond,
            body,
            body_span: (body_start, text.len()),
        })
    }
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// The byte range between the `(` and `)` that follow `keyword`.
fn parenthesized(text: &str, keyword: &str) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut search = 0;
    let at = loop {
        let found = text[search..].find(keyword)? + search;
        let before = found == 0 || !is_ident_char(bytes[found - 1]);
        let after = found + keyword.len();
        let after_ok = after >= bytes.len() || !is_ident_char(bytes[after]);
        if before && after_ok {
            break found;
        }
        search = after;
    };
    let open = text[at..].find('(')? + at;
    let close = matching_paren(text, open)?;
    Some((open, close))
}

fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate().skip(open) {
        let ch = *byte as char;
        if quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        match ch {
            '"' => quoted = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

struct Assignment {
    lhs: (usize, usize),
    rhs: (usize, usize),
    compound: Option<&'static str>,
}

/// Split a top-level assignment (`x = y;`, `x += y;`) into its spans.
fn split_assignment(text: &str) -> Option<Assignment> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut quoted = false;
    let mut escaped = false;
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            i += 1;
            continue;
        }
        match ch {
            '"' => quoted = true,
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '=' if depth == 0 => {
                if bytes.get(i + 1) == Some(&b'=') {
                    i += 2;
                    continue;
                }
                let previous = i.checked_sub(1).map(|at| bytes[at] as char);
                if matches!(previous, Some('=' | '!' | '<' | '>')) {
                    i += 1;
                    continue;
                }
                let compound = match previous {
                    Some('+') => Some("+"),
                    Some('-') => Some("-"),
                    Some('*') => Some("*"),
                    Some('/') => Some("/"),
                    Some('%') => Some("%"),
                    Some('&') => Some("&"),
                    Some('|') => Some("|"),
                    Some('^') => Some("^"),
                    Some('<') if i >= 2 && bytes[i - 2] == b'<' => Some("<<"),
                    Some('>') if i >= 2 && bytes[i - 2] == b'>' => Some(">>"),
                    _ => None,
                };
                let op_start = if compound.is_some() {
                    if matches!(compound, Some("<<" | ">>")) {
                        i - 2
                    } else {
                        i - 1
                    }
                } else {
                    i
                };
                let lhs = trim_span(text, 0, op_start);
                let mut rhs_end = text.len();
                while rhs_end > i + 1 && bytes[rhs_end - 1].is_ascii_whitespace() {
                    rhs_end -= 1;
                }
                if rhs_end > i + 1 && bytes[rhs_end - 1] == b';' {
                    rhs_end -= 1;
                }
                let rhs = trim_span(text, i + 1, rhs_end);
                if lhs.0 >= lhs.1 || rhs.0 >= rhs.1 {
                    return None;
                }
                return Some(Assignment { lhs, rhs, compound });
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn trim_span(text: &str, start: usize, end: usize) -> (usize, usize) {
    let bytes = text.as_bytes();
    let mut start = start.min(bytes.len());
    let mut end = end.min(bytes.len());
    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    (start, end)
}

// ---------------------------------------------------------------------------
// Control-flow graph
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    Simple,
    Cond,
    Return,
    Goto,
    Label,
    Break,
    Continue,
    Exit,
}

#[derive(Clone, Copy)]
struct ReadOcc {
    start: usize,
    end: usize,
    reg: Reg,
    width: u8,
    signed: bool,
    var: usize,
}

struct WriteOcc {
    start: usize,
    reg: Reg,
    width: u8,
    offset: u8,
    partial: bool,
    compound: Option<&'static str>,
    lhs: (usize, usize),
    rhs: (usize, usize),
    signed: bool,
    prev: usize,
}

struct Node {
    line: usize,
    range: (usize, usize),
    kind: NodeKind,
    succ: Vec<usize>,
    asm: bool,
    reads: Vec<ReadOcc>,
    write: Option<WriteOcc>,
    call: bool,
    call_span: (usize, usize),
    write_var: usize,
    call_var: usize,
    call_used: bool,
    asm_reads: Vec<Reg>,
    asm_writes: Vec<Reg>,
    asm_write_vars: Vec<(Reg, usize)>,
}

struct LoopCtx {
    break_to: usize,
    continue_to: Option<usize>,
}

struct Builder {
    nodes: Vec<Node>,
    labels: HashMap<String, usize>,
    gotos: Vec<(usize, String)>,
    broken: bool,
}

impl Builder {
    fn new() -> Self {
        Builder {
            nodes: Vec::new(),
            labels: HashMap::new(),
            gotos: Vec::new(),
            broken: false,
        }
    }

    fn push(&mut self, line: usize, start: usize, end: usize, kind: NodeKind) -> usize {
        self.nodes.push(Node {
            line,
            range: (start, end),
            kind,
            succ: Vec::new(),
            asm: false,
            reads: Vec::new(),
            write: None,
            call: false,
            call_span: (0, 0),
            write_var: NONE,
            call_var: NONE,
            call_used: false,
            asm_reads: Vec::new(),
            asm_writes: Vec::new(),
            asm_write_vars: Vec::new(),
        });
        self.nodes.len() - 1
    }

    fn flatten(&mut self, items: &[Item], after: usize, loops: &mut Vec<LoopCtx>) -> usize {
        let mut next = after;
        for item in items.iter().rev() {
            next = self.flatten_item(item, next, loops);
        }
        next
    }

    fn flatten_item(&mut self, item: &Item, after: usize, loops: &mut Vec<LoopCtx>) -> usize {
        match item {
            Item::Comment => after,
            Item::Simple { line, asm } => {
                let text_len = 0;
                let id = self.push(*line, text_len, text_len, NodeKind::Simple);
                self.nodes[id].asm = *asm;
                self.nodes[id].succ = vec![after];
                id
            }
            Item::Return { line } => self.push(*line, 0, 0, NodeKind::Return),
            Item::Break { line } => {
                let id = self.push(*line, 0, 0, NodeKind::Break);
                match loops.last() {
                    Some(ctx) => self.nodes[id].succ = vec![ctx.break_to],
                    None => self.broken = true,
                }
                id
            }
            Item::Continue { line } => {
                let id = self.push(*line, 0, 0, NodeKind::Continue);
                match loops.iter().rev().find_map(|ctx| ctx.continue_to) {
                    Some(target) => self.nodes[id].succ = vec![target],
                    None => self.broken = true,
                }
                id
            }
            Item::Label { line, name } => {
                let id = self.push(*line, 0, 0, NodeKind::Label);
                self.nodes[id].succ = vec![after];
                if self.labels.insert(name.clone(), id).is_some() {
                    self.broken = true;
                }
                id
            }
            Item::Goto { line, target } => {
                let id = self.push(*line, 0, 0, NodeKind::Goto);
                self.gotos.push((id, target.clone()));
                id
            }
            Item::If { line, then, els } => {
                let id = self.push(*line, 0, 0, NodeKind::Cond);
                let then_entry = self.flatten(then, after, loops);
                let else_entry = match els {
                    Some(els) => self.flatten(els, after, loops),
                    None => after,
                };
                self.nodes[id].succ = vec![then_entry, else_entry];
                id
            }
            Item::While { line, body } => {
                let id = self.push(*line, 0, 0, NodeKind::Cond);
                loops.push(LoopCtx {
                    break_to: after,
                    continue_to: Some(id),
                });
                let body_entry = self.flatten(body, id, loops);
                loops.pop();
                self.nodes[id].succ = vec![body_entry, after];
                id
            }
            Item::DoWhile { body, line } => {
                let id = self.push(*line, 0, 0, NodeKind::Cond);
                loops.push(LoopCtx {
                    break_to: after,
                    continue_to: Some(id),
                });
                let body_entry = self.flatten(body, id, loops);
                loops.pop();
                self.nodes[id].succ = vec![body_entry, after];
                body_entry
            }
            Item::Switch {
                line,
                cases,
                has_default,
            } => {
                let id = self.push(*line, 0, 0, NodeKind::Cond);
                loops.push(LoopCtx {
                    break_to: after,
                    continue_to: None,
                });
                let mut entries = Vec::with_capacity(cases.len() + 1);
                for case in cases {
                    entries.push(self.flatten(case, after, loops));
                }
                loops.pop();
                if !has_default {
                    entries.push(after);
                }
                self.nodes[id].succ = entries;
                id
            }
            Item::InlineIf {
                line,
                cond,
                body,
                body_span,
            } => {
                let cond_id = self.push(*line, cond.0, cond.1, NodeKind::Cond);
                let (kind, target) = match body {
                    InlineBody::Goto(target) => (NodeKind::Goto, Some(target.clone())),
                    InlineBody::Break => (NodeKind::Break, None),
                    InlineBody::Continue => (NodeKind::Continue, None),
                    InlineBody::Return => (NodeKind::Return, None),
                };
                let body_id = self.push(*line, body_span.0, body_span.1, kind);
                match kind {
                    NodeKind::Goto => {
                        if let Some(target) = target {
                            self.gotos.push((body_id, target));
                        }
                    }
                    NodeKind::Break => match loops.last() {
                        Some(ctx) => self.nodes[body_id].succ = vec![ctx.break_to],
                        None => self.broken = true,
                    },
                    NodeKind::Continue => {
                        match loops.iter().rev().find_map(|ctx| ctx.continue_to) {
                            Some(target) => self.nodes[body_id].succ = vec![target],
                            None => self.broken = true,
                        }
                    }
                    _ => {}
                }
                self.nodes[cond_id].succ = vec![body_id, after];
                cond_id
            }
        }
    }

    fn resolve_gotos(&mut self) -> bool {
        for (node, target) in self.gotos.drain(..) {
            let Some(&label) = self.labels.get(&target) else {
                return false;
            };
            self.nodes[node].succ = vec![label];
        }
        true
    }

    fn scan(
        &mut self,
        lines: &[PseudoLine],
        is_64: bool,
        effects: &HashMap<usize, (Vec<Reg>, Vec<Reg>)>,
    ) {
        for index in 0..self.nodes.len() {
            let (line, kind, asm, (start, end)) = {
                let node = &self.nodes[index];
                (node.line, node.kind, node.asm, node.range)
            };
            if kind == NodeKind::Exit {
                continue;
            }
            if asm {
                if let Some((reads, writes)) = effects.get(&line) {
                    let node = &mut self.nodes[index];
                    node.asm_reads = reads.clone();
                    node.asm_writes = writes.clone();
                }
                continue;
            }
            let (start, end) = if start == end {
                (0, lines[line].text.len())
            } else {
                (start, end)
            };
            let text = &lines[line].text;
            if start >= end || end > text.len() {
                continue;
            }
            if matches!(
                kind,
                NodeKind::Label | NodeKind::Goto | NodeKind::Break | NodeKind::Continue
            ) {
                continue;
            }
            let signed = text.contains("/* signed */");
            let tokens: Vec<Token> = scan_tokens(&text[start..end])
                .into_iter()
                .map(|mut token| {
                    token.start += start;
                    token.end += start;
                    token
                })
                .collect();
            let mut reads = Vec::new();
            let mut write = None;
            let mut call = false;
            let mut call_span = (0, 0);
            if kind == NodeKind::Simple {
                if let Some(assignment) = split_assignment(&text[start..end]) {
                    let lhs = (start + assignment.lhs.0, start + assignment.lhs.1);
                    let rhs = (start + assignment.rhs.0, start + assignment.rhs.1);
                    let lhs_tokens: Vec<Token> = tokens
                        .iter()
                        .copied()
                        .filter(|token| token.start >= lhs.0 && token.end <= lhs.1)
                        .collect();
                    let lhs_is_register = lhs_tokens.len() == 1
                        && lhs_tokens[0].start == lhs.0
                        && lhs_tokens[0].end == lhs.1;
                    if lhs_is_register {
                        let token = lhs_tokens[0];
                        let full = full_width(token.reg, is_64);
                        write = Some(WriteOcc {
                            start: token.start,
                            reg: token.reg,
                            width: token.width,
                            offset: token.offset,
                            partial: is_partial(token.width, full),
                            compound: assignment.compound,
                            lhs,
                            rhs,
                            signed,
                            prev: NONE,
                        });
                        if assignment.compound.is_some() {
                            reads.push(ReadOcc {
                                start: token.start,
                                end: token.end,
                                reg: token.reg,
                                width: token.width,
                                signed,
                                var: NONE,
                            });
                        }
                    }
                    for token in tokens {
                        if lhs_is_register
                            && token.start == lhs_tokens[0].start
                            && token.end == lhs_tokens[0].end
                        {
                            continue;
                        }
                        reads.push(ReadOcc {
                            start: token.start,
                            end: token.end,
                            reg: token.reg,
                            width: token.width,
                            signed,
                            var: NONE,
                        });
                    }
                } else {
                    for token in tokens {
                        reads.push(ReadOcc {
                            start: token.start,
                            end: token.end,
                            reg: token.reg,
                            width: token.width,
                            signed,
                            var: NONE,
                        });
                    }
                    if let Some(span) = call_expression(&text[start..end]) {
                        call = true;
                        call_span = (start + span.0, start + span.1);
                    }
                }
            } else {
                for token in tokens {
                    reads.push(ReadOcc {
                        start: token.start,
                        end: token.end,
                        reg: token.reg,
                        width: token.width,
                        signed,
                        var: NONE,
                    });
                }
            }
            let node = &mut self.nodes[index];
            node.reads = reads;
            node.write = write;
            node.call = call;
            node.call_span = call_span;
        }
    }
}

/// The span of a call expression that makes up a whole statement (`f(a);`, `(*p)();`).
fn call_expression(text: &str) -> Option<(usize, usize)> {
    let start = text.len() - text.trim_start().len();
    let trimmed = text.trim();
    if !trimmed.ends_with(");") {
        return None;
    }
    let body = &text[start..start + trimmed.len()];
    let mut found = body.starts_with("(*");
    if !found {
        let bytes = body.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if is_ident_start(bytes[i]) {
                let ident_start = i;
                i += 1;
                while i < bytes.len() && is_ident_char(bytes[i]) {
                    i += 1;
                }
                let ident = &body[ident_start..i];
                let mut after = i;
                while after < bytes.len() && bytes[after] == b' ' {
                    after += 1;
                }
                if bytes.get(after) == Some(&b'(') && !is_non_call_identifier(ident) {
                    found = true;
                    break;
                }
            } else {
                i += 1;
            }
        }
    }
    if !found {
        return None;
    }
    Some((start, start + trimmed.len() - 1))
}

fn is_non_call_identifier(ident: &str) -> bool {
    matches!(
        ident,
        "if" | "while"
            | "switch"
            | "for"
            | "return"
            | "sizeof"
            | "__asm"
            | "ror"
            | "rol"
            | "rcl"
            | "rcr"
            | "ctz"
            | "clz"
            | "popcnt"
            | "bswap"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "void"
            | "char"
            | "short"
            | "int"
            | "long"
            | "unsigned"
            | "signed"
            | "float"
            | "double"
            | "__m128"
    )
}

// ---------------------------------------------------------------------------
// Value flow
// ---------------------------------------------------------------------------

struct Vars {
    parent: Vec<usize>,
    width: Vec<u8>,
    signed: Vec<u32>,
    unsigned: Vec<u32>,
    /// A value produced by a machine instruction the lifter could not model. It has no C
    /// variable behind it, so it is printed with its register name until overwritten.
    raw: Vec<bool>,
    entry: [usize; REGS],
}

impl Vars {
    fn new() -> Self {
        Vars {
            parent: Vec::new(),
            width: Vec::new(),
            signed: Vec::new(),
            unsigned: Vec::new(),
            raw: Vec::new(),
            entry: [NONE; REGS],
        }
    }

    fn new_var(&mut self, width: u8) -> usize {
        let id = self.parent.len();
        self.parent.push(id);
        self.width.push(width);
        self.signed.push(0);
        self.unsigned.push(0);
        self.raw.push(false);
        id
    }

    fn new_raw_var(&mut self, width: u8) -> usize {
        let id = self.new_var(width);
        self.raw[id] = true;
        id
    }

    fn find(&mut self, value: usize) -> usize {
        let mut root = value;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut current = value;
        while self.parent[current] != root {
            let next = self.parent[current];
            self.parent[current] = root;
            current = next;
        }
        root
    }

    fn union(&mut self, left: usize, right: usize) -> usize {
        let left = self.find(left);
        let right = self.find(right);
        if left == right {
            return left;
        }
        self.parent[right] = left;
        left
    }

    fn add_use(&mut self, var: usize, width: u8, signed: bool) {
        self.width[var] = self.width[var].max(width);
        if signed {
            self.signed[var] += 1;
        } else {
            self.unsigned[var] += 1;
        }
    }

    /// A definition widens the value but must not affect its inferred signedness.
    fn add_def(&mut self, var: usize, width: u8) {
        self.width[var] = self.width[var].max(width);
    }
}

struct Program {
    nodes: Vec<Node>,
    /// Final union-find root of every original variable id.
    roots: Vec<usize>,
    widths: Vec<u8>,
    signed: Vec<bool>,
    raw: Vec<bool>,
    names: Vec<Option<String>>,
    declarations: Vec<String>,
    is_64: bool,
}

impl Program {
    fn analyze(nodes: Vec<Node>, _entry: usize, is_64: bool, lines: &[PseudoLine]) -> Option<Self> {
        let count = nodes.len();
        if count == 0 {
            return None;
        }
        let mut preds = vec![Vec::new(); count];
        for (index, node) in nodes.iter().enumerate() {
            for &succ in &node.succ {
                if succ < count {
                    preds[succ].push(index);
                }
            }
        }
        // Liveness over register families (a u64 bit per family).
        let mut uses = vec![0u64; count];
        let mut kill = vec![0u64; count];
        for (index, node) in nodes.iter().enumerate() {
            for read in &node.reads {
                uses[index] |= 1u64 << (read.reg as usize);
            }
            for read in &node.asm_reads {
                uses[index] |= 1u64 << (*read as usize);
            }
            if let Some(write) = &node.write {
                if write.partial {
                    uses[index] |= 1u64 << (write.reg as usize);
                } else {
                    kill[index] |= 1u64 << (write.reg as usize);
                }
            }
            for write in &node.asm_writes {
                kill[index] |= 1u64 << (*write as usize);
            }
            if node.call {
                kill[index] |= 1u64 << (Reg::Rax as usize);
            }
        }
        let mut live_in = vec![0u64; count];
        let mut queued = vec![true; count];
        let mut queue: std::collections::VecDeque<usize> = (0..count).rev().collect();
        let mut steps = 0usize;
        while let Some(index) = queue.pop_front() {
            queued[index] = false;
            steps += 1;
            if steps > 20_000_000 {
                return None;
            }
            let mut out = 0u64;
            for &succ in &nodes[index].succ {
                out |= live_in[succ];
            }
            let inn = uses[index] | (out & !kill[index]);
            if inn == live_in[index] {
                continue;
            }
            live_in[index] = inn;
            for &pred in &preds[index] {
                if !queued[pred] {
                    queued[pred] = true;
                    queue.push_back(pred);
                }
            }
        }
        let mut live_out = vec![0u64; count];
        for index in 0..count {
            for &succ in &nodes[index].succ {
                live_out[index] |= live_in[succ];
            }
        }

        let mut vars = Vars::new();
        let mut nodes = nodes;
        for node in &mut nodes {
            if let Some(write) = &node.write
                && write.compound.is_none()
            {
                node.write_var = vars.new_var(write.width);
            }
            if node.call {
                node.call_var = vars.new_var(0);
            }
            let mut asm_vars = Vec::with_capacity(node.asm_writes.len());
            for reg in &node.asm_writes {
                let width = full_width(*reg, is_64);
                asm_vars.push((*reg, vars.new_raw_var(width)));
            }
            node.asm_write_vars = asm_vars;
        }
        for index in 0..nodes.len() {
            if nodes[index].call_var != NONE {
                nodes[index].call_used = live_out[index] & (1u64 << (Reg::Rax as usize)) != 0;
            }
        }

        let mut entry_map = [NONE; REGS];
        for (reg, slot) in entry_map.iter_mut().enumerate() {
            *slot = vars.new_var(0);
            vars.entry[reg] = *slot;
        }
        let mut ins = vec![[NONE; REGS]; count];
        let mut outs = vec![[NONE; REGS]; count];
        let mut computed = vec![false; count];
        let mut queued = vec![true; count];
        let mut queue: std::collections::VecDeque<usize> = (0..count).collect();
        let mut steps = 0usize;
        while let Some(index) = queue.pop_front() {
            queued[index] = false;
            steps += 1;
            if steps > 20_000_000 {
                return None;
            }
            let new_in = if preds[index].is_empty() {
                entry_map
            } else {
                let mut map = [NONE; REGS];
                for reg in 0..REGS {
                    if live_in[index] & (1u64 << reg) == 0 {
                        continue;
                    }
                    let mut merged = NONE;
                    let mut merged_raw = false;
                    for &pred in &preds[index] {
                        let value = outs[pred][reg];
                        if value == NONE {
                            continue;
                        }
                        let root = vars.find(value);
                        merged_raw |= vars.raw[root];
                        merged = if merged == NONE {
                            root
                        } else {
                            vars.union(merged, root)
                        };
                    }
                    if merged != NONE && merged_raw {
                        let root = vars.find(merged);
                        vars.raw[root] = true;
                        merged = root;
                    }
                    map[reg] = merged;
                }
                map
            };
            if computed[index] && same_map(&new_in, &ins[index], &mut vars) {
                continue;
            }
            computed[index] = true;
            ins[index] = new_in;
            let mut out = new_in;
            if let Some(write) = &nodes[index].write {
                if write.compound.is_some() {
                    // A compound assignment updates the value it read, so it keeps that
                    // variable instead of introducing a second one.
                    let previous = new_in[write.reg as usize];
                    if previous != NONE {
                        out[write.reg as usize] = vars.find(previous);
                    }
                } else {
                    out[write.reg as usize] = vars.find(nodes[index].write_var);
                }
            }
            if nodes[index].call_var != NONE {
                out[Reg::Rax as usize] = vars.find(nodes[index].call_var);
            }
            for (reg, var) in &nodes[index].asm_write_vars {
                out[*reg as usize] = vars.find(*var);
            }
            outs[index] = out;
            for &succ in &nodes[index].succ {
                if !queued[succ] {
                    queued[succ] = true;
                    queue.push_back(succ);
                }
            }
        }

        // Resolve every read to the variable live at that point.
        for index in 0..count {
            let reads = std::mem::take(&mut nodes[index].reads);
            let mut resolved = Vec::with_capacity(reads.len());
            for mut read in reads {
                let var = ins[index][read.reg as usize];
                if var == NONE {
                    continue;
                }
                read.var = var;
                vars.add_use(var, read.width, read.signed);
                resolved.push(read);
            }
            nodes[index].reads = resolved;
            if let Some(write) = nodes[index].write.take() {
                let mut write = write;
                let prev = ins[index][write.reg as usize];
                if write.compound.is_some() {
                    match prev {
                        NONE => {
                            // No previous value to read: keep the line register-shaped.
                            nodes[index].write_var = vars.new_raw_var(write.width);
                        }
                        previous => {
                            write.prev = previous;
                            nodes[index].write_var = previous;
                            vars.add_def(previous, write.width);
                            if write.partial {
                                let full = full_width(write.reg, is_64);
                                vars.add_use(previous, full, false);
                            }
                        }
                    }
                } else if write.partial && prev != NONE {
                    write.prev = prev;
                    let full = full_width(write.reg, is_64);
                    vars.add_use(prev, full, false);
                }
                nodes[index].write = Some(write);
            }
            if nodes[index].call_used {
                vars.add_use(nodes[index].call_var, 0, false);
            }
        }

        // Aggregate widths and signedness onto the final roots.
        let total = vars.parent.len();
        let roots: Vec<usize> = (0..total).map(|id| vars.find(id)).collect();
        let mut widths = vec![0u8; total];
        let mut signed_uses = vec![0u32; total];
        let mut unsigned_uses = vec![0u32; total];
        let mut raw = vec![false; total];
        for (id, root) in roots.iter().enumerate() {
            let root = *root;
            widths[root] = widths[root].max(vars.width[id]);
            signed_uses[root] += vars.signed[id];
            unsigned_uses[root] += vars.unsigned[id];
            raw[root] |= vars.raw[id];
        }
        let signed: Vec<bool> = (0..total)
            .map(|root| signed_uses[root] > 0 && unsigned_uses[root] == 0)
            .collect();

        // Name the locals in source order.
        let mut occurrences: Vec<(usize, usize, usize)> = Vec::new();
        for node in &nodes {
            for read in &node.reads {
                let root = roots[read.var];
                if !raw[root] {
                    occurrences.push((node.line, read.start, root));
                }
            }
            if let Some(write) = &node.write {
                let root = roots[node.write_var];
                if !raw[root] {
                    occurrences.push((node.line, write.start, root));
                }
                if write.partial
                    && widths[root] > write.width
                    && write.prev != NONE
                    && !raw[roots[write.prev]]
                {
                    occurrences.push((node.line, write.start, roots[write.prev]));
                }
            }
            if node.call_used {
                occurrences.push((node.line, node.call_span.0, roots[node.call_var]));
            }
        }
        occurrences.sort_by_key(|(line, start, _)| (*line, *start));
        let mut reserved = HashSet::new();
        for line in lines {
            collect_identifiers(&line.text, &mut reserved);
        }
        let mut names = vec![None; total];
        let mut next = 1usize;
        for (_, _, root) in occurrences {
            if names[root].is_some() || raw[root] {
                continue;
            }
            loop {
                let candidate = format!("v{next}");
                next += 1;
                if !reserved.contains(&candidate) {
                    names[root] = Some(candidate);
                    break;
                }
            }
        }

        let mut declarations: Vec<(usize, String)> = Vec::new();
        for root in 0..total {
            if raw[root] {
                continue;
            }
            if let Some(name) = &names[root] {
                let number: usize = name[1..].parse().unwrap_or(0);
                declarations.push((
                    number,
                    format!("    {} {};", type_name(widths[root], signed[root]), name),
                ));
            }
        }
        declarations.sort_by_key(|(number, _)| *number);
        Some(Program {
            nodes,
            roots,
            widths,
            signed,
            raw,
            names,
            declarations: declarations.into_iter().map(|(_, text)| text).collect(),
            is_64,
        })
    }

    fn render(&self, lines: &[PseudoLine]) -> Option<Vec<PseudoLine>> {
        let mut line_nodes: Vec<Vec<usize>> = vec![Vec::new(); lines.len()];
        for (index, node) in self.nodes.iter().enumerate() {
            if node.kind == NodeKind::Exit || node.line == NONE {
                continue;
            }
            line_nodes[node.line].push(index);
        }
        let mut out = Vec::with_capacity(lines.len());
        for (line_index, line) in lines.iter().enumerate() {
            if line_nodes[line_index].is_empty() {
                out.push(line.clone());
                continue;
            }
            let text = line.text.as_str();
            let mut edits: Vec<Edit> = Vec::new();
            let mut write: Option<(usize, &WriteOcc)> = None;
            let mut call_var = NONE;
            for &index in &line_nodes[line_index] {
                let node = &self.nodes[index];
                for read in &node.reads {
                    let replacement = self.read_expression(read);
                    edits.push(Edit {
                        start: read.start,
                        end: read.end,
                        text: replacement,
                        order: 1,
                    });
                }
                if let Some(occurrence) = &node.write {
                    write = Some((index, occurrence));
                }
                if node.call_used {
                    call_var = node.call_var;
                }
            }
            let mut rendered = if let Some((node_index, write)) = write {
                let root = self.roots[self.nodes[node_index].write_var];
                if self.raw[root] {
                    // The value came out of an unmodelled instruction; keeping the register
                    // spelling is exact.
                    apply_edits(text, &mut edits)
                } else {
                    let name = self.names[root].clone()?;
                    let width = self.widths[root];
                    let rhs = render_range(text, write.rhs.0, write.rhs.1, &mut edits);
                    let prev = if write.partial && width > write.width && write.prev != NONE {
                        let prev_root = self.roots[write.prev];
                        if self.raw[prev_root] {
                            None
                        } else {
                            self.names[prev_root].clone()
                        }
                    } else {
                        None
                    };
                    render_write_line(
                        text,
                        write,
                        &name,
                        width,
                        self.signed[root],
                        prev.as_deref(),
                        &rhs,
                        self.is_64,
                    )
                }
            } else {
                apply_edits(text, &mut edits)
            };
            if call_var != NONE {
                let root = self.roots[call_var];
                if let Some(name) = &self.names[root] {
                    let at = line_nodes[line_index]
                        .iter()
                        .map(|&index| self.nodes[index].call_span.0)
                        .min()
                        .unwrap_or(0);
                    let at = at.min(rendered.len());
                    rendered.insert_str(at, &format!("{name} = "));
                }
            }
            out.push(PseudoLine {
                addr: line.addr,
                text: rendered,
            });
        }
        Some(out)
    }

    fn read_expression(&self, read: &ReadOcc) -> String {
        self.read_expression_of(self.roots[read.var], read.reg, read.width, read.signed)
    }

    fn read_expression_of(&self, root: usize, reg: Reg, width: u8, signed: bool) -> String {
        if self.raw[root] {
            return view_name(reg, width);
        }
        let name = self.names[root].clone().unwrap_or_default();
        let var_width = self.widths[root];
        if width >= var_width && (signed == self.signed[root] || width == 16) {
            return name;
        }
        let ty = type_name(width, signed);
        format!("({ty}){name}")
    }
}

impl WriteOcc {
    fn _unused(&self) {}
}

fn view_name(reg: Reg, width: u8) -> String {
    reg.name_width(width).to_owned()
}

/// Print one definition of a recovered local.
///
/// `prev` is the local that held the register before a read-modify-write; `None` means the
/// previous value came from an unmodelled instruction and is printed with its register name.
#[allow(clippy::too_many_arguments)]
fn render_write_line(
    text: &str,
    write: &WriteOcc,
    name: &str,
    width: u8,
    var_signed: bool,
    prev: Option<&str>,
    rhs: &str,
    is_64: bool,
) -> String {
    let indent = &text[..write.lhs.0.min(text.len())];
    if write.partial && width > write.width {
        let prev = prev
            .map(str::to_owned)
            .unwrap_or_else(|| view_name(write.reg, full_width(write.reg, is_64)));
        let unit = (1u64 << (write.width as u32 * 8)) - 1;
        let mask = !(unit << write.offset) & width_mask(width);
        let low_type = type_name(write.width, write.signed);
        let operand = if write.offset == 0 {
            format!("({low_type}){prev}")
        } else {
            format!("({low_type})({prev} >> {})", write.offset)
        };
        let low = match write.compound {
            Some(op) => format!("({low_type})({operand} {op} {rhs})"),
            None => format!("({low_type})({rhs})"),
        };
        let low = if write.offset == 0 {
            low
        } else {
            format!("(({low}) << {})", write.offset)
        };
        return format!("{indent}{name} = ({prev} & {mask:#x}) | {low};");
    }
    if let Some(op) = write.compound {
        let ty = type_name(write.width, write.signed);
        let operand = if width == write.width && var_signed == write.signed {
            name.to_owned()
        } else {
            format!("({ty}){name}")
        };
        if width > write.width || var_signed != write.signed {
            return format!("{indent}{name} = ({ty})({operand} {op} {rhs});");
        }
        return format!("{indent}{name} {op}= {rhs};");
    }
    if width > write.width {
        let ty = type_name(write.width, false);
        if rhs.trim_start().starts_with(&format!("({ty})")) {
            return format!("{indent}{name} = {rhs};");
        }
        return format!("{indent}{name} = ({ty})({rhs});");
    }
    format!("{indent}{name} = {rhs};")
}

fn width_mask(width: u8) -> u64 {
    if width >= 8 {
        u64::MAX
    } else {
        (1u64 << (width as u32 * 8)) - 1
    }
}

fn same_map(left: &[usize; REGS], right: &[usize; REGS], vars: &mut Vars) -> bool {
    for index in 0..REGS {
        if left[index] == right[index] {
            continue;
        }
        if left[index] == NONE || right[index] == NONE {
            return false;
        }
        if vars.find(left[index]) != vars.find(right[index]) {
            return false;
        }
    }
    true
}

struct Edit {
    start: usize,
    end: usize,
    text: String,
    order: u8,
}

fn apply_edits(text: &str, edits: &mut [Edit]) -> String {
    edits.sort_by_key(|edit| (edit.start, edit.order));
    let mut out = String::with_capacity(text.len() + 16);
    let mut cursor = 0;
    for edit in edits.iter() {
        if edit.start < cursor || edit.end > text.len() {
            continue;
        }
        out.push_str(&text[cursor..edit.start]);
        out.push_str(&edit.text);
        cursor = edit.end;
    }
    out.push_str(&text[cursor..]);
    out
}

fn render_range(text: &str, start: usize, end: usize, edits: &mut [Edit]) -> String {
    let mut inside: Vec<Edit> = Vec::new();
    for edit in edits.iter() {
        if edit.start >= start && edit.end <= end {
            inside.push(Edit {
                start: edit.start,
                end: edit.end,
                text: edit.text.clone(),
                order: edit.order,
            });
        }
    }
    inside.sort_by_key(|edit| (edit.start, edit.order));
    let mut out = String::with_capacity(end - start);
    let mut cursor = start;
    for edit in &inside {
        if edit.start < cursor {
            continue;
        }
        out.push_str(&text[cursor..edit.start]);
        out.push_str(&edit.text);
        cursor = edit.end;
    }
    out.push_str(&text[cursor..end]);
    out
}

/// Identifiers the generated names must not collide with.
fn collect_identifiers(text: &str, out: &mut HashSet<String>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if is_ident_start(bytes[i]) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_char(bytes[i]) {
                i += 1;
            }
            out.insert(text[start..i].to_owned());
        } else {
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use recraft_common::Address;

    fn line(addr: u64, text: &str) -> PseudoLine {
        PseudoLine {
            addr: Address::new(addr),
            text: text.to_owned(),
        }
    }

    fn run_opt(signature: &str, body: &[&str]) -> Option<String> {
        let mut lines = vec![line(0x1000, signature)];
        for (index, text) in body.iter().enumerate() {
            lines.push(line(0x1001 + index as u64, text));
        }
        lines.push(line(0x2000, "}"));
        if !recover_variables(&mut lines, true) {
            return None;
        }
        Some(
            lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    fn run(body: &[&str]) -> String {
        run_opt("int64 foo(void) {", body).expect("function should be transformed")
    }

    fn assert_no_registers(text: &str) {
        if let Some(token) = scan_tokens(text).first() {
            panic!(
                "register {:?} survived in:\n{text}",
                token.reg.name_width(token.width)
            );
        }
    }

    #[test]
    fn call_result_feeds_a_local() {
        let text = run(&["    sub_1000();", "    ecx = eax;", "    return ecx;"]);
        assert!(text.contains("v1 = sub_1000();"), "{text}");
        assert!(text.contains("v2 = v1;"), "{text}");
        assert_no_registers(&text);
    }

    #[test]
    fn straight_line_definitions_get_distinct_locals() {
        let text = run(&["    rax = arg1;", "    rcx = rax;", "    return rcx;"]);
        assert!(text.contains("v1 = arg1;"), "{text}");
        assert!(text.contains("v2 = v1;"), "{text}");
        assert!(text.contains("return v2;"), "{text}");
        assert_no_registers(&text);
    }

    #[test]
    fn if_else_arms_share_one_local() {
        let text = run(&[
            "    if (arg1) {",
            "        rax = 1;",
            "    } else {",
            "        rax = 2;",
            "    }",
            "    return rax;",
        ]);
        assert!(text.contains("v1 = 1;"), "{text}");
        assert!(text.contains("v1 = 2;"), "{text}");
        assert!(text.contains("return v1;"), "{text}");
        assert_no_registers(&text);
    }

    #[test]
    fn loop_carried_value_is_one_local() {
        let text = run(&[
            "    rax = 0;",
            "    while (arg1) {",
            "        rax += 1;",
            "    }",
            "    return rax;",
        ]);
        assert!(text.contains("v1 = 0;"), "{text}");
        assert!(text.contains("v1 += 1;"), "{text}");
        assert!(text.contains("return v1;"), "{text}");
        assert_no_registers(&text);
    }

    #[test]
    fn compound_assignment_updates_the_local_it_read() {
        let text = run(&["    rax = 5;", "    rax -= arg1;", "    return rax;"]);
        assert!(text.contains("v1 = 5;"), "{text}");
        assert!(text.contains("v1 -= arg1;"), "{text}");
        assert!(!text.contains("v2"), "{text}");
        assert_no_registers(&text);
    }

    #[test]
    fn wide_compound_assignment_casts_the_read() {
        let text = run(&["    eax = arg1;", "    eax += 1;", "    return rax;"]);
        assert_eq!(text.matches("uint32 v1;").count(), 0, "{text}");
        assert!(text.contains("uint64 v1;"), "{text}");
        assert!(text.contains("v1 = (uint32)((uint32)v1 + 1);"), "{text}");
        assert_no_registers(&text);
    }

    #[test]
    fn partial_byte_write_keeps_the_upper_bits() {
        let text = run(&["    rax = 0;", "    al = 1;", "    return rax;"]);
        assert!(
            text.contains("v2 = (v1 & 0xffffffffffffff00) | (uint8)(1);"),
            "{text}"
        );
        assert_no_registers(&text);
    }

    #[test]
    fn wide_read_of_a_32_bit_write_zero_extends() {
        let text = run(&["    eax = arg1;", "    return rax;"]);
        assert!(text.contains("v1 = (uint32)(arg1);"), "{text}");
        assert!(text.contains("return v1;"), "{text}");
        assert!(text.contains("uint64 v1;"), "{text}");
        assert_no_registers(&text);
    }

    #[test]
    fn signed_context_selects_a_signed_local() {
        let text = run(&[
            "    eax = arg1;",
            "    if (eax < 0 /* signed */) {",
            "        return 1;",
            "    }",
            "    return 0;",
        ]);
        assert!(text.contains("int32 v1;"), "{text}");
        assert!(text.contains("if (v1 < 0 /* signed */)"), "{text}");
        assert_no_registers(&text);
    }

    #[test]
    fn mixed_sign_context_casts_the_signed_read() {
        let text = run(&[
            "    eax = arg1;",
            "    if (eax == 0) {",
            "        return 1;",
            "    }",
            "    if (eax < 0 /* signed */) {",
            "        return 2;",
            "    }",
            "    return eax;",
        ]);
        assert!(text.contains("uint32 v1;"), "{text}");
        assert!(text.contains("(int32)v1 < 0"), "{text}");
    }

    #[test]
    fn switch_cases_share_the_selector_and_local() {
        let text = run(&[
            "    switch (arg1) {",
            "        case 0x1: {",
            "            rax = 1;",
            "            break;",
            "        }",
            "        default: {",
            "            rax = 2;",
            "        }",
            "    }",
            "    return rax;",
        ]);
        assert!(text.contains("switch (arg1)"), "{text}");
        assert!(text.contains("v1 = 1;"), "{text}");
        assert!(text.contains("v1 = 2;"), "{text}");
        assert_no_registers(&text);
    }

    #[test]
    fn asm_output_stays_a_register_until_overwritten() {
        let text = run(&[
            "    __asm(\"mov rax, gs:[0x30]\");",
            "    rcx = rax;",
            "    return rcx;",
        ]);
        assert!(text.contains("v1 = rax;"), "{text}");
        assert!(!text.contains("uint64 v2;"), "{text}");
    }

    #[test]
    fn goto_labels_are_flow_edges() {
        let text = run(&[
            "    loc_1001:",
            "    rax = 1;",
            "    if (arg1) goto loc_1001;",
            "    return rax;",
        ]);
        assert!(text.contains("v1 = 1;"), "{text}");
        assert!(text.contains("goto loc_1001;"), "{text}");
        assert!(text.contains("return v1;"), "{text}");
        assert_no_registers(&text);
    }

    #[test]
    fn functions_without_registers_are_left_alone() {
        let result = run_opt("int64 foo(int64 arg1) {", &["    return arg1;"]);
        assert!(result.is_none());
    }

    #[test]
    fn the_parameter_comment_loses_its_register_names() {
        let mut lines = vec![
            line(0x1000, "// L2 params: arg1=rcx, arg2=rdx"),
            line(0x1001, "int64 foo(int64 arg1, int64 arg2) {"),
            line(0x1002, "    rax = arg1;"),
            line(0x1003, "    return rax;"),
            line(0x1004, "}"),
        ];
        assert!(recover_variables(&mut lines, true));
        assert_eq!(lines[0].text, "// L2 params: arg1, arg2");
    }
}
