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

use std::collections::{BTreeMap, BTreeSet, HashMap};

use recraft_common::{Address, Result};
use recraft_core::Session;
use recraft_decode::iced_x86;
use recraft_ir::{BinOp, Expr, ExprId, Lifter, Reg, RegView, Stmt};

/// Windows x64 integer argument registers, in declaration order.
pub const X64_ARG_REGS: [Reg; 4] = [Reg::Rcx, Reg::Rdx, Reg::R8, Reg::R9];

/// x64 scalar floating-point argument registers (`float`/`double`).
pub const X64_FLOAT_ARG_REGS: [Reg; 4] = [Reg::Xmm0, Reg::Xmm1, Reg::Xmm2, Reg::Xmm3];

const MAX_ARG_SCAN_INSNS: usize = 4096;

/// Recovered argument names for one function.
#[derive(Debug, Clone, Default)]
pub struct ArgNames {
    names: BTreeMap<Reg, String>,
    /// x86 stack arguments keyed by `(base register, byte offset)`.
    stack: BTreeMap<(Reg, i64), String>,
    /// Inferred parameter types for register arguments (defaults to `int64`).
    types: BTreeMap<Reg, ArgType>,
    /// Parameter types for x86 stack arguments (from a user prototype).
    stack_types: BTreeMap<(Reg, i64), ArgType>,
    /// Type of a stack argument whose own type is unknown: the slot is 4 bytes wide on
    /// 32-bit targets and 8 on 64-bit ones.
    stack_slot_type: ArgType,
    /// Parameter order from a user prototype: register types alone cannot tell a mixed
    /// integer/float list apart, so the declared order is kept verbatim.
    declared: Option<Vec<(ArgType, String)>>,
}

/// Parameter type inferred from how the argument is used.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ArgType {
    Int32,
    #[default]
    Int64,
    /// Byte-sized memory access through the argument.
    CharPtr,
    /// 2-byte access (`wchar_t*`).
    WCharPtr,
    Int32Ptr,
    Int64Ptr,
    /// Mixed or unknown access widths behind a pointer.
    VoidPtr,
    /// Scalar floating-point parameter (`xmm0`..`xmm3`).
    F32,
    F64,
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
            ArgType::F32 => "f32",
            ArgType::F64 => "f64",
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

    /// True when the register is one of the x64 floating-point argument registers.
    pub fn is_float_arg(&self, reg: Reg) -> bool {
        X64_FLOAT_ARG_REGS.contains(&reg)
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

    /// Number of recovered argument locations (register + stack).
    pub fn count(&self) -> usize {
        self.names
            .keys()
            .filter(|reg| X64_ARG_REGS.contains(reg))
            .count()
            + self.stack.len()
    }

    /// Build arguments from a parsed user prototype (x64 register arguments; at most
    /// four integer and four floating-point slots are representable without stack-argument
    /// layout).
    pub fn from_prototype(params: &[ProtoParam]) -> Self {
        let mut names = BTreeMap::new();
        let mut types = BTreeMap::new();
        let mut integer_slot = 0;
        let mut float_slot = 0;
        for param in params {
            let reg = match param.ty {
                ArgType::F32 | ArgType::F64 => {
                    let Some(reg) = X64_FLOAT_ARG_REGS.get(float_slot).copied() else {
                        break;
                    };
                    float_slot += 1;
                    reg
                }
                _ => {
                    let Some(reg) = X64_ARG_REGS.get(integer_slot).copied() else {
                        break;
                    };
                    integer_slot += 1;
                    reg
                }
            };
            names.insert(reg, param.name.clone());
            types.insert(reg, param.ty);
        }
        let declared = params
            .iter()
            .map(|param| (param.ty, param.name.clone()))
            .collect();
        ArgNames {
            names,
            stack: BTreeMap::new(),
            types,
            stack_types: BTreeMap::new(),
            stack_slot_type: ArgType::Int64,
            declared: Some(declared),
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
        let mut params: Vec<String> = Vec::new();
        if let Some(declared) = &self.declared {
            params.extend(
                declared
                    .iter()
                    .map(|(ty, name)| format!("{} {name}", ty.c_name())),
            );
            return if params.is_empty() {
                "void".to_owned()
            } else {
                params.join(", ")
            };
        }
        // Fill gaps between recovered x64 argument registers: a function that reads rcx
        // and r8 still has the rdx parameter in its ABI, even if the body never uses it.
        if let Some(highest) = X64_ARG_REGS
            .iter()
            .rposition(|reg| self.names.contains_key(reg))
        {
            for (index, reg) in X64_ARG_REGS.iter().enumerate().take(highest + 1) {
                let name = self
                    .names
                    .get(reg)
                    .cloned()
                    .unwrap_or_else(|| format!("arg{}", index + 1));
                let ty = self.types.get(reg).copied().unwrap_or(ArgType::Int64);
                params.push(format!("{} {name}", ty.c_name()));
            }
        }
        for ((base, offset), name) in &self.stack {
            let ty = self
                .stack_types
                .get(&(*base, *offset))
                .copied()
                .unwrap_or(self.stack_slot_type);
            params.push(format!("{} {name}", ty.c_name()));
        }
        // A function whose parameters all arrive in registers: name the floating-point ones
        // after the integer slots. Mixed integer/float parameter *order* is not recoverable
        // without prototypes, so a function with integer arguments leaves `xmmN` unnamed.
        if self.count() == 0 {
            for reg in X64_FLOAT_ARG_REGS {
                let Some(name) = self.names.get(&reg) else {
                    continue;
                };
                let ty = self.types.get(&reg).copied().unwrap_or(ArgType::F32);
                params.push(format!("{} {name}", ty.c_name()));
            }
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
        parts.extend(X64_FLOAT_ARG_REGS.iter().filter_map(|reg| {
            self.names
                .get(reg)
                .map(|name| format!("{name}={}", reg.name64()))
        }));
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
    /// Scalar floating-point return in `xmm0` (`f32`/`f64`).
    F32,
    F64,
}

impl ReturnKind {
    pub const fn c_name(self) -> &'static str {
        match self {
            ReturnKind::Void => "void",
            ReturnKind::Int32 => "int32",
            ReturnKind::Int64 => "int64",
            ReturnKind::F32 => "f32",
            ReturnKind::F64 => "f64",
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
    let mut float_width: Option<u8> = None;
    let mut call_like = false;
    for item in &lifted {
        for stmt in &item.stmts {
            match stmt {
                Stmt::Call { .. } => call_like = true,
                Stmt::WriteReg { reg, .. } if reg.base == Reg::Rax => width = Some(reg.width),
                Stmt::WriteReg { reg, .. } if reg.base == Reg::Xmm0 => {
                    float_width = Some(reg.width);
                }
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
        // No integer return register is written: a value left in `xmm0` is a scalar float
        // return (`float f(...)`).
        None if float_width == Some(8) => ReturnKind::F64,
        None if float_width.is_some_and(|width| width < 8) => ReturnKind::F32,
        None if call_like => ReturnKind::Int64,
        None => ReturnKind::Void,
    })
}

/// Find argument locations that the function reads before writing.
pub fn recover_args(session: &mut Session, start: Address, end: Address) -> Result<ArgNames> {
    let ranges = function_ranges(session)?;
    let mut memo = BTreeMap::new();
    recover_args_at(session, start, end, 0, &mut memo, &ranges)
}

/// All defined functions keyed by start, so a nested scan does not query the project once
/// per call site.
fn function_ranges(session: &Session) -> Result<HashMap<u64, (Address, Address)>> {
    Ok(session
        .functions()?
        .into_iter()
        .map(|function| (function.start.get(), (function.start, function.end)))
        .collect())
}

/// How deep a direct-call chain is followed while a callee's argument count is needed.
/// A chain of wrappers is common (an API forwarded through a helper), while cycles and
/// very deep chains stop at the limit and simply contribute no extra arguments.
const MAX_CALL_DEPTH: usize = 4;

fn recover_args_at(
    session: &mut Session,
    start: Address,
    end: Address,
    depth: usize,
    memo: &mut BTreeMap<u64, usize>,
    ranges: &HashMap<u64, (Address, Address)>,
) -> Result<ArgNames> {
    if session.image().is_64 {
        recover_x64(session, start, end, depth, memo, ranges)
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

fn recover_x64(
    session: &mut Session,
    start: Address,
    end: Address,
    depth: usize,
    memo: &mut BTreeMap<u64, usize>,
    ranges: &HashMap<u64, (Address, Address)>,
) -> Result<ArgNames> {
    let scan = scan_incoming_args(session, start, end, depth, memo, ranges)?;
    let mut names = BTreeMap::new();
    for (index, reg) in X64_ARG_REGS.iter().enumerate() {
        if scan.incoming.contains(reg) {
            names.insert(*reg, format!("arg{}", index + 1));
        }
    }
    let mut types = infer_pointer_types(session, start, end, &names)?;
    // A parameter only ever read through its 32-bit half is an `int` in the source.
    // Memory-based pointer inference wins over the read width.
    for (reg, widths) in &scan.widths {
        if names.contains_key(reg)
            && !types.contains_key(reg)
            && widths.len() == 1
            && widths.contains(&4)
        {
            types.insert(*reg, ArgType::Int32);
        }
    }
    // A function with no integer parameters can still take scalar floats in `xmm0`..: name
    // them in slot order and type them from the read width. Mixed integer/float argument
    // order is not recoverable without a prototype, so those stay raw registers.
    if names.is_empty() {
        for (index, reg) in X64_FLOAT_ARG_REGS.iter().enumerate() {
            let Some(read_sizes) = scan.floats.get(reg) else {
                continue;
            };
            // A 16-byte load/store is a vector operation, not a scalar parameter.
            if read_sizes.contains(&16) && read_sizes.len() == 1 {
                continue;
            }
            let ty = if read_sizes.contains(&8) {
                ArgType::F64
            } else {
                ArgType::F32
            };
            names.insert(*reg, format!("arg{}", index + 1));
            types.insert(*reg, ty);
        }
    }
    Ok(ArgNames {
        names,
        stack: BTreeMap::new(),
        types,
        stack_types: BTreeMap::new(),
        stack_slot_type: ArgType::Int32,
        declared: None,
    })
}

/// Argument count of the function at `[start, end)`, without the pointer-type inference
/// (which rescans the whole range). Used while following direct calls, where only the
/// arity matters.
pub(crate) fn recover_arg_count(
    session: &mut Session,
    start: Address,
    end: Address,
    memo: &mut BTreeMap<u64, usize>,
    ranges: &HashMap<u64, (Address, Address)>,
) -> Result<usize> {
    if let Some(count) = memo.get(&start.get()) {
        return Ok(*count);
    }
    let count = if session.image().is_64 {
        scan_incoming_args(session, start, end, 0, memo, ranges)?
            .incoming
            .len()
    } else {
        recover_x86(session, start, end)?.count()
    };
    memo.insert(start.get(), count);
    Ok(count)
}

/// Registers whose incoming value the function consumes, and the widths those reads used.
struct IncomingScan {
    incoming: BTreeSet<Reg>,
    widths: BTreeMap<Reg, BTreeSet<u8>>,
    /// Floating-point argument registers read before being written, with read widths.
    floats: BTreeMap<Reg, BTreeSet<u8>>,
}

/// Registers whose incoming value the function consumes: read before their first write, or
/// consumed as an argument by a direct call.
fn scan_incoming_args(
    session: &mut Session,
    start: Address,
    end: Address,
    depth: usize,
    memo: &mut BTreeMap<u64, usize>,
    ranges: &HashMap<u64, (Address, Address)>,
) -> Result<IncomingScan> {
    let (lifted, exprs) = lift_function(session, start, end)?;
    let arg_registers: BTreeSet<Reg> = X64_ARG_REGS.into_iter().collect();
    let mut written: BTreeSet<Reg> = BTreeSet::new();
    let mut incoming: BTreeSet<Reg> = BTreeSet::new();
    let mut widths: BTreeMap<Reg, BTreeSet<u8>> = BTreeMap::new();
    let mut floats: BTreeMap<Reg, BTreeSet<u8>> = BTreeMap::new();

    for item in &lifted {
        for stmt in &item.stmts {
            // A write's value is read *before* the write, so `xor r8d, r8d` must not look
            // like an incoming `r8`; only live registers count (self-cancelling forms are
            // constants, see `walk_live_reads`).
            let mut reads: Vec<(Reg, u8)> = Vec::new();
            let mut float_sizes: BTreeSet<u8> = BTreeSet::new();
            match stmt {
                Stmt::WriteReg { value, .. } => {
                    walk_live_reads(&exprs, *value, &mut |view| {
                        reads.push((view.base, view.width));
                    });
                    walk_float_sizes(&exprs, *value, &mut |size| {
                        float_sizes.insert(size);
                    });
                }
                Stmt::Store { addr, value, .. } | Stmt::StoreFloat { addr, value, .. } => {
                    walk_live_reads(&exprs, *addr, &mut |view| {
                        reads.push((view.base, view.width));
                    });
                    walk_live_reads(&exprs, *value, &mut |view| {
                        reads.push((view.base, view.width));
                    });
                    walk_float_sizes(&exprs, *addr, &mut |size| {
                        float_sizes.insert(size);
                    });
                    walk_float_sizes(&exprs, *value, &mut |size| {
                        float_sizes.insert(size);
                    });
                }
                Stmt::Call { .. } | Stmt::CallIndirect { .. } | Stmt::Intrinsic { .. } => {}
            }
            for (base, width) in reads {
                if arg_registers.contains(&base) && !written.contains(&base) {
                    incoming.insert(base);
                    widths.entry(base).or_default().insert(width);
                }
                if X64_FLOAT_ARG_REGS.contains(&base) && !written.contains(&base) {
                    let sizes = floats.entry(base).or_default();
                    sizes.insert(width);
                    // The register view of an xmm operand is 16 bytes wide even for a
                    // scalar op; the memory operand's size tells `float` from `double`.
                    for size in &float_sizes {
                        sizes.insert(*size);
                    }
                }
            }
            // A call consumes the argument registers its callee declares. Without this a
            // wrapper that forwards its incoming `rcx` to a helper (`call_str` 鈫?            // `str_probe`) looks argument-less, and the call site loses the argument too.
            if let Stmt::Call { target, args } = stmt {
                let count = call_arity(session, *target, depth, memo, ranges)?;
                for reg in X64_ARG_REGS.iter().take(count.min(args.len())) {
                    if !written.contains(reg) {
                        incoming.insert(*reg);
                    }
                }
            }
            if let Stmt::WriteReg { reg, .. } = stmt {
                written.insert(reg.base);
            }
            // An unmodelled instruction may write an argument register (`cpuid` writes
            // `ecx`/`edx`, a bit test updates its first operand). Treating that register
            // as written keeps its later reads from looking like parameters.
            if let Stmt::Intrinsic { text } = stmt {
                for reg in clobbered_arg_registers(text) {
                    written.insert(reg);
                }
            }
        }
    }

    Ok(IncomingScan {
        incoming,
        widths,
        floats,
    })
}

/// Visit the byte sizes of floating-point memory operands inside an expression.
fn walk_float_sizes(exprs: &[Expr], id: ExprId, visit: &mut impl FnMut(u8)) {
    match &exprs[id as usize] {
        Expr::Const(_) | Expr::Temp(_) | Expr::Reg(_) => {}
        Expr::Load { addr, .. } => walk_float_sizes(exprs, *addr, visit),
        Expr::LoadFloat { addr, size } => {
            visit(*size);
            walk_float_sizes(exprs, *addr, visit);
        }
        Expr::Bin { lhs, rhs, .. } | Expr::Cmp { lhs, rhs, .. } => {
            walk_float_sizes(exprs, *lhs, visit);
            walk_float_sizes(exprs, *rhs, visit);
        }
        Expr::Un { arg, .. } | Expr::Cast { arg, .. } => walk_float_sizes(exprs, *arg, visit),
        Expr::Select {
            cond,
            then,
            otherwise,
        } => {
            walk_float_sizes(exprs, *cond, visit);
            walk_float_sizes(exprs, *then, visit);
            walk_float_sizes(exprs, *otherwise, visit);
        }
    }
}

/// Visit the registers an expression really reads, ignoring self-cancelling forms that the
/// printer folds to a constant (`xor edx, edx`, `sub edx, edx`, `and edx, 0`).
fn walk_live_reads(exprs: &[Expr], id: ExprId, visit: &mut impl FnMut(&RegView)) {
    match &exprs[id as usize] {
        Expr::Reg(view) => visit(view),
        Expr::Const(_) | Expr::Temp(_) => {}
        Expr::Bin { op, lhs, rhs } => {
            if crate::same_value(exprs, *lhs, *rhs) && matches!(op, BinOp::Xor | BinOp::Sub) {
                return;
            }
            if *op == BinOp::And
                && (matches!(exprs[*lhs as usize], Expr::Const(0))
                    || matches!(exprs[*rhs as usize], Expr::Const(0)))
            {
                return;
            }
            walk_live_reads(exprs, *lhs, visit);
            walk_live_reads(exprs, *rhs, visit);
        }
        Expr::Cmp { lhs, rhs, .. } => {
            walk_live_reads(exprs, *lhs, visit);
            walk_live_reads(exprs, *rhs, visit);
        }
        Expr::Load { addr, .. } | Expr::LoadFloat { addr, .. } => {
            walk_live_reads(exprs, *addr, visit);
        }
        Expr::Un { arg, .. } | Expr::Cast { arg, .. } => walk_live_reads(exprs, *arg, visit),
        Expr::Select {
            cond,
            then,
            otherwise,
        } => {
            walk_live_reads(exprs, *cond, visit);
            walk_live_reads(exprs, *then, visit);
            walk_live_reads(exprs, *otherwise, visit);
        }
    }
}

/// Argument registers an unmodelled instruction certainly writes.
///
/// The IR keeps unsupported instructions as assembly text, so the mnemonic and the first
/// register operand stand in for a write set: implicit outputs are listed (`cpuid`,
/// `rdtsc`, `rep` string ops), otherwise the first register outside a `[...]` memory
/// operand is the destination. A memory operand's base register is only read, so
/// `lock cmpxchg [rcx], rdx` does not hide a later use of `rcx`.
fn clobbered_arg_registers(text: &str) -> Vec<Reg> {
    let text = text.trim().to_ascii_lowercase();
    if matches!(text.as_str(), "int3" | "nop") {
        return Vec::new();
    }
    // Prefixes sit in front of the mnemonic (`lock cmpxchg`, `rep movsb`).
    let mut rest = text.as_str();
    loop {
        let stripped = ["lock ", "rep ", "repne ", "repe ", "bnd "]
            .iter()
            .find_map(|prefix| rest.strip_prefix(prefix));
        match stripped {
            Some(remaining) => rest = remaining,
            None => break,
        }
    }
    let (mnemonic, operands) = rest.split_once(' ').unwrap_or((rest, ""));
    let implicit: &[Reg] = match mnemonic {
        "cpuid" => &[Reg::Rax, Reg::Rbx, Reg::Rcx, Reg::Rdx],
        "rdtsc" | "rdtscp" => &[Reg::Rax, Reg::Rdx, Reg::Rcx],
        "syscall" => &[Reg::Rcx, Reg::R11],
        name if name.starts_with("movs")
            || name.starts_with("stos")
            || name.starts_with("lods")
            || name.starts_with("scas")
            || name.starts_with("cmps") =>
        {
            &[Reg::Rsi, Reg::Rdi, Reg::Rcx]
        }
        _ => &[],
    };
    let mut written: Vec<Reg> = implicit.to_vec();
    if let Some(reg) = first_argument_register(operands) {
        written.push(reg);
    }
    written
}

/// First argument register named outside a `[...]` memory operand in an operand list.
fn first_argument_register(operands: &str) -> Option<Reg> {
    let mut depth = 0usize;
    let mut token = String::new();
    for ch in operands.chars().chain(std::iter::once(' ')) {
        match ch {
            '[' => {
                depth += 1;
                token.clear();
            }
            ']' => {
                depth = depth.saturating_sub(1);
                token.clear();
            }
            _ if ch.is_ascii_alphanumeric() || ch == '_' => {
                if depth == 0 {
                    token.push(ch);
                }
            }
            _ => {
                if let Some(reg) = arg_register(&token) {
                    return Some(reg);
                }
                token.clear();
            }
        }
    }
    None
}

/// Map a register spelling to the ABI argument register it aliases.
fn arg_register(name: &str) -> Option<Reg> {
    Some(match name {
        "rax" | "eax" | "ax" | "al" | "ah" => Reg::Rax,
        "rcx" | "ecx" | "cx" | "cl" | "ch" => Reg::Rcx,
        "rdx" | "edx" | "dx" | "dl" | "dh" => Reg::Rdx,
        "r8" | "r8d" | "r8w" | "r8b" => Reg::R8,
        "r9" | "r9d" | "r9w" | "r9b" => Reg::R9,
        _ => return None,
    })
}

/// Number of arguments a direct call target consumes: a user prototype when present, else
/// the target's own recovered signature depth-first. Unknown targets (imports, thunks with
/// no recoverable body) consume nothing, so the caller's captured values still drive the
/// call-site rendering.
fn call_arity(
    session: &mut Session,
    target: Address,
    depth: usize,
    memo: &mut BTreeMap<u64, usize>,
    ranges: &HashMap<u64, (Address, Address)>,
) -> Result<usize> {
    if let Some(text) = session.prototype_at(target)?
        && let Some(proto) = parse_prototype(&text)
    {
        return Ok(proto.params.len());
    }
    if let Some(count) = memo.get(&target.get()) {
        return Ok(*count);
    }
    if depth >= MAX_CALL_DEPTH {
        return Ok(0);
    }
    let Some((callee_start, callee_end)) = ranges.get(&target.get()).copied() else {
        return Ok(0);
    };
    // Insert the placeholder first so a recursive call chain stops here instead of
    // recursing until the depth limit.
    memo.insert(target.get(), 0);
    let count = if session.image().is_64 {
        scan_incoming_args(session, callee_start, callee_end, depth + 1, memo, ranges)?
            .incoming
            .len()
    } else {
        recover_x86(session, callee_start, callee_end)?.count()
    };
    memo.insert(target.get(), count);
    Ok(count)
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
    let (lifted, exprs) = lift_function(session, start, end)?;
    let mut written: BTreeSet<Reg> = BTreeSet::new();
    // `r10 = arg1` keeps the argument's pointee evidence as long as neither side is
    // overwritten; the printer names field/array accesses through such a copy the same way.
    let mut aliases: BTreeMap<Reg, Reg> = BTreeMap::new();
    let mut sizes: BTreeMap<Reg, BTreeSet<u8>> = BTreeMap::new();
    for item in &lifted {
        for stmt in &item.stmts {
            let mut evidence: Vec<(ExprId, u8)> = Vec::new();
            match stmt {
                Stmt::WriteReg { value, .. } => {
                    visit_loads(&exprs, *value, &mut |addr, size| {
                        evidence.push((addr, size))
                    });
                }
                Stmt::Store { addr, value, size } => {
                    evidence.push((*addr, *size));
                    visit_loads(&exprs, *addr, &mut |load, width| {
                        evidence.push((load, width))
                    });
                    visit_loads(&exprs, *value, &mut |load, width| {
                        evidence.push((load, width))
                    });
                }
                Stmt::StoreFloat { addr, value, .. } => {
                    visit_loads(&exprs, *addr, &mut |load, width| {
                        evidence.push((load, width))
                    });
                    visit_loads(&exprs, *value, &mut |load, width| {
                        evidence.push((load, width))
                    });
                }
                Stmt::Call { args, .. } => {
                    for arg in args {
                        visit_loads(&exprs, *arg, &mut |load, width| {
                            evidence.push((load, width))
                        });
                    }
                }
                Stmt::CallIndirect { target } => {
                    visit_loads(&exprs, *target, &mut |load, width| {
                        evidence.push((load, width))
                    });
                }
                Stmt::Intrinsic { .. } => {}
            }
            for (addr, size) in evidence {
                if let Some(reg) = pointer_argument(&exprs, addr, names, &aliases, size) {
                    sizes.entry(reg).or_default().insert(size);
                }
            }
            if let Stmt::WriteReg { reg, value } = stmt {
                let base = reg.base;
                written.insert(base);
                aliases.remove(&base);
                if base != Reg::Rsp
                    && let Expr::Reg(source) = &exprs[*value as usize]
                    && source.base != base
                {
                    aliases.insert(base, source.base);
                }
            }
        }
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

/// Visit every memory load inside an expression with its address and width.
fn visit_loads(exprs: &[Expr], id: ExprId, visit: &mut impl FnMut(ExprId, u8)) {
    match &exprs[id as usize] {
        Expr::Load { addr, size } => {
            visit(*addr, *size);
            visit_loads(exprs, *addr, visit);
        }
        Expr::LoadFloat { addr, .. } => visit_loads(exprs, *addr, visit),
        Expr::Bin { lhs, rhs, .. } | Expr::Cmp { lhs, rhs, .. } => {
            visit_loads(exprs, *lhs, visit);
            visit_loads(exprs, *rhs, visit);
        }
        Expr::Un { arg, .. } | Expr::Cast { arg, .. } => visit_loads(exprs, *arg, visit),
        Expr::Select {
            cond,
            then,
            otherwise,
        } => {
            visit_loads(exprs, *cond, visit);
            visit_loads(exprs, *then, visit);
            visit_loads(exprs, *otherwise, visit);
        }
        Expr::Reg(_) | Expr::Const(_) | Expr::Temp(_) => {}
    }
}

/// Argument register whose pointer the memory address is based on, if the access width
/// identifies a pointee type. The address only has to originate from the argument: a copy
/// (`r10 = arg1`) or pointer arithmetic (`arg1 + 8`) keeps the pointee evidence.
fn pointer_argument(
    exprs: &[Expr],
    address: ExprId,
    names: &BTreeMap<Reg, String>,
    aliases: &BTreeMap<Reg, Reg>,
    size: u8,
) -> Option<Reg> {
    if !matches!(size, 1 | 2 | 4 | 8) {
        return None;
    }
    let mut candidates = Vec::new();
    address_registers(exprs, address, &mut candidates);
    for view in candidates {
        let source = resolve_alias(aliases, view.base);
        if names.contains_key(&source) {
            return Some(source);
        }
    }
    None
}

/// Registers of an address expression in preference order: the unscaled base first, then
/// fallbacks. A scaled index (`idx * 4`, `idx << 2`) is an element index, not the pointee.
fn address_registers(exprs: &[Expr], id: ExprId, out: &mut Vec<RegView>) {
    match &exprs[id as usize] {
        Expr::Reg(view) => out.push(*view),
        Expr::Cast { arg, .. } => address_registers(exprs, *arg, out),
        Expr::Bin {
            op: BinOp::Add | BinOp::Sub,
            lhs,
            rhs,
        } => {
            address_registers(exprs, *lhs, out);
            address_registers(exprs, *rhs, out);
        }
        _ => {}
    }
}

/// Follow `dst = src` copies back to the register that still holds the argument
/// (`rdx = r10; r10 = rcx` resolves to `rcx`). Reassigning a copy drops its alias, so the
/// chain stops at whichever register was overwritten last.
fn resolve_alias(aliases: &BTreeMap<Reg, Reg>, reg: Reg) -> Reg {
    let mut current = reg;
    for _ in 0..8 {
        match aliases.get(&current) {
            Some(next) if *next != current => current = *next,
            _ => break,
        }
    }
    current
}

fn recover_x86(session: &mut Session, start: Address, end: Address) -> Result<ArgNames> {
    let (lifted, exprs) = lift_function(session, start, end)?;
    let mut frame_established = false;
    let mut esp_delta: i64 = 0;
    let mut esp_unknown = false;
    let mut ebp_offsets: BTreeSet<i64> = BTreeSet::new();
    // adjusted argument offset -> observed `[esp+N]` offsets that produced it.
    let mut esp_args: BTreeMap<i64, Vec<i64>> = BTreeMap::new();

    for item in &lifted {
        for stmt in &item.stmts {
            if let Stmt::WriteReg { reg, value } = stmt {
                if reg.base == Reg::Rbp
                    && matches!(&exprs[*value as usize], Expr::Reg(view) if view.base == Reg::Rsp)
                {
                    frame_established = true;
                } else if reg.base == Reg::Rsp && !frame_established {
                    // Track the cumulative stack movement so `[esp+N]` reads after
                    // prologue pushes still map to incoming arguments.
                    match rsp_adjustment(&exprs, *value) {
                        Some(delta) => esp_delta += delta,
                        None => esp_unknown = true,
                    }
                }
            }
            walk_stmt_loads(stmt, &exprs, &mut |addr| {
                let Some((base, offset)) = stack_add(&exprs, addr) else {
                    return;
                };
                if frame_established && base == Reg::Rbp && offset >= 8 && (offset - 8) % 4 == 0 {
                    ebp_offsets.insert(offset);
                } else if !frame_established
                    && !esp_unknown
                    && base == Reg::Rsp
                    && offset >= 4
                    && offset % 4 == 0
                {
                    let adjusted = offset + esp_delta;
                    if adjusted >= 4 && adjusted % 4 == 0 {
                        esp_args.entry(adjusted).or_default().push(offset);
                    }
                }
            });
        }
    }

    let mut stack = BTreeMap::new();
    if frame_established {
        for (index, offset) in ebp_offsets.iter().enumerate() {
            stack.insert((Reg::Rbp, *offset), format!("arg{}", index + 1));
        }
    } else if !esp_unknown {
        for (index, (_, observed)) in esp_args.iter().enumerate() {
            for offset in observed {
                stack.insert((Reg::Rsp, *offset), format!("arg{}", index + 1));
            }
        }
    }
    Ok(ArgNames {
        names: BTreeMap::new(),
        stack,
        types: BTreeMap::new(),
        stack_types: BTreeMap::new(),
        stack_slot_type: ArgType::Int32,
        declared: None,
    })
}

/// Stack-pointer adjustment encoded in `rsp = rsp 卤 const` (push/pop/sub/add esp).
fn rsp_adjustment(exprs: &[Expr], value: ExprId) -> Option<i64> {
    let Expr::Bin { op, lhs, rhs } = &exprs[value as usize] else {
        return None;
    };
    let lhs_is_rsp = matches!(&exprs[*lhs as usize], Expr::Reg(view) if view.base == Reg::Rsp);
    if !lhs_is_rsp {
        return None;
    }
    let amount = match &exprs[*rhs as usize] {
        Expr::Const(value) => *value as i64,
        _ => return None,
    };
    match op {
        BinOp::Sub => Some(-amount),
        BinOp::Add => Some(amount),
        _ => None,
    }
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

fn walk_stmt_loads(stmt: &Stmt, exprs: &[Expr], visit: &mut impl FnMut(ExprId)) {
    match stmt {
        Stmt::WriteReg { value, .. } => walk_expr_loads(exprs, *value, visit),
        Stmt::Store { addr, value, .. } | Stmt::StoreFloat { addr, value, .. } => {
            walk_expr_loads(exprs, *addr, visit);
            walk_expr_loads(exprs, *value, visit);
        }
        Stmt::Call { .. } | Stmt::CallIndirect { .. } | Stmt::Intrinsic { .. } => {}
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
    if !pointer && lower.contains("float") {
        return Some(ReturnKind::F32);
    }
    if !pointer && lower.contains("double") {
        return Some(ReturnKind::F64);
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
    } else if base == "float" {
        ArgType::F32
    } else if base == "double" {
        ArgType::F64
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

    #[test]
    fn unmodelled_instructions_report_the_argument_registers_they_write() {
        assert!(clobbered_arg_registers("cpuid").contains(&Reg::Rcx));
        assert!(clobbered_arg_registers("cpuid").contains(&Reg::Rdx));
        assert_eq!(clobbered_arg_registers("btr edx, 0x1f"), vec![Reg::Rdx]);
        assert_eq!(clobbered_arg_registers("sbb edx, edx"), vec![Reg::Rdx]);
        // A memory operand only reads its base register.
        assert_eq!(
            clobbered_arg_registers("lock cmpxchg [rcx], rdx"),
            vec![Reg::Rdx]
        );
        assert_eq!(
            clobbered_arg_registers("rep movsb"),
            vec![Reg::Rsi, Reg::Rdi, Reg::Rcx]
        );
        assert!(clobbered_arg_registers("pshufd xmm1, xmm1, 0x0").is_empty());
        assert!(clobbered_arg_registers("nop").is_empty());
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
    fn signature_fills_gaps_between_argument_registers() {
        let mut args = ArgNames::default();
        args.names.insert(Reg::Rcx, "arg1".to_owned());
        args.names.insert(Reg::R8, "arg3".to_owned());
        args.names.insert(Reg::R9, "arg4".to_owned());
        assert_eq!(
            args.signature(),
            "int64 arg1, int64 arg2, int64 arg3, int64 arg4"
        );
    }

    #[test]
    fn float_arguments_print_with_their_type() {
        let mut args = ArgNames::default();
        args.names.insert(Reg::Xmm0, "arg1".to_owned());
        args.names.insert(Reg::Xmm1, "arg2".to_owned());
        args.types.insert(Reg::Xmm0, ArgType::F32);
        args.types.insert(Reg::Xmm1, ArgType::F64);
        assert_eq!(args.signature(), "f32 arg1, f64 arg2");
        assert_eq!(args.comment(), "arg1=xmm0, arg2=xmm1");
        // Float parameters do not count as integer ABI slots.
        assert_eq!(args.count(), 0);

        // With integer arguments the float order is unknown, so xmm stays unnamed.
        let mut mixed = ArgNames::default();
        mixed.names.insert(Reg::Rcx, "arg1".to_owned());
        mixed.names.insert(Reg::Xmm0, "arg2".to_owned());
        assert_eq!(mixed.signature(), "int64 arg1");
    }

    #[test]
    fn float_prototype_types_and_order_are_kept() {
        let parsed = parse_prototype("float scale(float factor, int count)").expect("prototype");
        assert_eq!(parsed.ret, ReturnKind::F32);
        let args = ArgNames::from_prototype(&parsed.params);
        assert_eq!(args.signature(), "f32 factor, int32 count");
        assert_eq!(args.count(), 1);
        assert_eq!(
            parse_prototype("double half(double value)").unwrap().ret,
            ReturnKind::F64
        );
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
        // A 32-bit slot is four bytes wide, so the unknown type is `int32`, not `int64`.
        assert!(
            args.signature().contains("int32 arg1"),
            "{}",
            args.signature()
        );
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
    fn x86_prologue_pushes_do_not_hide_arguments() {
        let Some((mut session, _dir)) = corpus_session("x86") else {
            return;
        };
        // noinline_helper pushes before touching its arguments; the cumulative esp
        // adjustment still maps the observed [esp+N] reads back to incoming arguments.
        let (start, end) = bounds(&session, "noinline_helper");
        let args = recover_args(&mut session, start, end).unwrap();
        let stack = args.stack_ordered();
        assert_eq!(stack.len(), 2, "{stack:?}");
        assert_eq!(stack[0].1, "arg1");
        assert_eq!(stack[1].1, "arg2");
        assert!(args.signature().contains("arg1"), "{}", args.signature());
    }
}
