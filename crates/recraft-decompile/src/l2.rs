// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! L2 variable recovery (first cut).
//!
//! - Windows x64: incoming integer arguments arrive in rcx, rdx, r8, r9. A register is
//!   accepted as an argument when it is read before its first write and becomes
//!   `arg1..arg4`.
//! - Win32 x86: arguments live on the stack. With a frame (`push ebp; mov ebp, esp`)
//!   `[ebp+8]`, `[ebp+0xc]`, ... are `arg1..`; without one `[esp+4]`, `[esp+8]`, ... are.
//! - Stack slots are named `var_XX` by the printer; types/SSA come later.

use std::collections::{BTreeMap, BTreeSet};

use recraft_common::{Address, Result};
use recraft_core::Session;
use recraft_decode::iced_x86;
use recraft_ir::{BinOp, Expr, ExprId, Lifter, Reg, RegView, Stmt};

/// Windows x64 integer argument registers, in declaration order.
pub const X64_ARG_REGS: [Reg; 4] = [Reg::Rcx, Reg::Rdx, Reg::R8, Reg::R9];

const MAX_ARG_SCAN_INSNS: usize = 4096;

/// Recovered argument names for one function.
#[derive(Debug, Clone, Default)]
pub struct ArgNames {
    names: BTreeMap<Reg, String>,
    /// x86 stack arguments keyed by `(base register, byte offset)`.
    stack: BTreeMap<(Reg, i64), String>,
    /// Inferred parameter types for register arguments (defaults to `int64`).
    types: BTreeMap<Reg, ArgType>,
    /// Parameter types for x86 stack arguments (from a user prototype, else `int64`).
    stack_types: BTreeMap<(Reg, i64), ArgType>,
}

/// Parameter type inferred from how the argument is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    Int32,
    Int64,
    /// Byte-sized memory access through the argument.
    CharPtr,
    /// 2-byte access (`wchar_t*`).
    WCharPtr,
    Int32Ptr,
    Int64Ptr,
    /// Mixed or unknown access widths behind a pointer.
    VoidPtr,
}

impl ArgType {
    pub const fn c_name(self) -> &'static str {
        match self {
            ArgType::Int32 => "int32",
            ArgType::Int64 => "int64",
            ArgType::CharPtr => "char*",
            ArgType::WCharPtr => "wchar_t*",
            ArgType::Int32Ptr => "int32*",
            ArgType::Int64Ptr => "int64*",
            ArgType::VoidPtr => "void*",
        }
    }

    pub const fn is_pointer(self) -> bool {
        matches!(
            self,
            ArgType::CharPtr
                | ArgType::WCharPtr
                | ArgType::Int32Ptr
                | ArgType::Int64Ptr
                | ArgType::VoidPtr
        )
    }
}

impl ArgNames {
    pub fn name_for(&self, reg: Reg) -> Option<&str> {
        self.names.get(&reg).map(String::as_str)
    }

    pub fn name_for_stack(&self, base: Reg, offset: i64) -> Option<&str> {
        self.stack.get(&(base, offset)).map(String::as_str)
    }

    /// True when the argument was inferred as a pointer (`char*`, `wchar_t*`, `int32*`,
    /// `int64*` or `void*`), i.e. field-style rendering through it is meaningful.
    pub fn is_pointer_arg(&self, reg: Reg) -> bool {
        self.types.get(&reg).is_some_and(|ty| ty.is_pointer())
    }

    /// Element size in bytes for a pointer-typed argument (`None` for `void*`/scalars).
    pub fn pointer_elem_size(&self, reg: Reg) -> Option<u64> {
        self.types.get(&reg).and_then(|ty| match ty {
            ArgType::CharPtr => Some(1),
            ArgType::WCharPtr => Some(2),
            ArgType::Int32Ptr => Some(4),
            ArgType::Int64Ptr => Some(8),
            _ => None,
        })
    }

    /// Calling-convention slot (0-based) of a register argument: `argN` -> `N - 1`.
    pub fn slot_of(&self, reg: Reg) -> Option<u8> {
        X64_ARG_REGS
            .iter()
            .position(|candidate| *candidate == reg)
            .map(|index| index as u8)
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && self.stack.is_empty()
    }

    /// Build arguments from a parsed user prototype (x64 register arguments; at most
    /// four slots are representable without stack-argument layout).
    pub fn from_prototype(params: &[ProtoParam]) -> Self {
        let mut names = BTreeMap::new();
        let mut types = BTreeMap::new();
        for (slot, param) in params.iter().take(X64_ARG_REGS.len()).enumerate() {
            let reg = X64_ARG_REGS[slot];
            names.insert(reg, param.name.clone());
            types.insert(reg, param.ty);
        }
        ArgNames {
            names,
            stack: BTreeMap::new(),
            types,
            stack_types: BTreeMap::new(),
        }
    }

    /// Override x86 stack-argument names/types from a parsed prototype, in stack order.
    pub fn apply_prototype_x86(&mut self, params: &[ProtoParam]) {
        let mut keys: Vec<(Reg, i64)> = self.stack.keys().copied().collect();
        keys.sort_by_key(|(_, offset)| *offset);
        for ((base, offset), param) in keys.iter().zip(params) {
            self.stack.insert((*base, *offset), param.name.clone());
            self.stack_types.insert((*base, *offset), param.ty);
        }
    }

    /// Register arguments in calling-convention order.
    pub fn ordered(&self) -> Vec<(&'static str, &str)> {
        self.ordered_regs()
            .into_iter()
            .map(|(reg, name)| (reg.name64(), name))
            .collect()
    }

    fn ordered_regs(&self) -> Vec<(Reg, &str)> {
        X64_ARG_REGS
            .iter()
            .filter_map(|reg| self.names.get(reg).map(|name| (*reg, name.as_str())))
            .collect()
    }

    /// Stack arguments as `([ebp+0x8], arg1)` in offset order.
    pub fn stack_ordered(&self) -> Vec<(String, &str)> {
        self.stack
            .iter()
            .map(|((base, offset), name)| {
                let prefix = if *base == Reg::Rbp { "ebp" } else { "esp" };
                (format!("[{prefix}+{offset:#x}]"), name.as_str())
            })
            .collect()
    }

    /// C parameter list (`int64 arg1, char* arg2`) or `void`.
    pub fn signature(&self) -> String {
        let mut params: Vec<String> = self
            .ordered_regs()
            .into_iter()
            .map(|(reg, name)| {
                let ty = self.types.get(&reg).copied().unwrap_or(ArgType::Int64);
                format!("{} {name}", ty.c_name())
            })
            .collect();
        for ((base, offset), name) in &self.stack {
            let ty = self
                .stack_types
                .get(&(*base, *offset))
                .copied()
                .unwrap_or(ArgType::Int64);
            params.push(format!("{} {name}", ty.c_name()));
        }
        if params.is_empty() {
            "void".to_owned()
        } else {
            params.join(", ")
        }
    }

    /// `arg1=rcx, arg2=[ebp+0x8]` for a leading comment.
    pub fn comment(&self) -> String {
        let mut parts: Vec<String> = self
            .ordered()
            .iter()
            .map(|(reg, name)| format!("{name}={reg}"))
            .collect();
        parts.extend(
            self.stack_ordered()
                .iter()
                .map(|(location, name)| format!("{name}={location}")),
        );
        parts.join(", ")
    }
}

/// Inferred return type for the pseudo-C signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnKind {
    Void,
    Int32,
    Int64,
}

impl ReturnKind {
    pub const fn c_name(self) -> &'static str {
        match self {
            ReturnKind::Void => "void",
            ReturnKind::Int32 => "int32",
            ReturnKind::Int64 => "int64",
        }
    }
}

/// Calling convention inferred from the function epilogue (x86) or fixed (x64).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallingConvention {
    /// Windows x64 fastcall: no keyword in the signature.
    System,
    Cdecl,
    Stdcall,
}

impl CallingConvention {
    pub const fn keyword(self) -> &'static str {
        match self {
            CallingConvention::System => "",
            CallingConvention::Cdecl => "__cdecl ",
            CallingConvention::Stdcall => "__stdcall ",
        }
    }
}

/// Everything the printer needs for an L2 signature and body.
#[derive(Debug, Clone)]
pub struct L2Info {
    pub args: ArgNames,
    pub ret: ReturnKind,
    pub abi: CallingConvention,
}

/// Recover argument names, return type and calling convention.
pub fn recover_l2(session: &mut Session, start: Address, end: Address) -> Result<L2Info> {
    let abi = if session.image().is_64 {
        CallingConvention::System
    } else {
        detect_x86_abi(session, start, end)?
    };
    let args = recover_args(session, start, end)?;
    let ret = recover_return(session, start, end)?;
    Ok(L2Info { args, ret, abi })
}

/// `ret imm16` (stdcall) vs `ret` (cdecl) in the function epilogue.
fn detect_x86_abi(
    session: &mut Session,
    start: Address,
    end: Address,
) -> Result<CallingConvention> {
    let mut addr = start;
    let mut last = None;
    while addr < end {
        let Some(insn) = session.insn_at(addr)? else {
            break;
        };
        if insn.is_invalid() {
            break;
        }
        let next = insn.next_addr();
        last = Some(insn);
        if next <= addr || next >= end {
            break;
        }
        addr = next;
    }
    let Some(insn) = last else {
        return Ok(CallingConvention::Cdecl);
    };
    if insn.mnemonic() == iced_x86::Mnemonic::Ret && insn.op_count() > 0 {
        return Ok(CallingConvention::Stdcall);
    }
    Ok(CallingConvention::Cdecl)
}

/// Return type from writes to `rax`/`eax` before the epilogue, with a tail-call or call
/// fallback: a function whose value comes straight from a call returns a full register.
fn recover_return(session: &mut Session, start: Address, end: Address) -> Result<ReturnKind> {
    let (lifted, _exprs) = lift_function(session, start, end)?;
    let mut width: Option<u8> = None;
    let mut call_like = false;
    for item in &lifted {
        for stmt in &item.stmts {
            match stmt {
                Stmt::Call { .. } => call_like = true,
                Stmt::WriteReg { reg, .. } if reg.base == Reg::Rax => width = Some(reg.width),
                _ => {}
            }
        }
        if let recraft_ir::Flow::Goto(target) = item.flow
            && (target < start || target >= end)
        {
            // Tail call: the callee's value is returned unchanged.
            call_like = true;
        }
    }
    Ok(match width {
        Some(8) => ReturnKind::Int64,
        Some(_) => ReturnKind::Int32,
        None if call_like => ReturnKind::Int64,
        None => ReturnKind::Void,
    })
}

/// Find argument locations that the function reads before writing.
pub fn recover_args(session: &mut Session, start: Address, end: Address) -> Result<ArgNames> {
    if session.image().is_64 {
        recover_x64(session, start, end)
    } else {
        recover_x86(session, start, end)
    }
}

fn lift_function(
    session: &mut Session,
    start: Address,
    end: Address,
) -> Result<(Vec<recraft_ir::LiftedInsn>, Vec<Expr>)> {
    let mut lifter = Lifter::new();
    let mut lifted = Vec::new();
    let mut addr = start;
    while addr < end && lifted.len() < MAX_ARG_SCAN_INSNS {
        let Some(insn) = session.insn_at(addr)? else {
            break;
        };
        if insn.is_invalid() {
            break;
        }
        let item = lifter.lift(&insn);
        let next = insn.next_addr();
        lifted.push(item);
        if next <= addr || next >= end {
            break;
        }
        addr = next;
    }
    Ok((lifted, lifter.into_exprs()))
}

fn recover_x64(session: &mut Session, start: Address, end: Address) -> Result<ArgNames> {
    let (lifted, exprs) = lift_function(session, start, end)?;
    let arg_registers: BTreeSet<Reg> = X64_ARG_REGS.into_iter().collect();
    let mut written: BTreeSet<Reg> = BTreeSet::new();
    let mut incoming: BTreeSet<Reg> = BTreeSet::new();

    for item in &lifted {
        for stmt in &item.stmts {
            walk_stmt_reads(stmt, &exprs, &mut |view: &RegView| {
                if arg_registers.contains(&view.base) && !written.contains(&view.base) {
                    incoming.insert(view.base);
                }
            });
            if let Stmt::WriteReg { reg, .. } = stmt {
                written.insert(reg.base);
            }
        }
    }

    let mut names = BTreeMap::new();
    for (index, reg) in X64_ARG_REGS.iter().enumerate() {
        if incoming.contains(reg) {
            names.insert(*reg, format!("arg{}", index + 1));
        }
    }
    let types = infer_pointer_types(session, start, end, &names)?;
    Ok(ArgNames {
        names,
        stack: BTreeMap::new(),
        types,
        stack_types: BTreeMap::new(),
    })
}

/// Infer pointer parameter types from accesses through the argument register.
///
/// A single size dominates: 1 -> `char*`, 2 -> `wchar_t*`, 4 -> `int32*`, 8 ->
/// `int64*`; mixed widths become `void*`. Arguments never used as a memory base keep
/// the default `int64`.
fn infer_pointer_types(
    session: &mut Session,
    start: Address,
    end: Address,
    names: &BTreeMap<Reg, String>,
) -> Result<BTreeMap<Reg, ArgType>> {
    if names.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut sizes: BTreeMap<Reg, BTreeSet<u8>> = BTreeMap::new();
    let mut addr = start;
    while addr < end {
        let Some(insn) = session.insn_at(addr)? else {
            break;
        };
        if insn.is_invalid() {
            break;
        }
        if insn.has_memory_operand() {
            // Indexed (base + index*scale) accesses are pointer evidence too: the base
            // register still points at the array/data the element is loaded from.
            let base = insn.memory_base();
            if let Some(view) = recraft_ir::from_iced(base)
                && names.contains_key(&view.base)
                && let Some(size) = access_size(insn.memory_size())
            {
                sizes.entry(view.base).or_default().insert(size);
            }
        }
        let next = insn.next_addr();
        if next <= addr || next >= end {
            break;
        }
        addr = next;
    }
    let mut types = BTreeMap::new();
    for (reg, observed) in sizes {
        let ty = if observed.len() != 1 {
            ArgType::VoidPtr
        } else {
            match observed.iter().next().copied().unwrap_or(8) {
                1 => ArgType::CharPtr,
                2 => ArgType::WCharPtr,
                4 => ArgType::Int32Ptr,
                _ => ArgType::Int64Ptr,
            }
        };
        types.insert(reg, ty);
    }
    Ok(types)
}

/// Byte width of a memory operand (SIMD/FPU sizes are not pointer candidates).
fn access_size(size: iced_x86::MemorySize) -> Option<u8> {
    use iced_x86::MemorySize as S;
    Some(match size {
        S::UInt8 | S::Int8 => 1,
        S::UInt16 | S::Int16 => 2,
        S::UInt32 | S::Int32 => 4,
        S::UInt64 | S::Int64 => 8,
        _ => return None,
    })
}

fn recover_x86(session: &mut Session, start: Address, end: Address) -> Result<ArgNames> {
    let (lifted, exprs) = lift_function(session, start, end)?;
    let mut frame_established = false;
    let mut esp_shifted = false;
    let mut ebp_offsets: BTreeSet<i64> = BTreeSet::new();
    let mut esp_offsets: BTreeSet<i64> = BTreeSet::new();

    for item in &lifted {
        for stmt in &item.stmts {
            if let Stmt::Intrinsic { text } = stmt
                && !frame_established
                && (text.starts_with("push ")
                    || text.starts_with("sub esp")
                    || text.starts_with("sub rsp"))
            {
                // esp-relative argument offsets are only stable until the stack moves.
                esp_shifted = true;
            }
            if let Stmt::WriteReg { reg, value } = stmt
                && reg.base == Reg::Rbp
                && matches!(&exprs[*value as usize], Expr::Reg(view) if view.base == Reg::Rsp)
            {
                frame_established = true;
            }
            if let Stmt::WriteReg { reg, .. } = stmt
                && !frame_established
                && reg.base == Reg::Rsp
            {
                // push/pop and `sub esp, N` are lifted as writes to the stack pointer.
                esp_shifted = true;
            }
            walk_stmt_loads(stmt, &exprs, &mut |addr| {
                let Some((base, offset)) = stack_add(&exprs, addr) else {
                    return;
                };
                if frame_established && base == Reg::Rbp && offset >= 8 && (offset - 8) % 4 == 0 {
                    ebp_offsets.insert(offset);
                } else if !frame_established
                    && !esp_shifted
                    && base == Reg::Rsp
                    && offset >= 4
                    && offset % 4 == 0
                {
                    // [esp+4] is the first argument (return address is at [esp]).
                    esp_offsets.insert(offset);
                }
            });
        }
    }

    let mut stack = BTreeMap::new();
    if frame_established {
        for (index, offset) in ebp_offsets.iter().enumerate() {
            stack.insert((Reg::Rbp, *offset), format!("arg{}", index + 1));
        }
    } else {
        for (index, offset) in esp_offsets.iter().enumerate() {
            stack.insert((Reg::Rsp, *offset), format!("arg{}", index + 1));
        }
    }
    Ok(ArgNames {
        names: BTreeMap::new(),
        stack,
        types: BTreeMap::new(),
        stack_types: BTreeMap::new(),
    })
}

fn stack_add(exprs: &[Expr], id: ExprId) -> Option<(Reg, i64)> {
    let Expr::Bin { op, lhs, rhs } = &exprs[id as usize] else {
        return None;
    };
    if *op != BinOp::Add {
        return None;
    }
    let base_of = |id: ExprId| match &exprs[id as usize] {
        Expr::Reg(view) if matches!(view.base, Reg::Rsp | Reg::Rbp) => Some(view.base),
        _ => None,
    };
    let const_of = |id: ExprId| match &exprs[id as usize] {
        Expr::Const(value) => Some(*value as i64),
        _ => None,
    };
    if let Some(base) = base_of(*lhs)
        && let Some(offset) = const_of(*rhs)
    {
        return Some((base, offset));
    }
    if let Some(base) = base_of(*rhs)
        && let Some(offset) = const_of(*lhs)
    {
        return Some((base, offset));
    }
    None
}

fn walk_stmt_reads(stmt: &Stmt, exprs: &[Expr], visit: &mut impl FnMut(&RegView)) {
    match stmt {
        Stmt::WriteReg { value, .. } => walk_expr(exprs, *value, visit),
        Stmt::Store { addr, value, .. } | Stmt::StoreFloat { addr, value, .. } => {
            walk_expr(exprs, *addr, visit);
            walk_expr(exprs, *value, visit);
        }
        Stmt::Call { .. } | Stmt::Intrinsic { .. } => {}
    }
}

fn walk_stmt_loads(stmt: &Stmt, exprs: &[Expr], visit: &mut impl FnMut(ExprId)) {
    match stmt {
        Stmt::WriteReg { value, .. } => walk_expr_loads(exprs, *value, visit),
        Stmt::Store { addr, value, .. } | Stmt::StoreFloat { addr, value, .. } => {
            walk_expr_loads(exprs, *addr, visit);
            walk_expr_loads(exprs, *value, visit);
        }
        Stmt::Call { .. } | Stmt::Intrinsic { .. } => {}
    }
}

/// Visits the address expression of every `Load` reachable from `id`.
fn walk_expr_loads(exprs: &[Expr], id: ExprId, visit: &mut impl FnMut(ExprId)) {
    match &exprs[id as usize] {
        Expr::Load { addr, .. } | Expr::LoadFloat { addr, .. } => {
            visit(*addr);
            walk_expr_loads(exprs, *addr, visit);
        }
        Expr::Bin { lhs, rhs, .. } | Expr::Cmp { lhs, rhs, .. } => {
            walk_expr_loads(exprs, *lhs, visit);
            walk_expr_loads(exprs, *rhs, visit);
        }
        Expr::Select {
            cond,
            then,
            otherwise,
        } => {
            walk_expr_loads(exprs, *cond, visit);
            walk_expr_loads(exprs, *then, visit);
            walk_expr_loads(exprs, *otherwise, visit);
        }
        Expr::Un { arg, .. } | Expr::Cast { arg, .. } => walk_expr_loads(exprs, *arg, visit),
        Expr::Reg(_) | Expr::Const(_) | Expr::Temp(_) => {}
    }
}

fn walk_expr(exprs: &[Expr], id: ExprId, visit: &mut impl FnMut(&RegView)) {
    match &exprs[id as usize] {
        Expr::Reg(view) => visit(view),
        Expr::Load { addr, .. } | Expr::LoadFloat { addr, .. } => walk_expr(exprs, *addr, visit),
        Expr::Bin { lhs, rhs, .. } | Expr::Cmp { lhs, rhs, .. } => {
            walk_expr(exprs, *lhs, visit);
            walk_expr(exprs, *rhs, visit);
        }
        Expr::Select {
            cond,
            then,
            otherwise,
        } => {
            walk_expr(exprs, *cond, visit);
            walk_expr(exprs, *then, visit);
            walk_expr(exprs, *otherwise, visit);
        }
        Expr::Un { arg, .. } | Expr::Cast { arg, .. } => walk_expr(exprs, *arg, visit),
        Expr::Const(_) | Expr::Temp(_) => {}
    }
}

/// One parameter of a parsed user prototype.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtoParam {
    pub ty: ArgType,
    /// Parameter name, or a generated `argN` when the declaration omitted it.
    pub name: String,
}

/// Parsed form of a user prototype like `int __stdcall foo(const char* text, int count)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtoSig {
    pub ret: ReturnKind,
    /// Function name from the declaration, when present.
    pub name: Option<String>,
    pub cc: Option<CallingConvention>,
    pub params: Vec<ProtoParam>,
}

/// Best-effort parser for a single C function declaration.
///
/// Supports `RET [__cdecl|__stdcall|__fastcall|__thiscall] name(TYPE [name], ...)`
/// with `const`/`volatile`/`struct` qualifiers and pointer stars; nested parentheses
/// (function pointers) and arrays are not supported and yield `None`.
pub fn parse_prototype(text: &str) -> Option<ProtoSig> {
    let text = text.trim().trim_end_matches(';').trim();
    if text.is_empty() {
        return None;
    }
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    if close < open {
        return None;
    }
    let head = text[..open].trim();
    let inside = text[open + 1..close].trim();
    if head.contains('(') || inside.contains('(') {
        return None; // function pointers / nested declarators
    }

    let mut cc = None;
    let mut head_tokens: Vec<&str> = Vec::new();
    for token in head.split_whitespace() {
        let lower = token.to_ascii_lowercase();
        if lower.contains("__stdcall") || lower == "stdcall" {
            cc = Some(CallingConvention::Stdcall);
        } else if lower.contains("__cdecl") || lower == "cdecl" {
            cc = Some(CallingConvention::Cdecl);
        } else if lower.contains("__fastcall") || lower.contains("__thiscall") {
            cc = Some(CallingConvention::System);
        } else {
            head_tokens.push(token);
        }
    }
    let name = head_tokens
        .iter()
        .rev()
        .find(|token| is_identifier(token))
        .map(|token| (*token).to_owned());
    if let Some(name_token) = name.as_ref()
        && let Some(index) = head_tokens.iter().rposition(|token| token == name_token)
    {
        head_tokens.remove(index);
    }
    let ret_text = head_tokens.join(" ");
    let ret = parse_return(&ret_text)?;

    let params = if inside.is_empty() || inside.eq_ignore_ascii_case("void") {
        Vec::new()
    } else {
        let mut params = Vec::new();
        for (index, raw) in inside.split(',').enumerate() {
            let param = parse_param(raw, index)?;
            params.push(param);
        }
        params
    };
    Some(ProtoSig {
        ret,
        name,
        cc,
        params,
    })
}

fn is_identifier(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        && !matches!(
            token.to_ascii_lowercase().as_str(),
            "void"
                | "int"
                | "char"
                | "short"
                | "long"
                | "unsigned"
                | "signed"
                | "const"
                | "volatile"
                | "struct"
                | "int32"
                | "int64"
                | "wchar_t"
                | "bool"
                | "float"
                | "double"
                | "size_t"
        )
}

fn parse_return(text: &str) -> Option<ReturnKind> {
    let normalized = text.trim();
    if normalized.is_empty() {
        return None;
    }
    let pointer = normalized.contains('*');
    let lower = normalized.to_ascii_lowercase();
    if !pointer && (lower == "void" || lower.ends_with(" void")) {
        return Some(ReturnKind::Void);
    }
    if pointer || lower.contains("int64") || lower.contains("long long") || lower.contains("size_t")
    {
        return Some(ReturnKind::Int64);
    }
    Some(ReturnKind::Int32)
}

fn parse_param(raw: &str, index: usize) -> Option<ProtoParam> {
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let mut name = format!("arg{}", index + 1);
    let mut type_tokens = tokens.clone();
    if tokens.len() > 1 {
        let last = tokens[tokens.len() - 1];
        if is_identifier(last) {
            name = last.to_owned();
            type_tokens.truncate(tokens.len() - 1);
        }
    }
    let type_text = type_tokens.join(" ");
    if type_text.trim().is_empty() || type_text.contains('[') {
        return None;
    }
    let pointer = type_text.contains('*');
    let base = type_text
        .replace(['*', '&'], " ")
        .split_whitespace()
        .filter(|token| {
            !matches!(
                token.to_ascii_lowercase().as_str(),
                "const" | "volatile" | "struct" | "unsigned" | "signed"
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let ty = if pointer {
        match base.to_ascii_lowercase().as_str() {
            "char" | "int8_t" | "uint8_t" => ArgType::CharPtr,
            "wchar_t" | "char16_t" => ArgType::WCharPtr,
            "int" | "int32" | "int32_t" | "uint32" | "uint32_t" => ArgType::Int32Ptr,
            "int64" | "int64_t" | "uint64" | "uint64_t" | "long long" => ArgType::Int64Ptr,
            _ => ArgType::VoidPtr,
        }
    } else if base.contains("int64") || base == "long long" || base == "size_t" {
        ArgType::Int64
    } else {
        ArgType::Int32
    };
    Some(ProtoParam { ty, name })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prototype_declarations() {
        let parsed = parse_prototype("int __stdcall field_probe(const int* values, int count)")
            .expect("valid prototype");
        assert_eq!(parsed.ret, ReturnKind::Int32);
        assert_eq!(parsed.cc, Some(CallingConvention::Stdcall));
        assert_eq!(parsed.name.as_deref(), Some("field_probe"));
        assert_eq!(parsed.params.len(), 2);
        assert_eq!(parsed.params[0].ty, ArgType::Int32Ptr);
        assert_eq!(parsed.params[0].name, "values");
        assert_eq!(parsed.params[1].ty, ArgType::Int32);
        assert_eq!(parsed.params[1].name, "count");

        let void = parse_prototype("void __cdecl cleanup(void);").expect("void prototype");
        assert_eq!(void.ret, ReturnKind::Void);
        assert_eq!(void.cc, Some(CallingConvention::Cdecl));
        assert!(void.params.is_empty());

        let ptr = parse_prototype("char* dup(const char* text)").expect("pointer return");
        assert_eq!(ptr.ret, ReturnKind::Int64);
        assert_eq!(ptr.params[0].ty, ArgType::CharPtr);

        assert!(parse_prototype("int (*callback)(int)").is_none());
    }
    use std::path::PathBuf;

    fn corpus_session(arch: &str) -> Option<(Session, tempfile::TempDir)> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../corpus/build/{arch}/corpus.exe"));
        let bytes = std::fs::read(path).ok()?;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("l2.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        Some((session, dir))
    }

    fn bounds(session: &Session, name: &str) -> (Address, Address) {
        let start = session.addr_of_name(name).unwrap().unwrap();
        let function = session
            .functions()
            .unwrap()
            .into_iter()
            .find(|function| function.start == start)
            .unwrap_or_else(|| panic!("{name} function missing"));
        (function.start, function.end)
    }

    #[test]
    fn x64_add_leaf_uses_two_argument_registers() {
        let Some((mut session, _dir)) = corpus_session("x64") else {
            return;
        };
        let (start, end) = bounds(&session, "add_leaf");
        let args = recover_args(&mut session, start, end).unwrap();
        let ordered = args.ordered();
        assert_eq!(ordered.len(), 2, "{ordered:?}");
        assert_eq!(ordered[0], ("rcx", "arg1"));
        assert_eq!(ordered[1], ("rdx", "arg2"));
    }

    #[test]
    fn x64_written_registers_are_not_arguments() {
        let Some((mut session, _dir)) = corpus_session("x64") else {
            return;
        };
        let (start, end) = bounds(&session, "branchy");
        let args = recover_args(&mut session, start, end).unwrap();
        assert_eq!(args.ordered().len(), 1, "{:?}", args.ordered());
    }

    #[test]
    fn x86_stack_arguments_are_recovered() {
        let Some((mut session, _dir)) = corpus_session("x86") else {
            return;
        };
        assert!(!session.image().is_64);
        let (start, end) = bounds(&session, "add_leaf");
        let args = recover_args(&mut session, start, end).unwrap();
        let stack = args.stack_ordered();
        assert_eq!(stack.len(), 2, "{stack:?}");
        assert_eq!(stack[0].1, "arg1");
        assert_eq!(stack[1].1, "arg2");
        assert!(args.signature().contains("int64 arg1"));
    }

    #[test]
    fn return_types_and_x64_abi() {
        let Some((mut session, _dir)) = corpus_session("x64") else {
            return;
        };
        let (start, end) = bounds(&session, "add_leaf");
        let info = recover_l2(&mut session, start, end).unwrap();
        assert_eq!(info.ret, ReturnKind::Int32);
        assert_eq!(info.abi, CallingConvention::System);
        assert_eq!(info.args.ordered().len(), 2);
    }

    #[test]
    fn x86_cdecl_vs_stdcall_is_detected() {
        let Some((mut session, _dir)) = corpus_session("x86") else {
            return;
        };
        let (start, end) = bounds(&session, "add_leaf");
        let cdecl = recover_l2(&mut session, start, end).unwrap();
        assert_eq!(cdecl.abi, CallingConvention::Cdecl);
        assert_eq!(cdecl.ret, ReturnKind::Int32);

        // The stdcall probe is exported under both the plain and decorated names.
        let (start, end) = bounds(&session, "_stdcall_add@8");
        let stdcall = recover_l2(&mut session, start, end).unwrap();
        assert_eq!(stdcall.abi, CallingConvention::Stdcall);
        assert_eq!(stdcall.ret, ReturnKind::Int32);
    }

    #[test]
    fn pointer_arguments_are_typed_from_memory_access_width() {
        let Some((mut session, _dir)) = corpus_session("x64") else {
            return;
        };
        let (start, end) = bounds(&session, "str_probe");
        let args = recover_args(&mut session, start, end).unwrap();
        assert!(
            args.signature().contains("char* arg1"),
            "{}",
            args.signature()
        );

        let (start, end) = bounds(&session, "wide_probe");
        let args = recover_args(&mut session, start, end).unwrap();
        assert!(
            args.signature().contains("wchar_t* arg1"),
            "{}",
            args.signature()
        );

        // Pure arithmetic arguments stay int64.
        let (start, end) = bounds(&session, "add_leaf");
        let args = recover_args(&mut session, start, end).unwrap();
        assert!(
            args.signature().contains("int64 arg1"),
            "{}",
            args.signature()
        );
    }

    #[test]
    fn x86_shifted_stack_falls_back_to_slots() {
        let Some((mut session, _dir)) = corpus_session("x86") else {
            return;
        };
        // noinline_helper pushes onto the stack before touching its arguments, so the
        // esp-relative heuristic stays conservative instead of mislabelling locals.
        let (start, end) = bounds(&session, "noinline_helper");
        let args = recover_args(&mut session, start, end).unwrap();
        assert!(args.is_empty(), "{:?}", args.stack_ordered());
    }
}
