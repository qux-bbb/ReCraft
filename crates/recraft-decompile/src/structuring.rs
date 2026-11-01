// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! L1 structure recovery: basic blocks 闁?`if`/`else` and `while` skeletons.
//!
//! Conservative by design. Dominators and post-dominators drive if/else diamonds and
//! natural loops; anything the structurer cannot prove (switch tables, irreducible
//! flow, indirect jumps) falls back to labelled `goto` output. Unvisited blocks are
//! appended as labelled regions so no code is silently dropped.
//!
//! Known approximation: statements executed by a loop header before its condition are
//! emitted once before the `while` (compilers normally put only a compare there).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use recraft_analysis::{Cfg, EdgeKind, build_cfg};
use recraft_common::{Address, Error, Result};
use recraft_core::Session;
use recraft_decode::iced_x86;
use recraft_ir::{Flow, Reg, RegView};

use crate::l2::{ReturnKind, X64_ARG_REGS, parse_prototype};
use crate::{ArgNames, BlockLift, L2Info, PseudoLine, Pseudocode, lift_block};

/// Render structured pseudo-C (L1) for a function produced by automatic analysis.
pub fn pseudocode_l1(session: &mut Session, start: Address) -> Result<Pseudocode> {
    structure(session, start, None)
}

/// Render structured pseudo-C with L2 signature, argument naming and stack slots.
pub fn pseudocode_l2(session: &mut Session, start: Address) -> Result<Pseudocode> {
    let functions = session.functions()?;
    let Some(function) = functions.iter().find(|function| function.start == start) else {
        return pseudocode_l1(session, start);
    };
    let info = crate::l2::recover_l2(session, function.start, function.end)?;
    structure(session, start, Some(info))
}

fn structure(session: &mut Session, start: Address, info: Option<L2Info>) -> Result<Pseudocode> {
    let mut info = info;
    let functions = session.functions()?;
    let Some(function) = functions.iter().find(|function| function.start == start) else {
        let mut fallback = crate::pseudocode_l0(session, start, 512)?;
        fallback.lines.insert(
            0,
            PseudoLine {
                addr: start,
                text: "// L1 unavailable: run analysis first".to_owned(),
            },
        );
        return Ok(fallback);
    };
    let cfg = build_cfg(session, function.start, function.end)?;
    if cfg.blocks.is_empty() {
        return Err(Error::internal(format!(
            "no basic blocks at {:#x}",
            start.get()
        )));
    }

    // A user prototype overrides the inferred signature: return type, calling
    // convention and (on x64) the register arguments with their declared names/types.
    let prototype_text = session.prototype_at(start)?;
    let parsed_proto = prototype_text.as_deref().and_then(parse_prototype);
    if let Some(parsed) = parsed_proto.as_ref() {
        if let Some(info) = info.as_mut() {
            info.ret = parsed.ret;
            if let Some(cc) = parsed.cc {
                info.abi = cc;
            }
            if session.image().is_64 {
                info.args = ArgNames::from_prototype(&parsed.params);
            } else {
                info.args.apply_prototype_x86(&parsed.params);
            }
        }
    }

    let args = info.as_ref().map(|info| &info.args);
    // `apply_type`: struct layouts bound to argument registers for this function.
    let mut arg_structs: BTreeMap<Reg, recraft_core::StructDef> = BTreeMap::new();
    if let Some(args) = args {
        for reg in X64_ARG_REGS {
            if let Some(slot) = args.slot_of(reg)
                && let Some(def) = session.struct_for_arg(start, slot)?
            {
                arg_structs.insert(reg, def);
            }
        }
    }
    let mut bodies = Vec::with_capacity(cfg.blocks.len());
    let no_counts: HashMap<u64, usize> = HashMap::new();
    // A phase timer for performance work: `RECRAFT_PROFILE=1 recraft decompile …` prints how
    // long each stage of the pipeline took.
    let profile = std::env::var_os("RECRAFT_PROFILE").is_some();
    let mut phase = std::time::Instant::now();
    let mut mark = |name: &str| {
        if profile {
            eprintln!(
                "[{:>8.1} ms] {name}",
                phase.elapsed().as_secs_f64() * 1000.0
            );
        }
        phase = std::time::Instant::now();
    };
    // The IAT slot map is the same for every block; build it once per function.
    let imports = crate::import_names(session);
    for block in &cfg.blocks {
        bodies.push(lift_block(
            session,
            block.start,
            block.end,
            args,
            &arg_structs,
            None,
            &no_counts,
            None,
            None,
            &imports,
            None,
            None,
            None,
            false,
        )?);
    }
    mark("first lift");
    // The prologue's frame pointer is the base for `[rbp + disp]` in every block. Blocks
    // that do not reload rbp otherwise name the same slot differently (or leave it raw).
    // The first pass has no stack displacement yet, so probe the entry block with the
    // displacement it always starts at.
    let frame_pointer = match cfg.blocks.first() {
        Some(block) => {
            let probe = lift_block(
                session,
                block.start,
                block.end,
                args,
                &arg_structs,
                None,
                &no_counts,
                Some((0, 0)),
                None,
                &imports,
                None,
                None,
                None,
                false,
            )?;
            probe
                .stack_pointers
                .iter()
                .find(|(reg, _)| *reg == Reg::Rbp)
                .copied()
        }
        None => None,
    };
    mark("frame probe");
    // Argument copies survive a block split only where every path agrees, so the alias map
    // is a must-analysis over the CFG: a block starts with the intersection of its
    // predecessors' outgoing aliases and applies its own copies on top.
    let alias_effects: Vec<Vec<(Reg, Option<Reg>)>> = bodies
        .iter()
        .map(|body| body.alias_writes.clone())
        .collect();
    let entry_aliases = entry_alias_map(&cfg, &alias_effects, &|reg| {
        args.is_some_and(|args| args.name_for(reg).is_some())
    });
    // Argument-register values also survive a block split when every path agrees and no call
    // in between clobbered them, so a call in a later block still prints its arguments.
    let entry_arg_values = entry_arg_value_map(&cfg, &bodies);
    // Infer argument counts for direct call targets from the callee's prologue. This
    // lets unknown calls render `sub(a, b)` with the right arity instead of dropping
    // arguments or printing all four ABI registers.
    let mut call_arg_counts: HashMap<u64, usize> = HashMap::new();
    let ranges: HashMap<u64, (Address, Address)> = functions
        .iter()
        .map(|function| (function.start.get(), (function.start, function.end)))
        .collect();
    // Shared across every call target of this function, so a chain of wrappers is scanned
    // once instead of once per caller.
    let mut arity_memo: BTreeMap<u64, usize> = BTreeMap::new();
    for target in bodies.iter().flat_map(|body| body.calls.iter().copied()) {
        if target == start.get() {
            if let Some(info) = info.as_ref()
                && info.args.count() > 0
            {
                call_arg_counts.insert(target, info.args.count());
            }
            continue;
        }
        if let Some((target_start, target_end)) = ranges.get(&target) {
            let mut target_start = *target_start;
            let mut target_end = *target_end;
            // Import thunks begin with a single `jmp`; their argument registers are
            // consumed by the real body, so infer from the destination instead.
            for _ in 0..4 {
                let Some(insn) = session.insn_at(target_start).ok().flatten() else {
                    break;
                };
                if insn.mnemonic() != iced_x86::Mnemonic::Jmp {
                    break;
                }
                let Some(next) = insn.branch_target() else {
                    break;
                };
                let Some((next_start, next_end)) = ranges.get(&next.get()).copied() else {
                    break;
                };
                if next_start == target_start {
                    break;
                }
                target_start = next_start;
                target_end = next_end;
            }
            if let Ok(count) = crate::l2::recover_arg_count(
                session,
                target_start,
                target_end,
                &mut arity_memo,
                &ranges,
            ) && count > 0
            {
                call_arg_counts.insert(target, count);
            }
        }
    }
    // Names of callees whose arity is known to be non-empty; only those can receive recovered
    // cross-block arguments (see `fill_split_call_arguments`). A self-recursive call is not in
    // `call_arg_counts`, so the function's own signature (or a scan of its prologue in L1)
    // supplies it.
    let mut callers_with_arguments: BTreeSet<String> = BTreeSet::new();
    for target in call_arg_counts.keys() {
        callers_with_arguments.insert(crate::resolve_name(session, Address::new(*target))?);
    }
    let self_has_arguments = match info.as_ref() {
        Some(info) => info.args.count() > 0,
        None => crate::l2::recover_arg_count(
            session,
            function.start,
            function.end,
            &mut arity_memo,
            &ranges,
        )
        .is_ok_and(|count| count > 0),
    };
    if self_has_arguments {
        callers_with_arguments.insert(crate::resolve_name(session, start)?);
    }
    // SSA-lite: which registers are already overwritten on every path reaching a block.
    // Re-lifting with that seed keeps `argN` off clobbered values at join points.
    let seeds = if args.is_some() {
        let written: Vec<BTreeSet<Reg>> = bodies.iter().map(|body| body.written.clone()).collect();
        entry_written(&cfg, &written)
    } else {
        vec![BTreeSet::new(); cfg.blocks.len()]
    };
    // Stack-slot naming needs each block's entry displacement. The first lifting pass
    // recorded the block-local net/min, so propagate those over the CFG and name every slot
    // relative to the bottom of the frame: the prologue, body and epilogue then agree on
    // what `var_XX` means.
    let (entry_stack, frame_base) = entry_stack_deltas(&cfg, &bodies);
    let flag_sources = block_flag_sources(session, &cfg)?;
    let mut seeded = Vec::with_capacity(cfg.blocks.len());
    mark("alias/arg analysis");
    for (index, block) in cfg.blocks.iter().enumerate() {
        seeded.push(lift_block(
            session,
            block.start,
            block.end,
            args,
            &arg_structs,
            Some(&seeds[index]),
            &call_arg_counts,
            entry_stack[index].map(|entry| (entry, frame_base)),
            flag_sources[index],
            &imports,
            frame_pointer,
            Some(&entry_aliases[index]),
            Some(&entry_arg_values[index]),
            true,
        )?);
    }
    bodies = seeded;
    mark("seeded lift");
    let mut literals: Vec<(String, Address)> = bodies
        .iter()
        .flat_map(|body| body.literals.iter().cloned())
        .collect();
    literals.sort();
    literals.dedup();
    let literal_texts = literals.clone();
    let mut structurer = Structurer::new(
        &cfg,
        bodies,
        info.as_ref().map(|info| info.args.clone()),
        function.end,
        call_arg_counts.clone(),
    );
    structurer.compute_rpo();
    structurer.compute_dominators();
    structurer.compute_post_dominators();
    structurer.compute_loops();

    let mut name = session
        .name_at(start)?
        .unwrap_or_else(|| format!("sub_{:x}", start.get()));
    if let Some(parsed_name) = parsed_proto.as_ref().and_then(|parsed| parsed.name.clone()) {
        name = parsed_name;
    }
    let mut lines = Vec::with_capacity(64);
    lines.push(PseudoLine {
        addr: start,
        text: format!(
            "// {} structure recovery ({} blocks)",
            if info.is_some() { "L2" } else { "L1" },
            cfg.blocks.len()
        ),
    });
    match &info {
        Some(info) => {
            if !info.args.is_empty() {
                lines.push(PseudoLine {
                    addr: start,
                    text: format!("// L2 params: {}", info.args.comment()),
                });
            }
            lines.push(PseudoLine {
                addr: start,
                text: format!(
                    "{} {}{}({}) {{",
                    info.ret.c_name(),
                    info.abi.keyword(),
                    name,
                    info.args.signature()
                ),
            });
        }
        None => {
            lines.push(PseudoLine {
                addr: start,
                text: format!("void {name}_l1(void) {{"),
            });
        }
    }
    mark("pre-emit");
    structurer.emit_all(&mut lines);
    mark("emit");
    hide_stack_probes(session, &mut lines)?;
    // `/GS` canaries and block splits are pure compiler artifacts; removing them before the
    // structural fold passes exposes the real return paths and call arguments. The cookie
    // setup has to be identified before `hide_callee_saves` treats its store as a spill.
    hide_security_cookies(&mut lines);
    hide_callee_saves(&mut lines);
    // x86 passes arguments on the stack, so the register-based recovery is x64-only.
    if session.image().is_64 {
        fill_split_call_arguments(&mut lines, &callers_with_arguments);
    }
    tidy_labels(&mut lines);
    for _ in 0..4 {
        drop_unreachable_tails(&mut lines);
    }
    fold_jumps_over_braces(&mut lines);
    fold_sibling_tail_gotos(&mut lines);
    fold_return_labels(&mut lines);
    fold_return_tail_labels(&mut lines);
    fold_last_label_tails(&mut lines);
    merge_identical_if_arms(&mut lines);
    fold_compound_assignments(&mut lines);
    fold_load_modify_store(&mut lines);
    simplify_negated_conditions(&mut lines);
    fold_single_use_temporaries(&mut lines);
    fold_store_copies(&mut lines);
    fold_carry_masks(&mut lines);
    drop_dead_register_writes(&mut lines);
    drop_unused_register_writes(&mut lines);
    fold_store_value_temporaries(&mut lines);
    strip_condition_parentheses(&mut lines);
    hoist_invariant_loop_setups(&mut lines);
    fold_constant_loops(&mut lines);
    merge_else_if(&mut lines);
    flatten_terminating_else(&mut lines);
    // The rewritten loop and `else if` conditions may now carry a negated comparison or the
    // generator's extra parentheses.
    simplify_negated_conditions(&mut lines);
    strip_condition_parentheses(&mut lines);
    collapse_nested_if_wrappers(&mut lines);
    strip_inline_condition_parentheses(&mut lines);
    tidy_labels(&mut lines);
    drop_unreachable_tails(&mut lines);
    drop_overwritten_assignments(&mut lines);
    drop_dead_slot_stores(&mut lines);
    // Epilogue restores no longer pair with anything once spills are gone, so identifying
    // them as bookkeeping also makes the argument-copy propagation safe.
    hide_epilogue_restores(&mut lines);
    // Duplicate spills to one slot were just collapsed, so more save/restore pairs are
    // recognizable now.
    hide_callee_saves(&mut lines);
    propagate_argument_spills(&mut lines);
    propagate_argument_copies(&mut lines);
    materialize_missing_labels(&mut lines);
    // Runs last: the return value is folded only once every rewrite that can move or drop
    // the producing statement has settled.
    if let Some(info) = info.as_ref() {
        fold_return_values(&mut lines, info.ret);
        // The folded `return` reads the value's temporary, so give that temporary the same
        // single-use inlining chance as a condition.
        fold_single_use_temporaries(&mut lines);
        fold_constant_byte_returns(&mut lines);
        fold_assigned_returns(&mut lines);
    }
    // Shared exit paths are now `return <value>;`; a jump to one is an early return.
    fold_trivial_return_labels(&mut lines);
    // Recover loops and guards from the labelled fallback text. Each fold can expose
    // another one (a loop body with its own skips), so repeat until settled.
    // A temporary read by the very next statement is a local variable in the source; folding
    // it must run before the loop folds so the statement text is final there too.
    for _ in 0..4 {
        let before = lines.len();
        inline_adjacent_temporaries(&mut lines);
        if lines.len() == before {
            break;
        }
    }
    for _ in 0..3 {
        let mut changed = fold_backward_gotos_into_loops(&mut lines);
        changed += fold_short_forward_gotos(&mut lines);
        if changed == 0 {
            break;
        }
    }
    fill_cross_block_arguments(&mut lines);
    // A function whose every exit is a bare `return;` returns nothing, so the signature says
    // `void` instead of keeping the register-width type the ABI would suggest. A body that
    // ends with a call is left alone: `f(); ret` may be `return f();` in the source.
    if let Some(signature) = lines.iter().position(|line| line.text.ends_with(") {")) {
        let returns_value = lines.iter().skip(signature + 1).any(|line| {
            let text = line.text.trim();
            text.starts_with("return ") && text != "return;"
        });
        let ends_with_call = lines
            .iter()
            .rev()
            .map(|line| line.text.trim())
            .find(|text| !text.is_empty() && *text != "}")
            .is_some_and(|text| {
                text.ends_with(");") && !text.starts_with("return") && !text.contains('=')
            });
        if !returns_value
            && !ends_with_call
            && let Some((ret, rest)) = lines[signature].text.split_once(' ')
            && matches!(
                ret,
                "int8" | "int16" | "int32" | "int64" | "uint8" | "uint16" | "uint32" | "uint64"
            )
        {
            lines[signature].text = format!("void {rest}");
        }
    }

    fold_constant_registers(&mut lines);
    // The constant fold just turned `rcx += rdx` into `rcx += 0x…`, so an accumulation can now
    // merge back into the assignment that produced its input.
    merge_accumulations(&mut lines);
    drop_noop_statements(&mut lines);
    mark("text passes");
    normalize_infinite_loops(&mut lines);
    name_known_struct_fields(&mut lines);
    name_structure_instances(&mut lines);
    name_void_pointer_fields(&mut lines);

    name_global_data(&mut lines);
    {
        let mut functions: HashMap<u64, String> = HashMap::new();
        for name in constants(&lines) {
            if ranges.contains_key(&name) {
                functions.insert(name, crate::resolve_name(session, Address::new(name))?);
            }
        }
        let data: Vec<(u64, u64)> = session
            .segments()
            .iter()
            .filter(|segment| {
                matches!(
                    segment.kind,
                    recraft_core::SegmentKind::Data | recraft_core::SegmentKind::Bss
                )
            })
            .map(|segment| (segment.range.start.get(), segment.range.end.get()))
            .collect();
        let literals: HashMap<u64, String> = literal_texts
            .iter()
            .map(|(text, address)| (address.get(), text.clone()))
            .collect();
        name_address_constants(&mut lines, &functions, &literals, &data);
    }
    if info.is_some()
        && let Some(frame) = entry_frame_size(session, &cfg)
    {
        // Show the size of the stack frame in the header: it tells a reader how much local
        // state the function keeps without walking the prologue.
        if let Some(line) = lines
            .iter_mut()
            .find(|line| line.text.starts_with("// L2 structure recovery ("))
        {
            line.text = format!("{}, frame {frame:#x}", line.text.trim_end_matches(')')) + ")";
        }
        name_stack_arguments(&mut lines, frame);
    }
    name_handle_slots(&mut lines);
    tidy_labels(&mut lines);
    hide_frame_pointer_setups(&mut lines);
    lines.push(PseudoLine {
        addr: start,
        text: "}".to_owned(),
    });
    // A function that still has jumps says so in the header: a reader can tell at a glance that
    // this body could not be reduced to structured control flow.
    let fallbacks = lines
        .iter()
        .filter(|line| line.text.contains("goto loc_"))
        .count();
    if fallbacks > 0
        && let Some(header) = lines
            .iter_mut()
            .find(|line| line.text.starts_with("// L2 structure recovery ("))
    {
        header.text = format!("{}, {fallbacks} goto)", header.text.trim_end_matches(')'));
    }
    // A user-set prototype is authoritative for naming/definitions; show it verbatim
    // above the generated signature (which already reflects its parsed types/names).
    if let Some(prototype) = session.prototype_at(start)? {
        lines.insert(
            0,
            PseudoLine {
                addr: start,
                text: format!("// prototype: {prototype}"),
            },
        );
    }
    Ok(Pseudocode {
        func_start: start,
        lines,
        literals,
    })
}

/// Rendered argument-register values that hold on entry to each block.
///
/// Must-analysis over the four x64 argument slots: a value is inherited only when every
/// predecessor leaves the same text in that register. Calls clear the slots, so a call site
/// never inherits a value that an intervening call would have destroyed.
/// `rbp = &var_100;` only establishes the frame base used for slot naming; the variable is
/// an artifact of the machine frame, not of the program. When the register is never named
/// again the line tells the reader nothing, so it is dropped.
fn hide_frame_pointer_setups(lines: &mut Vec<PseudoLine>) {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for line in lines.iter() {
        for token in line
            .text
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .filter(|token| !token.is_empty())
        {
            *counts.entry(token.to_owned()).or_insert(0) += 1;
        }
    }
    lines.retain(|line| {
        let text = line.text.trim();
        let Some((lhs, rhs)) = text
            .strip_suffix(';')
            .and_then(|statement| statement.split_once(" = "))
        else {
            return true;
        };
        let frame_register = matches!(
            lhs,
            "rbx" | "rbp" | "rsi" | "rdi" | "r12" | "r13" | "r14" | "r15"
        );
        let slot = rhs.starts_with("&var_");
        !(frame_register && slot && counts.get(lhs).copied().unwrap_or(0) == 1)
    });
}

/// Consume up to `limit` argument setups immediately above a tail call, in ABI order
/// (`rcx`, `rdx`, `r8`, `r9`). A setup is only taken when it is the last remaining line and
/// assigns the next register, so the values are still live in those registers at the call.
fn take_adjacent_arguments(lines: &mut Vec<PseudoLine>, limit: usize) -> Vec<String> {
    const ABI: [&str; 4] = ["rcx", "rdx", "r8", "r9"];
    let mut arguments = Vec::new();
    for register in ABI.iter().take(limit.min(ABI.len())) {
        let Some(last) = lines.last() else {
            break;
        };
        let Some((lhs, rhs)) = plain_assignment(&last.text) else {
            break;
        };
        if lhs != *register || rhs.is_empty() {
            break;
        }
        arguments.push(rhs.to_owned());
        lines.pop();
    }
    arguments
}
/// A Win32 structure that an API fills through a pointer argument: the APIs that reveal it
/// (`&var_XX` in their argument list) and the fields as (byte offset, name).
struct KnownStruct {
    apis: &'static [&'static str],
    fields: &'static [(&'static str, &'static str)],
}

/// `RECT` is filled by the rectangle getters, `POINT` by the cursor/screen mappers and `MSG`\n/// by the message-loop calls. Field names
/// are applied to later member accesses only, so the address passed to the API and the wide
/// clear before it keep the plain slot name.
const KNOWN_STRUCTS: [KnownStruct; 3] = [
    KnownStruct {
        apis: &[
            "AdjustWindowRectEx",
            "CopyRect",
            "GetClientRect",
            "GetUpdateRect",
            "GetWindowRect",
            "InflateRect",
            "InvalidateRect",
            "OffsetRect",
            "PtInRect",
            "SetRect",
        ],
        fields: &[("0", "left"), ("4", "top"), ("8", "right"), ("c", "bottom")],
    },
    KnownStruct {
        apis: &["GetCursorPos", "ClientToScreen", "ScreenToClient"],
        fields: &[("0", "x"), ("4", "y")],
    },
    KnownStruct {
        apis: &[
            "GetMessageW",
            "PeekMessageW",
            "DispatchMessageW",
            "TranslateMessage",
        ],
        fields: &[
            ("0", "hwnd"),
            ("8", "message"),
            ("10", "wParam"),
            ("18", "lParam"),
            ("20", "time"),
            ("28", "pt.x"),
            ("2c", "pt.y"),
        ],
    },
];

/// `GetClientRect(hwnd, &var_30)` makes the four 32-bit slots starting at `var_30` a `RECT`;
/// naming the later slots after their fields reads like the structure the API fills.
/// A direct call whose arguments were set up in another block records none, so it prints
/// `sub_X()`. The widest sibling call to the same target shows how many arguments it takes;
/// printing that many register placeholders keeps the arity of the site visible.
/// A constant moved into a register for a single comparison reads better inline:
/// `eax = 0x111; … if (arg2 > eax) {` becomes `if (arg2 > 0x111) {`. The assignment is only
/// dropped when the register is written again before any other read, so the value cannot be
/// live afterwards.
fn fold_constant_registers(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index < lines.len() {
        let text = lines[index].text.trim().to_owned();
        let Some(statement) = text.strip_suffix(';') else {
            index += 1;
            continue;
        };
        let Some((register, value)) = statement.split_once(" = ") else {
            index += 1;
            continue;
        };
        let register = register.trim().to_owned();
        let value = value.trim().to_owned();
        if !is_identifier(&register) || register.starts_with("var_") || register.starts_with("arg")
        {
            index += 1;
            continue;
        }
        if is_frame_register(&register) || !is_constant(&value) {
            index += 1;
            continue;
        }
        let mut names = vec![register.clone()];
        names.extend(narrow_register(&register).into_iter().map(str::to_owned));
        let Some(use_index) = first_mention_any(lines, index + 1, 9, &names) else {
            index += 1;
            continue;
        };
        let candidate = lines[use_index].text.clone();
        let trimmed = candidate.trim();
        let condition = trimmed
            .strip_prefix("if (")
            .and_then(|rest| rest.strip_suffix(") {"))
            .map(str::to_owned);
        // `rcx += rdx;` with `rdx = 0x140000000` is one arithmetic step: fold the constant into
        // the compound assignment. Only the right-hand side is a fold site — `rdx += rcx` writes
        // the register instead of reading it.
        let compound = if condition.is_none() {
            compound_operand(trimmed, &register)
        } else {
            None
        };
        let Some((folded, folded_value)) =
            Some(match (condition.as_deref(), compound.as_deref()) {
                (Some(condition), _) => {
                    // A 16- or 8-bit test reads the low half of the register: `eax = 0x5a4d; … != ax`
                    // folds the same way once the constant is truncated to the width compared.
                    if contains_token(condition, &register) {
                        (register.clone(), value.clone())
                    } else {
                        let Some(narrow) = narrow_register(&register).into_iter().find(|narrow| {
                            contains_token(condition, narrow)
                                && is_truncated_constant(&value, narrow)
                        }) else {
                            index += 1;
                            continue;
                        };
                        (narrow.to_owned(), truncate_constant(&value, narrow))
                    }
                }
                (None, Some(operand)) => {
                    if operand == register {
                        (register.clone(), value.clone())
                    } else {
                        let Some(narrow) = narrow_register(&register).into_iter().find(|narrow| {
                            operand == *narrow && is_truncated_constant(&value, narrow)
                        }) else {
                            index += 1;
                            continue;
                        };
                        (narrow.to_owned(), truncate_constant(&value, narrow))
                    }
                }
                (None, None) => {
                    index += 1;
                    continue;
                }
            })
        else {
            index += 1;
            continue;
        };
        if let Some(condition) = condition.as_deref() {
            // A bare truth test has no comparison to fold into.
            if condition.trim() == folded {
                index += 1;
                continue;
            }
            // Comparing two known constants would print a constant condition; the register form
            // at least keeps the code readable.
            if !has_identifier(&replace_token(condition, &folded, "")) {
                index += 1;
                continue;
            }
            let indent = " ".repeat(leading_spaces(&candidate));
            lines[use_index].text = format!(
                "{indent}if ({}) {{",
                replace_token(condition, &folded, &folded_value)
            );
        } else {
            lines[use_index].text = replace_token(&candidate, &folded, &folded_value);
        }
        // The register's next mention decides whether the assignment is dead.
        let dead = match first_mention(lines, use_index + 1, lines.len() - use_index - 1, &register)
        {
            Some(next) => assigns_to(lines[next].text.trim().trim_end_matches(';'), &register),
            None => true,
        };
        if dead {
            lines.remove(index);
            continue;
        }
        index += 1;
    }
}

// The register named on the right of a compound assignment (`x += rdx;` → `rdx`).
fn compound_operand(statement: &str, register: &str) -> Option<String> {
    let body = statement.trim_end_matches(';');
    for operator in [
        " += ", " -= ", " &= ", " |= ", " ^= ", " *= ", " <<= ", " >>= ", " /= ", " %= ",
    ] {
        if let Some((_, right)) = body.split_once(operator) {
            let right = right.trim();
            if right == register {
                return Some(right.to_owned());
            }
        }
    }
    None
}
/// The 16- and 8-bit names of a 32-bit register (`eax` → `ax`, `al`).
fn narrow_register(register: &str) -> Vec<&'static str> {
    let pair = match register {
        "eax" => ("ax", "al"),
        "ebx" => ("bx", "bl"),
        "ecx" => ("cx", "cl"),
        "edx" => ("dx", "dl"),
        "esi" => ("si", "si"),
        "edi" => ("di", "di"),
        "esp" => ("sp", "sp"),
        "ebp" => ("bp", "bp"),
        "r8d" => ("r8w", "r8b"),
        "r9d" => ("r9w", "r9b"),
        "r10d" => ("r10w", "r10b"),
        "r11d" => ("r11w", "r11b"),
        "r12d" => ("r12w", "r12b"),
        "r13d" => ("r13w", "r13b"),
        "r14d" => ("r14w", "r14b"),
        "r15d" => ("r15w", "r15b"),
        _ => return Vec::new(),
    };
    vec![pair.0, pair.1]
}

/// Whether a constant keeps the same value in the low 16 or 8 bits of the register, which is
/// the only case where folding it into a narrower comparison is exact.
fn is_truncated_constant(value: &str, narrow: &str) -> bool {
    let Some(number) = parse_constant(value) else {
        return false;
    };
    let mask: u64 = if narrow.ends_with('w') || narrow.ends_with('x') || narrow.ends_with('i') {
        0xffff
    } else {
        0xff
    };
    number & mask == number
}

/// The constant cut to the width of a narrower register, printed in the project's hex style.
fn truncate_constant(value: &str, narrow: &str) -> String {
    let Some(number) = parse_constant(value) else {
        return value.to_owned();
    };
    let mask: u64 = if narrow.ends_with('w') || narrow.ends_with('x') || narrow.ends_with('i') {
        0xffff
    } else {
        0xff
    };
    format!("0x{:x}", number & mask)
}

/// A literal as a number: `0x5a4d`, `18`, `0xffffffff80000002`.
fn parse_constant(value: &str) -> Option<u64> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix("0x") {
        return u64::from_str_radix(hex, 16).ok();
    }
    value.parse::<u64>().ok()
}
// The first line at or after `from` that mentions any of `names`, within `limit` lines.
fn first_mention_any(
    lines: &[PseudoLine],
    from: usize,
    limit: usize,
    names: &[String],
) -> Option<usize> {
    let end = (from + limit).min(lines.len());
    for (index, line) in lines.iter().enumerate().take(end).skip(from) {
        if names
            .iter()
            .any(|name| contains_token(&line.text, name.as_str()))
        {
            return Some(index);
        }
    }
    None
}
/// The first line at or after `from` that mentions `name` as a token, within `limit` lines.
fn first_mention(lines: &[PseudoLine], from: usize, limit: usize, name: &str) -> Option<usize> {
    let end = (from + limit).min(lines.len());
    for (index, line) in lines.iter().enumerate().take(end).skip(from) {
        if contains_token(&line.text, name) {
            return Some(index);
        }
    }
    None
}

/// Whether `text` holds an identifier (a number such as `0x12a` does not).
fn has_identifier(text: &str) -> bool {
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_ascii_digit() {
            while let Some(&next) = chars.peek() {
                if next.is_ascii_alphanumeric() {
                    chars.next();
                } else {
                    break;
                }
            }
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            return true;
        }
    }
    false
}

/// A literal number: `0x111`, `7`, `-1`.
fn is_constant(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if let Some(hex) = digits.strip_prefix("0x") {
        !hex.is_empty() && hex.chars().all(|ch| ch.is_ascii_hexdigit())
    } else {
        !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit())
    }
}

/// Statements that do nothing and only add noise: a self assignment (`rax = rax;`), an `if`
/// whose body is empty, and a label that is not referenced and only introduces the closing
/// brace of its block.
fn drop_noop_statements(lines: &mut Vec<PseudoLine>) {
    lines.retain(|line| {
        let text = line.text.trim();
        let Some(statement) = text.strip_suffix(';') else {
            return true;
        };
        let Some((lhs, rhs)) = statement.split_once(" = ") else {
            return true;
        };
        !(lhs.trim() == rhs.trim() && is_identifier(lhs.trim()))
    });
    let mut index = 0;
    while index + 1 < lines.len() {
        let text = lines[index].text.trim();
        if text.starts_with("if (") && text.ends_with(") {") && lines[index + 1].text.trim() == "}"
        {
            lines.remove(index + 1);
            lines.remove(index);
            index = index.saturating_sub(1);
            continue;
        }
        index += 1;
    }
    let mut references: HashMap<String, usize> = HashMap::new();
    for line in lines.iter() {
        for token in line
            .text
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .filter(|token| token.starts_with("loc_"))
        {
            *references.entry(token.to_owned()).or_insert(0) += 1;
        }
    }
    let mut index = 0;
    while index + 1 < lines.len() {
        let text = lines[index].text.trim();
        if let Some(label) = text.strip_suffix(':')
            && label.starts_with("loc_")
            && references.get(label).copied().unwrap_or(0) == 1
            && (lines[index + 1].text.trim() == "}" || lines[index + 1].text.trim() == "return;")
        {
            lines.remove(index);
            continue;
        }
        index += 1;
    }
}

fn fill_cross_block_arguments(lines: &mut [PseudoLine]) {
    let mut arities: HashMap<String, usize> = HashMap::new();
    for line in lines.iter() {
        let Some((target, arguments)) = direct_call(&line.text) else {
            continue;
        };
        let entry = arities.entry(target.to_owned()).or_insert(0);
        *entry = (*entry).max(count_arguments(arguments));
    }
    for line in lines.iter_mut() {
        let Some((target, arguments)) = direct_call(&line.text) else {
            continue;
        };
        if !arguments.trim().is_empty() {
            continue;
        }
        let Some(&arity) = arities.get(target) else {
            continue;
        };
        // A signature-table entry is authoritative about the target's arity.
        if crate::import_arity(target).is_some() {
            continue;
        }
        // Arguments past the fourth travel on the stack, whose position the register names
        // alone cannot place; leave those calls alone.
        if arity == 0 || arity > 4 {
            continue;
        }
        let placeholders: Vec<String> = crate::l2::X64_ARG_REGS
            .iter()
            .take(arity)
            .map(|reg| RegView::full(*reg).name().to_owned())
            .collect();
        line.text = line.text.replacen(
            &format!("{target}()"),
            &format!("{target}({})", placeholders.join(", ")),
            1,
        );
    }
}

/// The target and raw argument list of a `sub_X(…)` or `ApiName(…)` call line.
fn direct_call(text: &str) -> Option<(&str, &str)> {
    let trimmed = text.trim();
    let trimmed = trimmed.strip_prefix("return ").unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix(';')?;
    if !trimmed.ends_with(')') {
        return None;
    }
    let open = trimmed.find('(')?;
    let target = &trimmed[..open];
    let known = target.starts_with("sub_")
        || target
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase());
    if !known || target.contains(' ') {
        return None;
    }
    Some((target, &trimmed[open + 1..trimmed.len() - 1]))
}

/// Number of top-level arguments in `arguments`.
fn count_arguments(arguments: &str) -> usize {
    if arguments.trim().is_empty() {
        return 0;
    }
    let mut depth = 0usize;
    let mut count = 1;
    for ch in arguments.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    count
}

/// Address constants that name something read better than a bare number: a constant equal to
/// a function start is that function (a callback pointer), and one inside a data segment is
/// `off_<hex>` (the address of data). Numbers that match neither stay as they are.
fn name_address_constants(
    lines: &mut [PseudoLine],
    functions: &HashMap<u64, String>,
    literals: &HashMap<u64, String>,
    data: &[(u64, u64)],
) {
    let mut replacements: Vec<(String, String)> = Vec::new();
    for name in constants(lines) {
        if let Some(function) = functions.get(&name) {
            replacements.push((format!("0x{name:x}"), function.clone()));
        } else if let Some(literal) = literals.get(&name) {
            replacements.push((format!("0x{name:x}"), literal.clone()));
        } else if data
            .iter()
            .any(|(start, end)| name >= *start && name < *end)
        {
            replacements.push((format!("0x{name:x}"), format!("off_{name:x}")));
        }
    }
    if replacements.is_empty() {
        return;
    }
    for line in lines.iter_mut() {
        for (from, to) in &replacements {
            line.text = replace_token(&line.text, from, to);
        }
    }
}

/// The image addresses mentioned as `0x<hex>` constants with at least eight digits.
fn constants(lines: &[PseudoLine]) -> BTreeSet<u64> {
    let mut found = BTreeSet::new();
    let mut quoted = false;
    for line in lines {
        let text = line.text.as_str();
        let bytes = text.as_bytes();
        let mut index = 0;
        while index + 2 < bytes.len() {
            if bytes[index] == b'"' {
                quoted = !quoted;
                index += 1;
                continue;
            }
            if quoted || bytes[index] != b'0' || bytes[index + 1] != b'x' {
                index += 1;
                continue;
            }
            let mut end = index + 2;
            while end < bytes.len() && (bytes[end] as char).is_ascii_hexdigit() {
                end += 1;
            }
            if end >= index + 10
                && let Ok(value) = u64::from_str_radix(&text[index + 2..end], 16)
            {
                found.insert(value);
            }
            index = end.max(index + 1);
        }
    }
    found
}

/// Global data addressed by an absolute image address reads as a named word, like a symbol:
/// `*(u64*)(0x140036648)` becomes `qword_140036648`, `*(u32*)(0x140034460)` `dword_140034460`,
/// `*(u8*)(…)` `byte_…`, `*(u16*)(…)` `word_…`, `*(__m128*)(…)` `xmmword_…`. The width stays
/// visible in the name.
fn name_global_data(lines: &mut [PseudoLine]) {
    const KINDS: [(&str, &str); 7] = [
        ("u8", "byte_"),
        ("u16", "word_"),
        ("u32", "dword_"),
        ("u64", "qword_"),
        ("f32", "flt_"),
        ("f64", "dbl_"),
        ("__m128", "xmmword_"),
    ];
    for line in lines.iter_mut() {
        for (width, prefix) in KINDS {
            let head = format!("*({width}*)(0x");
            let mut search = 0;
            while let Some(position) = line.text[search..].find(&head) {
                let start = search + position;
                let after = &line.text[start + head.len()..];
                let digits = after
                    .chars()
                    .take_while(|ch| ch.is_ascii_hexdigit())
                    .count();
                // An image address has eight or more hex digits and the cast closes right
                // after it; anything else is a computed address and keeps its form.
                if digits < 8 || !after[digits..].starts_with(')') {
                    search = start + head.len();
                    continue;
                }
                let address = &after[..digits];
                let replacement = format!("{prefix}{address}");
                line.text
                    .replace_range(start..start + head.len() + digits + 1, &replacement);
                search = start + replacement.len();
            }
        }
    }
}

/// Incoming stack arguments of an x64 function: a read of `[rsp + disp]` beyond the frame the
/// prologue allocates is the caller's argument, not a local. The slot is renamed to the
/// parameter it is (`var_78` at frame 0x48 reads the second stack slot, `arg6`) and added to
/// the signature. Slots the function writes are outgoing arguments of its own calls and stay
/// as slots.
fn name_stack_arguments(lines: &mut [PseudoLine], frame: u64) {
    // Frame-relative displacements of every slot the function writes.
    let mut written: BTreeSet<u64> = BTreeSet::new();
    for line in lines.iter() {
        let text = line.text.trim();
        let Some((lhs, _)) = text.split_once(" = ") else {
            continue;
        };
        if let Some(offset) = slot_offset(lhs.trim()) {
            written.insert(offset);
        } else if let Some(offset) = slot_offset(lhs.trim().trim_start_matches('&')) {
            written.insert(offset);
        }
    }
    let mut stack_arguments: BTreeMap<u64, usize> = BTreeMap::new();
    for line in lines.iter() {
        for token in line
            .text
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        {
            let Some(offset) = slot_offset(token) else {
                continue;
            };
            // The first stack argument sits at frame + shadow space + return address.
            if offset < frame.saturating_add(0x28) || written.contains(&offset) {
                continue;
            }
            let index = (offset - frame - 0x28) / 8 + 5;
            stack_arguments.insert(offset, index as usize);
        }
    }
    if stack_arguments.is_empty() {
        return;
    }
    // Types are decided from the body before the signature line is rewritten.
    let mut stack_types: BTreeMap<usize, String> = BTreeMap::new();
    for (offset, index) in &stack_arguments {
        let slot = format!("var_{offset:x}");
        let ty = pointer_width_of_slot(lines, &slot)
            .map(|widths| match widths.as_slice() {
                [1] => "char*",
                [2] => "wchar_t*",
                [4] => "int32*",
                [8] => "int64*",
                _ => "void*",
            })
            .unwrap_or("int64");
        stack_types.insert(*index, ty.to_owned());
    }
    for line in lines.iter_mut() {
        for (offset, index) in &stack_arguments {
            let slot = format!("var_{offset:x}");
            line.text = replace_token(&line.text, &slot, &format!("arg{index}"));
        }
    }
    // Mention the stack arguments in the parameter comment.
    if let (Some(first), Some(highest)) = (
        stack_arguments.values().copied().min(),
        stack_arguments.values().copied().max(),
    ) && let Some(line) = lines
        .iter_mut()
        .find(|line| line.text.trim_start().starts_with("// L2 params:"))
    {
        line.text = format!("{}, arg{first}+=stack (through arg{highest})", line.text);
    }

    // Widen the signature to the highest stack argument it now uses. Parameters are positional,
    // so every index between the last declared one and `highest` has to be declared too.
    let highest = stack_arguments.values().copied().max().unwrap_or(0);
    for line in lines.iter_mut() {
        let text = line.text.trim_end().to_owned();
        if !text.ends_with('{') || !text.contains("sub_") || !text.contains("arg1") {
            continue;
        }
        let Some(close) = text.rfind(')') else {
            continue;
        };
        let open = text.find('(').unwrap_or(0);
        let declared = text[open + 1..close]
            .split(',')
            .filter_map(|parameter| {
                let name = parameter.trim().rsplit(' ').next()?;
                name.strip_prefix("arg")?.parse::<usize>().ok()
            })
            .max()
            .unwrap_or(0);
        if declared >= highest {
            continue;
        }
        let mut parameters: Vec<String> = Vec::new();
        for index in declared + 1..=highest {
            // The slot behind that parameter may be a pointer: type it by the access the
            // pseudo-C makes through the value.
            let ty = stack_types
                .get(&index)
                .map(String::as_str)
                .unwrap_or("int64");
            parameters.push(format!("{ty} arg{index}"));
        }
        let head = &text[..close];
        let tail = &text[close..];
        line.text = format!("{head}, {}{tail}", parameters.join(", "));
        break;
    }
}

/// The width of the memory access made through a value copied out of a stack slot, if the
/// pseudo-C shows one: the slot holds a pointer in that case.
fn pointer_width_of_slot(lines: &[PseudoLine], slot: &str) -> Option<Vec<u8>> {
    let mut widths: BTreeSet<u8> = BTreeSet::new();
    for (index, line) in lines.iter().enumerate() {
        let text = line.text.trim();
        let Some((lhs, rhs)) = text.trim_end_matches(';').split_once(" = ") else {
            continue;
        };
        if rhs.trim() != slot || !is_register_name(lhs.trim()) {
            continue;
        }
        let register = lhs.trim();
        // Walk forward while the register still holds that pointer.
        let mut depth = 0i32;
        for next in lines.iter().skip(index + 1).take(24) {
            let text = next.text.as_str();
            if assigns_register(text, register) {
                break;
            }
            for width in [1u8, 2, 4, 8, 16] {
                let cast = match width {
                    1 => "u8",
                    2 => "u16",
                    4 => "u32",
                    8 => "u64",
                    _ => "__m128",
                };
                let head = format!("*({cast}*)(({register}");
                if let Some(position) = text.find(&head) {
                    // Only a plain displacement keeps the register as the base.
                    let after = &text[position + head.len()..];
                    if after.starts_with(')') || after.starts_with(" + ") {
                        widths.insert(width);
                    }
                }
                let bare = format!("*({cast}*)({register})");
                if text.contains(&bare) {
                    widths.insert(width);
                }
            }
            depth += 1;
            if depth > 24 {
                break;
            }
        }
    }
    if widths.is_empty() {
        None
    } else {
        Some(widths.into_iter().collect())
    }
}

/// The stack the prologue reserves (`sub rsp, imm`) in the entry block, if it is a constant.
fn entry_frame_size(session: &mut recraft_core::Session, cfg: &Cfg) -> Option<u64> {
    let block = cfg.blocks.first()?;
    for cfg_insn in &block.insns {
        let insn = session.insn_at(cfg_insn.addr).ok()??;
        if insn.is_invalid() {
            return None;
        }
        match insn.mnemonic() {
            iced_x86::Mnemonic::Sub
                if insn.op_count() >= 2
                    && insn.op_register(0) == iced_x86::Register::RSP
                    && matches!(
                        insn.op_kind(1),
                        iced_x86::OpKind::Immediate8
                            | iced_x86::OpKind::Immediate8to16
                            | iced_x86::OpKind::Immediate8to32
                            | iced_x86::OpKind::Immediate8to64
                            | iced_x86::OpKind::Immediate16
                            | iced_x86::OpKind::Immediate32
                            | iced_x86::OpKind::Immediate32to64
                            | iced_x86::OpKind::Immediate64
                    ) =>
            {
                return u64::try_from(insn.immediate32to64()).ok();
            }
            // A push moves rsp by 8, so the frame offsets in the body would no longer be
            // relative to the value this scan returns.
            iced_x86::Mnemonic::Push => return None,
            iced_x86::Mnemonic::Nop | iced_x86::Mnemonic::Mov => continue,
            _ => return None,
        }
    }
    None
}

/// The displacement of a `var_<hex>` slot name.
fn slot_offset(token: &str) -> Option<u64> {
    let hex = token.strip_prefix("var_")?;
    u64::from_str_radix(hex, 16).ok()
}

/// Name the fields of a pointer argument the type inference left as `void*`: when every
/// access to `arg1 + 0x10` uses the same width, `*(u32*)((arg1 + 0x10))` reads
/// `arg1->field_10`. An offset that is read at two widths keeps its casts, because a field
/// name cannot say which one it means.
fn name_void_pointer_fields(lines: &mut [PseudoLine]) {
    let mut widths: HashMap<(String, u64), BTreeSet<String>> = HashMap::new();
    for line in lines.iter() {
        for (width, base, offset) in raw_field_uses(&line.text) {
            widths.entry((base, offset)).or_default().insert(width);
        }
    }
    if widths.is_empty() {
        return;
    }
    for line in lines.iter_mut() {
        for (width, base, offset) in raw_field_uses(&line.text) {
            let Some(seen) = widths.get(&(base.clone(), offset)) else {
                continue;
            };
            if seen.len() != 1 {
                continue;
            }
            let from = raw_field_text(&width, &base, offset);
            let to = format!("{base}->field_{offset:x}");
            line.text = line.text.replace(&from, &to);
        }
    }
}

/// The exact text [`raw_field_uses`] recognises for one access.
fn raw_field_text(width: &str, base: &str, offset: u64) -> String {
    if offset == 0 {
        format!("*({width}*)({base})")
    } else {
        let offset = if offset < 10 {
            format!("{offset}")
        } else {
            format!("0x{offset:x}")
        };
        format!("*({width}*)(({base} + {offset}))")
    }
}

/// The `*(u32*)((arg1 + 0x10))` and `*(u8*)(arg1)` occurrences of a line: width, base and
/// offset.
fn raw_field_uses(text: &str) -> Vec<(String, String, u64)> {
    const WIDTHS: [&str; 7] = ["u8", "u16", "u32", "u64", "f32", "f64", "__m128"];
    let mut uses = Vec::new();
    for width in WIDTHS {
        let head = format!("*({width}*)(");
        let mut rest = text;
        while let Some(position) = rest.find(&head) {
            let after = &rest[position + head.len()..];
            // `((arg1 + 0x10))` keeps the address in parentheses, `(arg1)` does not.
            let (inner, consumed) = if let Some(body) = after.strip_prefix('(') {
                let mut depth = 1usize;
                let mut end = None;
                for (offset, ch) in body.char_indices() {
                    match ch {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(offset);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                match end {
                    Some(end) => (&body[..end], end + 2),
                    None => break,
                }
            } else {
                let length = after
                    .chars()
                    .take_while(|ch| ch.is_ascii_alphanumeric())
                    .count();
                (&after[..length], length + 1)
            };
            if let Some((base, offset)) = parse_address(inner) {
                uses.push((width.to_owned(), base, offset));
            }
            rest = &after[consumed.min(after.len())..];
        }
    }
    uses
}

/// Split `arg1 + 0x10` (or a bare `arg1`) into the base name and the displacement.
fn parse_address(inner: &str) -> Option<(String, u64)> {
    let inner = inner.trim();
    let (base, offset) = match inner.split_once(" + ") {
        Some((base, offset)) => (base.trim(), Some(offset.trim())),
        None => (inner, None),
    };
    if !base.starts_with("arg") || !base[3..].chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let offset = match offset {
        Some(text) => parse_offset(text)?,
        None => 0,
    };
    Some((base.to_owned(), offset))
}

/// A rendered displacement: `0x10` is hexadecimal, `8` decimal.
fn parse_offset(text: &str) -> Option<u64> {
    if let Some(hex) = text.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).ok()
    } else {
        text.parse().ok()
    }
}

/// Handle-typed arguments of well-known APIs: (API, argument index, name). Specific kinds come
/// first so a slot passed to several APIs gets the most telling name.
const HANDLE_SLOTS: &[(&str, usize, &str)] = &[
    ("ReleaseMutex", 0, "hMutex"),
    ("ReleaseSemaphore", 0, "hSemaphore"),
    ("SetEvent", 0, "hEvent"),
    ("ResetEvent", 0, "hEvent"),
    ("RegCloseKey", 0, "hKey"),
    ("RegOpenKeyExW", 0, "hKey"),
    ("RegQueryValueExW", 0, "hKey"),
    ("RegSetValueExW", 0, "hKey"),
    ("RegEnumKeyExW", 0, "hKey"),
    ("DestroyIcon", 0, "hIcon"),
    ("DestroyCursor", 0, "hCursor"),
    ("UnhookWindowsHookEx", 0, "hHook"),
    ("WindowsCreateStringReference", 2, "hstringHeader"),
    ("WindowsCreateStringReference", 3, "hstring"),
    ("FindClose", 0, "hFind"),
    ("FreeLibrary", 0, "hModule"),
    ("DestroyWindow", 0, "hwnd"),
    ("ShowWindow", 0, "hwnd"),
    ("SendMessageW", 0, "hwnd"),
    ("PostMessageW", 0, "hwnd"),
    ("SetWindowPos", 0, "hwnd"),
    ("InvalidateRect", 0, "hwnd"),
    ("RedrawWindow", 0, "hwnd"),
    ("SetWindowTextW", 0, "hwnd"),
    ("GetWindowLongPtrW", 0, "hwnd"),
    ("DestroyMenu", 0, "hMenu"),
    ("ImageList_Destroy", 0, "himl"),
    ("WindowsDeleteString", 0, "hstring"),
    ("ReleaseDC", 1, "hdc"),
    ("CloseHandle", 0, "hObject"),
    ("DeleteObject", 0, "hObject"),
    ("WaitForSingleObject", 0, "hObject"),
    ("WaitForSingleObjectEx", 0, "hObject"),
];

/// Name the slot that a handle API receives, so `CloseHandle(var_28)` reads
/// `CloseHandle(hObject)`. Runs after the structure names, which claim their slots first.
fn name_handle_slots(lines: &mut [PseudoLine]) {
    let mut assigned: Vec<(u64, &'static str)> = Vec::new();
    for (api, index, name) in HANDLE_SLOTS {
        for line in lines.iter() {
            let text = line.text.as_str();
            if !text.contains(&format!("{api}(")) {
                continue;
            }
            let Some(arguments) = call_arguments(text, api) else {
                continue;
            };
            let Some(argument) = top_level_argument(arguments, *index) else {
                continue;
            };
            let Some(slot) = slot_name_address(argument.trim()) else {
                continue;
            };
            if !assigned.iter().any(|(existing, _)| *existing == slot) {
                assigned.push((slot, name));
            }
        }
    }
    if assigned.is_empty() {
        return;
    }
    let mut used: BTreeSet<String> = BTreeSet::new();
    for line in lines.iter() {
        for token in line
            .text
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .filter(|token| !token.is_empty())
        {
            used.insert(token.to_owned());
        }
    }
    for (slot, stem) in assigned {
        let from = format!("var_{slot:x}");
        // The structure pass may already have named this slot.
        if !used.contains(&from) {
            continue;
        }
        let mut name = stem.to_owned();
        let mut suffix = 2;
        while used.contains(&name) {
            name = format!("{stem}{suffix}");
            suffix += 1;
        }
        used.insert(name.clone());
        for line in lines.iter_mut() {
            line.text = replace_token(&line.text, &from, &name);
        }
    }
}

/// The raw argument list of the first `api(` call in `text`.
fn call_arguments<'a>(text: &'a str, api: &str) -> Option<&'a str> {
    let start = text.find(&format!("{api}("))? + api.len() + 1;
    let rest = &text[start..];
    let mut depth = 1usize;
    for (offset, ch) in rest.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&rest[..offset]);
                }
            }
            _ => {}
        }
    }
    None
}

/// The `index`-th top-level argument of `arguments`.
fn top_level_argument(arguments: &str, index: usize) -> Option<&str> {
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut current = 0usize;
    for (offset, ch) in arguments.char_indices() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                if current == index {
                    return Some(&arguments[start..offset]);
                }
                current += 1;
                start = offset + 1;
            }
            _ => {}
        }
    }
    (current == index).then(|| &arguments[start..])
}

/// The frame displacement of a bare `var_XX` argument.
fn slot_name_address(argument: &str) -> Option<u64> {
    // An out-parameter is written through its address (`&var_58`), so both forms name the slot.
    let argument = argument.strip_prefix('&').unwrap_or(argument);
    let digits = argument.strip_prefix("var_")?;
    u64::from_str_radix(digits, 16).ok()
}

/// The name a structure instance takes when an API fills it: a `RECT` is a rectangle, a
/// `POINT` a position, a `MSG` a message.
fn structure_instance_name(kind: &str) -> &'static str {
    match kind {
        "RECT" => "rect",
        "POINT" => "pt",
        _ => "msg",
    }
}

/// Give the slots that a known API fills a name, so the structure reads `rect.bottom`
/// instead of `var_30.bottom`. Runs after [`name_known_struct_fields`], which produced the
/// member accesses.
fn name_structure_instances(lines: &mut [PseudoLine]) {
    let mut instances: Vec<(u64, &'static str)> = Vec::new();
    for line in lines.iter() {
        let text = line.text.clone();
        let Some(structure) = KNOWN_STRUCTS.iter().find(|structure| {
            structure
                .apis
                .iter()
                .any(|api| text.contains(&format!("{api}(")))
        }) else {
            continue;
        };
        let Some(base) = slot_address_after(&text, "&var_") else {
            continue;
        };
        let kind = if structure.fields.iter().any(|(_, field)| *field == "right") {
            "RECT"
        } else if structure.fields.iter().any(|(_, field)| *field == "hwnd") {
            "MSG"
        } else {
            "POINT"
        };
        if !instances.iter().any(|(existing, _)| *existing == base) {
            instances.push((base, kind));
        }
    }
    if instances.is_empty() {
        return;
    }
    let mut used: BTreeSet<String> = BTreeSet::new();
    for line in lines.iter() {
        for token in line
            .text
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .filter(|token| !token.is_empty())
        {
            used.insert(token.to_owned());
        }
    }
    for (base, kind) in instances {
        let stem = structure_instance_name(kind);
        let mut name = stem.to_owned();
        let mut suffix = 2;
        while used.contains(&name) {
            name = format!("{stem}{suffix}");
            suffix += 1;
        }
        used.insert(name.clone());
        let from = format!("var_{base:x}");
        for line in lines.iter_mut() {
            line.text = replace_token(&line.text, &from, &name);
        }
    }
}

fn name_known_struct_fields(lines: &mut [PseudoLine]) {
    for index in 0..lines.len() {
        let text = lines[index].text.clone();
        let Some(structure) = KNOWN_STRUCTS.iter().find(|structure| {
            structure
                .apis
                .iter()
                .any(|api| text.contains(&format!("{api}(")))
        }) else {
            continue;
        };
        let Some(base) = slot_address_after(&text, "&var_") else {
            continue;
        };
        for (delta, field) in structure.fields {
            let Ok(offset) = u64::from_str_radix(delta, 16) else {
                continue;
            };
            let from = format!("var_{:x}", base + offset);
            let to = format!("var_{base:x}.{field}");
            // The structure's first field is named only where the slot is read: an address
            // (`&var_30`) and a wide clear (`var_30 = xmm0;`) keep the plain name.
            let skip_target = *delta == "0";
            for line in lines.iter_mut().skip(index + 1) {
                line.text = rename_struct_slot(&line.text, &from, &to, skip_target);
            }
        }
    }
}

/// Hexadecimal displacement of the first `&var_…` argument in `text`.
fn slot_address_after(text: &str, marker: &str) -> Option<u64> {
    let start = text.find(marker)? + marker.len();
    let digits: String = text[start..]
        .chars()
        .take_while(|ch| ch.is_ascii_hexdigit())
        .collect();
    u64::from_str_radix(&digits, 16).ok()
}

/// Rename `from` to `to` where it names a slot value. The address (`&var_40`) and a pointer
/// base (`(var_40 + 8)`) keep the plain name: both mean "the pointer stored in this slot".
/// `skip_target` also keeps an assignment target plain, because a store to the first slot can
/// be a wide clear of the whole structure (`var_40 = xmm0;`).
fn rename_struct_slot(text: &str, from: &str, to: &str, skip_target: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(position) = rest.find(from) {
        let before = &rest[..position];
        let after = &rest[position + from.len()..];
        let boundary = before
            .chars()
            .last()
            .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'))
            && after
                .chars()
                .next()
                .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'));
        let address_taken = before.trim_end().ends_with('&');
        let trimmed = after.trim_start();
        let pointer_base = trimmed.starts_with('+') || trimmed.starts_with('-');
        // Already a member path (`var_30.left`) or the tail of one: never rename twice.
        let member = trimmed.starts_with('.') || before.trim_end().ends_with('.');
        let target = skip_target
            && before.trim().is_empty()
            && (trimmed.starts_with('=') || trimmed.starts_with('+') || trimmed.starts_with('-'));
        out.push_str(before);
        if boundary && !address_taken && !pointer_base && !member && !target {
            out.push_str(to);
        } else {
            out.push_str(from);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// `do { ... } while (1);` is the structurer's no-condition loop; `while (1) { ... }` reads
/// the same and is idiomatic C, so the `do` form is rewritten.
fn normalize_infinite_loops(lines: &mut [PseudoLine]) {
    let mut index = 0;
    while index < lines.len() {
        if lines[index].text.trim() != "do {" {
            index += 1;
            continue;
        }
        let Some(close) = matching_brace(lines, index) else {
            index += 1;
            continue;
        };
        if lines[close].text.trim() != "} while (1);" {
            index += 1;
            continue;
        }
        let indent = " ".repeat(leading_spaces(&lines[index].text));
        lines[index].text = format!("{indent}while (1) {{");
        lines[close].text = format!("{indent}}}");
        // Nested loops inside the body are still candidates.
        index += 1;
    }
}

fn entry_arg_value_map(cfg: &Cfg, bodies: &[BlockLift]) -> Vec<[Option<String>; 4]> {
    let count = bodies.len();
    let mut predecessors = vec![Vec::new(); count];
    for (index, block) in cfg.blocks.iter().enumerate() {
        for (target, _) in &block.edges {
            if *target < count {
                predecessors[*target].push(index);
            }
        }
    }
    // A carried value remembers the registers it reads; overwriting one of them anywhere
    // between the definition and the call invalidates it.
    type Carried = (String, BTreeSet<Reg>);
    let mut entering: Vec<[Option<Carried>; 4]> = vec![Default::default(); count];
    let mut leaving: Vec<[Option<Carried>; 4]> = vec![Default::default(); count];
    for _ in 0..(count + 4) {
        let mut changed = false;
        for block in 0..count {
            let mut set: [Option<Carried>; 4] = Default::default();
            if block != cfg.entry
                && let Some(&first) = predecessors[block].first()
            {
                set = leaving[first].clone();
                for &pred in &predecessors[block][1..] {
                    for slot in 0..4 {
                        if set[slot] != leaving[pred][slot] {
                            set[slot] = None;
                        }
                    }
                }
            }
            if set != entering[block] {
                entering[block] = set.clone();
                changed = true;
            }
            let mut out = set;
            let written = &bodies[block].written;
            for value in out.iter_mut() {
                if let Some((_, reads)) = value
                    && reads.iter().any(|reg| written.contains(reg))
                {
                    *value = None;
                }
            }
            for (slot, value, reads) in &bodies[block].arg_writes {
                out[*slot] = value.clone().map(|text| (text, reads.clone()));
            }
            for value in out.iter_mut() {
                if let Some((_, reads)) = value
                    && reads.iter().any(|reg| written.contains(reg))
                {
                    *value = None;
                }
            }
            if out != leaving[block] {
                leaving[block] = out;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    entering
        .into_iter()
        .map(|slots| slots.map(|value| value.map(|(text, _)| text)))
        .collect()
}

/// Which argument copies hold on entry to each block.
///
/// Must-alias dataflow: `in[b]` is the intersection of the predecessors' outgoing maps (an
/// alias is only trusted when every path agrees), and `out[b]` applies the block's own
/// writes — a full-width copy of a register keeps its alias, any other write drops it.
fn entry_alias_map(
    cfg: &Cfg,
    effects: &[Vec<(Reg, Option<Reg>)>],
    is_argument: &dyn Fn(Reg) -> bool,
) -> Vec<BTreeMap<Reg, Reg>> {
    let count = effects.len();
    let mut predecessors = vec![Vec::new(); count];
    for (index, block) in cfg.blocks.iter().enumerate() {
        for (target, _) in &block.edges {
            if *target < count {
                predecessors[*target].push(index);
            }
        }
    }
    let mut entering: Vec<BTreeMap<Reg, Reg>> = vec![BTreeMap::new(); count];
    let mut leaving = entering.clone();
    for _ in 0..(count + 4) {
        let mut changed = false;
        for block in 0..count {
            let set: BTreeMap<Reg, Reg> = if block == cfg.entry {
                // Argument registers hold their own value on entry.
                [Reg::Rcx, Reg::Rdx, Reg::R8, Reg::R9]
                    .into_iter()
                    .filter(|reg| is_argument(*reg))
                    .map(|reg| (reg, reg))
                    .collect()
            } else {
                let mut set: Option<BTreeMap<Reg, Reg>> = None;
                for &pred in &predecessors[block] {
                    let out = &leaving[pred];
                    set = Some(match set {
                        None => out.clone(),
                        Some(current) => current
                            .into_iter()
                            .filter(|(reg, source)| out.get(reg) == Some(source))
                            .collect(),
                    });
                }
                set.unwrap_or_default()
            };
            if set != entering[block] {
                entering[block] = set.clone();
                changed = true;
            }
            let mut out = set;
            let mut written_here: BTreeSet<Reg> = BTreeSet::new();
            for (reg, source) in &effects[block] {
                written_here.insert(*reg);
                let target = source.and_then(|source| {
                    if written_here.contains(&source) {
                        return None;
                    }
                    out.get(&source)
                        .copied()
                        .or_else(|| is_argument(source).then_some(source))
                });
                match target {
                    Some(target) => {
                        out.insert(*reg, target);
                    }
                    None => {
                        out.remove(reg);
                    }
                }
            }
            if out != leaving[block] {
                leaving[block] = out;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    entering
}

/// Which registers may already have been overwritten whenever a block is entered.
///
/// Forward may-write dataflow: `in[b] = 闁?out[pred]`, `out[b] = in[b] 闁?written[b]`.
/// The function entry starts empty (incoming argument registers are intact there).
fn entry_written(cfg: &Cfg, written: &[BTreeSet<Reg>]) -> Vec<BTreeSet<Reg>> {
    let count = written.len();
    let mut predecessors = vec![Vec::new(); count];
    for (index, block) in cfg.blocks.iter().enumerate() {
        for (target, _) in &block.edges {
            if *target < count {
                predecessors[*target].push(index);
            }
        }
    }
    let mut entering: Vec<BTreeSet<Reg>> = vec![BTreeSet::new(); count];
    let mut leaving = written.to_vec();
    for _ in 0..(count + 4) {
        let mut changed = false;
        for block in 0..count {
            let mut set = BTreeSet::new();
            if block != cfg.entry {
                for &pred in &predecessors[block] {
                    set.extend(leaving[pred].iter().copied());
                }
            }
            if set != entering[block] {
                entering[block] = set;
                changed = true;
            }
            let mut out = entering[block].clone();
            out.extend(written[block].iter().copied());
            if out != leaving[block] {
                leaving[block] = out;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    entering
}

struct Block {
    entry: Address,
    flow: Flow,
    condition: Option<String>,
    taken: Option<usize>,
    not_taken: Option<usize>,
    goto_target: Option<usize>,
    has_table: bool,
    /// Jump-table targets in table (case) order.
    table_targets: Vec<usize>,
    /// Traced selector register of the jump table, if any.
    table_selector: Option<recraft_ir::RegView>,
}

struct Structurer {
    blocks: Vec<Block>,
    entry: usize,
    successors: Vec<Vec<usize>>,
    predecessors: Vec<Vec<usize>>,
    rpo: Vec<usize>,
    idom: Vec<Option<usize>>,
    ipdom: Vec<Option<usize>>,
    loop_headers: BTreeSet<usize>,
    loop_of: Vec<Option<usize>>,
    loop_blocks: BTreeMap<usize, BTreeSet<usize>>,
    /// Active loops while emitting bodies: `(header, exit)` for break/continue.
    loop_stack: Vec<(usize, usize)>,
    /// Loop headers currently emitted as an unstructured `while (1)` body (avoids
    /// recursing back into `emit_loop` for the same header).
    fallback_headers: BTreeSet<usize>,
    /// Top-level loops whose inner exit leaves the loop: their whole body is emitted as
    /// labels so one jump cannot place the enclosing continuation on the wrong path.
    flat_headers: BTreeSet<usize>,
    /// Inferred argument counts for direct call targets, used to bound how many setup lines
    /// a tail call may take as arguments.
    call_arg_counts: HashMap<u64, usize>,
    emitted: Vec<bool>,
    labeled: Vec<bool>,
    bodies: Vec<BlockLift>,
    /// Instruction addresses of `push`/`pop` bookkeeping kept out of the listing.
    hidden: BTreeSet<u64>,
    args: Option<ArgNames>,
    /// Exclusive end of the analysed function. CFG construction can follow a tail `jmp`
    /// into the next function; such blocks are foreign code and only the jump back to the
    /// caller (`return sub_x();`) should be rendered.
    end: Address,
}

impl Structurer {
    fn new(
        cfg: &Cfg,
        bodies: Vec<BlockLift>,
        args: Option<ArgNames>,
        end: Address,
        call_arg_counts: HashMap<u64, usize>,
    ) -> Self {
        let index_of =
            |address: Address| cfg.blocks.iter().position(|block| block.start == address);
        let mut blocks = Vec::with_capacity(cfg.blocks.len());
        let mut successors = vec![Vec::new(); cfg.blocks.len()];
        for (index, block) in cfg.blocks.iter().enumerate() {
            let mut taken = None;
            let mut not_taken = None;
            let mut goto_target = None;
            let mut has_table = false;
            for &(target, kind) in &block.edges {
                match kind {
                    EdgeKind::Taken => taken = Some(target),
                    EdgeKind::NotTaken | EdgeKind::Fallthrough => not_taken = Some(target),
                    EdgeKind::Unconditional => goto_target = Some(target),
                    EdgeKind::JumpTable => has_table = true,
                }
                if !successors[index].contains(&target) {
                    successors[index].push(target);
                }
            }
            let lifted = &bodies[index];
            // Prefer the CFG's own table (ordered targets + traced selector); the
            // deduplicated JumpTable edges are only a fallback.
            let dispatch = cfg
                .jump_tables
                .iter()
                .find(|(addr, _)| **addr >= block.start && **addr < block.end);
            let (table_targets, table_selector) = if let Some((_, table)) = dispatch {
                (
                    table
                        .targets
                        .iter()
                        .filter_map(|target| index_of(*target))
                        .collect(),
                    table.selector,
                )
            } else {
                (
                    block
                        .edges
                        .iter()
                        .filter(|(_, kind)| *kind == EdgeKind::JumpTable)
                        .map(|(target, _)| *target)
                        .collect(),
                    None,
                )
            };
            blocks.push(Block {
                entry: block.start,
                flow: lifted.flow,
                condition: lifted.condition.clone(),
                taken,
                not_taken,
                goto_target: match lifted.flow {
                    Flow::Goto(address) => index_of(address),
                    _ => goto_target,
                },
                has_table,
                table_targets,
                table_selector,
            });
        }
        let mut predecessors = vec![Vec::new(); cfg.blocks.len()];
        for (index, nexts) in successors.iter().enumerate() {
            for &next in nexts {
                predecessors[next].push(index);
            }
        }
        let entry = cfg.entry;
        let mut emitted = vec![false; bodies.len()];
        for (index, block) in blocks.iter().enumerate() {
            if block.entry.get() >= end.get() {
                emitted[index] = true;
            }
        }
        Structurer {
            blocks,
            entry,
            successors,
            predecessors,
            rpo: Vec::new(),
            idom: vec![None; bodies.len()],
            ipdom: vec![None; bodies.len()],
            loop_headers: BTreeSet::new(),
            loop_of: vec![None; bodies.len()],
            loop_blocks: BTreeMap::new(),
            loop_stack: Vec::new(),
            fallback_headers: BTreeSet::new(),
            flat_headers: BTreeSet::new(),
            call_arg_counts,
            emitted,
            labeled: vec![false; bodies.len()],
            hidden: hidden_push_pop_pairs(
                cfg.blocks
                    .iter()
                    .map(|block| block.start)
                    .zip(bodies.iter()),
                end,
            ),
            bodies,
            args,
            end,
        }
    }

    // ------------------------------------------------------------ graph math --

    fn compute_rpo(&mut self) {
        let count = self.blocks.len();
        let mut visited = vec![false; count];
        let mut postorder = Vec::with_capacity(count);
        let mut stack = vec![(self.entry, 0usize)];
        visited[self.entry] = true;
        while let Some((node, next_index)) = stack.pop() {
            if next_index < self.successors[node].len() {
                stack.push((node, next_index + 1));
                let next = self.successors[node][next_index];
                if !visited[next] {
                    visited[next] = true;
                    stack.push((next, 0));
                }
            } else {
                postorder.push(node);
            }
        }
        self.rpo = postorder.into_iter().rev().collect();
    }

    fn compute_dominators(&mut self) {
        let count = self.blocks.len();
        let mut idom: Vec<Option<usize>> = vec![None; count];
        idom[self.entry] = Some(self.entry);
        let order: BTreeMap<usize, usize> = self
            .rpo
            .iter()
            .enumerate()
            .map(|(index, node)| (*node, index))
            .collect();
        let mut changed = true;
        while changed {
            changed = false;
            for &node in self.rpo.iter().skip(1) {
                let mut new_idom = None;
                for &pred in &self.predecessors[node] {
                    if idom[pred].is_some() {
                        new_idom = Some(match new_idom {
                            None => pred,
                            Some(current) => intersect(&order, &idom, current, pred),
                        });
                    }
                }
                if let Some(new_idom) = new_idom
                    && idom[node] != Some(new_idom)
                {
                    idom[node] = Some(new_idom);
                    changed = true;
                }
            }
        }
        self.idom = idom;
    }

    /// Post-dominators with a virtual exit node; `None` means "can reach function end
    /// without reconverging", which disables if/else packaging for that header.
    fn compute_post_dominators(&mut self) {
        let count = self.blocks.len();
        let virtual_exit = count;
        let mut predecessors = vec![Vec::new(); count + 1];
        for node in 0..count {
            if self.successors[node].is_empty() {
                predecessors[virtual_exit].push(node);
            }
            for &next in &self.successors[node] {
                predecessors[next].push(node);
            }
        }
        let mut visited = vec![false; count + 1];
        let mut postorder = Vec::new();
        let mut stack = vec![(virtual_exit, 0usize)];
        visited[virtual_exit] = true;
        while let Some((node, next_index)) = stack.pop() {
            if next_index < predecessors[node].len() {
                stack.push((node, next_index + 1));
                let next = predecessors[node][next_index];
                if !visited[next] {
                    visited[next] = true;
                    stack.push((next, 0));
                }
            } else {
                postorder.push(node);
            }
        }
        let rpo: Vec<usize> = postorder.into_iter().rev().collect();
        let order: BTreeMap<usize, usize> = rpo
            .iter()
            .enumerate()
            .map(|(index, node)| (*node, index))
            .collect();
        let mut ipdom: Vec<Option<usize>> = vec![None; count + 1];
        ipdom[virtual_exit] = Some(virtual_exit);
        // Exit blocks post-dominate directly from the virtual exit.
        for (node, successors) in self.successors.iter().enumerate() {
            if successors.is_empty() {
                ipdom[node] = Some(virtual_exit);
            }
        }
        let mut changed = true;
        while changed {
            changed = false;
            for &node in rpo.iter().skip(1) {
                if node == virtual_exit {
                    continue;
                }
                let mut new_ipdom = None;
                for &next in &self.successors[node] {
                    if ipdom[next].is_some() {
                        new_ipdom = Some(match new_ipdom {
                            None => next,
                            Some(current) => intersect(&order, &ipdom, current, next),
                        });
                    }
                }
                if let Some(new_ipdom) = new_ipdom
                    && ipdom[node] != Some(new_ipdom)
                {
                    ipdom[node] = Some(new_ipdom);
                    changed = true;
                }
            }
        }
        self.ipdom = (0..count)
            .map(|node| ipdom[node].filter(|&value| value != virtual_exit))
            .collect();
    }

    fn compute_loops(&mut self) {
        let count = self.blocks.len();
        let mut loops: Vec<(usize, BTreeSet<usize>)> = Vec::new();
        for node in 0..count {
            for &next in &self.successors[node] {
                if self.dominates(next, node) {
                    let mut body = BTreeSet::new();
                    body.insert(next);
                    // Natural loop: the back-edge source and everything reaching it
                    // without passing through the header (a self-loop is just {header}).
                    if node != next {
                        let mut work = vec![node];
                        while let Some(block) = work.pop() {
                            if body.insert(block) {
                                for &pred in &self.predecessors[block] {
                                    if !body.contains(&pred) {
                                        work.push(pred);
                                    }
                                }
                            }
                        }
                    }
                    loops.push((next, body));
                }
            }
        }
        // Innermost loop wins for shared blocks (sort by size, smallest first).
        loops.sort_by_key(|(_, body)| body.len());
        for (header, body) in loops {
            self.loop_headers.insert(header);
            self.loop_of[header] = Some(header);
            for &block in &body {
                if block != header && self.loop_of[block].is_none() {
                    self.loop_of[block] = Some(header);
                }
            }
            self.loop_blocks.entry(header).or_insert(body);
        }
    }

    fn dominates(&self, candidate: usize, node: usize) -> bool {
        let mut current = node;
        for _ in 0..=self.blocks.len() {
            if current == candidate {
                return true;
            }
            match self.idom[current] {
                Some(parent) if parent != current => current = parent,
                _ => return false,
            }
        }
        false
    }

    // -------------------------------------------------------------- emission --

    fn emit_all(&mut self, lines: &mut Vec<PseudoLine>) {
        self.flat_headers = self.flat_loop_headers();
        if !self.flat_headers.is_empty() {
            lines.push(PseudoLine {
                addr: self.blocks[self.entry].entry,
                text: "// some loops keep labels (an inner loop exits its parent)".to_owned(),
            });
        }
        self.emit_linear(lines, self.entry, 1, &BTreeSet::new());
        for index in 0..self.blocks.len() {
            if self.emitted[index] {
                continue;
            }
            self.emitted[index] = true;
            self.labeled[index] = true;
            lines.push(PseudoLine {
                addr: self.blocks[index].entry,
                text: format!("{}loc_{:x}:", indent(1), self.blocks[index].entry.get()),
            });
            self.emit_simple(lines, index, 1);
            self.emit_raw_flow(lines, index, 1);
        }
    }

    /// Top-level loops that contain a nested loop whose body jumps out of the top loop into
    /// a different region. `while` bodies cannot express that jump (the parent's
    /// continuation would move), so such a loop is emitted as labelled blocks instead —
    /// locally, so the rest of the function still gets structured.
    fn flat_loop_headers(&self) -> BTreeSet<usize> {
        flat_loop_headers_of(&self.loop_blocks, &self.successors)
    }

    /// Emit a loop body as labelled blocks, in address order, with raw gotos for every edge.
    /// Control enters at the header and leaves through the same gotos the machine code uses,
    /// so the enclosing structure never assumes a single continuation. `None` means the
    /// region cannot be emitted inside this `stop` scope (the caller falls back to the
    /// unstructured loop); `Some(None)` means every path returns.
    fn emit_flat_region(
        &mut self,
        lines: &mut Vec<PseudoLine>,
        header: usize,
        level: usize,
        stop: &BTreeSet<usize>,
    ) -> Option<Option<usize>> {
        let body = self.loop_blocks.get(&header)?.clone();
        let exit = self.ipdom[header].filter(|exit| !body.contains(exit));
        // Inside a surrounding region the continuation has to be one of that region's stop
        // blocks, otherwise the text after the loop would belong to the wrong scope.
        if !stop.is_empty() && !exit.is_some_and(|exit| stop.contains(&exit)) {
            return None;
        }
        if header != self.entry && !self.labeled[header] {
            lines.push(PseudoLine {
                addr: self.blocks[header].entry,
                text: format!(
                    "{}loc_{:x}:",
                    indent(level),
                    self.blocks[header].entry.get()
                ),
            });
            self.labeled[header] = true;
        }
        for block in body.iter().copied() {
            if self.emitted[block] {
                continue;
            }
            self.emitted[block] = true;
            if block != header && self.predecessors[block].len() > 1 && !self.labeled[block] {
                lines.push(PseudoLine {
                    addr: self.blocks[block].entry,
                    text: format!("{}loc_{:x}:", indent(level), self.blocks[block].entry.get()),
                });
                self.labeled[block] = true;
            }
            self.emit_simple(lines, block, level);
            self.emit_raw_flow(lines, block, level);
        }
        Some(exit)
    }

    fn emit_linear(
        &mut self,
        lines: &mut Vec<PseudoLine>,
        start: usize,
        level: usize,
        stop: &BTreeSet<usize>,
    ) {
        let mut current = start;
        let mut guard = 0usize;
        while guard <= self.blocks.len() + 8 {
            guard += 1;
            if current >= self.blocks.len() || self.emitted[current] || stop.contains(&current) {
                return;
            }
            if let Some(header) = self.loop_of[current]
                && header != current
                && !self.emitted[header]
            {
                // Entered a loop body without its header first: keep output safe.
                self.emitted[current] = true;
                self.emit_simple(lines, current, level);
                self.emit_raw_flow(lines, current, level);
                return;
            }
            if self.loop_headers.contains(&current) && !self.fallback_headers.contains(&current) {
                let flat = if self.flat_headers.contains(&current) {
                    self.emit_flat_region(lines, current, level, stop)
                } else {
                    None
                };
                match flat {
                    Some(Some(exit)) => {
                        current = exit;
                        continue;
                    }
                    Some(None) => return,
                    None => {}
                }
                match self.emit_loop(lines, current, level) {
                    Some(exit) => {
                        current = exit;
                        continue;
                    }
                    None => {
                        self.emitted[current] = true;
                        self.emit_simple(lines, current, level);
                        self.emit_raw_flow(lines, current, level);
                        return;
                    }
                }
            }
            if self.predecessors[current].len() > 1
                && !self.labeled[current]
                && current != self.entry
            {
                lines.push(PseudoLine {
                    addr: self.blocks[current].entry,
                    text: format!(
                        "{}loc_{:x}:",
                        indent(level),
                        self.blocks[current].entry.get()
                    ),
                });
                self.labeled[current] = true;
            }
            self.emitted[current] = true;
            let block_statements = lines.len();
            self.emit_simple(lines, current, level);
            if self.blocks[current].has_table {
                self.emit_switch(lines, current, level, block_statements);
                return;
            }

            match self.blocks[current].flow {
                Flow::Return => {
                    lines.push(PseudoLine {
                        addr: self.blocks[current].entry,
                        text: format!("{}return;", indent(level)),
                    });
                    return;
                }
                Flow::Goto(address) => {
                    if address.get() >= self.end.get() {
                        // A tail call keeps the caller's argument setup when it sits on the
                        // immediately preceding lines, bounded by the callee's arity (the
                        // same rule the cross-block call fill uses for direct calls).
                        let limit = self
                            .call_arg_counts
                            .get(&address.get())
                            .copied()
                            .unwrap_or(1);
                        let arguments = take_adjacent_arguments(lines, limit);
                        let call = if arguments.is_empty() {
                            format!("{}return sub_{:x}();", indent(level), address.get())
                        } else {
                            format!(
                                "{}return sub_{:x}({});",
                                indent(level),
                                address.get(),
                                arguments.join(", ")
                            )
                        };
                        lines.push(PseudoLine {
                            addr: self.blocks[current].entry,
                            text: call,
                        });
                        return;
                    }
                    let Some(target) = self.blocks[current].goto_target else {
                        self.emit_raw_flow(lines, current, level);
                        return;
                    };
                    if let Some(&(loop_header, loop_exit)) = self.loop_stack.last() {
                        if target == loop_exit {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!("{}break;", indent(level)),
                            });
                            return;
                        }
                        if target == loop_header {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!("{}continue;", indent(level)),
                            });
                            return;
                        }
                    }
                    if target == current {
                        self.emit_raw_flow(lines, current, level);
                        return;
                    }
                    if stop.contains(&target) {
                        // Leaving the current arm/region: the enclosing construct resumes
                        // at the join, so no explicit jump is needed. Inside a loop body
                        // only targets that stay in the body may fall out; anything else
                        // that is not break/continue must stay an explicit jump.
                        if let Some(&(loop_header, _)) = self.loop_stack.last() {
                            let inside_body = self
                                .loop_blocks
                                .get(&loop_header)
                                .is_some_and(|body| body.contains(&target));
                            if !inside_body {
                                self.emit_raw_flow(lines, current, level);
                            }
                            return;
                        }
                        return;
                    }
                    if self.emitted[target] {
                        if self.duplicate_leaf(lines, target, level) {
                            return;
                        }
                        self.emit_raw_flow(lines, current, level);
                        return;
                    }
                    current = target;
                }
                Flow::CondGoto { .. } => {
                    let (Some(condition), Some(taken), Some(not_taken)) = (
                        self.blocks[current].condition.clone(),
                        self.blocks[current].taken,
                        self.blocks[current].not_taken,
                    ) else {
                        self.emit_raw_flow(lines, current, level);
                        return;
                    };
                    if taken == not_taken {
                        current = taken;
                        continue;
                    }
                    // A branch that leaves the analysed function is a tail call into the
                    // next function; the CFG may have followed that jump.
                    if self.blocks[taken].entry.get() >= self.end.get() {
                        lines.push(PseudoLine {
                            addr: self.blocks[current].entry,
                            text: format!(
                                "{}if ({condition}) return sub_{:x}();",
                                indent(level),
                                self.blocks[taken].entry.get()
                            ),
                        });
                        current = not_taken;
                        continue;
                    }
                    if self.blocks[not_taken].entry.get() >= self.end.get() {
                        lines.push(PseudoLine {
                            addr: self.blocks[current].entry,
                            text: format!(
                                "{}if ({}) return sub_{:x}();",
                                indent(level),
                                negate(&condition),
                                self.blocks[not_taken].entry.get()
                            ),
                        });
                        current = taken;
                        continue;
                    }
                    // Inside a loop body: a branch to the loop exit is `break`, a branch
                    // back to the header is `continue`.
                    if let Some(&(loop_header, loop_exit)) = self.loop_stack.last() {
                        if taken == loop_exit {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!("{}if ({condition}) break;", indent(level)),
                            });
                            current = not_taken;
                            continue;
                        }
                        if not_taken == loop_exit {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!(
                                    "{}if ({}) break;",
                                    indent(level),
                                    negate(&condition)
                                ),
                            });
                            current = taken;
                            continue;
                        }
                        if taken == loop_header {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!("{}if ({condition}) continue;", indent(level)),
                            });
                            current = not_taken;
                            continue;
                        }
                        if not_taken == loop_header {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!(
                                    "{}if ({}) continue;",
                                    indent(level),
                                    negate(&condition)
                                ),
                            });
                            current = taken;
                            continue;
                        }
                    }
                    let join = self.ipdom[current];
                    if let Some(join) = join {
                        // A join inside the enclosing region's stop set is fine: the arms
                        // stop at it and the region ends there (early-exit chains).
                        if join >= self.blocks.len() || self.emitted[join] {
                            // The join was consumed by an earlier arm. If the taken arm is
                            // a short acyclic tail, duplicate it here rather than leaving a
                            // dangling labelled jump and abandoning the fallthrough path.
                            let mut arm_lines = Vec::new();
                            if self.duplicate_leaf(&mut arm_lines, taken, level + 1) {
                                lines.push(PseudoLine {
                                    addr: self.blocks[current].entry,
                                    text: format!("{}if ({condition}) {{", indent(level)),
                                });
                                lines.extend(arm_lines);
                                lines.push(PseudoLine {
                                    addr: self.blocks[current].entry,
                                    text: format!("{}}}", indent(level)),
                                });
                                current = not_taken;
                                continue;
                            }
                            let mut else_lines = Vec::new();
                            if self.duplicate_leaf(&mut else_lines, not_taken, level + 1) {
                                lines.push(PseudoLine {
                                    addr: self.blocks[current].entry,
                                    text: format!("{}if (!{condition}) {{", indent(level)),
                                });
                                lines.extend(else_lines);
                                lines.push(PseudoLine {
                                    addr: self.blocks[current].entry,
                                    text: format!("{}}}", indent(level)),
                                });
                                current = taken;
                                continue;
                            }
                            self.emit_raw_flow(lines, current, level);
                            return;
                        }
                        let mut arm_stop = stop.clone();
                        arm_stop.insert(join);
                        // Emit both arms into buffers so empty arms do not produce empty
                        // `if` blocks (common for early-exit joins).
                        let mut then_lines = Vec::new();
                        if !self.emitted[taken] {
                            self.emit_linear(&mut then_lines, taken, level + 1, &arm_stop);
                        } else {
                            self.duplicate_leaf(&mut then_lines, taken, level + 1);
                        }
                        let mut else_lines = Vec::new();
                        if !self.emitted[not_taken] {
                            self.emit_linear(&mut else_lines, not_taken, level + 1, &arm_stop);
                        } else {
                            self.duplicate_leaf(&mut else_lines, not_taken, level + 1);
                        }
                        let entry = self.blocks[current].entry;
                        match (then_lines.is_empty(), else_lines.is_empty()) {
                            (true, true) => {}
                            (true, false) => {
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}if (!{condition}) {{", indent(level)),
                                });
                                lines.extend(else_lines);
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}}}", indent(level)),
                                });
                            }
                            (false, true) => {
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}if ({condition}) {{", indent(level)),
                                });
                                lines.extend(then_lines);
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}}}", indent(level)),
                                });
                            }
                            (false, false) => {
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}if ({condition}) {{", indent(level)),
                                });
                                lines.extend(then_lines);
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}}} else {{", indent(level)),
                                });
                                lines.extend(else_lines);
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}}}", indent(level)),
                                });
                            }
                        }
                        current = join;
                        continue;
                    }
                    // No reconvergence (both arms terminate): emit the taken arm as an
                    // early-exit `if` and continue with the fallthrough path.
                    //
                    // A target shared by several failure checks must not be inlined at
                    // the first check: doing so consumes the block, and later checks fall
                    // back to a raw `goto` that also *ends the current region*, pushing
                    // the rest of the loop body outside the `while`. Emit a labelled jump
                    // instead and keep following the fallthrough path.
                    let shared_loop_exit = self.loop_stack.last().is_some_and(|(header, _)| {
                        self.loop_blocks
                            .get(header)
                            .is_some_and(|body| body.contains(&taken))
                    });
                    if taken != not_taken
                        && !self.emitted[taken]
                        && shared_loop_exit
                        && self.predecessors[taken].len() > 1
                    {
                        lines.push(PseudoLine {
                            addr: self.blocks[current].entry,
                            text: format!(
                                "{}if ({condition}) goto loc_{:x};",
                                indent(level),
                                self.blocks[taken].entry.get()
                            ),
                        });
                        current = not_taken;
                        continue;
                    }
                    if !self.emitted[taken] && taken != not_taken {
                        lines.push(PseudoLine {
                            addr: self.blocks[current].entry,
                            text: format!("{}if ({condition}) {{", indent(level)),
                        });
                        self.emit_linear(lines, taken, level + 1, stop);
                        lines.push(PseudoLine {
                            addr: self.blocks[current].entry,
                            text: format!("{}}}", indent(level)),
                        });
                        current = not_taken;
                        continue;
                    }
                    if self.emitted[taken] && taken != not_taken {
                        let mut arm_lines = Vec::new();
                        if self.duplicate_leaf(&mut arm_lines, taken, level + 1) {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!("{}if ({condition}) {{", indent(level)),
                            });
                            lines.extend(arm_lines);
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!("{}}}", indent(level)),
                            });
                            current = not_taken;
                            continue;
                        }
                    }
                    self.emit_raw_flow(lines, current, level);
                    return;
                }
                _ => {
                    let Some(next) = self.blocks[current].not_taken else {
                        return;
                    };
                    if next == current {
                        return;
                    }
                    current = next;
                }
            }
        }
    }

    fn emit_loop(
        &mut self,
        lines: &mut Vec<PseudoLine>,
        header: usize,
        level: usize,
    ) -> Option<usize> {
        // Loop constructs are emitted without a label, but another path may still jump to
        // the header (an external `goto`, or a duplicate arm). Keep a label in front of the
        // construct so those jumps have a target; the final tidy pass drops it when unused.
        if header != self.entry && !self.labeled[header] {
            lines.push(PseudoLine {
                addr: self.blocks[header].entry,
                text: format!(
                    "{}loc_{:x}:",
                    indent(level),
                    self.blocks[header].entry.get()
                ),
            });
            self.labeled[header] = true;
        }
        let body = self.loop_blocks.get(&header)?.clone();
        if self.blocks[header].condition.is_none() {
            // No condition at the header: try a do-while whose latch tests the condition
            // at the bottom of the body.
            return self.emit_do_while(lines, header, level, &body);
        }
        let condition = self.blocks[header].condition.clone()?;
        let taken = self.blocks[header].taken?;
        let not_taken = self.blocks[header].not_taken?;
        // One successor stays in the loop, the other leaves it; otherwise this is not a
        // structured pre-test loop and the caller falls back to gotos.
        let (inside, exit, condition) = if body.contains(&taken) && !body.contains(&not_taken) {
            (taken, not_taken, condition)
        } else if body.contains(&not_taken) && !body.contains(&taken) {
            (not_taken, taken, negate(&condition))
        } else {
            return self.emit_unstructured_loop(lines, header, level, &body);
        };

        // A single-block loop whose header is also the latch is a do-while: emit the
        // statements then test the condition at the bottom.
        if body.len() == 1 {
            self.loop_stack.push((header, exit));
            lines.push(PseudoLine {
                addr: self.blocks[header].entry,
                text: format!("{}do {{", indent(level)),
            });
            self.emitted[header] = true;
            self.emit_simple(lines, header, level + 1);
            lines.push(PseudoLine {
                addr: self.blocks[header].entry,
                text: format!("{}}} while ({condition});", indent(level)),
            });
            self.loop_stack.pop();
            return Some(exit);
        }

        // Header statements run once per iteration: keep them inside the loop and
        // break when the condition says to leave.
        self.loop_stack.push((header, exit));
        lines.push(PseudoLine {
            addr: self.blocks[header].entry,
            text: format!("{}while (1) {{", indent(level)),
        });
        self.emitted[header] = true;
        self.emit_simple(lines, header, level + 1);
        // `condition` above is the condition that stays in the loop (the fallthrough path
        // after the rewrite), so leaving takes its negation.
        lines.push(PseudoLine {
            addr: self.blocks[header].entry,
            text: format!("{}if ({}) break;", indent(level + 1), negate(&condition)),
        });

        let mut stop = BTreeSet::new();
        for block in 0..self.blocks.len() {
            if !body.contains(&block) {
                stop.insert(block);
            }
        }
        if inside != header {
            self.emit_linear(lines, inside, level + 1, &stop);
        }
        lines.push(PseudoLine {
            addr: self.blocks[header].entry,
            text: format!("{}}}", indent(level)),
        });
        for block in &body {
            if !self.emitted[*block] {
                self.emitted[*block] = true;
                lines.push(PseudoLine {
                    addr: self.blocks[*block].entry,
                    text: format!(
                        "{}loc_{:x}:",
                        indent(level),
                        self.blocks[*block].entry.get()
                    ),
                });
                self.emit_simple(lines, *block, level);
                self.emit_raw_flow(lines, *block, level);
            }
        }
        self.loop_stack.pop();
        Some(exit)
    }

    /// The header condition does not split into "stay in loop" vs "leave loop" (e.g. a
    /// short-circuit guard at the top of the body). Emit `while (1) { <header structured>
    /// ... }` and let break/continue carry the exits.
    fn emit_unstructured_loop(
        &mut self,
        lines: &mut Vec<PseudoLine>,
        header: usize,
        level: usize,
        body: &BTreeSet<usize>,
    ) -> Option<usize> {
        if self.emitted[header] || self.fallback_headers.contains(&header) {
            return None;
        }
        let mut exit = None;
        for &block in body {
            for &next in &self.successors[block] {
                if !body.contains(&next) {
                    exit = Some(next);
                    break;
                }
            }
            if exit.is_some() {
                break;
            }
        }
        let exit = exit?;
        let mut stop = BTreeSet::new();
        for block in 0..self.blocks.len() {
            if !body.contains(&block) {
                stop.insert(block);
            }
        }
        lines.push(PseudoLine {
            addr: self.blocks[header].entry,
            text: format!("{}while (1) {{", indent(level)),
        });
        self.loop_stack.push((header, exit));
        self.fallback_headers.insert(header);
        self.emit_linear(lines, header, level + 1, &stop);
        self.fallback_headers.remove(&header);
        self.loop_stack.pop();
        lines.push(PseudoLine {
            addr: self.blocks[header].entry,
            text: format!("{}}}", indent(level)),
        });
        for block in body {
            if !self.emitted[*block] {
                self.emitted[*block] = true;
                lines.push(PseudoLine {
                    addr: self.blocks[*block].entry,
                    text: format!(
                        "{}loc_{:x}:",
                        indent(level + 1),
                        self.blocks[*block].entry.get()
                    ),
                });
                self.emit_simple(lines, *block, level + 1);
                self.emit_raw_flow(lines, *block, level + 1);
            }
        }
        Some(exit)
    }

    /// Emit `do { ... } while (cond);` when the header has no condition and a latch block
    /// inside the body branches back to the header (or leaves the loop).
    fn emit_do_while(
        &mut self,
        lines: &mut Vec<PseudoLine>,
        header: usize,
        level: usize,
        body: &BTreeSet<usize>,
    ) -> Option<usize> {
        let mut latch = None;
        for &block in body {
            if block == header {
                continue;
            }
            let Some(condition) = self.blocks[block].condition.clone() else {
                continue;
            };
            let (Some(taken), Some(not_taken)) =
                (self.blocks[block].taken, self.blocks[block].not_taken)
            else {
                continue;
            };
            let (exit, continue_condition) = if taken == header && !body.contains(&not_taken) {
                (not_taken, condition)
            } else if not_taken == header && !body.contains(&taken) {
                (taken, negate(&condition))
            } else {
                continue;
            };
            latch = Some((block, exit, continue_condition));
            break;
        }
        let (latch, exit, continue_condition) = latch?;

        let mut stop = BTreeSet::new();
        for block in 0..self.blocks.len() {
            if !body.contains(&block) {
                stop.insert(block);
            }
        }
        let mut body_stop = stop.clone();
        body_stop.insert(latch);

        lines.push(PseudoLine {
            addr: self.blocks[header].entry,
            text: format!("{}do {{", indent(level)),
        });
        self.emitted[header] = true;
        self.loop_stack.push((header, exit));
        self.emit_simple(lines, header, level + 1);
        // Follow the header's flow into the body (stop before the latch).
        if let Some(next) = self.blocks[header].not_taken
            && next != latch
        {
            self.emit_linear(lines, next, level + 1, &body_stop);
        } else if let Some(target) = self.blocks[header].goto_target
            && target != latch
        {
            self.emit_linear(lines, target, level + 1, &body_stop);
        }
        if !self.emitted[latch] {
            self.emitted[latch] = true;
            self.emit_simple(lines, latch, level + 1);
        }
        lines.push(PseudoLine {
            addr: self.blocks[latch].entry,
            text: format!("{}}} while ({continue_condition});", indent(level)),
        });
        // Any body blocks that were not reached linearly stay as labelled leftovers.
        for block in body {
            if !self.emitted[*block] {
                self.emitted[*block] = true;
                lines.push(PseudoLine {
                    addr: self.blocks[*block].entry,
                    text: format!(
                        "{}loc_{:x}:",
                        indent(level + 1),
                        self.blocks[*block].entry.get()
                    ),
                });
                self.emit_simple(lines, *block, level + 1);
                self.emit_raw_flow(lines, *block, level + 1);
            }
        }
        self.loop_stack.pop();
        Some(exit)
    }

    fn emit_switch(
        &mut self,
        lines: &mut Vec<PseudoLine>,
        block: usize,
        level: usize,
        block_statements: usize,
    ) {
        let targets = self.blocks[block].table_targets.clone();
        if targets.is_empty() {
            self.emit_raw_flow(lines, block, level);
            return;
        }
        // The block's statements are the compiler's dispatch (bounds-scaled table load and
        // indirect jump); `switch` already says what they do. Drop them only when the range
        // holds nothing but register arithmetic and that jump, so a call or store that
        // happens to share the block is never hidden.
        if is_dispatch_only(&lines[block_statements..]) {
            lines.truncate(block_statements);
        }
        let entry = self.blocks[block].entry;
        // Prefer a traced selector: map argument registers back to `argN` when intact.
        let selector = self.blocks[block]
            .table_selector
            .map(|view| {
                if let Some(args) = self.args.as_ref()
                    && let Some(name) = args.name_for(view.base)
                {
                    return name.to_owned();
                }
                view.name().to_owned()
            })
            .unwrap_or_else(|| "index".to_owned());
        lines.push(PseudoLine {
            addr: entry,
            text: format!(
                "{}/* jump table: {} targets, selector={selector} */",
                indent(level),
                targets.len()
            ),
        });
        lines.push(PseudoLine {
            addr: entry,
            text: format!("{}switch ({selector}) {{", indent(level)),
        });
        let join = self.ipdom[block];
        if self.switch_inline_safe(&targets, join) {
            let mut previous = None;
            for (case, &target) in targets.iter().enumerate() {
                if previous == Some(target) {
                    lines.push(PseudoLine {
                        addr: entry,
                        text: format!("{}case {case}:", indent(level + 1)),
                    });
                    continue;
                }
                previous = Some(target);
                lines.push(PseudoLine {
                    addr: self.blocks[target].entry,
                    text: format!("{}case {case}: {{", indent(level + 1)),
                });
                self.emitted[target] = true;
                self.emit_simple(lines, target, level + 2);
                match &self.blocks[target].flow {
                    Flow::Return => lines.push(PseudoLine {
                        addr: self.blocks[target].entry,
                        text: format!("{}return;", indent(level + 2)),
                    }),
                    Flow::Goto(_) => lines.push(PseudoLine {
                        addr: self.blocks[target].entry,
                        text: format!("{}break;", indent(level + 2)),
                    }),
                    _ => {}
                }
                lines.push(PseudoLine {
                    addr: self.blocks[target].entry,
                    text: format!("{}}}", indent(level + 1)),
                });
            }
            lines.push(PseudoLine {
                addr: entry,
                text: format!("{}}}", indent(level)),
            });
            return;
        }
        for (case, target) in targets.iter().enumerate() {
            lines.push(PseudoLine {
                addr: self.blocks[*target].entry,
                text: format!(
                    "{}case {case}: goto loc_{:x};",
                    indent(level + 1),
                    self.blocks[*target].entry.get()
                ),
            });
        }
        lines.push(PseudoLine {
            addr: entry,
            text: format!("{}}}", indent(level)),
        });
        // Case bodies are emitted as labelled regions by the unvisited-block pass.
    }

    /// Inline a jump table only when every target is a distinct, already-unemitted leaf
    /// that returns or jumps to the switch join; otherwise keep the label/goto form.
    fn switch_inline_safe(&self, targets: &[usize], join: Option<usize>) -> bool {
        let mut seen = BTreeSet::new();
        let mut previous = None;
        for &target in targets {
            if let Some(previous) = previous {
                if previous == target {
                    continue;
                }
                if seen.contains(&target) {
                    return false; // non-adjacent duplicate case target
                }
            }
            previous = Some(target);
            if !seen.insert(target) {
                continue;
            }
            if self.emitted[target] {
                return false;
            }
            match &self.blocks[target].flow {
                Flow::Return => {}
                Flow::Goto(goto) if join.is_some_and(|index| self.blocks[index].entry == *goto) => {
                }
                _ => return false,
            }
        }
        true
    }

    /// Re-emit an already-emitted straight-line chain that ends in `return`/tail call
    /// instead of emitting a `goto` to it. The chain is duplicated only inside the one
    /// branch that reaches it, so extra copies never both execute; this removes most
    /// shared-epilogue and shared-tail `goto`s.
    fn duplicate_leaf(&mut self, lines: &mut Vec<PseudoLine>, block: usize, level: usize) -> bool {
        let mut chain: Vec<usize> = Vec::new();
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        let mut current = block;
        let mut budget = 24usize;
        while budget > 0 {
            if current >= self.blocks.len() || !seen.insert(current) {
                return false;
            }
            if self.blocks[current].has_table || self.loop_headers.contains(&current) {
                return false;
            }
            chain.push(current);
            budget -= 1;
            match self.blocks[current].flow {
                Flow::Return => break,
                Flow::Goto(_) if self.blocks[current].goto_target.is_none() => break,
                Flow::Goto(_) => {
                    let Some(target) = self.blocks[current].goto_target else {
                        return false;
                    };
                    current = target;
                }
                Flow::Fallthrough => {
                    let Some(next) = self.blocks[current].not_taken else {
                        // No successor: the block ends in padding/an external tail call.
                        break;
                    };
                    if next == current {
                        return false;
                    }
                    current = next;
                }
                Flow::CondGoto { .. } => return false,
            }
        }
        if budget == 0 {
            return false;
        }
        let Some(&last) = chain.last() else {
            return false;
        };
        let ends_leaf = matches!(self.blocks[last].flow, Flow::Return)
            || (self.blocks[last].goto_target.is_none()
                && matches!(self.blocks[last].flow, Flow::Goto(_)))
            // A block ending in `int3`/an external tail call can have no successor without
            // being a `Flow::Return`; it still terminates this duplicated leaf.
            || self.successors[last].is_empty();
        if !ends_leaf {
            return false;
        }
        let statements: usize = chain
            .iter()
            .map(|index| self.bodies[*index].statements.len())
            .sum();
        if statements > 24 {
            return false;
        }
        for &index in &chain {
            self.emit_simple(lines, index, level);
        }
        if matches!(self.blocks[last].flow, Flow::Return) || self.successors[last].is_empty() {
            lines.push(PseudoLine {
                addr: self.blocks[last].entry,
                text: format!("{}return;", indent(level)),
            });
        } else {
            self.emit_raw_flow(lines, last, level);
        }
        true
    }

    fn emit_simple(&self, lines: &mut Vec<PseudoLine>, block: usize, level: usize) {
        for statement in &self.bodies[block].statements {
            if self.hidden.contains(&statement.addr.get()) {
                continue;
            }
            lines.push(PseudoLine {
                addr: statement.addr,
                text: format!("{}{}", indent(level), statement.text),
            });
        }
    }

    fn emit_raw_flow(&mut self, lines: &mut Vec<PseudoLine>, block: usize, level: usize) {
        let entry = self.blocks[block].entry;
        match &self.blocks[block].flow {
            Flow::Return => lines.push(PseudoLine {
                addr: entry,
                text: format!("{}return;", indent(level)),
            }),
            Flow::Goto(target) => {
                if target.get() >= self.end.get() {
                    lines.push(PseudoLine {
                        addr: entry,
                        text: format!("{}return sub_{:x}();", indent(level), target.get()),
                    });
                } else if self.blocks[block].goto_target.is_some() {
                    // Labelled leftovers are emitted while a loop is still active; keep
                    // its back edge as `continue` and its exit as `break`.
                    if let Some(&(header, exit)) = self.loop_stack.last() {
                        if Some(header) == self.blocks[block].goto_target {
                            lines.push(PseudoLine {
                                addr: entry,
                                text: format!("{}continue;", indent(level)),
                            });
                            return;
                        }
                        if Some(exit) == self.blocks[block].goto_target {
                            lines.push(PseudoLine {
                                addr: entry,
                                text: format!("{}break;", indent(level)),
                            });
                            return;
                        }
                    }
                    lines.push(PseudoLine {
                        addr: entry,
                        text: format!("{}goto loc_{:x};", indent(level), target.get()),
                    });
                } else {
                    lines.push(PseudoLine {
                        addr: entry,
                        text: format!("{}return sub_{:x}();", indent(level), target.get()),
                    });
                }
            }
            Flow::CondGoto { target, .. } => {
                let condition = self.blocks[block]
                    .condition
                    .clone()
                    .unwrap_or_else(|| "/* cond */".to_owned());
                if target.get() >= self.end.get() {
                    lines.push(PseudoLine {
                        addr: entry,
                        text: format!(
                            "{}if ({condition}) return sub_{:x}();",
                            indent(level),
                            target.get()
                        ),
                    });
                    return;
                }
                if let Some(&(header, exit)) = self.loop_stack.last() {
                    if Some(header) == self.blocks[block].taken {
                        lines.push(PseudoLine {
                            addr: entry,
                            text: format!("{}if ({condition}) continue;", indent(level)),
                        });
                        return;
                    }
                    if Some(header) == self.blocks[block].not_taken {
                        lines.push(PseudoLine {
                            addr: entry,
                            text: format!("{}if ({}) continue;", indent(level), negate(&condition)),
                        });
                        return;
                    }
                    if Some(exit) == self.blocks[block].taken {
                        lines.push(PseudoLine {
                            addr: entry,
                            text: format!("{}if ({condition}) break;", indent(level)),
                        });
                        return;
                    }
                    if Some(exit) == self.blocks[block].not_taken {
                        lines.push(PseudoLine {
                            addr: entry,
                            text: format!("{}if ({}) break;", indent(level), negate(&condition)),
                        });
                        return;
                    }
                }
                lines.push(PseudoLine {
                    addr: entry,
                    text: format!(
                        "{}if ({condition}) goto loc_{:x};",
                        indent(level),
                        target.get()
                    ),
                });
            }
            _ => {}
        }
    }
}

/// Top-level loops that contain a nested loop whose body jumps out of the top loop. Such a
/// loop cannot be expressed with `while`/`break` alone, so its body is emitted as labelled
/// blocks (the rest of the function still structures normally).
fn flat_loop_headers_of(
    loops: &BTreeMap<usize, BTreeSet<usize>>,
    successors: &[Vec<usize>],
) -> BTreeSet<usize> {
    let mut headers = BTreeSet::new();
    for (header, body) in loops {
        let top_level = !loops.iter().any(|(other, outer)| {
            other != header && outer.len() > body.len() && outer.contains(header)
        });
        if !top_level {
            continue;
        }
        let leaves = loops.iter().any(|(inner, inner_body)| {
            inner != header
                && body.contains(inner)
                && inner_body.len() < body.len()
                && inner_body.iter().any(|block| {
                    successors
                        .get(*block)
                        .is_some_and(|nexts| nexts.iter().any(|next| !body.contains(next)))
                })
        });
        if leaves {
            headers.insert(*header);
        }
    }
    headers
}

/// For a block that starts with a conditional branch, the pure flag-setting instruction of
/// its unique predecessor that ends with a conditional branch. A branch does not modify the
/// flags, so the second test compares the same operands; re-lifting `cmp`/`test`/`bt` there
/// expresses the condition in the new block's own arena.
fn block_flag_sources(session: &mut Session, cfg: &Cfg) -> Result<Vec<Option<Address>>> {
    use iced_x86::Mnemonic as M;
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); cfg.blocks.len()];
    for (index, block) in cfg.blocks.iter().enumerate() {
        for &(target, _) in &block.edges {
            if target < cfg.blocks.len() {
                predecessors[target].push(index);
            }
        }
    }
    let mut sources = vec![None; cfg.blocks.len()];
    for (index, block) in cfg.blocks.iter().enumerate() {
        let Some(first) = session.insn_at(block.start)? else {
            continue;
        };
        if first.flow() != recraft_decode::FlowKind::ConditionalBranch {
            continue;
        }
        let Some(&predecessor) = predecessors[index].first() else {
            continue;
        };
        if predecessors[index].len() != 1 {
            continue;
        }
        // Walk the predecessor: the last flag definition must be a pure comparison, with
        // nothing else touching the flags after it.
        let mut addr = cfg.blocks[predecessor].start;
        let mut last: Option<Address> = None;
        let mut clobbered = false;
        while addr < cfg.blocks[predecessor].end {
            let Some(insn) = session.insn_at(addr)? else {
                break;
            };
            let next = insn.next_addr();
            if next <= addr {
                break;
            }
            match insn.mnemonic() {
                M::Cmp | M::Test | M::Bt => {
                    last = Some(addr);
                    clobbered = false;
                }
                M::Add
                | M::Sub
                | M::And
                | M::Or
                | M::Xor
                | M::Neg
                | M::Inc
                | M::Dec
                | M::Shl
                | M::Shr
                | M::Sar
                | M::Rol
                | M::Ror
                | M::Imul
                | M::Mul
                | M::Div
                | M::Idiv => {
                    clobbered = true;
                }
                _ => {}
            }
            addr = next;
        }
        if !clobbered {
            sources[index] = last;
        }
    }
    Ok(sources)
}

/// `rsp` displacement at each block entry, plus the depth of the frame bottom.
///
/// Propagation stops at a block whose stack-pointer updates are not constant steps, and a
/// block reached with two different displacements is left unknown (an alloca or a dynamic
/// stack adjustment); such blocks keep the unnormalised names.
fn entry_stack_deltas(cfg: &Cfg, bodies: &[BlockLift]) -> (Vec<Option<i64>>, i64) {
    let count = cfg.blocks.len();
    let mut entry: Vec<Option<i64>> = vec![None; count];
    let mut conflict = vec![false; count];
    if count > 0 {
        entry[0] = Some(0);
    }
    let mut changed = true;
    while changed {
        changed = false;
        for (index, block) in cfg.blocks.iter().enumerate() {
            let (Some(start), Some(net)) = (entry[index], bodies[index].stack_net) else {
                continue;
            };
            let next = start + net;
            for &(target, _) in &block.edges {
                if target >= count || conflict[target] {
                    continue;
                }
                match entry[target] {
                    None => {
                        entry[target] = Some(next);
                        changed = true;
                    }
                    Some(existing) if existing != next => {
                        conflict[target] = true;
                        changed = true;
                    }
                    _ => {}
                }
            }
        }
    }
    for (index, conflicted) in conflict.iter().enumerate() {
        if *conflicted {
            entry[index] = None;
        }
    }
    let frame_base = cfg
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(index, _)| Some(entry[index]? + bodies[index].stack_min?))
        .min()
        .unwrap_or(0);
    (entry, frame_base)
}

/// Instruction addresses of matched `push`/`pop` pairs. The value a push stores is read
/// only by the matching pop, so both lines are compiler bookkeeping rather than source
/// logic. Pairs are matched per register in LIFO order (the first push is read by the last
/// pop) and only when the counts agree, so an unbalanced use (e.g. `push` as a stack
/// argument) keeps every line visible.
fn hidden_push_pop_pairs<'a>(
    blocks: impl Iterator<Item = (Address, &'a BlockLift)>,
    end: Address,
) -> BTreeSet<u64> {
    let mut pushes: BTreeMap<Reg, Vec<u64>> = BTreeMap::new();
    let mut pops: BTreeMap<Reg, Vec<u64>> = BTreeMap::new();
    for (start, body) in blocks {
        // A tail jump can pull the next function's blocks into the CFG; its `pop`s must not
        // pair with this function's `push`es.
        if start >= end {
            continue;
        }
        for &(addr, reg) in &body.pushes {
            pushes.entry(reg).or_default().push(addr);
        }
        for &(addr, reg) in &body.pops {
            pops.entry(reg).or_default().push(addr);
        }
    }
    let mut hidden = BTreeSet::new();
    for (reg, saved) in &pushes {
        let Some(restored) = pops.get(reg) else {
            continue;
        };
        if saved.is_empty() || saved.len() != restored.len() {
            continue;
        }
        for (push, pop) in saved.iter().zip(restored.iter().rev()) {
            hidden.insert(*push);
            hidden.insert(*pop);
        }
    }
    hidden
}

fn is_callee_saved(reg: &str) -> bool {
    matches!(
        reg,
        "rbx" | "rbp" | "rsi" | "rdi" | "r12" | "r13" | "r14" | "r15"
    )
}

/// Drop callee-saved restores that lead directly to a `return`. `rsi = var_50; return;`
/// only restores the caller's register; it does not contribute to the pseudo-code result
/// (the return value is in `rax`) and hides the fact that `rsi` still held an argument.
fn is_epilogue_restore(text: &str) -> bool {
    let Some((lhs, rhs)) = plain_assignment(text) else {
        return false;
    };
    if !is_callee_saved(lhs) || contains_call(rhs) {
        return false;
    }
    // A callee-saved assignment immediately before `return` cannot affect the returned
    // value (which is in rax) or any later statement; it only restores the caller's
    // register, so any side-effect-free expression qualifies.
    true
}

fn hide_epilogue_restores(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index < lines.len() {
        if !is_epilogue_restore(lines[index].text.trim()) {
            index += 1;
            continue;
        }
        let mut cursor = index + 1;
        let mut terminates = false;
        while cursor < lines.len() {
            let next = lines[cursor].text.trim();
            if next.is_empty() {
                cursor += 1;
                continue;
            }
            if next == "return;" {
                terminates = true;
                break;
            }
            if !is_epilogue_restore(next) {
                break;
            }
            cursor += 1;
        }
        if terminates {
            lines.remove(index);
        } else {
            index += 1;
        }
    }
}

/// Fold the value left in the return register into the `return` statement:
/// `eax = f(); return;` becomes `return f();`, `eax += x; return;` becomes
/// `return (eax + x);`, and a bare `return;` in a value-returning function at least reads
/// `return eax;`. The compiler leaves the result in `eax`/`rax`, so the folded form states
/// what the source returned rather than naming a scratch register after the fact.
///
/// Only a statement directly above the `return` at the same brace depth is folded: there
/// the value is computed unconditionally on the path that returns. Other paths keep their
/// bare `return;` unless the return register is written somewhere in the function, so a
/// `void`-looking tail never invents a value.
fn fold_return_values(lines: &mut Vec<PseudoLine>, ret: ReturnKind) {
    let register = match ret {
        ReturnKind::Void => return,
        ReturnKind::Int32 => "eax",
        ReturnKind::Int64 => "rax",
        // A float result stays in `xmm0`; there is no variable to fold it into, so the
        // statement keeps reading `return;`.
        ReturnKind::F32 | ReturnKind::F64 => return,
    };
    // A value-returning function may compute only into the 32-bit half; both spellings
    // name the same machine register.
    let is_return_register = |name: &str| name == "eax" || name == "rax";
    let writes_return_register = lines.iter().any(|line| {
        let text = line.text.trim();
        plain_assignment(text).is_some_and(|(lhs, _)| is_return_register(lhs))
            || parse_compound(text).is_some_and(|(lhs, _, _)| is_return_register(lhs))
    });
    // (return line, statement to remove, replacement), applied bottom-up so removing a
    // statement does not shift a return that was already handled.
    let mut folds: Vec<(usize, Option<usize>, String)> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.text.trim() != "return;" {
            continue;
        }
        let spaces = leading_spaces(&line.text);
        let replacement = index.checked_sub(1).and_then(|previous| {
            let statement = lines[previous].text.trim();
            // The value is only certain when the assignment is the immediately preceding
            // statement of the same block: a closing brace or a label starts another path.
            let same_block = leading_spaces(&lines[previous].text) == spaces
                && !statement.starts_with('}')
                && !statement.starts_with("loc_")
                && statement.ends_with(';');
            if !same_block {
                return None;
            }
            if let Some((lhs, rhs)) = plain_assignment(statement) {
                return is_return_register(lhs).then(|| (previous, format!("return {rhs};")));
            }
            if let Some((lhs, op, value)) = parse_compound(statement) {
                return is_return_register(lhs)
                    .then(|| (previous, format!("return ({lhs} {op} {value});")));
            }
            // A call as the last statement leaves its result in the return register.
            (statement.ends_with(");") && contains_call(statement) && !statement.contains(" = "))
                .then(|| (previous, format!("return {statement}")))
        });
        let replacement = match replacement {
            Some((previous, text)) => (Some(previous), text),
            None if writes_return_register => (None, format!("return {register};")),
            None => continue,
        };
        folds.push((
            index,
            replacement.0,
            format!("{}{}", " ".repeat(spaces), replacement.1),
        ));
    }
    for (index, previous, text) in folds.into_iter().rev() {
        lines[index].text = text;
        if let Some(previous) = previous {
            lines.remove(previous);
        }
    }
}

/// Last-resort repair for a `goto` whose target block was emitted without a label: insert
/// the label in front of the first statement carrying that address. Emitted lines keep the
/// instruction address, so the jump lands at the right place even when the structurer
/// forgot to label a single-predecessor block (seen on Go and kernel32 samples).
fn materialize_missing_labels(lines: &mut Vec<PseudoLine>) {
    // A jump back to the entry address must label the first statement, not the header comment
    // above the function (the header shares the entry address).
    let body_start = lines
        .iter()
        .position(|line| {
            let text = line.text.trim_end();
            text.ends_with('{') && !text.trim_start().starts_with("//")
        })
        .map(|index| index + 1)
        .unwrap_or(0);
    for _ in 0..64 {
        let labels: BTreeSet<u64> = lines
            .iter()
            .filter_map(|line| {
                let text = line.text.trim();
                text.strip_suffix(':')
                    .and_then(|label| label.strip_prefix("loc_"))
                    .and_then(|hex| u64::from_str_radix(hex, 16).ok())
            })
            .collect();
        let missing: BTreeSet<u64> = lines
            .iter()
            .filter_map(|line| goto_target(&line.text))
            .filter(|target| !labels.contains(target))
            .collect();
        if missing.is_empty() {
            return;
        }
        let mut inserted = false;
        for target in missing {
            let statement_at = |line: &PseudoLine| {
                let text = line.text.trim();
                line.addr.get() == target
                    && !text.is_empty()
                    && text != "}"
                    && !text.starts_with("//")
                    && !(text.starts_with("loc_") && text.ends_with(':'))
            };
            let index = lines
                .iter()
                .enumerate()
                .skip(body_start)
                .find(|(_, line)| statement_at(line))
                .map(|(index, _)| index)
                .or_else(|| {
                    lines
                        .iter()
                        .enumerate()
                        .skip(body_start)
                        .find(|(_, line)| {
                            line.addr.get() > target
                                && !line.text.trim().is_empty()
                                && line.text.trim() != "}"
                        })
                        .map(|(index, _)| index)
                });
            if let Some(index) = index {
                let indent = " ".repeat(leading_spaces(&lines[index].text));
                lines.insert(
                    index,
                    PseudoLine {
                        addr: Address::new(target),
                        text: format!("{indent}loc_{target:x}:"),
                    },
                );
                inserted = true;
            }
        }
        if !inserted {
            return;
        }
    }
}

/// Hide compiler callee-saved spills (`var_XX = rbx` ... `rbx = var_XX`) when the slot
/// is used for nothing else. They preserve the caller's register but add no meaning to
/// pseudo-C. One save may be paired with several restores (every return path restores the
/// register); a slot with any other reference stays visible.
fn hide_callee_saves(lines: &mut Vec<PseudoLine>) {
    let mut saves: BTreeMap<String, (&str, usize)> = BTreeMap::new();
    for (index, line) in lines.iter().enumerate() {
        let text = line.text.trim();
        let Some((lhs, rhs)) = text.strip_suffix(';').and_then(|t| t.split_once(" = ")) else {
            continue;
        };
        if !lhs.starts_with("var_") || !is_register_name(rhs) {
            continue;
        }
        saves.entry(lhs.to_owned()).or_insert((rhs, index));
    }
    if saves.is_empty() {
        return;
    }

    let mut remove = BTreeSet::new();
    // Index of the first branch or call: prologue spills precede it.
    let first_control = lines.iter().position(|line| {
        let text = line.text.trim();
        is_control_line(text) || is_call_line(text)
    });
    // One reverse index instead of a full scan per saved slot: this pass runs over every
    // function and the quadratic form dominated the pipeline on large functions.
    let mut mentions: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, line) in lines.iter().enumerate() {
        for token in line
            .text
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .filter(|token| is_stack_slot_name(token))
        {
            mentions.entry(token).or_default().push(index);
        }
    }
    for (slot, (reg, save_index)) in &saves {
        let references: Vec<usize> = mentions.get(slot.as_str()).cloned().unwrap_or_default();
        if references.len() > 8 {
            continue;
        }
        if references.len() == 1 {
            // A lone store in the prologue is a spill whose restore never made it into the
            // listing. After the first branch or call it may instead be an outgoing stack
            // argument that the call reads, so it is kept.
            if first_control.is_none_or(|control| *save_index < control) {
                remove.insert(*save_index);
            }
            continue;
        }
        let expected_restore = format!("{reg} = {slot};");
        let mut restores = Vec::new();
        for &index in &references {
            if index == *save_index {
                continue;
            }
            if index < *save_index || lines[index].text.trim() != expected_restore {
                restores.clear();
                break;
            }
            restores.push(index);
        }
        if restores.is_empty() {
            continue;
        }
        remove.insert(*save_index);
        remove.extend(restores);
    }
    if remove.is_empty() {
        return;
    }
    let mut index = 0;
    lines.retain(|_| {
        let keep = !remove.contains(&index);
        index += 1;
        keep
    });
}

/// MSVC's `__chkstk` probe touches the guard pages of a frame larger than one page: the
/// frame size goes into `eax` and the helper is called before the frame is set up. It has no
/// source-level effect, so the call and its size load are compiler bookkeeping.
fn hide_stack_probes(session: &mut Session, lines: &mut Vec<PseudoLine>) -> Result<()> {
    if lines.len() < 2 {
        return Ok(());
    }
    let mut probes: BTreeMap<u64, bool> = BTreeMap::new();
    let mut drop = vec![false; lines.len()];
    for index in 0..lines.len() {
        let text = lines[index].text.trim();
        let Some(target) = text
            .strip_suffix("();")
            .and_then(|call| call.strip_prefix("sub_"))
            .and_then(|hex| u64::from_str_radix(hex, 16).ok())
        else {
            continue;
        };
        let probe = match probes.get(&target) {
            Some(known) => *known,
            None => {
                let known = crate::is_stack_probe(session, Address::new(target))?;
                probes.insert(target, known);
                known
            }
        };
        if !probe {
            continue;
        }
        drop[index] = true;
        if index > 0 {
            let previous = lines[index - 1].text.trim();
            let size_load = previous
                .strip_suffix(';')
                .and_then(|statement| statement.split_once(" = "))
                .is_some_and(|(lhs, rhs)| (lhs == "eax" || lhs == "rax") && rhs.starts_with("0x"));
            if size_load {
                drop[index - 1] = true;
            }
        }
    }
    if drop.iter().any(|dropped| *dropped) {
        let mut kept = Vec::with_capacity(lines.len());
        for (index, line) in lines.drain(..).enumerate() {
            if !drop[index] {
                kept.push(line);
            }
        }
        *lines = kept;
    }
    Ok(())
}

/// True when the value is a frame slot holding the cookie. The printer names it (`var_XX`)
/// when the block knows the frame alias; without that (L1, where no register seeds are
/// used) the check keeps the raw stack expression (`*(u64*)((rbp + 0x200))`), so both forms
/// identify the check.
fn is_cookie_slot(value: &str) -> bool {
    is_stack_slot_name(value)
        || (value.starts_with("*(u64*)(")
            && (reads_register(value, "rbp") || reads_register(value, "rsp")))
}

/// MSVC's `/GS` stack cookie is compiler bookkeeping, not program logic: the prologue stores
/// `__security_cookie ^ rsp` in a frame slot and every return path folds it back and calls
/// `__security_check_cookie`. Source code has neither, and printing the check as the result
/// (`return sub_1400019f0(rcx);`) hides the value the function really returns. The prologue
/// stores a global mixed with the stack pointer, and only a function with such a setup has
/// its `frame-slot ^ rsp` calls removed, so an unrelated tail call cannot match. The setup
/// goes too when nothing reads its slot any more.
fn hide_security_cookies(lines: &mut Vec<PseudoLine>) {
    if lines.len() < 4 {
        return;
    }
    // Setup: `rax = *(u64*)(0x...); rax = (rax ^ rsp); var_XX = rax;`
    let mut cookies: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for index in 0..lines.len().saturating_sub(2) {
        let Some((register, load)) = plain_assignment(&lines[index].text) else {
            continue;
        };
        if !is_identifier(register) || !load.starts_with("*(u64*)(0x") || !load.ends_with(')') {
            continue;
        }
        if !folds_stack_pointer(register, &lines[index + 1].text) {
            continue;
        }
        let Some((slot, source)) = plain_assignment(&lines[index + 2].text) else {
            continue;
        };
        if source != register || !is_stack_slot_name(slot) {
            continue;
        }
        cookies
            .entry(slot.to_owned())
            .or_insert((index, register.to_owned()));
    }
    if cookies.is_empty() {
        return;
    }

    // Check: `rcx = var_XX; rcx = (rcx ^ rsp); sub_...(rcx); return;` before the return call
    // is folded, or the folded `return sub_...(rcx);` afterwards.
    let mut drop = vec![false; lines.len()];
    let mut index = 0;
    while index + 2 < lines.len() {
        let Some((register, slot)) = plain_assignment(&lines[index].text) else {
            index += 1;
            continue;
        };
        // The slot names of the store and the check can disagree when the frame base is
        // recovered differently on the two paths, so the `^ rsp` fold and the call shape
        // identify the check, not the slot name.
        if !is_cookie_slot(slot) || !folds_stack_pointer(register, &lines[index + 1].text) {
            index += 1;
            continue;
        }
        let third = lines[index + 2].text.trim().to_owned();
        let spaces = leading_spaces(&lines[index + 2].text);
        if let Some(call) = third
            .strip_prefix("return ")
            .and_then(|t| t.strip_suffix(';'))
        {
            if calls_cookie_check(call, register) {
                drop[index] = true;
                drop[index + 1] = true;
                lines[index + 2].text = format!("{}return;", " ".repeat(spaces));
                index += 3;
                continue;
            }
        }
        if let Some(call) = third.strip_suffix(';')
            && calls_cookie_check(call, register)
        {
            // The check is a statement call with no result of its own; frame restores or a
            // bare `return;` may follow it.
            drop[index] = true;
            drop[index + 1] = true;
            drop[index + 2] = true;
            index += 3;
            continue;
        }
        index += 1;
    }

    // A cookie slot that no check reads any more only carried frame canaries (or the check
    // path is not expressible in pseudo-C). Its setup is dead once the register it used is
    // rewritten before being read.
    for (slot, (setup, register)) in &cookies {
        let read_elsewhere = lines.iter().enumerate().any(|(index, line)| {
            !drop[index]
                && index != *setup
                && index != setup + 1
                && index != setup + 2
                && mentions_slot(&line.text, slot)
        });
        if read_elsewhere {
            continue;
        }
        // Walk forward: a write of the same register first makes the cookie value dead, a
        // read keeps it alive, and for `rax` a call (which clobbers it) ends the value too.
        let mut dead = true;
        for (index, line) in lines.iter().enumerate().skip(setup + 3) {
            if drop[index] {
                continue;
            }
            let text = line.text.trim();
            if text.is_empty() {
                continue;
            }
            if reads_register(text, register) {
                dead = plain_assignment(text)
                    .is_some_and(|(lhs, _)| register_base_any(lhs) == register_base_any(register));
                break;
            }
            if register_base_any(register) == Some(0) && is_call_line(text) {
                dead = true;
                break;
            }
        }
        if dead {
            drop[*setup] = true;
            drop[*setup + 1] = true;
            drop[*setup + 2] = true;
        }
    }
    if drop.iter().any(|gone| *gone) {
        let mut index = 0;
        lines.retain(|_| {
            let keep = !drop[index];
            index += 1;
            keep
        });
    }
}

/// True when the statement mixes `register` with the stack pointer, in either the raw
/// (`rax = (rax ^ rsp);`) or the folded (`rax ^= rsp;`) spelling.
fn folds_stack_pointer(register: &str, text: &str) -> bool {
    let mixes = |base: &str| base == "rsp" || base == "esp";
    if let Some((folded, op, base)) = parse_compound(text) {
        return folded == register && op == "^" && mixes(base);
    }
    plain_assignment(text).is_some_and(|(target, value)| {
        target == register
            && ["rsp", "esp"]
                .iter()
                .any(|base| value == format!("({register} ^ {base})"))
    })
}

/// True for `sub_...(rcx)` / `Api(rcx)` where the callee receives only the cookie value.
fn calls_cookie_check(call: &str, register: &str) -> bool {
    let Some(open) = call.find('(') else {
        return false;
    };
    call.ends_with(')')
        && is_identifier(&call[..open])
        && &call[open + 1..call.len() - 1] == register
}

/// x64 argument slot of a register spelling (`rcx`/`ecx`/`cl` -> 0, `r8`/`r8d` -> 2).
fn abi_argument_slot(name: &str) -> Option<usize> {
    const ARGUMENTS: [&str; 4] = ["rcx", "rdx", "r8", "r9"];
    let base = register_base_any(name)?;
    ARGUMENTS
        .iter()
        .position(|argument| register_base_any(argument) == Some(base))
}

/// Blocks are split at jump targets, so a call can sit in its own block while its arguments
/// were set up in the block that falls through into it; the printer then renders the call
/// without arguments. Recover them from the straight-line run above the call, stopping at
/// anything that ends the run (a label, branch, brace or another call, whose clobbering of
/// the argument registers would make the value unknown).
///
/// A callee with a known positive argument count is filled from the whole run. Otherwise only
/// a direct `sub_...` call whose first argument was written on the immediately preceding line
/// is filled: a stray register value further up belongs to whatever produced it, not to an
/// argument-less call (`GetProcessHeap()`), and an import's arity is unknown without a
/// prototype.
fn fill_split_call_arguments(lines: &mut [PseudoLine], callers_with_arguments: &BTreeSet<String>) {
    let mut values: [Option<String>; 4] = [None, None, None, None];
    // The previous line defines the first argument register (the setup/call block split).
    let mut first_argument_written_just_now = false;
    for line in lines.iter_mut() {
        let spaces = leading_spaces(&line.text);
        let text = line.text.trim();
        if text.is_empty() {
            continue;
        }
        let control = text.starts_with('{')
            || text.starts_with('}')
            || (text.starts_with("loc_") && text.ends_with(':'))
            || text.starts_with("goto ")
            || text.starts_with("return")
            || text.starts_with("if ")
            || text.starts_with("else")
            || text.starts_with("while ")
            || text.starts_with("do ")
            || text.starts_with("switch ")
            || text.starts_with("for ")
            || text.starts_with("break")
            || text.starts_with("continue")
            || text.starts_with("__asm");
        if control || is_call_line(text) {
            if is_call_line(text) {
                let head = text[..text.len() - 3].trim_end();
                let known = callers_with_arguments.contains(head);
                let adjacent = head.starts_with("sub_") && first_argument_written_just_now;
                if text.ends_with("();") && values[0].is_some() && (known || adjacent) {
                    let Some(last) = values.iter().rposition(Option::is_some) else {
                        unreachable!("the first slot is known")
                    };
                    let arguments = (0..=last)
                        .map(|slot| match &values[slot] {
                            Some(value) => value.clone(),
                            None => ["rcx", "rdx", "r8", "r9"][slot].to_owned(),
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    line.text = format!("{}{head}({arguments});", " ".repeat(spaces));
                }
            }
            values = [None, None, None, None];
            first_argument_written_just_now = false;
            continue;
        }
        let mut wrote_argument = false;
        if let Some((lhs, rhs)) = plain_assignment(text)
            && let Some(slot) = abi_argument_slot(lhs)
        {
            // A sub-register write does not define the whole argument; keep the register
            // name so the width stays visible. A value that reads its own register was
            // computed from the previous value, so the register already holds the result.
            let full = matches!(
                lhs,
                "rcx" | "rdx" | "r8" | "r9" | "ecx" | "edx" | "r8d" | "r9d"
            );
            values[slot] = Some(if full && !reads_register(rhs, lhs) {
                rhs.to_owned()
            } else {
                lhs.to_owned()
            });
            wrote_argument = slot == 0;
        } else if let Some((lhs, _, _)) = parse_compound(text)
            && let Some(slot) = abi_argument_slot(lhs)
        {
            values[slot] = Some(lhs.to_owned());
            wrote_argument = slot == 0;
        }
        first_argument_written_just_now = wrote_argument;
    }
}

/// A call statement (`Api(...);`, `sub_x(...);`, `(*rax)(...);`), as opposed to a
/// declaration or a branch.
fn is_call_line(text: &str) -> bool {
    let text = text.trim();
    if !text.ends_with(");") || text.starts_with("return ") {
        return false;
    }
    text.starts_with("(*")
        || text
            .split('(')
            .next()
            .is_some_and(|head| is_identifier(head.trim()))
}

/// True for any general-purpose register spelling (`rax`/`eax`/`ax`/`al`, `r8d`, ...).
fn is_machine_register(text: &str) -> bool {
    const NAMED: [&str; 39] = [
        "rax", "eax", "ax", "al", "ah", "rbx", "ebx", "bx", "bl", "bh", "rcx", "ecx", "cx", "cl",
        "ch", "rdx", "edx", "dx", "dl", "dh", "rsi", "esi", "si", "sil", "rdi", "edi", "di", "dil",
        "rsp", "esp", "sp", "spl", "rbp", "ebp", "bp", "bpl", "rip", "eip", "ip",
    ];
    if NAMED.contains(&text) {
        return true;
    }
    let Some(rest) = text.strip_prefix('r') else {
        return false;
    };
    matches!(
        rest.trim_end_matches(['d', 'w', 'b']).parse::<u8>(),
        Ok(8..=15)
    )
}

/// True when the lines are only a jump-table dispatch: arithmetic on machine registers
/// followed by the indirect jump itself.
fn is_dispatch_only(lines: &[PseudoLine]) -> bool {
    let Some((last, setup)) = lines.split_last() else {
        return false;
    };
    if !last.text.trim().starts_with("__asm(\"jmp") {
        return false;
    }
    setup.iter().all(|line| {
        let text = line.text.trim();
        plain_assignment(text)
            .map(|(lhs, _)| lhs)
            .or_else(|| parse_compound(text).map(|(lhs, _, _)| lhs))
            .is_some_and(is_machine_register)
    })
}

fn is_register_name(text: &str) -> bool {
    matches!(
        text,
        "rax"
            | "rbx"
            | "rcx"
            | "rdx"
            | "rsi"
            | "rdi"
            | "rbp"
            | "rsp"
            | "r8"
            | "r9"
            | "r10"
            | "r11"
            | "r12"
            | "r13"
            | "r14"
            | "r15"
    )
}

fn mentions_slot(text: &str, slot: &str) -> bool {
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|token| token == slot)
}

/// A stack-slot name as the printer emits it (`var_20`, `var_n48`, `frame_10`).
fn is_stack_slot_name(text: &str) -> bool {
    ["var_", "frame_"].iter().any(|prefix| {
        text.strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty())
    })
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

/// Drop straight-line code emitted after an unconditional `return;` inside the same
/// block. Structuring can leave an alternate-entry tail after the common return block
/// (`return; al = 1; goto epilogue;`); without this pass those dead lines also count as a
/// goto fallback and hide the recovered structure.
fn drop_unreachable_tails(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index < lines.len() {
        if lines[index].text.trim() != "return;" {
            index += 1;
            continue;
        }
        let indent = leading_spaces(&lines[index].text);
        let mut end = index + 1;
        let mut depth = 0usize;
        while end < lines.len() {
            let text = lines[end].text.trim();
            let is_label = text.starts_with("loc_") && text.ends_with(':');
            if depth == 0 {
                if is_label {
                    // A forward `goto` may appear after the label; removing the label
                    // would leave that jump dangling. Only drop labels that no live jump
                    // names.
                    let label = text.trim_end_matches(':');
                    let referenced = lines
                        .iter()
                        .any(|line| line.text.contains(&format!("goto {label}")));
                    if !referenced {
                        // An orphan label after `return` cannot be reached; drop it and
                        // keep scanning the dead tail behind it.
                        lines.remove(end);
                        continue;
                    }
                }
                if text.starts_with('}') || leading_spaces(&lines[end].text) < indent || is_label {
                    break;
                }
            }
            // A nested `if { ... }` in the dead tail is removed as a whole; stopping at
            // its closing brace would leave a stray `}` behind.
            if text.starts_with('}') {
                depth = depth.saturating_sub(1);
            }
            depth += text.matches('{').count();
            depth = depth.saturating_sub(
                text.matches('}')
                    .count()
                    .saturating_sub(usize::from(text.starts_with('}'))),
            );
            end += 1;
        }
        if end > index + 1 {
            // A nested label inside the region may be the target of a live jump from
            // outside it; then the region is reachable through that jump and must stay.
            let labels: BTreeSet<&str> = lines[index + 1..end]
                .iter()
                .filter_map(|line| {
                    let text = line.text.trim();
                    text.strip_suffix(':')
                        .filter(|label| label.starts_with("loc_"))
                })
                .collect();
            let referenced_outside = lines.iter().enumerate().any(|(line_index, line)| {
                if (index + 1..end).contains(&line_index) {
                    return false;
                }
                goto_target(&line.text)
                    .is_some_and(|target| labels.contains(format!("loc_{target:x}").as_str()))
            });
            if !referenced_outside {
                lines.drain(index + 1..end);
            }
        }
        index += 1;
    }
}

fn leading_spaces(text: &str) -> usize {
    text.len() - text.trim_start().len()
}

/// A jump at the end of a region whose target label follows only closing braces is a
/// fallthrough: both the jump and the normal `}` path reach the same label. Drop it.
fn fold_jumps_over_braces(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index < lines.len() {
        let Some(target) = goto_target(&lines[index].text) else {
            index += 1;
            continue;
        };
        let mut next = index + 1;
        while next < lines.len() && lines[next].text.trim() == "}" {
            next += 1;
        }
        if next < lines.len() && lines[next].text.trim() == format!("loc_{target:x}:") {
            lines.remove(index);
            continue;
        }
        index += 1;
    }
}

/// Longest skipped region folded into a guarded block. Longer skips read better as a jump:
/// wrapping hundreds of lines would indent the rest of the function past the screen.
const MAX_FORWARD_SKIP: usize = 64;

/// Parse `if (cond) goto loc_X;` into the condition and the target address.
fn conditional_goto(text: &str) -> Option<(&str, u64)> {
    let (condition, rest) = text
        .trim()
        .strip_prefix("if (")?
        .split_once(") goto loc_")?;
    let target = rest
        .strip_suffix(';')
        .and_then(|hex| u64::from_str_radix(hex, 16).ok())?;
    Some((condition, target))
}

/// `if (cond) goto L;` skips the straight-line region up to `L`. When that region is short
/// and brace-balanced, guard it with `if (<not cond>) { ... }` instead: both forms execute
/// the region exactly when the condition is false, and the braces make the skipped code
/// visible where it belongs. Labels inside the region stay valid (C labels are function
/// scoped), so jumps into it keep working.
///
/// Candidates are collected once and applied from the bottom up, so the label lookup stays
/// linear even on the ten-thousand-line functions the flat fallback produces.
fn fold_short_forward_gotos(lines: &mut Vec<PseudoLine>) -> usize {
    let mut labels: HashMap<u64, usize> = HashMap::new();
    for (index, line) in lines.iter().enumerate() {
        if let Some(address) = line
            .text
            .trim()
            .strip_suffix(':')
            .and_then(|label| label.strip_prefix("loc_"))
            .and_then(|hex| u64::from_str_radix(hex, 16).ok())
        {
            labels.insert(address, index);
        }
    }
    let mut candidates: Vec<(usize, usize, String)> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some((condition, target)) = conditional_goto(&line.text) else {
            continue;
        };
        let Some(&target_index) = labels.get(&target) else {
            continue;
        };
        if target_index <= index + 1 || target_index - index - 1 > MAX_FORWARD_SKIP {
            continue;
        }
        candidates.push((index, target_index, negate(condition)));
    }
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.0));
    let mut insertions: Vec<usize> = Vec::new();
    let mut folded = 0usize;
    for (goto, target, condition) in candidates {
        // A fold applied below already shifted every line after its closing brace.
        let mut target = target;
        for &position in &insertions {
            if position <= target {
                target += 1;
            }
        }
        if target <= goto + 1 {
            continue;
        }
        let indent = leading_spaces(&lines[goto].text);
        if leading_spaces(&lines[target].text) != indent {
            continue;
        }
        // The skipped region must balance its own braces and never close the enclosing one.
        let mut depth = 0isize;
        for line in &lines[goto + 1..target] {
            depth += line.text.matches('{').count() as isize;
            depth -= line.text.matches('}').count() as isize;
            if depth < 0 {
                break;
            }
        }
        if depth != 0 {
            continue;
        }
        lines[goto].text = format!("{}if ({}) {{", " ".repeat(indent), condition);
        for line in &mut lines[goto + 1..target] {
            line.text.insert_str(0, "    ");
        }
        let addr = lines[target].addr;
        lines.insert(
            target,
            PseudoLine {
                addr,
                text: format!("{}}}", " ".repeat(indent)),
            },
        );
        insertions.push(target);
        folded += 1;
    }
    folded
}

/// A label whose region ends by jumping back to itself is a loop. Converting the jump into
/// `do { ... } while (cond);` shows the repetition instead of the label chase; the body is
/// executed first either way. The region must be brace-balanced, the back-edge last, and
/// free of `break`/`continue`, whose binding to the enclosing loop would change when a new
/// loop is introduced.
fn fold_backward_gotos_into_loops(lines: &mut Vec<PseudoLine>) -> usize {
    // One pass collects the labels and the last jump to each of them.
    let mut labels: HashMap<u64, usize> = HashMap::new();
    let mut jumps: HashMap<u64, (usize, Option<String>)> = HashMap::new();
    for (index, line) in lines.iter().enumerate() {
        let text = line.text.trim();
        if let Some(address) = text
            .strip_suffix(':')
            .and_then(|label| label.strip_prefix("loc_"))
            .and_then(|hex| u64::from_str_radix(hex, 16).ok())
        {
            labels.insert(address, index);
        }
        if let Some((condition, target)) = conditional_goto(text) {
            jumps.insert(target, (index, Some(condition.to_owned())));
        } else if let Some(target) = goto_target(text) {
            jumps.insert(target, (index, None));
        }
    }
    let mut candidates: Vec<(usize, usize, Option<String>)> = Vec::new();
    for (address, label) in &labels {
        if let Some((latch, condition)) = jumps.get(address)
            && *latch > *label + 1
        {
            candidates.push((*label, *latch, condition.clone()));
        }
    }
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.0));
    let mut insertions: Vec<usize> = Vec::new();
    let mut folded = 0usize;
    for (label, latch, condition) in candidates {
        let mut latch = latch;
        for &position in &insertions {
            if position <= latch {
                latch += 1;
            }
        }
        if latch <= label + 1 {
            continue;
        }
        let indent = leading_spaces(&lines[label].text);
        if leading_spaces(&lines[latch].text) != indent {
            continue;
        }
        let mut depth = 0isize;
        let mut usable = true;
        for line in &lines[label + 1..latch] {
            let text = line.text.trim();
            // `break`/`continue` would bind to the loop introduced below instead of their
            // original target, so a region using them keeps its labels.
            if text.ends_with("break;") || text.ends_with("continue;") {
                usable = false;
                break;
            }
            depth += text.matches('{').count() as isize;
            depth -= text.matches('}').count() as isize;
            if depth < 0 {
                usable = false;
                break;
            }
        }
        if !usable || depth != 0 {
            continue;
        }
        let close = match condition {
            Some(condition) => format!("{}}} while ({condition});", " ".repeat(indent)),
            None => format!("{}}} while (1);", " ".repeat(indent)),
        };
        lines[latch].text = close;
        for line in &mut lines[label + 1..latch] {
            line.text.insert_str(0, "    ");
        }
        let addr = lines[label].addr;
        lines.insert(
            label + 1,
            PseudoLine {
                addr,
                text: format!("{}do {{", " ".repeat(indent)),
            },
        );
        insertions.push(label + 1);
        folded += 1;
    }
    folded
}

/// Fold an unconditional jump at the end of one sibling arm to the short straight-line
/// tail of the other arm. Both the jump and the normal `}` fallthrough leave the parent
/// at the same point, so the tail can be copied and the jump removed.
fn fold_sibling_tail_gotos(lines: &mut Vec<PseudoLine>) {
    // Label positions once: looking the target up by scanning made this quadratic.
    let mut labels: BTreeMap<String, usize> = BTreeMap::new();
    for (index, line) in lines.iter().enumerate() {
        let text = line.text.trim();
        if text.ends_with(':') && text.starts_with("loc_") {
            labels.insert(text.to_owned(), index);
        }
    }
    let mut index = 0;
    while index < lines.len() {
        let text = lines[index].text.trim();
        let conditional = text
            .split_once(") goto loc_")
            .map(|(condition, _)| condition.strip_prefix("if (").unwrap_or(condition));
        if !text.starts_with("goto loc_") && conditional.is_none() {
            index += 1;
            continue;
        }
        let Some(target) = goto_target(text) else {
            index += 1;
            continue;
        };
        let goto_indent = leading_spaces(&lines[index].text);
        let mut after = index + 1;
        while after < lines.len() && lines[after].text.trim().is_empty() {
            after += 1;
        }
        if after >= lines.len()
            || leading_spaces(&lines[after].text) >= goto_indent
            || !lines[after].text.trim().starts_with('}')
        {
            index += 1;
            continue;
        }
        let close_indent = leading_spaces(&lines[after].text);
        let label = format!("loc_{target:x}:");
        let Some(&label_index) = labels.get(&label) else {
            index += 1;
            continue;
        };
        let label_indent = leading_spaces(&lines[label_index].text);
        if label_indent <= goto_indent {
            index += 1;
            continue;
        }
        let mut tail = Vec::new();
        let mut cursor = label_index + 1;
        let mut valid = false;
        while cursor < lines.len() {
            let text = lines[cursor].text.trim();
            if text.is_empty() {
                cursor += 1;
                continue;
            }
            if leading_spaces(&lines[cursor].text) < label_indent {
                valid = lines[cursor].text.trim().starts_with('}')
                    && leading_spaces(&lines[cursor].text) >= close_indent;
                break;
            }
            if text.starts_with('}')
                || text.starts_with("if (")
                || text.starts_with("while")
                || text.starts_with("for ")
                || text.starts_with("switch")
                || text.starts_with("goto loc_")
                || text.starts_with("loc_")
            {
                break;
            }
            tail.push(text.to_owned());
            cursor += 1;
        }
        if !valid || tail.is_empty() || tail.len() > 8 {
            index += 1;
            continue;
        }
        let indent = " ".repeat(goto_indent);
        let mut replacement = Vec::new();
        if let Some(condition) = conditional {
            replacement.push(PseudoLine {
                addr: lines[index].addr,
                text: format!("{indent}if ({condition}) {{"),
            });
            for body in &tail {
                replacement.push(PseudoLine {
                    addr: lines[index].addr,
                    text: format!("{indent}    {body}"),
                });
            }
            replacement.push(PseudoLine {
                addr: lines[index].addr,
                text: format!("{indent}}}"),
            });
        } else {
            for body in &tail {
                replacement.push(PseudoLine {
                    addr: lines[index].addr,
                    text: format!("{indent}{body}"),
                });
            }
        }
        let count = replacement.len();
        lines.splice(index..=index, replacement);
        // Splices are rare, so shifting the index is cheaper than rescanning.
        let delta = count as isize - 1;
        if delta != 0 {
            for value in labels.values_mut() {
                if *value > index {
                    *value = (*value as isize + delta) as usize;
                }
            }
        }
        index += count;
    }
}

/// Replace jumps to a label whose straight-line tail ends in `return;` with a copy of
/// that tail. Unlike the last-label pass this works for any label and needs no epilogue:
/// the return is part of the copied block.
fn fold_return_tail_labels(lines: &mut Vec<PseudoLine>) {
    let mut tails = BTreeMap::new();
    for (index, line) in lines.iter().enumerate() {
        let text = line.text.trim();
        let Some(label) = text.strip_suffix(':') else {
            continue;
        };
        if !label.starts_with("loc_") {
            continue;
        }
        let mut tail = Vec::new();
        let mut depth = 0usize;
        for line in &lines[index + 1..] {
            let text = line.text.trim();
            if text.is_empty() {
                continue;
            }
            if depth == 0 && text.starts_with('}') {
                break;
            }
            if text.starts_with("while")
                || text.starts_with("for ")
                || text.starts_with("switch")
                || text.starts_with("goto loc_")
            {
                tail.clear();
                break;
            }
            // An inner label only names a block inside the shared tail; the copied body
            // does not need it and the original label still serves jumps into its block.
            if text.starts_with("loc_") && text.ends_with(':') {
                continue;
            }
            if text.starts_with('}') {
                depth = depth.saturating_sub(1);
            }
            depth += text.matches('{').count();
            depth = depth.saturating_sub(
                text.matches('}')
                    .count()
                    .saturating_sub(usize::from(text.starts_with('}'))),
            );
            tail.push(text.to_owned());
            if depth == 0 && tail.len() > 48 {
                break;
            }
        }
        if tail.len() <= 48
            && depth == 0
            && tail
                .last()
                .is_some_and(|line| line.starts_with("return ") || line == "return;")
        {
            tails.insert(label.to_owned(), tail);
        }
    }
    if tails.is_empty() {
        return;
    }
    let mut output = Vec::with_capacity(lines.len());
    for line in lines.drain(..) {
        let text = line.text.trim();
        let Some(target) = goto_target(&line.text) else {
            output.push(line);
            continue;
        };
        let Some(tail) = tails.get(&format!("loc_{target:x}")) else {
            output.push(line);
            continue;
        };
        let indent = " ".repeat(leading_spaces(&line.text));
        let conditional = text
            .split_once(") goto loc_")
            .map(|(condition, _)| condition.strip_prefix("if (").unwrap_or(condition));
        if let Some(condition) = conditional {
            output.push(PseudoLine {
                addr: line.addr,
                text: format!("{indent}if ({condition}) {{"),
            });
            append_indented_tail(
                &mut output,
                line.addr,
                &format!("{indent}    "),
                tail.as_slice(),
            );
            output.push(PseudoLine {
                addr: line.addr,
                text: format!("{indent}}}"),
            });
        } else {
            append_indented_tail(&mut output, line.addr, &indent, tail.as_slice());
        }
    }
    *lines = output;
}

/// Fold `x = (x + y);` into `x += y;`. Compiler-generated pseudo-code spells every
/// arithmetic update as a load and a separate write of the destination register; the
/// compound form matches the original source and removes one temporary write per update.
// `rax = &var_38; rax += 8;` is one step in the source: fold an accumulation into the
/// assignment that produced its input. Only adjacent statements about the same target qualify,
/// the right-hand side must be a constant and the input must be side-effect free, so no call is
/// moved across the merge. A cast to a narrower type is left alone: the register add is 64-bit
/// and folding it under the cast would change where the value wraps.
fn merge_accumulations(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index + 1 < lines.len() {
        let first = lines[index].text.trim().to_owned();
        let second = lines[index + 1].text.trim().to_owned();
        let (Some((target, value)), Some((added_to, operator, amount))) =
            (plain_assignment(&first), accumulation_step(&second))
        else {
            index += 1;
            continue;
        };
        if target != added_to || is_cast_to_narrow(value) || has_call(value) {
            index += 1;
            continue;
        }
        let indent = " ".repeat(leading_spaces(&lines[index].text));
        lines[index].text = format!("{indent}{target} = ({value} {operator} {amount});");
        lines.remove(index + 1);
        // The merged statement may now be the input of the next step.
    }
}

/// `x += 8;` -> (`x`, `+`, `8`). Only a constant amount folds.
fn accumulation_step(statement: &str) -> Option<(&str, &'static str, &str)> {
    let body = statement.strip_suffix(';')?;
    for (written, operator) in [
        (" += ", "+"),
        (" -= ", "-"),
        (" &= ", "&"),
        (" |= ", "|"),
        (" ^= ", "^"),
        (" <<= ", "<<"),
        (" >>= ", ">>"),
    ] {
        if let Some((left, right)) = body.split_once(written) {
            let right = right.trim();
            if !is_constant(right) || left.trim().is_empty() {
                return None;
            }
            return Some((left.trim(), operator, right));
        }
    }
    None
}

// Whether the expression calls a function. A cast (`(int64)x`) is not a call, so the type
/// keywords are skipped when the identifier before `(` is inspected.
fn has_call(expression: &str) -> bool {
    let bytes = expression.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'(' || index == 0 {
            continue;
        }
        let name: String = expression[..index]
            .chars()
            .rev()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if !name.is_empty() && !is_type_name(&name) {
            return true;
        }
    }
    false
}

/// Whether `name` names a type, so `(u32)x` is a cast.
fn is_type_name(name: &str) -> bool {
    matches!(
        name,
        "u8" | "u16"
            | "u32"
            | "u64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "char"
            | "short"
            | "int"
            | "long"
            | "wchar_t"
            | "float"
            | "double"
            | "void"
            | "unsigned"
            | "signed"
            | "bool"
            | "size_t"
            | "sizeof"
    )
}
/// Whether the expression starts with a cast to a narrower integer type. Arithmetic under such
/// a cast happens in the narrow type, so a following 64-bit add cannot be folded underneath it.
fn is_cast_to_narrow(expression: &str) -> bool {
    let Some(rest) = expression.trim_start().strip_prefix('(') else {
        return false;
    };
    let Some((kind, _)) = rest.split_once(')') else {
        return false;
    };
    matches!(
        kind.trim(),
        "u8" | "u16"
            | "u32"
            | "i8"
            | "i16"
            | "i32"
            | "char"
            | "short"
            | "int8"
            | "int16"
            | "int32"
            | "uint8"
            | "uint16"
            | "uint32"
    )
}

fn fold_compound_assignments(lines: &mut [PseudoLine]) {
    for line in lines.iter_mut() {
        let Some(statement) = line.text.trim().strip_suffix(';') else {
            continue;
        };
        let Some((lhs, rhs)) = statement.split_once(" = ") else {
            continue;
        };
        let lhs = lhs.trim();
        if !is_identifier(lhs) {
            continue;
        }
        let rhs = rhs.trim();
        let Some(inner) = rhs
            .strip_prefix('(')
            .and_then(|text| text.strip_suffix(')'))
        else {
            continue;
        };
        let Some(rest) = inner.strip_prefix(lhs) else {
            continue;
        };
        if !rest.starts_with(' ') {
            continue;
        }
        let rest = rest.trim_start();
        let Some(operator) =
            ["<<", ">>", "+", "-", "*", "&", "|", "^"]
                .into_iter()
                .find(|operator| {
                    rest.strip_prefix(operator)
                        .is_some_and(|value| value.starts_with(' '))
                })
        else {
            continue;
        };
        let mut value = rest[operator.len()..].trim();
        if value.is_empty() {
            continue;
        }
        // `x = (x + -0x30)` -> `x -= 0x30`; `x |= -1` sets every bit, i.e. assigns -1.
        let mut operator = operator;
        if let Some(stripped) = value.strip_prefix('-') {
            match operator {
                "+" => {
                    operator = "-";
                    value = stripped;
                }
                "-" => {
                    operator = "+";
                    value = stripped;
                }
                _ => {}
            }
        }
        let indent = " ".repeat(leading_spaces(&line.text));
        if operator == "|" && value == "-1" {
            line.text = format!("{indent}{lhs} = -1;");
            continue;
        }
        line.text = format!("{indent}{lhs} {operator}= {value};");
    }
}

/// Rewrite `if (!(x == 0)) {` as `if ((x != 0)) {`. A negated comparison is harder to
/// read than its flipped form, and the structurer produces many of them when it inverts a
/// branch to follow the fallthrough path. Conditions joined by `&&`/`||` are left alone
/// because flipping their operators would not be equivalent.
fn simplify_negated_conditions(lines: &mut [PseudoLine]) {
    for line in lines.iter_mut() {
        let text = line.text.trim();
        let (keyword, indent) = if text.starts_with("if (") {
            ("if", leading_spaces(&line.text))
        } else if text.starts_with("while (") {
            ("while", leading_spaces(&line.text))
        } else if text.starts_with("} else if (") {
            ("} else if", leading_spaces(&line.text))
        } else {
            continue;
        };
        let rest = &text[text.find('(').unwrap_or(0) + 1..];
        let Some(condition) = rest.strip_suffix(") {") else {
            continue;
        };
        let Some(inner) = condition
            .strip_prefix("!(")
            .and_then(|inner| inner.strip_suffix(')'))
        else {
            continue;
        };
        if inner.contains("&&")
            || inner.contains("||")
            || inner.contains('?')
            || inner.contains("!(")
        {
            continue;
        }
        let negated = negate(inner);
        if negated == format!("!({inner})") {
            continue;
        }
        line.text = format!("{}{keyword} (({negated})) {{", " ".repeat(indent));
    }
}

/// Drop the extra parentheses the generator wraps around a whole condition:
/// `if ((x == 0))` becomes `if (x == 0)`. Only a wrapper that spans the entire condition
/// is removed, so operator precedence inside the condition is untouched.
fn strip_condition_parentheses(lines: &mut [PseudoLine]) {
    for line in lines.iter_mut() {
        let text = line.text.clone();
        let trimmed = text.trim();
        // `if (`, `while (`, `} else if (`, `} while (` — the condition starts right after.
        let Some(open) = ["if (", "while (", "} else if (", "} while ("]
            .iter()
            .find_map(|prefix| trimmed.starts_with(prefix).then(|| prefix.len() - 1))
        else {
            continue;
        };
        let offset = leading_spaces(&text) + open;
        let Some(close) = matching_close(&text[offset..]).map(|index| offset + index) else {
            continue;
        };
        let condition = &text[offset + 1..close];
        if !condition.starts_with('(') || matching_close(condition) != Some(condition.len() - 1) {
            continue;
        }
        let inner = &condition[1..condition.len() - 1];
        if inner.is_empty() {
            continue;
        }
        // Keep whatever follows the `)` — a block `{`, `;`, or a single statement.
        line.text = format!("{}{}{}", &text[..offset + 1], inner, &text[close..]);
    }
}

/// Byte index of the `)` matching the `(` at index 0.
fn matching_close(text: &str) -> Option<usize> {
    let mut depth = 0isize;
    for (index, ch) in text.char_indices() {
        match ch {
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

/// Remove an assignment that the very next statement overwrites without reading it:
/// `var_n8 = rdi; var_n8 = r14;` keeps only the second store. Compilers spill several
/// incoming registers to the same slot before the frame settles; those earlier stores are
/// dead and only add noise. The value being dropped must be side-effect free (no call), so
/// a discarded call is never removed.
fn drop_overwritten_assignments(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index + 1 < lines.len() {
        let Some((first_lhs, first_rhs)) = plain_assignment(&lines[index].text) else {
            index += 1;
            continue;
        };
        let Some((second_lhs, second_rhs)) = plain_assignment(&lines[index + 1].text) else {
            index += 1;
            continue;
        };
        if first_lhs != second_lhs || !is_identifier(first_lhs) {
            index += 1;
            continue;
        }
        if contains_call(first_rhs) || mentions_slot(second_rhs, first_lhs) {
            index += 1;
            continue;
        }
        lines.remove(index);
    }
}

/// Split `x = rhs;` into `(x, rhs)` for a plain assignment to an identifier.
fn plain_assignment(text: &str) -> Option<(&str, &str)> {
    let statement = text.trim().strip_suffix(';')?;
    if statement.contains(" += ")
        || statement.contains(" -= ")
        || statement.contains(" *= ")
        || statement.contains(" &= ")
        || statement.contains(" |= ")
        || statement.contains(" ^= ")
        || statement.contains(" <<= ")
        || statement.contains(" >>= ")
    {
        return None;
    }
    let (lhs, rhs) = statement.split_once(" = ")?;
    Some((lhs.trim(), rhs.trim()))
}

/// True when the expression calls a function (`sub_...(...)`, an API, `__asm(...)`).
fn contains_call(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = index;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            let mut next = index;
            while next < bytes.len() && bytes[next] == b' ' {
                next += 1;
            }
            if next < bytes.len() && bytes[next] == b'(' && next > start {
                return true;
            }
            continue;
        }
        index += 1;
    }
    false
}

/// True when the text is exactly one identifier (no operators or spaces inside).
fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// Drop a register write whose register is never mentioned again in the whole function
/// (`rbp = &var_b1;` in a stub, a setup register whose call was inlined away). All spellings
/// count (`rax`/`eax`/`al`), and a function containing assembly is skipped: the text can read
/// a register without naming it (`rep movsb` uses `rsi`/`rdi`/`rcx`).
fn drop_unused_register_writes(lines: &mut Vec<PseudoLine>) {
    if lines
        .iter()
        .any(|line| line.text.trim_start().starts_with("__asm("))
    {
        return;
    }
    let mut mentions = [0usize; 16];
    for line in lines.iter() {
        for token in line
            .text
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        {
            if let Some(base) = register_base_any(token) {
                mentions[base as usize] += 1;
            }
        }
    }
    let mut index = 0;
    while index < lines.len() {
        let Some((target, _)) = plain_assignment(&lines[index].text) else {
            index += 1;
            continue;
        };
        let Some(base) = register_base_any(target) else {
            index += 1;
            continue;
        };
        // `rax`/`eax` is the return value: L1 prints a bare `return;` and the caller still
        // reads it, so a write that only looks unused at this level must stay. Argument
        // registers stay too — a call prints only the arguments it could recover, and the
        // setup line is the only record of the rest.
        if matches!(base, 0 | 1 | 2 | 8 | 9) {
            index += 1;
            continue;
        }
        if mentions[base as usize] == 1 {
            lines.remove(index);
        } else {
            index += 1;
        }
    }
}

/// A `while (1)` body often starts by preparing a value the first test needs — typically the
/// address of a local used as a sentinel (`rax = &var_30; if (rcx <= rax) break;`). When that
/// value is loop-invariant the setup hoists above the loop, which lets the loop-condition
/// pass turn the `break` into a real `while` condition.
fn hoist_invariant_loop_setups(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index + 2 < lines.len() {
        if lines[index].text.trim() != "while (1) {" {
            index += 1;
            continue;
        }
        let indent = leading_spaces(&lines[index].text);
        let Some((target, value)) = plain_assignment(&lines[index + 1].text) else {
            index += 1;
            continue;
        };
        let target = target.to_owned();
        let value = value.to_owned();
        if leading_spaces(&lines[index + 1].text) != indent + 4
            || wide_register_base(&target).is_none()
            || !(value.starts_with("&var_")
                || value.starts_with("&frame_")
                || value.parse::<i64>().is_ok())
        {
            index += 1;
            continue;
        }
        let test = lines[index + 2].text.trim();
        if !(test.starts_with("if (") && test.ends_with(") break;") && mentions_slot(test, &target))
        {
            index += 1;
            continue;
        }
        // The register must survive the body unchanged until the loop tests it again: a call
        // clobbers caller-saved registers, and any write would make the hoisted value stale.
        let Some(end) = block_end(lines, index) else {
            index += 1;
            continue;
        };
        let clobbered = lines[index + 2..end].iter().any(|line| {
            let text = line.text.trim();
            text.starts_with("__asm(")
                || is_call_line(text)
                || plain_assignment(text)
                    .is_some_and(|(lhs, _)| wide_register_base(lhs) == wide_register_base(&target))
                || (text.starts_with(&format!("{target} ")) && text.contains("= "))
        });
        if clobbered {
            index += 1;
            continue;
        }
        let setup = lines.remove(index + 1);
        lines.insert(
            index,
            PseudoLine {
                addr: setup.addr,
                text: format!("{}{} = {value};", " ".repeat(indent), target),
            },
        );
        index += 2;
    }
}

/// Index of the `}` that closes the block opened on `start`'s line (which must end in `{`).
fn block_end(lines: &[PseudoLine], start: usize) -> Option<usize> {
    let opened = lines[start].text.trim().ends_with('{');
    if !opened {
        return None;
    }
    let mut depth = 0usize;
    for (offset, line) in lines[start..].iter().enumerate() {
        let text = line.text.trim();
        depth += text.matches('{').count();
        depth = depth.saturating_sub(text.matches('}').count());
        if depth == 0 {
            return Some(start + offset);
        }
    }
    None
}

/// A temporary that exists only to be stored folds into the store:
/// `ecx = (uint32)(*(u16*)(rax)); *(u32*)((r8 + 8)) = ecx;` becomes
/// `*(u32*)((r8 + 8)) = (uint32)(*(u16*)(rax));`. The register must be dead after the store,
/// otherwise the copy is kept and the value stays on its own line.
fn fold_store_value_temporaries(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index + 1 < lines.len() {
        let indent = leading_spaces(&lines[index].text);
        let Some((target, value)) = plain_assignment(&lines[index].text) else {
            index += 1;
            continue;
        };
        if wide_register_base(target).is_none()
            || mentions_slot(value, target)
            || leading_spaces(&lines[index + 1].text) != indent
        {
            index += 1;
            continue;
        }
        let store = lines[index + 1].text.trim().to_owned();
        let Some(head) = store.strip_suffix(&format!(" = {target};")) else {
            index += 1;
            continue;
        };
        // The destination may be memory (`*(u32*)(...)`) or a named stack slot.
        if !(head.starts_with("*(u") || is_stack_slot_name(head))
            || !register_dead_after(lines, index + 1, target)
        {
            index += 1;
            continue;
        }
        lines[index + 1].text = format!("{}{head} = {value};", " ".repeat(indent));
        lines.remove(index);
        index += 1;
    }
}

/// True when the register is not read again in the same straight-line run: the next mention
/// is a full write, or the run ends (`}`, `return;`, end of listing). Calls, branches and
/// assembly make the answer "still live" — they can read a register we do not print.
fn register_dead_after(lines: &[PseudoLine], at: usize, target: &str) -> bool {
    let indent = leading_spaces(&lines[at].text);
    for line in &lines[at + 1..] {
        let text = line.text.trim();
        let line_indent = leading_spaces(&line.text);
        if line_indent < indent || text == "}" || text == "return;" {
            return true;
        }
        if line_indent != indent
            || is_control_line(text)
            || is_call_line(text)
            || text.starts_with("__asm(")
        {
            return false;
        }
        if mentions_slot(text, target) {
            return plain_assignment(text).is_some_and(|(lhs, rhs)| {
                wide_register_base(lhs)
                    .is_some_and(|base| wide_register_base(target).is_some_and(|own| base == own))
                    && !reads_register(rhs, target)
            });
        }
    }
    true
}

/// `rdx = (c ? a : b); return rdx;` is a value built for the return: fold the assignment into
/// the returning line (`return (c ? a : b);`) so the temporary does not appear at all.
fn fold_assigned_returns(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index + 1 < lines.len() {
        let indent = leading_spaces(&lines[index].text);
        let Some((target, value)) = plain_assignment(&lines[index].text) else {
            index += 1;
            continue;
        };
        if wide_register_base(target).is_none()
            || leading_spaces(&lines[index + 1].text) != indent
            || lines[index + 1].text.trim() != format!("return {target};")
        {
            index += 1;
            continue;
        }
        lines[index].text = format!("{}return {value};", " ".repeat(indent));
        lines.remove(index + 1);
        index += 1;
    }
}

/// A label whose whole block computes the return value and returns is the function's own
/// exit path, so a `goto` to it is an early return. Folding the jump into
/// `return <value>;` removes the label chase and, once nothing references the label any
/// more, the label itself.
fn fold_trivial_return_labels(lines: &mut [PseudoLine]) {
    let mut tails: BTreeMap<String, String> = BTreeMap::new();
    for index in 0..lines.len() {
        let text = lines[index].text.trim();
        let Some(label) = text.strip_suffix(':') else {
            continue;
        };
        if !label.starts_with("loc_") {
            continue;
        }
        let mut result = None;
        let mut assignment: Option<String> = None;
        for candidate in &lines[index + 1..] {
            let text = candidate.text.trim();
            if text.is_empty() {
                continue;
            }
            if text.starts_with('{') || text.starts_with('}') {
                break;
            }
            // Another entry into the same tail; the copied return still ends at the same
            // statement, so keep scanning.
            if text.starts_with("loc_") && text.ends_with(':') {
                continue;
            }
            if text == "return;" {
                result = Some(match assignment {
                    Some(rhs) => format!("return {rhs};"),
                    None => "return;".to_owned(),
                });
                break;
            }
            if let Some(value) = text
                .strip_prefix("return ")
                .and_then(|t| t.strip_suffix(';'))
            {
                result = Some(format!("return {value};"));
                break;
            }
            if assignment.is_some() {
                break;
            }
            match plain_assignment(text) {
                Some((lhs, rhs)) if lhs == "eax" || lhs == "rax" => {
                    assignment = Some(rhs.to_owned());
                }
                _ => break,
            }
        }
        if let Some(result) = result {
            tails.insert(label.to_owned(), result);
        }
    }
    if tails.is_empty() {
        return;
    }
    for line in lines.iter_mut() {
        let text = line.text.trim();
        let Some(target) = goto_target(&line.text) else {
            continue;
        };
        let Some(result) = tails.get(&format!("loc_{target:x}")) else {
            continue;
        };
        let indent = " ".repeat(leading_spaces(&line.text));
        line.text = match text.split_once(") goto loc_") {
            Some((condition, _)) => {
                let condition = condition.strip_prefix("if (").unwrap_or(condition);
                format!("{indent}if ({condition}) {result}")
            }
            None => format!("{indent}{result}"),
        };
    }
}

/// `al = 1; return eax;` is how a compiled `bool` return looks: fold the constant into the
/// return so the value sits on the line that returns it (`return 1;`). Only constants are
/// folded — a byte register read would hide the width it was produced at.
fn fold_constant_byte_returns(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index + 1 < lines.len() {
        let indent = leading_spaces(&lines[index].text);
        if leading_spaces(&lines[index + 1].text) != indent
            || lines[index + 1].text.trim() != "return eax;"
        {
            index += 1;
            continue;
        }
        let text = lines[index].text.trim();
        let Some(value) = text
            .strip_prefix("al = ")
            .and_then(|rest| rest.strip_suffix(';'))
        else {
            index += 1;
            continue;
        };
        let constant = value
            .strip_prefix("0x")
            .map(|digits| i64::from_str_radix(digits, 16).is_ok())
            .unwrap_or_else(|| value.parse::<i64>().is_ok());
        if !constant {
            index += 1;
            continue;
        }
        lines[index].text = format!("{}return {value};", " ".repeat(indent));
        lines.remove(index + 1);
        index += 1;
    }
}

/// Drop a register write whose next mention in the same straight-line run is a full write to
/// the same 32/64-bit register: the value is never read. Copies like
/// `rax = var_b8; r9d = 0; r8d = 0; rax = var_b0;` are compiler register shuffling and only
/// add noise. Calls, branches, labels and assembly statements stop the scan, and an 8/16-bit
/// write (`al = 0`) does not kill the wider value.
fn drop_dead_register_writes(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index < lines.len() {
        let Some((target, value)) = plain_assignment(&lines[index].text) else {
            index += 1;
            continue;
        };
        let Some(base) = wide_register_base(target) else {
            index += 1;
            continue;
        };
        if contains_call(value) {
            index += 1;
            continue;
        }
        let indent = leading_spaces(&lines[index].text);
        let mut cursor = index + 1;
        let mut dead = false;
        while cursor < lines.len() {
            let text = lines[cursor].text.trim();
            if leading_spaces(&lines[cursor].text) != indent
                || is_control_line(text)
                || is_call_line(text)
                || text.starts_with("__asm(")
            {
                break;
            }
            if mentions_slot(text, target) {
                dead = plain_assignment(text).is_some_and(|(lhs, rhs)| {
                    wide_register_base(lhs) == Some(base) && !reads_register(rhs, target)
                });
                break;
            }
            cursor += 1;
        }
        if dead {
            lines.remove(index);
        } else {
            index += 1;
        }
    }
}

/// Base register of any spelling (`rax`/`eax`/`ax`/`al` -> 0, `r10d` -> 10).
fn register_base_any(name: &str) -> Option<u8> {
    const WIDE64: [&str; 16] = [
        "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12",
        "r13", "r14", "r15",
    ];
    const WIDE32: [&str; 16] = [
        "eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi", "r8d", "r9d", "r10d", "r11d",
        "r12d", "r13d", "r14d", "r15d",
    ];
    const LOW16: [&str; 8] = ["ax", "cx", "dx", "bx", "sp", "bp", "si", "di"];
    const LOW8: [&str; 8] = ["al", "cl", "dl", "bl", "spl", "bpl", "sil", "dil"];
    const HIGH8: [&str; 4] = ["ah", "ch", "dh", "bh"];
    for table in [&WIDE64[..], &WIDE32[..]] {
        if let Some(index) = table.iter().position(|entry| *entry == name) {
            return Some(index as u8);
        }
    }
    for table in [&LOW16[..], &LOW8[..]] {
        if let Some(index) = table.iter().position(|entry| *entry == name) {
            return Some(index as u8);
        }
    }
    if let Some(index) = HIGH8.iter().position(|entry| *entry == name) {
        return Some(index as u8);
    }
    let rest = name.strip_prefix('r')?;
    let digits = rest.trim_end_matches(['d', 'w', 'b']);
    if digits == rest {
        return None;
    }
    digits
        .parse::<u8>()
        .ok()
        .filter(|value| (8..=15).contains(value))
}

/// True when the text reads any spelling of the register (`rax`, `eax`, `al`, ...).
fn reads_register(text: &str, name: &str) -> bool {
    let Some(base) = register_base_any(name) else {
        return false;
    };
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|token| register_base_any(token) == Some(base))
}

/// Base register of a 32/64-bit spelling (`rax`/`eax` -> 0, `r10`/`r10d` -> 10).
fn wide_register_base(name: &str) -> Option<u8> {
    const BASES: [&str; 16] = [
        "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12",
        "r13", "r14", "r15",
    ];
    const BASES32: [&str; 16] = [
        "eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi", "r8d", "r9d", "r10d", "r11d",
        "r12d", "r13d", "r14d", "r15d",
    ];
    BASES
        .iter()
        .position(|base| *base == name)
        .or_else(|| BASES32.iter().position(|base| *base == name))
        .map(|index| index as u8)
}

/// Fold the consumers of a `sbb x, x` carry mask (`x = cond ? -1 : 0`):
/// `x = -(x)` turns the branches into `1`/`0` (a boolean), and `x &= y` selects `y`
/// (a conditional mask). Both stay one statement instead of spreading the idiom over two.
fn fold_carry_masks(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index + 1 < lines.len() {
        let Some((slot, value)) = plain_assignment(&lines[index].text) else {
            index += 1;
            continue;
        };
        let Some((condition, then, otherwise)) = parse_const_select(value) else {
            index += 1;
            continue;
        };
        if !((then == -1 && otherwise == 0) || (then == 0 && otherwise == -1)) {
            index += 1;
            continue;
        }
        let indent = " ".repeat(leading_spaces(&lines[index].text));
        let next = lines[index + 1].text.trim();
        if next == format!("{slot} = -({slot});") {
            lines[index].text = format!(
                "{indent}{slot} = ({condition} ? {} : {});",
                -then, -otherwise
            );
            lines.remove(index + 1);
            continue;
        }
        if let Some(mask) = next
            .strip_prefix(&format!("{slot} &= "))
            .and_then(|rest| rest.strip_suffix(';'))
        {
            let (then, otherwise) = if then == -1 { (mask, "0") } else { ("0", mask) };
            lines[index].text = format!("{indent}{slot} = ({condition} ? {then} : {otherwise});");
            lines.remove(index + 1);
            continue;
        }
        index += 1;
    }
}

/// Parse `((cond) ? -1 : 0)` into `(cond, -1, 0)`.
fn parse_const_select(value: &str) -> Option<(&str, i64, i64)> {
    let inner = value.strip_prefix('(')?.strip_suffix(')')?;
    let (condition, rest) = inner.rsplit_once(" ? ")?;
    let (then, otherwise) = rest.split_once(" : ")?;
    Some((
        condition.trim(),
        then.trim().parse().ok()?,
        otherwise.trim().parse().ok()?,
    ))
}

/// Fold a register round-trip that only feeds one store: `rax = &var_20; var_30 = rax;`
/// followed by another write to `rax` becomes `var_30 = &var_20;`. The value is consumed by
/// the store and the register is redefined right after, so nothing else can read it.
fn fold_store_copies(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index + 1 < lines.len() {
        let indent = leading_spaces(&lines[index].text);
        let (Some((temporary, value)), Some((slot, source))) = (
            plain_assignment(&lines[index].text),
            plain_assignment(&lines[index + 1].text),
        ) else {
            index += 1;
            continue;
        };
        if leading_spaces(&lines[index + 1].text) != indent
            || !is_identifier(temporary)
            || source != temporary
            || slot == temporary
            || mentions_slot(value, temporary)
        {
            index += 1;
            continue;
        }
        // The register must be dead after the store. Within the straight-line run that
        // starts right after it, the next mention has to be a full write; calls and
        // branches stop the scan because the value may be used implicitly.
        let mut cursor = index + 2;
        let mut dead = false;
        while cursor < lines.len() {
            let text = lines[cursor].text.trim();
            if leading_spaces(&lines[cursor].text) != indent
                || is_control_line(text)
                || is_call_line(text)
            {
                break;
            }
            if mentions_slot(text, temporary) {
                dead = plain_assignment(text).is_some_and(|(lhs, _)| lhs == temporary);
                break;
            }
            cursor += 1;
        }
        if !dead {
            index += 1;
            continue;
        }
        lines[index + 1].text = format!("{}{slot} = {value};", " ".repeat(indent));
        lines.remove(index);
    }
}

/// Inline a temporary that is written once and read exactly once by the next statement:
/// `eax = *(u32*)(rdi); if ((eax == 0))` becomes `if ((*(u32*)(rdi) == 0))`. The write
/// only names the value, so folding it into the single reader removes a register round-trip
/// and makes the condition read like source. Conditions with `&&`/`||` are skipped because
/// inlining could change when the right-hand side is evaluated.
///
/// Token frequencies are counted once instead of per candidate; a fold removes the write and
/// the read it replaces, so only that name's count changes.
/// Inline a temporary into the statement that reads it, when the temporary is written once
/// and read once in the whole function and the read is the immediately following line.
///
/// The rewrite keeps the evaluation order (the value is still computed where it was) and only
/// moves the expression text, which is what a decompiler's local-variable folding does:
/// `eax = *(u64*)(rbx); f(eax);` becomes `f(*(u64*)(rbx));`.
fn inline_adjacent_temporaries(lines: &mut Vec<PseudoLine>) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for line in lines.iter() {
        for token in line
            .text
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .filter(|token| !token.is_empty())
        {
            *counts.entry(token.to_owned()).or_insert(0) += 1;
        }
    }
    let mut index = 0;
    while index + 1 < lines.len() {
        let statement = lines[index].text.trim();
        let Some(statement) = statement.strip_suffix(';') else {
            index += 1;
            continue;
        };
        let Some((lhs, rhs)) = statement.split_once(" = ") else {
            index += 1;
            continue;
        };
        let lhs = lhs.trim().to_owned();
        let rhs = rhs.trim().to_owned();
        // Only a register temporary can move into the next statement: a stack slot is a
        // variable of the function (`&var_30` is its address), not an expression to inline.
        let register = is_identifier(&lhs) && !lhs.starts_with("var_") && !lhs.starts_with("arg");
        if !register || rhs.is_empty() || is_frame_register(&lhs) {
            index += 1;
            continue;
        }
        if counts.get(lhs.as_str()).copied().unwrap_or(0) != 2 {
            index += 1;
            continue;
        }
        // A conditional expression reads worse inline than as a named temporary.
        if contains_token(&rhs, &lhs) || rhs.contains('?') {
            index += 1;
            continue;
        }
        let next = lines[index + 1].text.clone();
        let trimmed = next.trim();
        if trimmed.starts_with("if (")
            || trimmed.starts_with("while")
            || trimmed.starts_with("return ")
            || trimmed.starts_with("goto ")
            || trimmed.starts_with("do {")
            || trimmed.starts_with("else")
            || trimmed.starts_with('}')
            || trimmed.starts_with('{')
            || trimmed.starts_with("loc_")
            || trimmed.contains("__asm(")
        {
            index += 1;
            continue;
        }
        let Some(statement) = trimmed.strip_suffix(';') else {
            index += 1;
            continue;
        };
        if statement.is_empty()
            || assigns_to(statement, &lhs)
            || statement.contains(&format!("&{lhs}"))
        {
            index += 1;
            continue;
        }
        if statement
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .filter(|token| *token == lhs)
            .count()
            != 1
        {
            index += 1;
            continue;
        }
        let replacement = if needs_parentheses(&rhs) {
            format!("({rhs})")
        } else {
            rhs.clone()
        };
        let indent = " ".repeat(leading_spaces(&next));
        lines[index + 1].text =
            format!("{indent}{};", replace_token(statement, &lhs, &replacement));
        if let Some(count) = counts.get_mut(&lhs) {
            *count = count.saturating_sub(2);
        }
        lines.remove(index);
        continue;
    }
}

/// The frame registers carry the stack bookkeeping and are never a user temporary.
fn is_frame_register(name: &str) -> bool {
    matches!(
        name,
        "rsp" | "esp" | "sp" | "spl" | "rbp" | "ebp" | "bp" | "bpl"
    )
}

/// Whether `text` mentions `name` as a whole token.
fn contains_token(text: &str, name: &str) -> bool {
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|token| token == name)
}

/// Whether the statement writes `name` (`eax = …`, `eax += …`); a store through it is a read.
fn assigns_to(statement: &str, name: &str) -> bool {
    let trimmed = statement.trim_start();
    let Some(rest) = trimmed.strip_prefix(name) else {
        return false;
    };
    let rest = rest.trim_start();
    matches!(
        rest.chars().next(),
        Some('=' | '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '<' | '>' | '!' | '?')
    ) && !rest.starts_with("->")
}

/// A substituted expression needs parentheses when it has an operator outside parentheses:
/// `a + b` must land as `(a + b) * 2`, while a cast chain like `(u32)(x)` does not.
fn needs_parentheses(expression: &str) -> bool {
    let mut depth = 0usize;
    for (position, ch) in expression.char_indices() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            // A leading operator is a prefix (`*(u64*)(p)`, `-1`) and binds tighter than
            // anything the surrounding statement can add.
            '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '<' | '>' | '?' | ':' | ','
                if depth == 0 && position > 0 =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn fold_single_use_temporaries(lines: &mut Vec<PseudoLine>) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for line in lines.iter() {
        for token in line
            .text
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .filter(|token| !token.is_empty())
        {
            *counts.entry(token.to_owned()).or_insert(0) += 1;
        }
    }
    let mut index = 0;
    while index + 1 < lines.len() {
        let statement = lines[index].text.trim();
        let Some(statement) = statement.strip_suffix(';') else {
            index += 1;
            continue;
        };
        let Some((lhs, rhs)) = statement.split_once(" = ") else {
            index += 1;
            continue;
        };
        let lhs = lhs.trim().to_owned();
        let rhs = rhs.trim().to_owned();
        if !is_identifier(&lhs) || rhs.is_empty() {
            index += 1;
            continue;
        }
        if counts.get(lhs.as_str()).copied().unwrap_or(0) != 2 {
            index += 1;
            continue;
        }
        let next = lines[index + 1].text.clone();
        let trimmed = next.trim();
        let indent = " ".repeat(leading_spaces(&next));
        if let Some(rest) = trimmed.strip_prefix("if (") {
            let Some(condition) = rest.strip_suffix(") {") else {
                index += 1;
                continue;
            };
            if condition.contains("&&") || condition.contains("||") {
                index += 1;
                continue;
            }
            if condition
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .filter(|token| **token == lhs)
                .count()
                != 1
            {
                index += 1;
                continue;
            }
            let condition = replace_token(condition, &lhs, &rhs);
            lines[index + 1].text = format!("{indent}if ({condition}) {{");
            if let Some(count) = counts.get_mut(&lhs) {
                *count = count.saturating_sub(2);
            }
            lines.remove(index);
            continue;
        }
        if let Some(expression) = trimmed
            .strip_prefix("return ")
            .and_then(|rest| rest.strip_suffix(';'))
        {
            // A returned expression is evaluated right after the assignment, so the
            // temporary can be inlined the same way as a condition.
            if expression.contains("&&") || expression.contains("||") {
                index += 1;
                continue;
            }
            if expression
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .filter(|token| **token == lhs)
                .count()
                != 1
            {
                index += 1;
                continue;
            }
            lines[index + 1].text =
                format!("{indent}return {};", replace_token(expression, &lhs, &rhs));
            if let Some(count) = counts.get_mut(&lhs) {
                *count = count.saturating_sub(2);
            }
            lines.remove(index);
            continue;
        }
        index += 1;
    }
}

/// Replace whole identifier tokens, never substrings (`rax` inside `rax2`), and never
/// inside a quoted string literal.
fn replace_token(text: &str, token: &str, replacement: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut word = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for ch in text.chars() {
        if quoted {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        if ch == '"' {
            if word == token {
                output.push_str(replacement);
            } else {
                output.push_str(&word);
            }
            word.clear();
            quoted = true;
            output.push(ch);
            continue;
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            word.push(ch);
        } else {
            if word == token {
                output.push_str(replacement);
            } else {
                output.push_str(&word);
            }
            word.clear();
            output.push(ch);
        }
    }
    if word == token {
        output.push_str(replacement);
    } else {
        output.push_str(&word);
    }
    output
}

/// Drop a store to a stack slot when the slot is overwritten later in the same
/// straight-line run without being read. Compilers materialize intermediate values into
/// locals that are immediately replaced (`var_30 = 0; ... var_30 = rax;`); only the last
/// store matters. Scanning stops at the first control-flow line, so a value that may be
/// read on another path is never removed.
fn drop_dead_slot_stores(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index < lines.len() {
        let Some((slot, rhs)) = plain_assignment(&lines[index].text) else {
            index += 1;
            continue;
        };
        if !slot.starts_with("var_") || !is_identifier(slot) || mentions_slot(rhs, slot) {
            index += 1;
            continue;
        }
        if contains_call(rhs) {
            index += 1;
            continue;
        }
        let mut cursor = index + 1;
        let mut dead = false;
        while cursor < lines.len() {
            let text = lines[cursor].text.trim();
            if text.is_empty() {
                cursor += 1;
                continue;
            }
            if is_control_line(text) {
                break;
            }
            // A call can read the outgoing stack area (arguments 5+), so a slot store is
            // not dead just because no later statement names it.
            if contains_call(text) {
                break;
            }
            if mentions_slot(text, slot) {
                dead = plain_assignment(text).is_some_and(|(lhs, _)| lhs == slot);
                break;
            }
            cursor += 1;
        }
        if dead {
            lines.remove(index);
        } else {
            index += 1;
        }
    }
}

/// Lines that end a straight-line run: branches, loops, labels, calls through a raw
/// expression and closing braces.
fn is_control_line(text: &str) -> bool {
    text.starts_with("if (")
        || text.starts_with("while")
        || text.starts_with("for ")
        || text.starts_with("do {")
        || text.starts_with("switch")
        || text.starts_with("goto ")
        || text.starts_with("return")
        || text.starts_with("break;")
        || text.starts_with("continue;")
        || text.starts_with("loc_")
        || text.starts_with('}')
        || text.starts_with("(*")
}

/// Collapse `r = *(T*)(a); r -= k; *(T*)(a) = r;` into `*(T*)(a) -= k;`. The temporary
/// register only carries the loaded value back to the same slot, so the update can be
/// written against the lvalue directly, as the original source did.
fn fold_load_modify_store(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index + 2 < lines.len() {
        let first = lines[index].text.trim();
        let second = lines[index + 1].text.trim();
        let third = lines[index + 2].text.trim();
        let Some((register, ty, address)) = parse_load(first) else {
            index += 1;
            continue;
        };
        let Some((compound_register, operator, value)) = parse_compound(second) else {
            index += 1;
            continue;
        };
        let Some((store_ty, store_address, stored)) = parse_store(third) else {
            index += 1;
            continue;
        };
        if compound_register != register
            || stored != register
            || store_ty != ty
            || store_address != address
        {
            index += 1;
            continue;
        }
        // The fold drops the instruction that defined `register`. If a later statement still
        // reads it (notably `return;` returning `eax`), the value would be lost.
        if register_read_before_write(lines, index + 3, register) {
            index += 1;
            continue;
        }
        let indent = " ".repeat(leading_spaces(&lines[index].text));
        let addr = lines[index].addr;
        lines.splice(
            index..index + 3,
            [PseudoLine {
                addr,
                text: format!("{indent}*({ty})({address}) {operator}= {value};"),
            }],
        );
        index += 1;
    }
}

/// True when `register` is read on some later statement before being completely redefined.
/// `return;` counts as a read of `eax`/`rax`, since that is the value the caller receives.
fn register_read_before_write(lines: &[PseudoLine], start: usize, register: &str) -> bool {
    for line in &lines[start..] {
        let text = line.text.trim();
        if text == "return;" && matches!(register, "eax" | "rax") {
            return true;
        }
        let assignment = plain_assignment(text);
        let compound = parse_compound(text);
        let writes = assignment.is_some_and(|(lhs, _)| lhs == register)
            || compound.is_some_and(|(lhs, _, _)| lhs == register);
        if !mentions_slot(text, register) {
            if writes {
                return false;
            }
            continue;
        }
        // The register is mentioned: a write that reads it still uses the old value, a
        // plain write kills the definition, anything else reads it.
        let reads_own_value = assignment.is_some_and(|(_, rhs)| mentions_slot(rhs, register))
            || compound.is_some_and(|(_, _, value)| mentions_slot(value, register));
        return reads_own_value || !writes;
    }
    false
}

/// Parse `r = *(T)(a);` into `(r, T, a)`.
fn parse_load(text: &str) -> Option<(&str, &str, &str)> {
    let statement = text.strip_suffix(';')?;
    let (lhs, rhs) = statement.split_once(" = ")?;
    let lhs = lhs.trim();
    if !is_identifier(lhs) {
        return None;
    }
    let (ty, address) = parse_dereference(rhs.trim())?;
    Some((lhs, ty, address))
}

/// Parse `*(T)(a) = r;` into `(T, a, r)`.
fn parse_store(text: &str) -> Option<(&str, &str, &str)> {
    let statement = text.strip_suffix(';')?;
    let (lhs, rhs) = statement.split_once(" = ")?;
    let (ty, address) = parse_dereference(lhs.trim())?;
    let register = rhs.trim();
    if !is_identifier(register) {
        return None;
    }
    Some((ty, address, register))
}

/// Parse `r op= value;` into `(r, op, value)`.
fn parse_compound(text: &str) -> Option<(&str, &str, &str)> {
    let statement = text.strip_suffix(';')?;
    for operator in ["<<=", ">>=", "+=", "-=", "*=", "&=", "|=", "^="] {
        let Some(position) = statement.find(operator) else {
            continue;
        };
        if position == 0 || !statement[..position].ends_with(' ') {
            continue;
        }
        let register = statement[..position].trim();
        let value = statement[position + operator.len()..].trim();
        if is_identifier(register) && !value.is_empty() {
            return Some((register, &operator[..operator.len() - 1], value));
        }
    }
    None
}

fn parse_dereference(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("*(")?;
    let (ty, rest) = rest.split_once(')')?;
    let rest = rest.trim();
    let address = rest.strip_prefix('(')?.strip_suffix(')')?;
    Some((ty, address))
}

/// Propagate an argument copied to a callee-saved register in the straight-line prologue:
/// `rsi = arg1;` followed by many `rsi` reads becomes direct `arg1` reads and the copy is
/// dropped. Sound only when the copy runs unconditionally (no branch/label before it) and
/// the register is never assigned again anywhere in the function; the register must be
/// callee-saved, because a call could otherwise clobber it between copy and read.
/// A slot that only ever receives one argument is that argument, not a local: `var_40 = arg4;`
/// plus later reads of `var_40` become reads of `arg4`, and the spill disappears. The rewrite
/// needs the slot to be written exactly once and never have its address taken (a call could
/// write through `&var_40`).
fn propagate_argument_spills(lines: &mut Vec<PseudoLine>) {
    let mut writes: HashMap<String, usize> = HashMap::new();
    let mut address_taken: BTreeSet<String> = BTreeSet::new();
    for line in lines.iter() {
        let text = line.text.trim();
        if let Some((lhs, _)) = text.split_once(" = ")
            && lhs.trim().starts_with("var_")
        {
            *writes.entry(lhs.trim().to_owned()).or_default() += 1;
        }
        for slot in slots_with_address(&line.text) {
            address_taken.insert(slot);
        }
    }
    let mut spills: Vec<(String, String)> = Vec::new();
    for line in lines.iter() {
        let text = line.text.trim().trim_end_matches(';');
        let Some((lhs, rhs)) = text.split_once(" = ") else {
            continue;
        };
        let slot = lhs.trim();
        let argument = rhs.trim();
        if !slot.starts_with("var_") || !argument.starts_with("arg") {
            continue;
        }
        if argument[3..].chars().all(|ch| ch.is_ascii_digit())
            && writes.get(slot) == Some(&1)
            && !address_taken.contains(slot)
        {
            spills.push((slot.to_owned(), argument.to_owned()));
        }
    }
    if spills.is_empty() {
        return;
    }
    for line in lines.iter_mut() {
        for (slot, argument) in &spills {
            line.text = replace_token(&line.text, slot, argument);
        }
    }
    // The spill line is now `argN = argN;`.
    lines.retain(|line| {
        let text = line.text.trim().trim_end_matches(';');
        let Some((lhs, rhs)) = text.split_once(" = ") else {
            return true;
        };
        lhs.trim() != rhs.trim()
    });
}

/// The `var_…` slots whose address is taken (`&var_20`) in `text`.
fn slots_with_address(text: &str) -> Vec<String> {
    let mut slots = Vec::new();
    let mut rest = text;
    while let Some(position) = rest.find("&var_") {
        let after = &rest[position + 1..];
        let length = after
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .count();
        if length > 4 {
            slots.push(after[..length].to_owned());
        }
        rest = &rest[position + 5..];
    }
    slots
}

fn propagate_argument_copies(lines: &mut Vec<PseudoLine>) {
    let mut copies: Vec<(usize, String, String)> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let text = line.text.trim();
        if text.starts_with('}')
            || text.starts_with("if (")
            || text.starts_with("while")
            || text.starts_with("for ")
            || text.starts_with("do {")
            || text.starts_with("switch")
            || text.starts_with("goto ")
            || text.starts_with("return")
            || text.starts_with("loc_")
        {
            break;
        }
        let Some((lhs, rhs)) = plain_assignment(text) else {
            continue;
        };
        if rhs.starts_with("arg")
            && rhs[3..].chars().all(|ch| ch.is_ascii_digit())
            && matches!(lhs, "rbx" | "rsi" | "rdi" | "r12" | "r13" | "r14" | "r15")
        {
            copies.push((index, lhs.to_owned(), rhs.to_owned()));
        }
    }
    copies.retain(|(_, register, _)| {
        lines
            .iter()
            .filter(|line| assigns_register(&line.text, register))
            .count()
            == 1
    });
    if copies.is_empty() {
        return;
    }
    for (index, register, argument) in &copies {
        for (line_index, line) in lines.iter_mut().enumerate() {
            if line_index <= *index || !line.text.contains(register.as_str()) {
                continue;
            }
            line.text = replace_token(&line.text, register, argument);
        }
    }
    for (index, _, _) in copies.into_iter().rev() {
        lines.remove(index);
    }
}

/// True when the statement writes the given register (`rsi = ...`, `rsi += ...`).
fn assigns_register(text: &str, register: &str) -> bool {
    let text = text.trim();
    let Some(rest) = text.strip_prefix(register) else {
        return false;
    };
    rest.starts_with(" = ")
        || rest.starts_with(" += ")
        || rest.starts_with(" -= ")
        || rest.starts_with(" *= ")
        || rest.starts_with(" &= ")
        || rest.starts_with(" |= ")
        || rest.starts_with(" ^= ")
        || rest.starts_with(" <<= ")
        || rest.starts_with(" >>= ")
}

/// Merge consecutive `if (c) { body }` arms that share an identical terminating body into
/// one `if (c1 || c2) { body }`. Compilers lower a short-circuit `||` chain into a series
/// of conditional jumps to the same block; without this pass the structurer duplicates the
/// body once per condition, which hides the original `||` expression.
fn merge_identical_if_arms(lines: &mut Vec<PseudoLine>) {
    let mut braces = line_braces(lines);
    let mut index = 0;
    while index < lines.len() {
        let Some((first_condition, first_close)) = standalone_if_block(lines, index, &braces)
        else {
            index += 1;
            continue;
        };
        let key = body_key(&lines[index + 1..first_close]);
        let Some(key) = key.filter(|key| !key.is_empty()) else {
            index += 1;
            continue;
        };
        if !body_terminates(&lines[index + 1..first_close]) {
            index += 1;
            continue;
        }
        let mut conditions = vec![first_condition];
        let mut last_close = first_close;
        loop {
            let mut next = last_close + 1;
            while next < lines.len() && lines[next].text.trim().is_empty() {
                next += 1;
            }
            let Some((condition, close)) = standalone_if_block(lines, next, &braces) else {
                break;
            };
            if leading_spaces(&lines[next].text) != leading_spaces(&lines[index].text) {
                break;
            }
            let body = &lines[next + 1..close];
            if body_key(body) != Some(key.clone()) || !body_terminates(body) {
                break;
            }
            conditions.push(condition);
            last_close = close;
        }
        if conditions.len() < 2 {
            index = first_close + 1;
            continue;
        }
        let indent = " ".repeat(leading_spaces(&lines[index].text));
        lines[index].text = format!("{indent}if ({}) {{", conditions.join(" || "));
        lines.drain(first_close + 1..=last_close);
        // The merge rewrote lines, so the precomputed brace counts are stale.
        braces = line_braces(lines);
        index = first_close + 1;
    }
}

/// Recognise a standalone `if (cond) {` block at `index` and return its condition plus the
/// line index of its closing brace. An `else` arm disqualifies the block.
fn standalone_if_block(
    lines: &[PseudoLine],
    index: usize,
    braces: &[isize],
) -> Option<(String, usize)> {
    let line = lines.get(index)?;
    let text = line.text.trim();
    let condition = text.strip_prefix("if (")?.strip_suffix(") {")?.to_owned();
    let mut depth = 0isize;
    for offset in 0..lines.len().saturating_sub(index) {
        depth += braces[index + offset];
        if depth == 0 {
            let text = lines[index + offset].text.trim();
            if text.starts_with("} else") || text.starts_with("else") {
                return None;
            }
            return Some((condition, index + offset));
        }
    }
    None
}

/// The condition of a block `if` line (`if (cond) {`), or `None` for anything else.
fn if_condition(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("if (")?;
    let condition = rest.strip_suffix(") {")?;
    // `if () {` is already a lint finding; never manufacture one here.
    if condition.trim().is_empty() || condition.contains(") {") {
        return None;
    }
    Some(condition)
}

/// Index of the line closing the block opened at `open`, using precomputed brace counts.
fn block_end_by_braces(braces: &[isize], open: usize) -> Option<usize> {
    let mut depth = 0isize;
    for (index, net) in braces.iter().enumerate().skip(open) {
        depth += net;
        if depth == 0 {
            return Some(index);
        }
    }
    None
}

/// An `if` whose entire body is another `if` reads as two nested blocks, but the source had one
/// condition: `if (a) { if (b) { … } }` folds into `if (a && b) { … }`. Neither arm may have an
/// `else` — otherwise the inner `else` would rebind — so only the plain nesting is folded, and
/// the body moves one indentation level out to keep the surrounding shape.
fn collapse_nested_if_wrappers(lines: &mut Vec<PseudoLine>) {
    let mut braces = line_braces(lines);
    let mut index = 0;
    while index + 1 < lines.len() {
        let outer = lines[index].text.clone();
        let outer_trimmed = outer.trim_start();
        let Some(outer_condition) = if_condition(outer_trimmed) else {
            index += 1;
            continue;
        };
        let inner = lines[index + 1].text.clone();
        let inner_trimmed = inner.trim_start();
        let Some(inner_condition) = if_condition(inner_trimmed) else {
            index += 1;
            continue;
        };
        let outer_indent = outer.len() - outer_trimmed.len();
        let inner_indent = inner.len() - inner_trimmed.len();
        if inner_indent <= outer_indent {
            index += 1;
            continue;
        }
        let Some(inner_end) = block_end_by_braces(&braces, index + 1) else {
            index += 1;
            continue;
        };
        // The inner block has to be the whole outer body, and the inner `if` must own no
        // `else`: the close right after it is the outer one.
        if lines[inner_end].text.trim() != "}" {
            index += 1;
            continue;
        }
        if lines.get(inner_end + 1).map(|line| line.text.trim()) != Some("}") {
            index += 1;
            continue;
        }
        if lines[index + 2..inner_end].iter().any(|line| {
            let text = line.text.trim();
            (text == "} else {" || text.starts_with("else"))
                && line.text.len() - line.text.trim_start().len() == inner_indent
        }) {
            index += 1;
            continue;
        }
        if inner_end == index + 2 {
            // Empty inner body: folding would only produce `if (a && b) { }`.
            index += 1;
            continue;
        }
        let merged = merge_conditions(outer_condition, inner_condition);
        lines[index].text = format!("{}if ({merged}) {{", " ".repeat(outer_indent));
        // Drop the inner header and its close, and lift the body one level.
        lines.remove(index + 1);
        lines.remove(inner_end - 1);
        let shift = inner_indent - outer_indent;
        for line in lines[index + 1..inner_end - 1].iter_mut() {
            let text = line.text.clone();
            if text.trim().is_empty() {
                continue;
            }
            let indent = text.len() - text.trim_start().len();
            if indent < shift {
                continue;
            }
            line.text = text[shift..].to_owned();
        }
        braces = line_braces(lines);
        // The merged `if` may now wrap another single `if`; retry at the same index.
    }
}

/// Join two conditions for an `if`, parenthesising a side that carries a top-level `||`.
fn merge_conditions(outer: &str, inner: &str) -> String {
    // `a && a` is just `a`; the CFG splits a repeated test into two blocks.
    if outer.trim() == inner.trim() {
        return outer.trim().to_owned();
    }
    let wrap = |condition: &str| -> String {
        if has_top_level_or(condition) {
            format!("({condition})")
        } else {
            condition.to_owned()
        }
    };
    format!("{} && {}", wrap(outer), wrap(inner))
}

/// Whether `condition` has a `||` outside any parentheses.
fn has_top_level_or(condition: &str) -> bool {
    let mut depth = 0isize;
    let bytes = condition.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b'|' if depth == 0 && bytes.get(index + 1) == Some(&b'|') => return true,
            _ => {}
        }
        index += 1;
    }
    false
}

/// Net braces opened by each line, so a block scan does not recount them per candidate.
fn line_braces(lines: &[PseudoLine]) -> Vec<isize> {
    lines
        .iter()
        .map(|line| {
            let text = line.text.trim();
            text.matches('{').count() as isize - text.matches('}').count() as isize
        })
        .collect()
}

/// Structural fingerprint of an arm body. Label definitions are ignored so a body that
/// was emitted with a shared-block label can still match its unlabelled copies; a `goto`
/// inside the body is not merged because copying it would relocate the jump's labels.
fn body_key(body: &[PseudoLine]) -> Option<String> {
    let mut lines = Vec::with_capacity(body.len());
    for line in body {
        let text = line.text.trim();
        if text.is_empty() || (text.starts_with("loc_") && text.ends_with(':')) {
            continue;
        }
        if text.starts_with("goto loc_") {
            return None;
        }
        lines.push(text);
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// The copy runs at most once per path only if every path through it leaves the current
/// block; a `return` (or loop `break`/`continue`) on the last line guarantees that.
fn body_terminates(body: &[PseudoLine]) -> bool {
    let Some(last) = body.iter().rev().find(|line| !line.text.trim().is_empty()) else {
        return false;
    };
    let text = last.text.trim();
    (text.starts_with("return") && text.ends_with(';')) || text == "break;" || text == "continue;"
}

/// A jump to the last labelled tail can be turned into a copy of that short tail followed
/// by `return`; there is no later block in the function to continue into.
fn fold_last_label_tails(lines: &mut Vec<PseudoLine>) {
    let Some((label_index, target)) = lines.iter().enumerate().rev().find_map(|(index, line)| {
        let text = line.text.trim();
        let label = text.strip_suffix(':')?;
        let addr = label.strip_prefix("loc_")?;
        Some((index, u64::from_str_radix(addr, 16).ok()?))
    }) else {
        return;
    };
    let mut tail = Vec::new();
    let mut depth = 0usize;
    for line in &lines[label_index + 1..] {
        let text = line.text.trim();
        if text.is_empty() {
            continue;
        }
        if depth == 0 && text.starts_with('}') {
            break;
        }
        if text.starts_with("while")
            || text.starts_with("for ")
            || text.starts_with("switch")
            || text.starts_with("goto loc_")
            || text.starts_with("loc_")
        {
            return;
        }
        if text.starts_with('}') {
            depth = depth.saturating_sub(1);
        }
        depth += text.matches('{').count();
        depth = depth.saturating_sub(
            text.matches('}')
                .count()
                .saturating_sub(usize::from(text.starts_with('}'))),
        );
        tail.push(text.to_owned());
        if depth == 0 && tail.len() > 16 {
            break;
        }
    }
    if depth != 0 || tail.len() > 128 {
        return;
    }
    // The target exits every enclosing branch and runs the function epilogue. Copy that
    // epilogue too; otherwise a bare `return` would skip stack/cookie restoration.
    if tail.last().is_none_or(|line| line != "return;") {
        let Some(return_index) = lines.iter().rposition(|line| line.text.trim() == "return;")
        else {
            return;
        };
        let mut epilogue_start = return_index;
        while epilogue_start > 0 {
            let text = lines[epilogue_start - 1].text.trim();
            if text.starts_with('}')
                || text.starts_with("if (")
                || text.starts_with("while")
                || text.starts_with("for ")
                || text.starts_with("switch")
                || text.starts_with("goto loc_")
                || text.starts_with("loc_")
            {
                break;
            }
            epilogue_start -= 1;
        }
        let epilogue: Vec<String> = lines[epilogue_start..=return_index]
            .iter()
            .map(|line| line.text.trim().to_owned())
            .collect();
        if epilogue.len() > 32 {
            return;
        }
        tail.extend(epilogue);
    }
    let needs_return = false;
    let mut output = Vec::with_capacity(lines.len());
    for line in lines.drain(..) {
        let text = line.text.trim();
        let Some(jump) = goto_target(&line.text) else {
            output.push(line);
            continue;
        };
        if jump != target {
            output.push(line);
            continue;
        }
        let indent = " ".repeat(leading_spaces(&line.text));
        let conditional = text
            .split_once(") goto loc_")
            .map(|(condition, _)| condition.strip_prefix("if (").unwrap_or(condition));
        if let Some(condition) = conditional {
            output.push(PseudoLine {
                addr: line.addr,
                text: format!("{indent}if ({condition}) {{"),
            });
            append_indented_tail(
                &mut output,
                line.addr,
                &format!("{indent}    "),
                tail.as_slice(),
            );
            if needs_return {
                output.push(PseudoLine {
                    addr: line.addr,
                    text: format!("{indent}    return;"),
                });
            }
            output.push(PseudoLine {
                addr: line.addr,
                text: format!("{indent}}}"),
            });
        } else {
            append_indented_tail(&mut output, line.addr, &indent, tail.as_slice());
            if needs_return {
                output.push(PseudoLine {
                    addr: line.addr,
                    text: format!("{indent}return;"),
                });
            }
        }
    }
    *lines = output;
}

/// Copy a balanced tail while restoring one indentation level per brace. The source lines
/// are trimmed, so nested `if`/`else` bodies would otherwise collapse to one column.
fn append_indented_tail(output: &mut Vec<PseudoLine>, addr: Address, base: &str, tail: &[String]) {
    let mut depth = 0usize;
    for body in tail {
        let text = body.trim();
        if text.starts_with('}') {
            depth = depth.saturating_sub(1);
        }
        output.push(PseudoLine {
            addr,
            text: format!("{base}{}{text}", "    ".repeat(depth)),
        });
        if text.ends_with('{') {
            depth += 1;
        }
    }
}

/// Replace jumps to a label whose own block is exactly `return;` with a direct `return;`.
/// These labels are the common function epilogue after stack restores have already been
/// hidden; keeping the jump obscures an otherwise structured early exit.
fn fold_return_labels(lines: &mut [PseudoLine]) {
    let mut returns = BTreeSet::new();
    for (index, line) in lines.iter().enumerate() {
        let text = line.text.trim();
        let Some(label) = text.strip_suffix(':') else {
            continue;
        };
        if !label.starts_with("loc_") {
            continue;
        }
        // Only the statements belonging to this label's block count. A label nested inside
        // an `if` is followed by the enclosing `}` long before the next label, so scanning
        // to the next label would swallow unrelated sibling code.
        let mut depth = 0usize;
        let mut body = Vec::new();
        for candidate in &lines[index + 1..] {
            let text = candidate.text.trim();
            if text.is_empty() {
                continue;
            }
            if depth == 0
                && (text.starts_with('}') || (text.starts_with("loc_") && text.ends_with(':')))
            {
                break;
            }
            if text.starts_with('}') {
                depth = depth.saturating_sub(1);
            }
            depth += text.matches('{').count();
            depth = depth.saturating_sub(
                text.matches('}')
                    .count()
                    .saturating_sub(usize::from(text.starts_with('}'))),
            );
            body.push(text);
        }
        body.retain(|text| !text.starts_with('}'));
        if body == ["return;"] || body.is_empty() {
            returns.insert(label.to_owned());
        }
    }
    if returns.is_empty() {
        return;
    }
    for line in lines.iter_mut() {
        let text = line.text.trim();
        let Some((prefix, target)) = text
            .strip_prefix("goto loc_")
            .map(|target| ("", target))
            .or_else(|| text.split_once(") goto loc_"))
        else {
            continue;
        };
        let target = target.trim_end_matches(';');
        if !returns.contains(&format!("loc_{target}")) {
            continue;
        }
        let indent = " ".repeat(leading_spaces(&line.text));
        line.text = if prefix.is_empty() {
            format!("{indent}return;")
        } else {
            let condition = prefix.strip_prefix("if (").unwrap_or(prefix);
            format!("{indent}if ({condition}) return;")
        };
    }
}

/// Remove labels/gotos that add no control flow: a `goto` whose target label is the
/// very next line, and labels that are never referenced. Fallback output otherwise
/// contains long runs of `goto loc_x; loc_x:` pairs that hide the actual structure.
fn tidy_labels(lines: &mut Vec<PseudoLine>) {
    if lines.len() < 2 {
        return;
    }
    // Drop `goto L;` when `L:` follows: both paths reach the same line.
    let capacity = lines.len();
    let mut output = Vec::with_capacity(capacity);
    let mut iter = lines.drain(..).peekable();
    while let Some(line) = iter.next() {
        let redundant = goto_target(&line.text).is_some_and(|target| {
            iter.peek()
                .is_some_and(|next| next.text.trim() == format!("loc_{target:x}:"))
        });
        if !redundant {
            output.push(line);
        }
    }
    drop(iter);
    *lines = output;

    // Collect every jump target in one pass; a label no jump names is dead.
    let mut referenced: HashSet<u64> = HashSet::new();
    for line in lines.iter() {
        if let Some(target) = goto_target(&line.text) {
            referenced.insert(target);
        }
    }
    lines.retain(|line| match label_address(line.text.trim()) {
        Some(address) => referenced.contains(&address),
        None => true,
    });
}

/// Address of a `loc_<hex>:` definition, if the line is one.
fn label_address(text: &str) -> Option<u64> {
    text.strip_suffix(':')
        .and_then(|label| label.strip_prefix("loc_"))
        .and_then(|hex| u64::from_str_radix(hex, 16).ok())
}

fn goto_target(text: &str) -> Option<u64> {
    let text = text.trim();
    // A jump may follow a `case` label (`case 0: goto loc_…;`) or a condition, so look for the
    // keyword anywhere instead of only at the start of the line.
    let rest = if let Some(rest) = text.strip_prefix("goto loc_") {
        rest
    } else if let Some((_, rest)) = text.split_once(") goto loc_") {
        rest
    } else if let Some((_, rest)) = text.split_once("goto loc_") {
        rest
    } else {
        return None;
    };
    rest.strip_suffix(';')
        .and_then(|hex| u64::from_str_radix(hex, 16).ok())
}

fn intersect(
    order: &BTreeMap<usize, usize>,
    dom: &[Option<usize>],
    mut left: usize,
    mut right: usize,
) -> usize {
    while left != right {
        while order.get(&left).copied().unwrap_or(0) > order.get(&right).copied().unwrap_or(0) {
            left = dom[left].unwrap_or(left);
        }
        while order.get(&right).copied().unwrap_or(0) > order.get(&left).copied().unwrap_or(0) {
            right = dom[right].unwrap_or(right);
        }
    }
    left
}

/// Drop the generator's wrapping parentheses in one-line conditionals:
/// `if ((x == 0)) break;` becomes `if (x == 0) break;`, and likewise for `continue`,
/// `return ...` and `goto loc_...`.
fn strip_inline_condition_parentheses(lines: &mut [PseudoLine]) {
    for line in lines.iter_mut() {
        let text = line.text.trim();
        let Some(rest) = text.strip_prefix("if ") else {
            continue;
        };
        let Some(close) = matching_close(rest) else {
            continue;
        };
        let tail = &rest[close + 1..];
        let tail_ok = matches!(tail, " break;" | " continue;")
            || (tail.starts_with(" return ") && tail.ends_with(';'))
            || (tail.starts_with(" goto ") && tail.ends_with(';'));
        if !tail_ok {
            continue;
        }
        let condition = &rest[1..close];
        if !condition.starts_with('(') || matching_close(condition) != Some(condition.len() - 1) {
            continue;
        }
        let inner = &condition[1..condition.len() - 1];
        if inner.is_empty() {
            continue;
        }
        let indent = " ".repeat(leading_spaces(&line.text));
        line.text = format!("{indent}if ({inner}){tail}");
    }
}

/// Drop an `else` whose `if` arm always leaves the construct (`return`, `break`, `continue`
/// or `goto`): the arm after it is just the following code. Unwrapping it removes a level of
/// nesting and matches the early-exit style of the original source.
fn flatten_terminating_else(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index < lines.len() {
        if lines[index].text.trim() != "} else {" {
            index += 1;
            continue;
        }
        let else_indent = leading_spaces(&lines[index].text);
        let Some(previous) = index.checked_sub(1) else {
            index += 1;
            continue;
        };
        if leading_spaces(&lines[previous].text) != else_indent + 4
            || !terminates_arm(lines[previous].text.trim())
        {
            index += 1;
            continue;
        }
        let Some(close) = closing_brace(lines, index + 1, 1) else {
            index += 1;
            continue;
        };
        lines[index].text = format!("{}}}", " ".repeat(else_indent));
        for line in &mut lines[index + 1..close] {
            let strip = leading_spaces(&line.text).min(4);
            line.text = line.text[strip..].to_owned();
        }
        lines.remove(close);
        index += 1;
    }
}

/// Last statement of a block, as far as control flow is concerned: it never falls through.
fn terminates_arm(text: &str) -> bool {
    text == "break;"
        || text == "continue;"
        || (text.starts_with("return") && text.ends_with(';'))
        || (text.starts_with("goto ") && text.ends_with(';'))
}

/// Index of the line whose `}` brings the brace depth back to zero, starting at `start`
/// already inside `depth` open braces.
fn closing_brace(lines: &[PseudoLine], start: usize, depth: usize) -> Option<usize> {
    let mut depth = depth;
    for (offset, line) in lines[start..].iter().enumerate() {
        depth += line.text.matches('{').count();
        depth = depth.saturating_sub(line.text.matches('}').count());
        if depth == 0 {
            return Some(start + offset);
        }
    }
    None
}

/// Merge `} else { if (cond) { ... } }` into `} else if (cond) { ... }` when the nested
/// `if` is the only statement of the else arm. Sources spell an else-if chain that way and
/// the extra nesting hides the chain; the body keeps its meaning and only moves one level
/// left.
fn merge_else_if(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index + 1 < lines.len() {
        if lines[index].text.trim() != "} else {" {
            index += 1;
            continue;
        }
        let else_indent = leading_spaces(&lines[index].text);
        let nested = lines[index + 1].text.trim();
        if leading_spaces(&lines[index + 1].text) != else_indent + 4
            || !nested.starts_with("if (")
            || !nested.ends_with(") {")
        {
            index += 1;
            continue;
        }
        let Some(close) = matching_brace(lines, index + 1) else {
            index += 1;
            continue;
        };
        // The else arm must contain nothing but that `if`: its close brace is immediately
        // followed by the else arm's own close brace.
        if close + 1 >= lines.len()
            || lines[close + 1].text.trim() != "}"
            || leading_spaces(&lines[close + 1].text) != else_indent
        {
            index += 1;
            continue;
        }
        let condition = nested.strip_prefix("if ").unwrap_or(nested);
        lines[index].text = format!("{}}} else if {condition}", " ".repeat(else_indent));
        for line in &mut lines[index + 2..close] {
            let strip = leading_spaces(&line.text).min(4);
            line.text = line.text[strip..].to_owned();
        }
        lines.remove(close);
        lines.remove(index + 1);
        index += 1;
    }
}

/// Rewrite `while (1)` loops whose only exit test the structurer could not fold.
///
/// Compilers lower pre-tested loops and do-while loops to an unconditional loop with a
/// single `if (cond) break;`. A test as the first body statement is the loop condition:
/// `while (1) { if (c) break; body }` 闁?`while (!c) { body }`, because `continue` reaches
/// the same test in both forms. A test as the last body statement is a do-while latch:
/// `while (1) { body; if (c) break; }` 闁?`do { body } while (!c);` 闁?but only when the body
/// has no `continue`, which would reach the latch before the test in the original loop.
fn fold_constant_loops(lines: &mut Vec<PseudoLine>) {
    let mut index = 0;
    while index < lines.len() {
        if lines[index].text.trim() != "while (1) {" {
            index += 1;
            continue;
        }
        let Some(close) = matching_brace(lines, index) else {
            index += 1;
            continue;
        };
        if close <= index + 1 {
            index += 1;
            continue;
        }
        let indent = " ".repeat(leading_spaces(&lines[index].text));
        let body_indent = leading_spaces(&lines[index].text) + 4;
        let first = &lines[index + 1];
        if leading_spaces(&first.text) == body_indent
            && let Some(condition) = break_test_condition(first.text.trim())
        {
            let condition = negate(strip_wrapping_parens(condition));
            lines[index].text = format!("{indent}while ({condition}) {{");
            lines.remove(index + 1);
            index += 1;
            continue;
        }
        let last = &lines[close - 1];
        if leading_spaces(&last.text) == body_indent
            && let Some(condition) = break_test_condition(last.text.trim())
            && !lines[index + 1..close]
                .iter()
                .any(|line| line.text.trim() == "continue;")
        {
            let condition = negate(strip_wrapping_parens(condition));
            lines[index].text = format!("{indent}do {{");
            lines.remove(close);
            lines[close - 1].text = format!("{indent}}} while ({condition});");
            index += 1;
            continue;
        }
        index += 1;
    }
}

/// Index of the line closing the block opened on `start` (`{`/`}` balanced by text).
fn matching_brace(lines: &[PseudoLine], start: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, line) in lines[start..].iter().enumerate() {
        let (open, close) = brace_counts(&line.text);
        depth += open;
        depth = depth.saturating_sub(close);
        if depth == 0 {
            return Some(start + offset);
        }
    }
    None
}

/// Braces in `text` outside string literals: a format string may contain `{` or `}` and
/// must not affect block matching.
fn brace_counts(text: &str) -> (usize, usize) {
    let mut open = 0;
    let mut close = 0;
    let mut quoted = false;
    let mut escaped = false;
    for ch in text.chars() {
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
            '{' => open += 1,
            '}' => close += 1,
            _ => {}
        }
    }
    (open, close)
}

/// Condition of a one-line `if (cond) break;` statement.
fn break_test_condition(text: &str) -> Option<&str> {
    let condition = text.strip_prefix("if (")?.strip_suffix(") break;")?;
    Some(condition)
}

/// Strip one pair of parentheses wrapping the whole text (`(a >= b)` 闁?`a >= b`).
fn strip_wrapping_parens(text: &str) -> &str {
    let text = text.trim();
    if text.starts_with('(') && matching_close(text) == Some(text.len() - 1) {
        &text[1..text.len() - 1]
    } else {
        text
    }
}

fn negate(condition: &str) -> String {
    let trimmed = condition.trim();
    // Flip exactly one comparison operator. Replacing them in a chain would undo the
    // earlier substitution (`==` -> `!=` -> `==`), and a conjunction needs De Morgan,
    // which the explicit `!` wrapper expresses correctly.
    if trimmed.contains("&&") || trimmed.contains("||") {
        return format!("!({trimmed})");
    }
    for (from, to) in [
        (" == ", " != "),
        (" != ", " == "),
        (" <= ", " > "),
        (" >= ", " < "),
        (" < ", " >= "),
        (" > ", " <= "),
    ] {
        if trimmed.contains(from) {
            return trimmed.replacen(from, to, 1);
        }
    }
    format!("!({trimmed})")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Expr, ExprId, LiftedInsn, RegView, Stmt, stack_layout};
    use recraft_analysis::{BasicBlock, analyze, persist};
    use tempfile::TempDir;

    /// Turn indented source lines into pseudo lines for the text passes.
    fn snippet(lines: &[&str]) -> Vec<PseudoLine> {
        lines
            .iter()
            .map(|text| PseudoLine {
                addr: Address::new(0x1000),
                text: (*text).to_owned(),
            })
            .collect()
    }

    fn joined(lines: &[PseudoLine]) -> String {
        lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_corpus_output_has_no_foldable_nested_if_left() {
        // Regression fence for the fold: the pass must already be at its fixpoint on real
        // output, so running it again over every snapshot changes nothing.
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/corpus");
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        let mut checked = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("c") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let mut lines: Vec<PseudoLine> = text
                .lines()
                .map(|line| PseudoLine {
                    addr: Address::new(0x1000),
                    text: line.to_owned(),
                })
                .collect();
            let before = joined(&lines);
            collapse_nested_if_wrappers(&mut lines);
            assert_eq!(before, joined(&lines), "{}", path.display());
            checked += 1;
        }
        assert!(checked > 20, "only {checked} snapshots read");
    }
    #[test]
    fn an_accumulation_merges_into_its_input_assignment() {
        let mut lines = snippet(&[
            "    rcx = (int64)(dword_14000003c);",
            "    rcx += 0x140000000;",
        ]);
        merge_accumulations(&mut lines);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0].text.trim(),
            "rcx = ((int64)(dword_14000003c) + 0x140000000);"
        );
    }
    #[test]
    fn nested_single_if_wrappers_collapse_into_one_condition() {
        let mut lines = snippet(&[
            "void sub_1(int64 arg1) {",
            "    if (arg1 != 0) {",
            "        if (arg2 != 0) {",
            "            eax = 0;",
            "        }",
            "    }",
            "}",
        ]);
        collapse_nested_if_wrappers(&mut lines);
        let text = joined(&lines);
        assert!(text.contains("if (arg1 != 0 && arg2 != 0) {"), "{text}");
        assert!(!text.contains("if (arg2 != 0) {"), "{text}");
        assert!(text.contains("\n        eax = 0;\n"), "{text}");
    }

    #[test]
    fn nested_if_with_an_else_is_left_alone() {
        let mut lines = snippet(&[
            "void sub_1(int64 arg1) {",
            "    if (arg1 != 0) {",
            "        if (arg2 != 0) {",
            "            eax = 0;",
            "        } else {",
            "            eax = 1;",
            "        }",
            "    }",
            "}",
        ]);
        collapse_nested_if_wrappers(&mut lines);
        assert!(joined(&lines).contains("if (arg2 != 0) {"));
    }

    #[test]
    fn a_disjunction_is_parenthesised_when_merged() {
        let mut lines = snippet(&[
            "void sub_1(int64 arg1) {",
            "    if (arg1 != 0 || arg3 != 0) {",
            "        if (arg2 != 0) {",
            "            eax = 0;",
            "        }",
            "    }",
            "}",
        ]);
        collapse_nested_if_wrappers(&mut lines);
        let text = joined(&lines);
        assert!(
            text.contains("if ((arg1 != 0 || arg3 != 0) && arg2 != 0) {"),
            "{text}"
        );
    }
    #[test]
    fn conditional_sibling_tail_goto_is_folded() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x1000),
                text: "    if (flag) goto loc_20;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1000),
                text: "}".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x20),
                text: "        loc_20:".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x21),
                text: "        value = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x22),
                text: "    }".to_owned(),
            },
        ];
        fold_sibling_tail_gotos(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("goto loc_"), "{text}");
        assert!(text.contains("if (flag) {"), "{text}");
        assert!(text.contains("value = 1;"), "{text}");
    }

    #[test]
    fn sibling_tail_accepts_else_boundary() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x1000),
                text: "    if (flag) goto loc_20;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1000),
                text: "} else {".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x20),
                text: "        loc_20:".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x21),
                text: "        value = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x22),
                text: "    }".to_owned(),
            },
        ];
        fold_sibling_tail_gotos(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("goto loc_"), "{text}");
        assert!(text.contains("if (flag) {"), "{text}");
    }

    #[test]
    fn sibling_tail_goto_is_folded() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x1000),
                text: "    goto loc_20;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1000),
                text: "}".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x20),
                text: "        loc_20:".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x21),
                text: "        value = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x22),
                text: "    }".to_owned(),
            },
        ];
        fold_sibling_tail_gotos(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("goto loc_"), "{text}");
        assert!(text.contains("value = 1;"), "{text}");
    }

    #[test]
    fn return_tail_label_copies_a_balanced_if() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x1000),
                text: "    goto loc_20;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x20),
                text: "    loc_20:".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x21),
                text: "    if (flag) {".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x22),
                text: "        value = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x23),
                text: "    }".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x24),
                text: "    return;".to_owned(),
            },
        ];
        fold_return_tail_labels(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("goto loc_"), "{text}");
        assert!(text.contains("if (flag) {"), "{text}");
        assert!(text.contains("return;"), "{text}");
    }

    #[test]
    fn orphan_label_after_return_is_removed() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x1000),
                text: "    return;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1001),
                text: "    loc_1001:".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1002),
                text: "    value = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1000),
                text: "}".to_owned(),
            },
        ];
        drop_unreachable_tails(&mut lines);
        assert_eq!(lines.len(), 2, "{lines:?}");
    }

    #[test]
    fn forward_goto_keeps_its_label_after_return() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x1000),
                text: "    return;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1001),
                text: "    loc_1001:".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1002),
                text: "    value = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1003),
                text: "    if (flag) goto loc_1001;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1000),
                text: "}".to_owned(),
            },
        ];
        drop_unreachable_tails(&mut lines);
        assert!(
            lines.iter().any(|line| line.text.trim() == "loc_1001:"),
            "{lines:?}"
        );
        assert_eq!(lines.len(), 5, "{lines:?}");
    }

    #[test]
    fn forward_goto_skips_a_dead_tail() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x1000),
                text: "    return;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1001),
                text: "    dead = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1002),
                text: "    loc_1002:".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1003),
                text: "    live = 2;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1004),
                text: "    if (flag) goto loc_1002;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1000),
                text: "}".to_owned(),
            },
        ];
        drop_unreachable_tails(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("dead = 1;"), "{text}");
        assert!(text.contains("loc_1002:"), "{text}");
        assert!(text.contains("live = 2;"), "{text}");
    }

    #[test]
    fn last_label_tail_copies_balanced_if_and_epilogue() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x1000),
                text: "    goto loc_20;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x20),
                text: "    loc_20:".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x21),
                text: "    if (flag) {".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x22),
                text: "        value = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x23),
                text: "    }".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x24),
                text: "    rbx = saved;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x25),
                text: "    return;".to_owned(),
            },
        ];
        fold_last_label_tails(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("goto loc_"), "{text}");
        assert!(text.contains("if (flag) {"), "{text}");
        assert!(text.contains("rbx = saved;"), "{text}");
    }

    #[test]
    fn last_label_tail_replaces_goto() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x1000),
                text: "    goto loc_20;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x20),
                text: "    loc_20:".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x20),
                text: "    value = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x21),
                text: "    return;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1000),
                text: "}".to_owned(),
            },
        ];
        fold_last_label_tails(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("goto loc_"), "{text}");
        assert!(text.contains("value = 1;"), "{text}");
        assert!(text.contains("return;"), "{text}");
    }

    #[test]
    fn unreachable_tail_after_return_is_removed() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x1000),
                text: "    return;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1001),
                text: "    value = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1002),
                text: "    goto loc_1000;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1000),
                text: "}".to_owned(),
            },
        ];
        drop_unreachable_tails(&mut lines);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(lines[0].text.trim(), "return;");
    }

    #[test]
    fn tidy_labels_removes_adjacent_gotos_and_orphans() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x10),
                text: "    goto loc_20;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x20),
                text: "    loc_20:".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x20),
                text: "    x = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x30),
                text: "    loc_30:".to_owned(),
            },
        ];
        tidy_labels(&mut lines);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "    x = 1;");
    }

    #[test]
    fn callee_saved_spills_are_hidden_only_for_unrelated_slots() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x10),
                text: "    var_8 = rbx;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x14),
                text: "    var_10 = rdi;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x18),
                text: "    eax = var_10;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1c),
                text: "    rbx = var_8;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x20),
                text: "    rdi = var_10;".to_owned(),
            },
        ];
        hide_callee_saves(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("var_8"), "{text}");
        assert!(text.contains("var_10 = rdi"), "{text}");
        assert!(text.contains("rdi = var_10"), "{text}");
    }

    #[test]
    fn callee_save_with_several_restores_is_hidden() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x10),
                text: "    var_8 = rbx;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x14),
                text: "    eax = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x18),
                text: "    rbx = var_8;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1c),
                text: "    return;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x20),
                text: "    rbx = var_8;".to_owned(),
            },
        ];
        hide_callee_saves(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("var_8"), "{text}");
        assert!(text.contains("eax = 1;"), "{text}");
    }

    #[test]
    fn single_reference_saves_are_kept_for_outgoing_stack_arguments() {
        // A prologue spill whose restore is not in the listing is dead bookkeeping.
        let mut lines = vec![test_line("    var_n8 = rdi;"), test_line("    eax = 1;")];
        hide_callee_saves(&mut lines);
        assert_eq!(lines.len(), 1, "a lone prologue spill is hidden");

        // The same store after a call may be a stack argument that the call reads.
        let mut lines = vec![
            test_line("    Api();"),
            test_line("    var_n8 = rdi;"),
            test_line("    eax = 1;"),
        ];
        hide_callee_saves(&mut lines);
        assert_eq!(lines.len(), 3, "a stack argument stays visible");

        let mut lines = vec![
            test_line("    var_20 = rbx;"),
            test_line("    RtlVirtualUnwind(0, var_50, rdi, rax);"),
        ];
        drop_dead_slot_stores(&mut lines);
        assert_eq!(lines.len(), 2, "a call may read the outgoing stack area");
    }

    #[test]
    fn terminating_else_arms_are_flattened() {
        let mut lines = vec![
            test_line("    if (a) {"),
            test_line("        return 1;"),
            test_line("    } else {"),
            test_line("        x = 2;"),
            test_line("    }"),
            test_line("    return x;"),
        ];
        flatten_terminating_else(&mut lines);
        let text: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(
            text,
            vec![
                "    if (a) {",
                "        return 1;",
                "    }",
                "    x = 2;",
                "    return x;",
            ]
        );

        // A then-arm that falls through keeps its else.
        let mut lines = vec![
            test_line("    if (a) {"),
            test_line("        x = 1;"),
            test_line("    } else {"),
            test_line("        x = 2;"),
            test_line("    }"),
        ];
        flatten_terminating_else(&mut lines);
        assert_eq!(lines.len(), 5, "{lines:?}");
    }

    #[test]
    fn single_line_conditions_lose_the_double_parentheses() {
        let mut lines = vec![
            test_line("    if ((al == 0)) return;"),
            test_line("    if ((x)) {"),
            test_line("        if ((a) && (b)) break;"),
        ];
        strip_condition_parentheses(&mut lines);
        let text: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(
            text,
            vec![
                "    if (al == 0) return;",
                "    if (x) {",
                "        if ((a) && (b)) break;",
            ]
        );
    }

    #[test]
    fn assigned_returns_fold_into_the_return() {
        let mut lines = vec![
            test_line("    ebx = 0x80004002;"),
            test_line("    return ebx;"),
            test_line("    rdx = ((eax == 0) ? arg1 : rdx);"),
            test_line("    return rdx;"),
            test_line("    rsi = rbp;"),
            test_line("    rsi += 1;"),
        ];
        fold_assigned_returns(&mut lines);
        let text: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(
            text,
            vec![
                "    return 0x80004002;",
                "    return ((eax == 0) ? arg1 : rdx);",
                "    rsi = rbp;",
                "    rsi += 1;",
            ]
        );
    }

    #[test]
    fn invariant_loop_setups_hoist_above_the_loop() {
        let mut lines = vec![
            test_line("    while (1) {"),
            test_line("        rax = &var_30;"),
            test_line("        if (rcx <= rax) break;"),
            test_line("        rcx -= 1;"),
            test_line("    }"),
        ];
        hoist_invariant_loop_setups(&mut lines);
        let text: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(
            text,
            vec![
                "    rax = &var_30;",
                "    while (1) {",
                "        if (rcx <= rax) break;",
                "        rcx -= 1;",
                "    }"
            ]
        );
    }

    #[test]
    fn unused_register_writes_are_dropped_but_not_the_return_register() {
        let mut lines = vec![
            test_line("    rbp = &var_b1;"),
            test_line("    eax = var_8;"),
            test_line("    rcx = 1;"),
            test_line("    rcx += rdx;"),
            test_line("    return;"),
        ];
        drop_unused_register_writes(&mut lines);
        let text: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(
            text,
            vec![
                "    eax = var_8;",
                "    rcx = 1;",
                "    rcx += rdx;",
                "    return;"
            ]
        );
    }

    #[test]
    fn store_value_temporaries_fold_into_the_store() {
        let mut lines = vec![
            test_line("    ecx = (uint32)(*(u16*)(rax));"),
            test_line("    *(u32*)((r8 + 8)) = ecx;"),
            test_line("    return;"),
        ];
        fold_store_value_temporaries(&mut lines);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(
            lines[0].text,
            "    *(u32*)((r8 + 8)) = (uint32)(*(u16*)(rax));"
        );

        // A named stack slot as the destination folds the same way.
        let mut lines = vec![
            test_line("    ecx = 0;"),
            test_line("    var_8 = ecx;"),
            test_line("    eax = var_8;"),
            test_line("    return;"),
        ];
        fold_store_value_temporaries(&mut lines);
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert_eq!(lines[0].text, "    var_8 = 0;");
    }

    #[test]
    fn constant_byte_returns_fold_into_the_return() {
        let mut lines = vec![
            test_line("        al = 1;"),
            test_line("        return eax;"),
            test_line("        al = dil;"),
            test_line("        return eax;"),
        ];
        fold_constant_byte_returns(&mut lines);
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert_eq!(lines[0].text, "        return 1;");
        assert_eq!(lines[1].text, "        al = dil;");
    }

    #[test]
    fn dead_register_writes_drop_before_a_full_redefinition() {
        let mut lines = vec![
            test_line("    var_8 = 1;"),
            test_line("    ecx = 0;"),
            test_line("    eax = var_8;"),
            test_line("    var_8 = ecx;"),
            test_line("    eax = var_8;"),
            test_line("    return;"),
        ];
        drop_dead_register_writes(&mut lines);
        let text: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(text.len(), 5, "{text:?}");
        assert_eq!(text[2], "    var_8 = ecx;");
    }

    #[test]
    fn carry_masks_fold_into_boolean_or_masked_value() {
        let mut lines = vec![
            test_line("    edx = ((ecx != 0) ? -1 : 0);"),
            test_line("    edx = -(edx);"),
        ];
        fold_carry_masks(&mut lines);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines[0].text, "    edx = ((ecx != 0) ? 1 : 0);");

        let mut lines = vec![
            test_line("    rdi = ((rax != 0) ? -1 : 0);"),
            test_line("    rdi &= rcx;"),
        ];
        fold_carry_masks(&mut lines);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines[0].text, "    rdi = ((rax != 0) ? rcx : 0);");
    }

    #[test]
    fn store_copies_fold_into_the_store() {
        let mut lines = vec![
            test_line("    rcx = &var_58;"),
            test_line("    var_30 = rcx;"),
            test_line("    rcx = &var_60;"),
        ];
        fold_store_copies(&mut lines);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(lines[0].text, "    var_30 = &var_58;");
        assert_eq!(lines[1].text, "    rcx = &var_60;");

        // A call may read the register implicitly, so the copy stays visible.
        let mut lines = vec![
            test_line("    rcx = &var_58;"),
            test_line("    var_30 = rcx;"),
            test_line("    Api();"),
            test_line("    rcx = 0;"),
        ];
        fold_store_copies(&mut lines);
        assert_eq!(lines.len(), 4, "{lines:?}");
    }

    #[test]
    fn matched_push_pop_pairs_are_hidden() {
        fn body(pushes: Vec<(u64, Reg)>, pops: Vec<(u64, Reg)>) -> BlockLift {
            BlockLift {
                statements: Vec::new(),
                condition: None,
                flow: Flow::Fallthrough,
                written: BTreeSet::new(),
                calls: BTreeSet::new(),
                pushes,
                pops,
                stack_net: None,
                stack_min: None,
                stack_pointers: Vec::new(),
                alias_writes: Vec::new(),
                arg_writes: Vec::new(),
                literals: Vec::new(),
            }
        }
        let entry = body(vec![(0x10, Reg::Rbx), (0x11, Reg::Rdi)], Vec::new());
        let exit = body(Vec::new(), vec![(0x50, Reg::Rdi), (0x51, Reg::Rbx)]);
        let blocks = [
            (Address::new(0x1000), &entry),
            (Address::new(0x1040), &exit),
        ];
        let hidden = hidden_push_pop_pairs(blocks.into_iter(), Address::new(0x1060));
        assert_eq!(hidden, BTreeSet::from([0x10, 0x11, 0x50, 0x51]));

        // An unbalanced push (a stack argument) keeps everything visible.
        let entry = body(vec![(0x10, Reg::Rbx), (0x11, Reg::Rcx)], Vec::new());
        let exit = body(Vec::new(), vec![(0x50, Reg::Rbx)]);
        let blocks = [
            (Address::new(0x1000), &entry),
            (Address::new(0x1040), &exit),
        ];
        let hidden = hidden_push_pop_pairs(blocks.into_iter(), Address::new(0x1060));
        assert_eq!(hidden, BTreeSet::from([0x10, 0x50]));

        // A pop past the function end belongs to the next function.
        let entry = body(vec![(0x10, Reg::Rbx)], Vec::new());
        let exit = body(Vec::new(), vec![(0x90, Reg::Rbx)]);
        let blocks = [
            (Address::new(0x1000), &entry),
            (Address::new(0x2000), &exit),
        ];
        let hidden = hidden_push_pop_pairs(blocks.into_iter(), Address::new(0x1100));
        assert!(hidden.is_empty(), "{hidden:?}");
    }

    fn analyzed_corpus() -> Option<(Session, TempDir)> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/x64/corpus.exe");
        let bytes = std::fs::read(path).ok()?;
        let dir = TempDir::new().unwrap();
        let project = dir.path().join("l1.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = analyze(&mut session).unwrap();
        persist(&mut session, &result).unwrap();
        Some((session, dir))
    }

    fn analyzed_x86_corpus() -> Option<(Session, TempDir)> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/x86/corpus.exe");
        let bytes = std::fs::read(path).ok()?;
        let dir = TempDir::new().unwrap();
        let project = dir.path().join("l1-x86.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = analyze(&mut session).unwrap();
        persist(&mut session, &result).unwrap();
        Some((session, dir))
    }

    fn function_start(session: &Session, name: &str) -> Option<Address> {
        let address = session.addr_of_name(name).ok().flatten()?;
        session
            .functions()
            .ok()?
            .iter()
            .find(|function| function.start == address)
            .map(|function| function.start)
    }

    fn render(session: &mut Session, name: &str) -> String {
        let start = function_start(session, name).unwrap_or_else(|| panic!("{name} missing"));
        pseudocode_l1(session, start).unwrap().text()
    }

    #[test]
    fn nested_loop_leaving_parent_flattens_only_that_loop() {
        let mut loops = BTreeMap::new();
        loops.insert(0, BTreeSet::from([0, 1, 2, 3, 4]));
        loops.insert(2, BTreeSet::from([2, 3]));
        let mut successors = vec![Vec::new(); 6];
        successors[1] = vec![2];
        successors[2] = vec![3];
        successors[3] = vec![2, 5]; // inner latch leaves the outer loop to block 5
        successors[4] = vec![0];
        assert_eq!(
            flat_loop_headers_of(&loops, &successors),
            BTreeSet::from([0])
        );

        // A single loop exiting normally needs no labelled body.
        let mut single = BTreeMap::new();
        single.insert(0, BTreeSet::from([0, 1]));
        let successors = vec![vec![1], vec![0, 2], vec![]];
        assert!(flat_loop_headers_of(&single, &successors).is_empty());

        // A nested loop whose exits stay inside the parent keeps the parent structured.
        let mut inner_ok = BTreeMap::new();
        inner_ok.insert(0, BTreeSet::from([0, 1, 2, 3]));
        inner_ok.insert(1, BTreeSet::from([1, 2]));
        let mut successors = vec![Vec::new(); 5];
        successors[0] = vec![1];
        successors[1] = vec![2, 3];
        successors[2] = vec![1, 3];
        successors[3] = vec![0, 4];
        assert!(flat_loop_headers_of(&inner_ok, &successors).is_empty());
    }

    #[test]
    fn string_literals_keep_their_data_address() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "corpus_banner").unwrap();
        let code = pseudocode_l2(&mut session, start).unwrap();
        assert!(
            code.lines
                .iter()
                .any(|line| line.text.contains("return \"ReCraft analysis corpus\";")),
            "banner literal missing:\n{}",
            code.text()
        );
        let (_, addr) = code
            .literals
            .iter()
            .find(|(text, _)| text == "\"ReCraft analysis corpus\"")
            .unwrap_or_else(|| panic!("no banner literal in {:?}", code.literals));
        let mut bytes = [0u8; 32];
        let read = session.read_at(*addr, &mut bytes).unwrap();
        assert!(
            bytes[..read].starts_with(b"ReCraft analysis corpus\0"),
            "the recorded address must be the string's start: {addr:?} in {:?}",
            code.literals
        );
    }

    #[test]
    fn branchy_becomes_if() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let text = render(&mut session, "branchy");
        assert!(text.contains("if ("), "{text}");
        assert!(text.contains('{') && text.contains('}'), "{text}");
    }

    #[test]
    fn loopy_becomes_while() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let text = render(&mut session, "loopy");
        assert!(text.contains("while ("), "{text}");
    }

    #[test]
    fn switch_replaces_the_jump_table_dispatch() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "switcher_big").unwrap();
        let text = pseudocode_l1(&mut session, start).unwrap().text();
        assert!(text.contains("switch (ecx)"), "{text}");
        assert!(!text.contains("__asm(\"jmp"), "dispatch left:\n{text}");
        assert!(!text.contains("rdx = 0x140000000"), "base load:\n{text}");
    }

    #[test]
    fn only_register_arithmetic_before_an_indirect_jump_is_dispatch_noise() {
        let lines = vec![
            test_line("    rax = (int64)(arg1);"),
            test_line("    rdx = 0x140000000;"),
            test_line("    ecx = *(u32*)(((rdx + (rax * 4)) + 0x1550));"),
            test_line("    rcx += rdx;"),
            test_line("    __asm(\"jmp rcx\");"),
        ];
        assert!(is_dispatch_only(&lines));
        let mut with_store = lines.clone();
        with_store.insert(1, test_line("    *(u32*)(0x140003000) = 1;"));
        assert!(!is_dispatch_only(&with_store));
        let mut with_call = lines.clone();
        with_call.insert(0, test_line("    sub_1();"));
        assert!(!is_dispatch_only(&with_call));
    }

    #[test]
    fn machine_register_spellings_are_recognised() {
        for name in ["rax", "eax", "ax", "al", "r8", "r8d", "r15b", "spl", "ip"] {
            assert!(is_machine_register(name), "{name}");
        }
        for name in ["arg1", "var_20", "r16", "eaxx", "xmm0", "dil2"] {
            assert!(!is_machine_register(name), "{name}");
        }
    }

    #[test]
    fn switch_is_structured_from_jump_table() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let text = render(&mut session, "switcher_big");
        // The selector register is traced back through the index move.
        assert!(text.contains("switch (ecx)"), "{text}");
        assert!(text.contains("case 0:"), "{text}");
        assert!(text.contains("case 19:"), "{text}");
        // Simple leaf cases are inlined; no goto fallback remains.
        assert!(text.contains("case 0: {"), "{text}");
        assert!(!text.contains("goto loc_"), "{text}");
    }

    #[test]
    fn l2_switch_selector_becomes_argument() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "switcher_big").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("switch (arg1)"), "{text}");
    }

    #[test]
    fn recursive_has_if_and_self_reference() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let text = render(&mut session, "recursive");
        assert!(text.contains("if ("), "{text}");
        assert!(text.contains("recursive"), "{text}");
    }

    #[test]
    fn l2_names_arguments_and_stack_slots() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "add_leaf").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("int64 arg1"), "{text}");
        assert!(text.contains("int64 arg2"), "{text}");
        assert!(
            text.contains("(arg1 +") && text.contains("arg2"),
            "argument reads were not renamed:\n{text}"
        );

        // `uses_helper` keeps a real stack local (its slot is written from a register, not
        // from an argument, so it cannot be folded away); the argument/loop-only test cases
        // have their spills hidden as callee-save bookkeeping.
        let nested = function_start(&session, "uses_helper").unwrap();
        let text = pseudocode_l2(&mut session, nested).unwrap().text();
        assert!(text.contains("var_"), "stack slots were not named:\n{text}");
    }

    #[test]
    fn l2_signature_has_return_type_and_abi() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "add_leaf").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(
            text.contains("int32 add_leaf(int64 arg1, int64 arg2)"),
            "{text}"
        );
    }

    #[test]
    fn carried_argument_value_is_dropped_when_its_register_is_written() {
        let address = |value: u64| Address::new(value);
        let block = |index: usize, start: u64, edges: Vec<(usize, EdgeKind)>| BasicBlock {
            index,
            start: address(start),
            end: address(start + 0x10),
            insns: Vec::new(),
            edges,
        };
        let cfg = Cfg {
            function: address(0x1000),
            entry: 0,
            blocks: vec![
                block(0, 0x1000, vec![(1, EdgeKind::Unconditional)]),
                block(1, 0x1010, vec![(2, EdgeKind::Unconditional)]),
                block(2, 0x1020, Vec::new()),
            ],
            external: Vec::new(),
            jump_tables: BTreeMap::new(),
        };
        let lift =
            |arg_writes: Vec<(usize, Option<String>, BTreeSet<Reg>)>, written: Reg| BlockLift {
                statements: Vec::new(),
                condition: None,
                flow: Flow::Fallthrough,
                written: BTreeSet::from([written]),
                calls: BTreeSet::new(),
                pushes: Vec::new(),
                pops: Vec::new(),
                stack_net: None,
                stack_min: None,
                stack_pointers: Vec::new(),
                alias_writes: Vec::new(),
                arg_writes,
                literals: Vec::new(),
            };
        // Block 0 loads rcx from rbx; block 1 overwrites rbx, so block 2 cannot trust it.
        let bodies = vec![
            lift(
                vec![(
                    0,
                    Some("*(u64*)(rbx)".to_owned()),
                    BTreeSet::from([Reg::Rbx]),
                )],
                Reg::Rcx,
            ),
            lift(Vec::new(), Reg::Rbx),
            lift(Vec::new(), Reg::Rax),
        ];
        let values = entry_arg_value_map(&cfg, &bodies);
        assert_eq!(values[1][0].as_deref(), Some("*(u64*)(rbx)"));
        assert!(values[2][0].is_none());
    }
    #[test]
    fn address_constants_name_what_they_point_at() {
        let mut functions = HashMap::new();
        functions.insert(0x140001260, "sub_140001260".to_owned());
        let mut literals = HashMap::new();
        literals.insert(0x14002b950, "\"Software\"".to_owned());
        let data = [(0x1400341f8, 0x140034800)];
        let mut lines = vec![
            test_line("    rax = 0x140001260;"),
            test_line("    rcx = 0x14002b950;"),
            test_line("    rdx = 0x1400341f8;"),
            // Neither a function nor data: the constant stays.
            test_line("    rbx = 0x2b992ddfa232;"),
        ];
        name_address_constants(&mut lines, &functions, &literals, &data);
        assert_eq!(lines[0].text, "    rax = sub_140001260;");
        assert_eq!(lines[1].text, "    rcx = \"Software\";");
        assert_eq!(lines[2].text, "    rdx = off_1400341f8;");
        assert_eq!(lines[3].text, "    rbx = 0x2b992ddfa232;");
    }
    #[test]
    fn global_data_reads_as_a_named_word() {
        let mut lines = vec![
            test_line("    eax = *(u32*)(0x140034460);"),
            test_line("    *(u64*)(0x140036648) = rax;"),
            test_line("    rcx = *(u8*)(0x140035390);"),
            // A computed address is not a global and keeps its cast.
            test_line("    rdx = *(u32*)(0x140034460 + 4);"),
        ];
        name_global_data(&mut lines);
        assert_eq!(lines[0].text, "    eax = dword_140034460;");
        assert_eq!(lines[1].text, "    qword_140036648 = rax;");
        assert_eq!(lines[2].text, "    rcx = byte_140035390;");
        assert_eq!(lines[3].text, "    rdx = *(u32*)(0x140034460 + 4);");
    }

    #[test]
    fn void_pointer_arguments_get_field_names() {
        let mut lines = vec![
            test_line("    eax = *(u32*)((arg1 + 0x10));"),
            test_line("    ecx = *(u32*)((arg1 + 0x10));"),
            test_line("    edx = *(u8*)(arg1);"),
            test_line("    esi = *(u32*)((arg2 + 0x18));"),
            test_line("    edi = *(u8*)((arg2 + 0x18));"),
        ];
        name_void_pointer_fields(&mut lines);
        assert_eq!(lines[0].text, "    eax = arg1->field_10;");
        assert_eq!(lines[1].text, "    ecx = arg1->field_10;");
        assert_eq!(lines[2].text, "    edx = arg1->field_0;");
        // Two widths at one offset keep their casts: a field name cannot say which.
        assert_eq!(lines[3].text, "    esi = *(u32*)((arg2 + 0x18));");
        assert_eq!(lines[4].text, "    edi = *(u8*)((arg2 + 0x18));");
    }
    #[test]
    fn handle_arguments_get_a_name() {
        let mut lines = vec![
            test_line("    var_28 = 0;"),
            test_line("    CloseHandle(var_28);"),
        ];
        name_handle_slots(&mut lines);
        assert_eq!(lines[0].text, "    hObject = 0;");
        assert_eq!(lines[1].text, "    CloseHandle(hObject);");

        // The more telling kind wins over the generic handle name.
        let mut lines = vec![
            test_line("    ReleaseMutex(var_30);"),
            test_line("    CloseHandle(var_30);"),
        ];
        name_handle_slots(&mut lines);
        assert_eq!(lines[0].text, "    ReleaseMutex(hMutex);");
        assert_eq!(lines[1].text, "    CloseHandle(hMutex);");

        // The argument index of the entry is used (ReleaseDC's device context is second).
        let mut lines = vec![test_line("    ReleaseDC(hwnd, var_40);")];
        name_handle_slots(&mut lines);
        assert_eq!(lines[0].text, "    ReleaseDC(hwnd, hdc);");

        // A computed argument is not a slot and keeps its text.
        let mut lines = vec![test_line("    CloseHandle(var_18 + 4);")];
        name_handle_slots(&mut lines);
        assert_eq!(lines[0].text, "    CloseHandle(var_18 + 4);");
    }
    #[test]
    fn structure_instances_get_a_name() {
        let mut lines = vec![
            test_line("    var_30 = 0;"),
            test_line("    GetClientRect(hwnd, &var_30);"),
            test_line("    edi = var_30.bottom;"),
        ];
        name_known_struct_fields(&mut lines);
        name_structure_instances(&mut lines);
        assert_eq!(lines[0].text, "    rect = 0;");
        assert_eq!(lines[1].text, "    GetClientRect(hwnd, &rect);");
        assert_eq!(lines[2].text, "    edi = rect.bottom;");

        // A name already used by the function gets a suffix.
        let mut lines = vec![
            test_line("    rect = 1;"),
            test_line("    var_40 = 0;"),
            test_line("    GetClientRect(hwnd, &var_40);"),
        ];
        name_known_struct_fields(&mut lines);
        name_structure_instances(&mut lines);
        assert_eq!(lines[1].text, "    rect2 = 0;");
        assert_eq!(lines[2].text, "    GetClientRect(hwnd, &rect2);");
        assert_eq!(lines[0].text, "    rect = 1;");
    }
    #[test]
    fn a_jump_to_the_entry_labels_the_body_not_the_header() {
        let at = |text: &str, address: u64| PseudoLine {
            addr: Address::new(address),
            text: text.to_owned(),
        };
        let mut lines = vec![
            at("// L2 structure recovery (2 blocks)", 0x1000),
            at("int32 sub_40(int64 arg1) {", 0x1000),
            at("    // entry comment", 0x1000),
            at("    eax = arg1;", 0x1000),
            at("    goto loc_1000;", 0x1010),
            at("}", 0x1010),
        ];
        materialize_missing_labels(&mut lines);
        // The label belongs to the first statement, below the header and its comment.
        assert_eq!(lines[0].text, "// L2 structure recovery (2 blocks)");
        assert_eq!(lines[1].text, "int32 sub_40(int64 arg1) {");
        assert_eq!(lines[2].text, "    // entry comment");
        assert_eq!(lines[3].text, "    loc_1000:");
        assert_eq!(lines[4].text, "    eax = arg1;");
    }
    #[test]
    fn an_argument_spilled_once_is_that_argument() {
        let mut lines = vec![
            test_line("    var_40 = arg4;"),
            test_line("    if (var_40 == 0) {"),
            test_line("        call(var_40);"),
            test_line("    }"),
        ];
        propagate_argument_spills(&mut lines);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].text, "    if (arg4 == 0) {");
        assert_eq!(lines[1].text, "        call(arg4);");

        // A slot written twice, or one whose address is taken, stays a local.
        let mut lines = vec![
            test_line("    var_20 = arg1;"),
            test_line("    var_20 = 0;"),
            test_line("    var_30 = arg2;"),
            test_line("    read(&var_30);"),
        ];
        propagate_argument_spills(&mut lines);
        assert_eq!(lines[0].text, "    var_20 = arg1;");
        assert_eq!(lines[2].text, "    var_30 = arg2;");
    }
    #[test]
    fn stack_arguments_are_typed_by_the_access_through_them() {
        let mut lines = vec![
            test_line("// L2 params: arg1=rcx"),
            test_line("int64 sub_40(int64 arg1) {"),
            test_line("    r8 = var_78;"),
            test_line("    *(u64*)(r8) = arg1;"),
            test_line("    return 0;"),
            test_line("}"),
        ];
        name_stack_arguments(&mut lines, 0x48);
        // var_78 is entry+0x30 (arg6) and is written through as a 64-bit pointer.
        assert!(lines[1].text.contains("int64* arg6"), "{}", lines[1].text);
        assert_eq!(lines[2].text, "    r8 = arg6;");
    }
    #[test]
    fn stack_arguments_name_the_callers_parameters() {
        let mut lines = vec![
            test_line("// L2 params: arg1=rcx"),
            test_line("int64 sub_40(int64 arg1) {"),
            test_line("    rcx = var_70;"),
            test_line("    rax = var_78;"),
            test_line("    var_30 = rax;"),
            test_line("    return rax;"),
            test_line("}"),
        ];
        // Frame 0x48: var_70 sits at entry+0x28 (arg5), var_78 at entry+0x30 (arg6).
        name_stack_arguments(&mut lines, 0x48);
        assert_eq!(
            lines[1].text,
            "int64 sub_40(int64 arg1, int64 arg2, int64 arg3, int64 arg4, int64 arg5, int64 arg6) {"
        );
        assert_eq!(
            lines[0].text,
            "// L2 params: arg1=rcx, arg5+=stack (through arg6)"
        );
        assert_eq!(lines[2].text, "    rcx = arg5;");
        assert_eq!(lines[3].text, "    rax = arg6;");
        // A slot inside the frame is a local and keeps its name.
        assert_eq!(lines[4].text, "    var_30 = rax;");
    }
    #[test]
    fn constants_move_into_their_comparison() {
        let mut lines = vec![
            test_line("    eax = 0x111;"),
            test_line("    var_40 = arg4;"),
            test_line("    if (arg2 > eax) {"),
            test_line("        eax = 0x233;"),
            test_line("        if (ebp > eax) {"),
        ];
        fold_constant_registers(&mut lines);
        // The constant lands in the comparison and both dead assignments go away.
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].text, "    var_40 = arg4;");
        assert_eq!(lines[1].text, "    if (arg2 > 0x111) {");
        assert_eq!(lines[2].text, "        if (ebp > 0x233) {");

        // A live temporary keeps its assignment.
        let mut lines = vec![
            test_line("    rbx = 0x2b992ddfa232;"),
            test_line("    if (rax == rbx) {"),
            test_line("    }"),
            test_line("    rcx = rbx;"),
        ];
        fold_constant_registers(&mut lines);
        assert_eq!(lines[0].text, "    rbx = 0x2b992ddfa232;");
        assert_eq!(lines[1].text, "    if (rax == 0x2b992ddfa232) {");
    }
    #[test]
    fn noop_statements_are_dropped() {
        let mut lines = vec![
            test_line("    rax = rax;"),
            test_line("    var_10 = var_10;"),
            test_line("    if (eax == 0) {"),
            test_line("    }"),
            test_line("    var_20 = 1;"),
        ];
        fold_constant_registers(&mut lines);
        drop_noop_statements(&mut lines);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "    var_20 = 1;");
    }
    #[test]
    fn plain_argument_values_can_be_repeated() {
        assert!(crate::plain_argument_value("0"));
        assert!(crate::plain_argument_value("var_30"));
        assert!(crate::plain_argument_value("&var_40"));
        // A member read is a memory load too.
        assert!(!crate::plain_argument_value("arg1->field_0"));
        // A load or an arithmetic expression would be evaluated twice.
        assert!(!crate::plain_argument_value("*(u64*)((rcx + 8))"));
        assert!(!crate::plain_argument_value("(rax + 0x10)"));
        assert!(!crate::plain_argument_value(""));
    }
    #[test]
    fn cross_block_calls_take_the_sibling_arity() {
        let mut lines = vec![
            test_line("    sub_140002088(a, b, c);"),
            test_line("    sub_140002088();"),
            test_line("    sub_140003000(1, 2, 3, 4, 5);"),
            test_line("    sub_140003000();"),
        ];
        fill_cross_block_arguments(&mut lines);
        assert_eq!(lines[1].text, "    sub_140002088(rcx, rdx, r8);");
        // Five arguments place one on the stack, which the register names cannot locate.
        assert_eq!(lines[3].text, "    sub_140003000();");

        // The same evidence works for an import, unless the signature table already knows.
        let mut lines = vec![
            test_line("    SomeApi(a, b);"),
            test_line("    SomeApi();"),
            test_line("    CloseHandle(a);"),
            test_line("    CloseHandle();"),
        ];
        fill_cross_block_arguments(&mut lines);
        assert_eq!(lines[1].text, "    SomeApi(rcx, rdx);");
        assert_eq!(lines[3].text, "    CloseHandle();");
    }
    #[test]
    fn adjacent_temporaries_are_inlined_into_their_reader() {
        let mut lines = vec![
            test_line("    eax = *(u64*)(rbx);"),
            test_line("    sub_140001000(eax);"),
        ];
        inline_adjacent_temporaries(&mut lines);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "    sub_140001000(*(u64*)(rbx));");

        // A stack slot is a variable, not an expression: its address must stay intact.
        let mut lines = vec![
            test_line("    rax = 1;"),
            test_line("    var_30 = rax;"),
            test_line("    SetThreadpoolTimer(h, &var_30, 0, 0);"),
        ];
        inline_adjacent_temporaries(&mut lines);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "    var_30 = 1;");
        assert_eq!(lines[1].text, "    SetThreadpoolTimer(h, &var_30, 0, 0);");

        // A temporary read twice cannot be folded.
        let mut lines = vec![
            test_line("    eax = var_10;"),
            test_line("    f(eax, eax);"),
        ];
        inline_adjacent_temporaries(&mut lines);
        assert_eq!(lines.len(), 2);
    }
    #[test]
    fn known_import_arities_are_sorted_and_stable() {
        assert_eq!(crate::import_arity("CloseHandle"), Some(1));
        assert_eq!(crate::import_arity("GetProcessHeap"), Some(0));
        assert_eq!(crate::import_arity("RegQueryInfoKeyW"), Some(12));
        // An API the table does not know falls back to the register-setup rule.
        assert_eq!(crate::import_arity("PeekMessageW"), None);
        assert!(
            crate::IMPORT_ARITY
                .windows(2)
                .all(|pair| pair[0].0 < pair[1].0),
            "the table must stay sorted by API name"
        );
    }
    #[test]
    fn msg_arguments_name_their_fields() {
        let mut lines = vec![
            test_line("    var_40 = xmm0;"),
            test_line("    GetMessageW(&var_40, 0, 0, 0);"),
            test_line("    if (var_48 == 0x50) {"),
            test_line("        return var_50;"),
            test_line("    }"),
        ];
        name_known_struct_fields(&mut lines);
        assert_eq!(lines[2].text, "    if (var_40.message == 0x50) {");
        assert_eq!(lines[3].text, "        return var_40.wParam;");

        // A second message call must not re-prefix the fields already named.
        let mut lines = vec![
            test_line("    GetMessageW(&var_40, 0, 0, 0);"),
            test_line("    TranslateMessage(&var_40);"),
            test_line("    return var_50;"),
        ];
        name_known_struct_fields(&mut lines);
        assert_eq!(lines[2].text, "    return var_40.wParam;");
    }
    #[test]
    fn rect_arguments_name_their_fields() {
        let mut lines = vec![
            test_line("    var_30 = xmm0;"),
            test_line("    GetClientRect(hwnd, &var_30);"),
            test_line("    edi = var_3c;"),
            test_line("    edi -= var_34;"),
            test_line("    var_38 = 0;"),
            test_line("    rax = *(u64*)((var_3c + 8));"),
        ];
        name_known_struct_fields(&mut lines);
        assert_eq!(lines[1].text, "    GetClientRect(hwnd, &var_30);");
        assert_eq!(lines[2].text, "    edi = var_30.bottom;");
        assert_eq!(lines[3].text, "    edi -= var_30.top;");
        assert_eq!(lines[4].text, "    var_30.right = 0;");
        // A pointer base keeps the plain slot name: there the slot holds a pointer, not a
        // field.
        assert_eq!(lines[5].text, "    rax = *(u64*)((var_3c + 8));");
        // The wide clear before the call is not turned into a first-field store.
        assert_eq!(lines[0].text, "    var_30 = xmm0;");
    }
    #[test]
    fn tail_call_takes_adjacent_argument_setups() {
        let mut lines = vec![test_line("    rdx = rsi;"), test_line("    rcx = var_10;")];
        let arguments = take_adjacent_arguments(&mut lines, 2);
        assert_eq!(arguments, vec!["var_10".to_owned(), "rsi".to_owned()]);
        assert!(lines.is_empty());

        // A setup that is not the next ABI register stops the scan.
        let mut lines = vec![test_line("    rcx = var_10;"), test_line("    var_20 = 1;")];
        assert!(take_adjacent_arguments(&mut lines, 4).is_empty());
        assert_eq!(lines.len(), 2);
    }
    #[test]
    fn infinite_do_while_is_written_as_a_while_loop() {
        let mut lines = vec![
            test_line("    do {"),
            test_line("        var_10 = 1;"),
            test_line("    } while (1);"),
        ];
        normalize_infinite_loops(&mut lines);
        assert_eq!(lines[0].text, "    while (1) {");
        assert_eq!(lines[2].text, "    }");

        // A loop with a real condition keeps its `do` form.
        let mut lines = vec![
            test_line("    do {"),
            test_line("        var_10 = 1;"),
            test_line("    } while (var_10 != 0);"),
        ];
        normalize_infinite_loops(&mut lines);
        assert_eq!(lines[0].text, "    do {");
    }
    #[test]
    fn single_use_frame_pointer_setup_is_hidden() {
        let mut lines = vec![
            test_line("int32 f(void) {"),
            test_line("    rbp = &var_100;"),
            test_line("    var_10 = 0;"),
            test_line("    return 0;"),
            test_line("}"),
        ];
        hide_frame_pointer_setups(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("rbp"), "{text}");

        // A register that is named again keeps its definition.
        let mut lines = vec![
            test_line("int32 f(void) {"),
            test_line("    rbp = &var_100;"),
            test_line("    sub_140001000(rbp);"),
            test_line("    return 0;"),
            test_line("}"),
        ];
        hide_frame_pointer_setups(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("rbp = &var_100;"), "{text}");
    }
    #[test]
    fn alias_dataflow_requires_every_path_to_agree() {
        let address = |value: u64| Address::new(value);
        let block = |index: usize, start: u64, edges: Vec<(usize, EdgeKind)>| BasicBlock {
            index,
            start: address(start),
            end: address(start + 0x10),
            insns: Vec::new(),
            edges,
        };
        let cfg = Cfg {
            function: address(0x1000),
            entry: 0,
            blocks: vec![
                block(
                    0,
                    0x1000,
                    vec![(1, EdgeKind::Taken), (2, EdgeKind::Fallthrough)],
                ),
                block(1, 0x1010, vec![(3, EdgeKind::Unconditional)]),
                block(2, 0x1020, vec![(3, EdgeKind::Unconditional)]),
                block(3, 0x1030, Vec::new()),
            ],
            external: Vec::new(),
            jump_tables: BTreeMap::new(),
        };
        // Only one arm copies arg4 into r14: the join cannot trust the alias.
        let effects = vec![
            Vec::new(),
            vec![(Reg::R14, Some(Reg::R9))],
            Vec::new(),
            Vec::new(),
        ];
        let is_argument = |reg: Reg| reg == Reg::R9;
        let aliases = entry_alias_map(&cfg, &effects, &is_argument);
        assert!(!aliases[3].contains_key(&Reg::R14));
        // Both arms copying the same argument makes it hold at the join.
        let effects = vec![
            Vec::new(),
            vec![(Reg::R14, Some(Reg::R9))],
            vec![(Reg::R14, Some(Reg::R9))],
            Vec::new(),
        ];
        let is_argument = |reg: Reg| reg == Reg::R9;
        let aliases = entry_alias_map(&cfg, &effects, &is_argument);
        assert_eq!(aliases[3].get(&Reg::R14), Some(&Reg::R9));
        // An overwrite in one arm clears the other arm's alias.
        let effects = vec![
            Vec::new(),
            vec![(Reg::R14, Some(Reg::R9))],
            vec![(Reg::R14, None)],
            Vec::new(),
        ];
        let is_argument = |reg: Reg| reg == Reg::R9;
        let aliases = entry_alias_map(&cfg, &effects, &is_argument);
        assert!(!aliases[3].contains_key(&Reg::R14));
    }

    #[test]
    fn stack_layout_resolves_a_register_restore() {
        let address = |value: u64| Address::new(value);
        let rsp = Expr::Reg(RegView::full(Reg::Rsp));
        let r11 = Expr::Reg(RegView::full(Reg::R11));
        let exprs = vec![
            rsp,
            r11,
            Expr::Const(0x50),
            Expr::Bin {
                op: recraft_ir::BinOp::Add,
                lhs: 0,
                rhs: 2,
            },
        ];
        let stmt = |reg: Reg, value: ExprId| Stmt::WriteReg {
            reg: RegView::full(reg),
            value,
        };
        let items = vec![
            LiftedInsn {
                addr: address(0x1000),
                // `lea r11, [rsp+0x50]`
                stmts: vec![stmt(Reg::R11, 3)],
                flow: Flow::Fallthrough,
            },
            LiftedInsn {
                addr: address(0x1004),
                stmts: vec![stmt(Reg::Rsp, 1)],
                flow: Flow::Fallthrough,
            },
        ];
        // The second instruction is `mov rsp, r11`; the layout must use r11's offset.
        assert_eq!(stack_layout(&exprs, &items), (Some(0x50), Some(0)));
    }
    #[test]
    fn dataflow_marks_arguments_written_on_any_path() {
        let address = |value: u64| Address::new(value);
        let block = |index: usize, start: u64, edges: Vec<(usize, EdgeKind)>| BasicBlock {
            index,
            start: address(start),
            end: address(start + 0x10),
            insns: Vec::new(),
            edges,
        };
        let cfg = Cfg {
            function: address(0x1000),
            entry: 0,
            blocks: vec![
                block(
                    0,
                    0x1000,
                    vec![(1, EdgeKind::Taken), (2, EdgeKind::Fallthrough)],
                ),
                block(1, 0x1010, vec![(3, EdgeKind::Unconditional)]),
                block(2, 0x1020, vec![(3, EdgeKind::Unconditional)]),
                block(3, 0x1030, Vec::new()),
            ],
            external: Vec::new(),
            jump_tables: BTreeMap::new(),
        };
        let mut written = vec![BTreeSet::new(); 4];
        written[1].insert(Reg::Rcx);
        let seeds = entry_written(&cfg, &written);
        // Entering the writing block still holds the original argument.
        assert!(!seeds[1].contains(&Reg::Rcx));
        // The other arm is untouched.
        assert!(!seeds[2].contains(&Reg::Rcx));
        // At the join one path has overwritten rcx: may-write unions it.
        assert!(seeds[3].contains(&Reg::Rcx));
    }

    #[test]
    fn l2_keeps_the_value_of_a_folded_counter_update() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "corpus_counter_bump").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        // The increment stays visible *and* the loaded value is not lost to the caller. The
        // accumulation folds into the load (`eax = (dword_… + 1);`), so the assertions look at
        // the increment and at the store that keeps the value in memory.
        assert!(text.contains("dword_140003028 + 1)"), "{text}");
        assert!(text.contains("dword_140003028 = eax;"), "{text}");
        assert!(text.contains("return eax;"), "{text}");
    }

    #[test]
    fn l2_ignores_zeroing_idioms_and_follows_pointer_origins() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "bytes_op").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        // `xor r8d, r8d` must not become a parameter, and the pointer copied through
        // `r10`/`rdx` still belongs to the first argument.
        assert!(
            text.contains("int32 bytes_op(void* arg1, int32 arg2)"),
            "{text}"
        );
        assert!(!text.contains("arg3"), "{text}");
    }

    #[test]
    fn l2_types_scalar_arguments_by_read_width() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "count_bits").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("int32 count_bits(int32 arg1)"), "{text}");
    }

    #[test]
    fn l2_pointer_parameters_show_pointer_types() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "str_probe").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("int32 str_probe(char* arg1)"), "{text}");
    }

    #[test]
    fn l2_signature_follows_a_forwarded_argument() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "call_str").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("int32 call_str(int64 arg1)"), "{text}");
        assert!(text.contains("str_probe(arg1)"), "{text}");
    }

    #[test]
    fn l2_renders_struct_field_access_through_pointer_arguments() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "field_probe").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("int32 field_probe(int32* arg1)"), "{text}");
        assert!(text.contains("arg1->field_10"), "{text}");
    }

    #[test]
    fn user_prototype_is_shown_above_the_signature_and_undoes() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "field_probe").unwrap();
        session
            .execute(recraft_core::Command::SetPrototype {
                addr: start,
                text: Some("int field_probe(const int* values)".to_owned()),
            })
            .unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(
            text.contains("// prototype: int field_probe(const int* values)"),
            "{text}"
        );
        session.undo().unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(!text.contains("// prototype:"), "{text}");
    }

    #[test]
    fn applied_struct_type_names_fields_in_pseudocode() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "field_probe").unwrap();
        session
            .execute(recraft_core::Command::DefineStruct {
                def: recraft_core::StructDef {
                    name: "Probe".to_owned(),
                    size: 0x18,
                    fields: vec![recraft_core::StructField {
                        offset: 0x10,
                        name: "value".to_owned(),
                        kind: "i32".to_owned(),
                    }],
                },
            })
            .unwrap();
        session
            .execute(recraft_core::Command::ApplyType {
                func: start,
                slot: 0,
                struct_name: Some("Probe".to_owned()),
            })
            .unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("arg1->value"), "{text}");
        assert!(!text.contains("field_10"), "{text}");

        // Clearing the binding restores the offset-style fallback.
        session
            .execute(recraft_core::Command::ApplyType {
                func: start,
                slot: 0,
                struct_name: None,
            })
            .unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("arg1->field_10"), "{text}");
    }

    #[test]
    fn user_prototype_overrides_signature_and_argument_names() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "field_probe").unwrap();
        session
            .execute(recraft_core::Command::SetPrototype {
                addr: start,
                text: Some("int __stdcall field_probe(const int* values, int count)".to_owned()),
            })
            .unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(
            text.contains("int32 __stdcall field_probe(int32* values, int32 count)"),
            "{text}"
        );
        assert!(text.contains("values->field_10"), "{text}");
        session.undo().unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(!text.contains("__stdcall"), "{text}");
    }

    #[test]
    fn scaled_index_addressing_renders_array_subscript() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "array_probe").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        // The index is only read through `edx`, so it is a scalar `int`.
        assert!(
            text.contains("int32 array_probe(int32* arg1, int32 arg2)"),
            "{text}"
        );
        assert!(text.contains("arg1["), "{text}");
        assert!(!text.contains("*(u32*)((arg1 +"), "{text}");
    }

    #[test]
    fn x86_prototype_maps_stack_arguments() {
        let Some((mut session, _dir)) = analyzed_x86_corpus() else {
            return;
        };
        let start = function_start(&session, "_stdcall_add@8").expect("stdcall_add");
        session
            .execute(recraft_core::Command::SetPrototype {
                addr: start,
                text: Some("int __stdcall stdcall_add(int a, int b)".to_owned()),
            })
            .unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(
            text.contains("int32 __stdcall stdcall_add(int32 a, int32 b)"),
            "{text}"
        );
        assert!(text.contains("return (a + b);"), "{text}");
    }

    #[test]
    fn early_exit_chains_and_guard_then_loop_are_structured() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        for name in ["early_chain", "bytes_op"] {
            let start = function_start(&session, name).unwrap();
            let text = pseudocode_l1(&mut session, start).unwrap().text();
            assert!(
                !text.contains("goto loc_"),
                "{name} still has a goto: {text}"
            );
        }
        let start = function_start(&session, "bytes_op").unwrap();
        let text = pseudocode_l1(&mut session, start).unwrap().text();
        // Single-block loops become do-while; no goto fallback remains.
        assert!(text.contains("do {"), "{text}");
        assert!(text.contains("while ("), "{text}");
    }

    #[test]
    fn single_block_loops_emit_do_while() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "count_bits").unwrap();
        let text = pseudocode_l1(&mut session, start).unwrap().text();
        assert!(text.contains("do {"), "{text}");
        assert!(text.contains("} while ("), "{text}");
        assert!(!text.contains("goto loc_"), "{text}");
    }

    #[test]
    fn jump_table_cases_inline_simple_leaves() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "switcher_big").unwrap();
        let text = pseudocode_l1(&mut session, start).unwrap().text();
        assert!(text.contains("switch ("), "{text}");
        assert!(text.contains("case 0: {"), "{text}");
        assert!(text.contains("return;"), "{text}");
        assert!(!text.contains("goto loc_"), "{text}");
    }

    #[test]
    fn complex_loop_headers_stay_structured() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "nested_loops").unwrap();
        let text = pseudocode_l1(&mut session, start).unwrap().text();
        assert!(!text.contains("goto loc_"), "{text}");
        // The loop's single exit test became a real condition instead of `while (1)`.
        assert!(!text.contains("while (1)"), "{text}");
        assert!(text.matches("do {").count() >= 2, "{text}");
    }

    #[test]
    fn inline_conditions_lose_their_extra_parentheses() {
        let mut lines = vec![
            test_line("    if ((eax == 0)) break;"),
            test_line("    if ((rdx != rax)) continue;"),
            test_line("    if ((al != 0)) return sub_1(7);"),
            test_line("    if ((rcx == 0)) goto loc_20;"),
            test_line("    if (a && b) break;"),
        ];
        strip_inline_condition_parentheses(&mut lines);
        assert_eq!(lines[0].text, "    if (eax == 0) break;");
        assert_eq!(lines[1].text, "    if (rdx != rax) continue;");
        assert_eq!(lines[2].text, "    if (al != 0) return sub_1(7);");
        assert_eq!(lines[3].text, "    if (rcx == 0) goto loc_20;");
        assert_eq!(lines[4].text, "    if (a && b) break;");
    }

    #[test]
    fn single_statement_else_arms_become_else_if() {
        let mut lines = vec![
            test_line("    if (a) {"),
            test_line("        x = 1;"),
            test_line("    } else {"),
            test_line("        if (b) {"),
            test_line("            x = 2;"),
            test_line("        }"),
            test_line("    }"),
        ];
        merge_else_if(&mut lines);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[2].text, "    } else if (b) {");
        assert_eq!(lines[3].text, "        x = 2;");
        assert_eq!(lines[4].text, "    }");

        // An else arm with more than the nested `if` keeps its braces.
        let mut lines = vec![
            test_line("    } else {"),
            test_line("        if (b) {"),
            test_line("        }"),
            test_line("        y = 2;"),
            test_line("    }"),
        ];
        merge_else_if(&mut lines);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0].text, "    } else {");
    }

    #[test]
    fn constant_loops_take_the_break_test_as_their_condition() {
        // Test first: it is the loop condition.
        let mut lines = vec![
            test_line("    while (1) {"),
            test_line("        if ((i >= n /* signed */)) break;"),
            test_line("        i += 1;"),
            test_line("    }"),
        ];
        fold_constant_loops(&mut lines);
        assert_eq!(lines[0].text, "    while (i < n /* signed */) {");
        assert_eq!(lines.len(), 3);

        // Test last: it is a do-while latch.
        let mut lines = vec![
            test_line("    while (1) {"),
            test_line("        i += 1;"),
            test_line("        if ((rdx == 0)) break;"),
            test_line("    }"),
        ];
        fold_constant_loops(&mut lines);
        assert_eq!(lines[0].text, "    do {");
        assert_eq!(lines.last().unwrap().text, "    } while (rdx != 0);");

        // A `continue` would reach the latch before the test, so the loop stays as is.
        let mut lines = vec![
            test_line("    while (1) {"),
            test_line("        continue;"),
            test_line("        if ((rdx == 0)) break;"),
            test_line("    }"),
        ];
        fold_constant_loops(&mut lines);
        assert_eq!(lines[0].text, "    while (1) {");
    }

    #[test]
    fn call_sites_show_arguments_with_a_callee_prototype() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "recursive").unwrap();
        let before = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(before.contains("recursive(rcx)"), "{before}");
        session
            .execute(recraft_core::Command::SetPrototype {
                addr: start,
                text: Some("int recursive(int n)".to_owned()),
            })
            .unwrap();
        let after = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(after.contains("int recursive(int n)"), "{after}");
        assert!(after.contains("recursive(rcx)"), "{after}");
        assert!(!after.contains("recursive()"), "{after}");
    }

    #[test]
    fn x86_call_arguments_are_recovered_from_pushes() {
        let Some((mut session, _dir)) = analyzed_x86_corpus() else {
            return;
        };
        let start = function_start(&session, "recursive").expect("x86 recursive");
        session
            .execute(recraft_core::Command::SetPrototype {
                addr: start,
                text: Some("int __cdecl recursive(int n)".to_owned()),
            })
            .unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("recursive("), "{text}");
        assert!(!text.contains("recursive()"), "{text}");
    }

    #[test]
    fn call_arguments_get_pointer_casts_from_callee_prototype() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let callee = function_start(&session, "str_probe").unwrap();
        session
            .execute(recraft_core::Command::SetPrototype {
                addr: callee,
                text: Some("int str_probe(const char* text)".to_owned()),
            })
            .unwrap();
        let caller = function_start(&session, "call_str").unwrap();
        let text = pseudocode_l2(&mut session, caller).unwrap().text();
        assert!(text.contains("str_probe((char*)"), "{text}");
        assert!(!text.contains("str_probe()"), "{text}");
    }

    #[test]
    fn output_is_deterministic() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "branchy").unwrap();
        let first = pseudocode_l1(&mut session, start).unwrap().text();
        let second = pseudocode_l1(&mut session, start).unwrap().text();
        assert_eq!(first, second);
    }

    fn test_line(text: &str) -> PseudoLine {
        PseudoLine {
            addr: Address::new(0x1000),
            text: text.to_owned(),
        }
    }

    #[test]
    fn negate_flips_exactly_one_comparison() {
        assert_eq!(negate("x == 0"), "x != 0");
        assert_eq!(negate("x != 0"), "x == 0");
        assert_eq!(negate("a < b"), "a >= b");
        assert_eq!(negate("a >= b"), "a < b");
        assert_eq!(negate("a <= b"), "a > b");
        assert_eq!(negate("flag"), "!(flag)");
        assert_eq!(negate("a && b"), "!(a && b)");
    }

    #[test]
    fn compound_updates_lvalue_stores_and_negations_are_folded() {
        let mut lines = vec![
            test_line("    eax = *(u32*)(rdi);"),
            test_line("    eax = (eax - 1);"),
            test_line("    *(u32*)(rdi) = eax;"),
            test_line("    rax = (rax ^ rsp);"),
        ];
        fold_compound_assignments(&mut lines);
        fold_load_modify_store(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("*(u32*)(rdi) -= 1;"), "{text}");
        assert!(text.contains("rax ^= rsp;"), "{text}");
        assert!(!text.contains("eax = (eax - 1);"), "{text}");
        assert!(!text.contains("*(u32*)(rdi) = eax;"), "{text}");

        // A later read keeps the loaded value alive: `return;` returns `eax`.
        let mut lines = vec![
            test_line("    eax = *(u32*)(rdi);"),
            test_line("    eax += 1;"),
            test_line("    *(u32*)(rdi) = eax;"),
            test_line("    return;"),
        ];
        fold_load_modify_store(&mut lines);
        assert_eq!(lines.len(), 4, "{lines:?}");
        assert_eq!(lines[0].text, "    eax = *(u32*)(rdi);");

        let mut lines = vec![test_line("    if (!(eax == 0)) {")];
        simplify_negated_conditions(&mut lines);
        assert_eq!(lines[0].text, "    if ((eax != 0)) {");

        let mut lines = vec![
            test_line("    eax = (eax + -0x30);"),
            test_line("    ecx = (ecx | -1);"),
        ];
        fold_compound_assignments(&mut lines);
        assert_eq!(lines[0].text, "    eax -= 0x30;");
        assert_eq!(lines[1].text, "    ecx = -1;");
    }

    #[test]
    fn identical_terminating_arms_merge_into_or() {
        let mut lines = vec![
            test_line("    if ((eax != 0)) {"),
            test_line("        sub_1();"),
            test_line("        return;"),
            test_line("    }"),
            test_line("    if ((edx != 0)) {"),
            test_line("        sub_1();"),
            test_line("        return;"),
            test_line("    }"),
        ];
        merge_identical_if_arms(&mut lines);
        assert_eq!(lines.len(), 4, "{lines:?}");
        assert_eq!(lines[0].text, "    if ((eax != 0) || (edx != 0)) {");
    }

    #[test]
    fn single_use_temporary_is_inlined_into_the_reader() {
        let mut lines = vec![
            test_line("    eax = r8d;"),
            test_line("    if ((eax == 0)) {"),
            test_line("    }"),
        ];
        fold_single_use_temporaries(&mut lines);
        assert_eq!(lines[0].text, "    if ((r8d == 0)) {");
        let mut lines = vec![test_line("    eax = 7;"), test_line("    return eax;")];
        fold_single_use_temporaries(&mut lines);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "    return 7;");
        // The same holds for an expression around the temporary, as produced by
        // `fold_return_values`.
        let mut lines = vec![
            test_line("    eax = r8d;"),
            test_line("    return (eax + 1);"),
        ];
        fold_single_use_temporaries(&mut lines);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "    return (r8d + 1);");
    }

    #[test]
    fn returns_carry_the_value_left_in_the_return_register() {
        let mut lines = vec![test_line("    eax = sub_1();"), test_line("    return;")];
        fold_return_values(&mut lines, ReturnKind::Int32);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "    return sub_1();");

        let mut lines = vec![test_line("    eax += ebx;"), test_line("    return;")];
        fold_return_values(&mut lines, ReturnKind::Int32);
        assert_eq!(lines[0].text, "    return (eax + ebx);");

        // A value computed on another path: the return still names the register.
        let mut lines = vec![
            test_line("    if (flag) {"),
            test_line("        eax = 1;"),
            test_line("    }"),
            test_line("    return;"),
        ];
        fold_return_values(&mut lines, ReturnKind::Int32);
        assert_eq!(lines.last().unwrap().text, "    return eax;");

        // A call as the last statement leaves its result in the return register.
        let mut lines = vec![test_line("    rax = sub_2(1);"), test_line("    return;")];
        fold_return_values(&mut lines, ReturnKind::Int64);
        assert_eq!(lines[0].text, "    return sub_2(1);");

        let mut lines = vec![test_line("    sub_3(rax);"), test_line("    return;")];
        fold_return_values(&mut lines, ReturnKind::Int64);
        assert_eq!(lines[0].text, "    return sub_3(rax);");
    }

    #[test]
    fn l2_returns_the_computed_value() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "add_leaf").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("return (arg1 + arg2);"), "{text}");
        assert!(!text.contains("return;"), "{text}");
    }

    #[test]
    fn void_and_unused_return_registers_keep_a_bare_return() {
        let mut lines = vec![test_line("    eax = 1;"), test_line("    return;")];
        fold_return_values(&mut lines, ReturnKind::Void);
        assert_eq!(lines[1].text, "    return;");

        let mut lines = vec![test_line("    return;")];
        fold_return_values(&mut lines, ReturnKind::Int32);
        assert_eq!(lines[0].text, "    return;");
    }

    #[test]
    fn condition_wrappers_are_stripped_once() {
        let mut lines = vec![
            test_line("    if ((eax == 0)) {"),
            test_line("    while ((rdx != 1)) {"),
            test_line("    if ((a) && (b)) {"),
        ];
        strip_condition_parentheses(&mut lines);
        assert_eq!(lines[0].text, "    if (eax == 0) {");
        assert_eq!(lines[1].text, "    while (rdx != 1) {");
        assert_eq!(lines[2].text, "    if ((a) && (b)) {");
    }

    #[test]
    fn overwritten_assignments_are_dropped_unless_the_value_is_read() {
        let mut lines = vec![
            test_line("    var_n8 = rdi;"),
            test_line("    var_n8 = r14;"),
            test_line("    var_n8 = r15;"),
            test_line("    rax = 1;"),
            test_line("    rax = (rax + 1);"),
            test_line("    eax = sub_1();"),
            test_line("    eax = 0;"),
        ];
        drop_overwritten_assignments(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(lines.len(), 5, "{text}");
        assert!(text.contains("var_n8 = r15;"), "{text}");
        assert!(!text.contains("var_n8 = rdi;"), "{text}");
        assert!(text.contains("(rax + 1)"), "{text}");
        assert!(text.contains("sub_1();"), "{text}");
    }

    #[test]
    fn overwritten_slot_store_is_dropped_only_without_a_read() {
        let mut lines = vec![
            test_line("    var_30 = 0;"),
            test_line("    eax = 1;"),
            test_line("    var_30 = rax;"),
        ];
        drop_dead_slot_stores(&mut lines);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(lines[0].text, "    eax = 1;");
        let mut lines = vec![
            test_line("    var_30 = 0;"),
            test_line("    eax = var_30;"),
            test_line("    var_30 = rax;"),
        ];
        drop_dead_slot_stores(&mut lines);
        assert_eq!(lines.len(), 3, "{lines:?}");
    }

    #[test]
    fn epilogue_restores_and_argument_copies_collapse() {
        let mut lines = vec![
            test_line("    rsi = arg1;"),
            test_line("    al = *(u8*)(rsi);"),
            test_line("    rsi = var_50;"),
            test_line("    r15 = *(u64*)(rsp);"),
            test_line("    return;"),
        ];
        hide_epilogue_restores(&mut lines);
        propagate_argument_spills(&mut lines);
        propagate_argument_copies(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("rsi = var_50;"), "{text}");
        assert!(!text.contains("rsi = arg1;"), "{text}");
        assert!(text.contains("*(u8*)(arg1)"), "{text}");
    }

    #[test]
    fn generated_signatures_declare_every_argument_they_use() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let starts: Vec<Address> = session
            .functions()
            .unwrap()
            .iter()
            .map(|function| function.start)
            .collect();
        for start in starts {
            let text = pseudocode_l2(&mut session, start).unwrap().text();
            let header = text
                .lines()
                .find(|line| line.trim_end().ends_with('{') && !line.trim_start().starts_with("//"))
                .unwrap_or_default();
            let declared = header
                .split('(')
                .nth(1)
                .unwrap_or_default()
                .split(')')
                .next()
                .unwrap_or_default()
                .split(',')
                .filter_map(|parameter| {
                    let name = parameter.trim().rsplit(' ').next()?;
                    name.strip_prefix("arg")?.parse::<usize>().ok()
                })
                .max()
                .unwrap_or(0);
            let used = text
                .lines()
                .flat_map(|line| {
                    line.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                        .filter_map(|token| {
                            token
                                .strip_prefix("arg")
                                .and_then(|digits| digits.parse::<usize>().ok())
                        })
                        .collect::<Vec<_>>()
                })
                .max()
                .unwrap_or(0);
            assert!(
                used <= declared,
                "arg{used} used but {declared} declared:\n{text}"
            );
        }
    }

    #[test]
    fn generated_labels_stay_inside_the_function() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let starts: Vec<Address> = session
            .functions()
            .unwrap()
            .iter()
            .map(|function| function.start)
            .collect();
        for start in starts {
            for text in [
                pseudocode_l1(&mut session, start).unwrap().text(),
                pseudocode_l2(&mut session, start).unwrap().text(),
            ] {
                let signature = text
                    .lines()
                    .position(|line| {
                        line.trim_end().ends_with('{') && !line.trim_start().starts_with("//")
                    })
                    .unwrap_or(0);
                for line in text.lines().take(signature) {
                    assert!(
                        !line.trim().starts_with("loc_"),
                        "label above the signature:\n{text}"
                    );
                }
            }
        }
    }
    #[test]
    fn generated_output_has_balanced_braces() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let starts: Vec<Address> = session
            .functions()
            .unwrap()
            .iter()
            .map(|function| function.start)
            .collect();
        for start in starts {
            for text in [
                pseudocode_l1(&mut session, start).unwrap().text(),
                pseudocode_l2(&mut session, start).unwrap().text(),
            ] {
                assert_eq!(
                    text.matches('{').count(),
                    text.matches('}').count(),
                    "unbalanced braces at {start:?}:\n{text}"
                );
                assert!(!text.contains("if ()"), "empty condition:\n{text}");
            }
        }
    }

    #[test]
    fn missing_labels_are_materialized_at_the_statement_address() {
        let mut lines = vec![
            PseudoLine {
                addr: Address::new(0x10),
                text: "    if (flag) goto loc_20;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x20),
                text: "    value = 1;".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x21),
                text: "    return;".to_owned(),
            },
        ];
        materialize_missing_labels(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("    loc_20:\n    value = 1;"), "{text}");
    }

    #[test]
    fn generated_gotos_always_have_a_label() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let starts: Vec<Address> = session
            .functions()
            .unwrap()
            .iter()
            .map(|function| function.start)
            .collect();
        for start in starts {
            let text = pseudocode_l1(&mut session, start).unwrap().text();
            let labels: BTreeSet<String> = text
                .lines()
                .filter_map(|line| {
                    line.trim()
                        .strip_suffix(':')
                        .filter(|label| label.starts_with("loc_"))
                        .map(str::to_owned)
                })
                .collect();
            for line in text.lines() {
                if let Some(target) = goto_target(line) {
                    assert!(
                        labels.contains(&format!("loc_{target:x}")),
                        "goto without label {target:x} in {start:?}:\n{text}"
                    );
                }
            }
        }
    }

    #[test]
    fn security_cookie_setup_and_checks_are_hidden() {
        let mut lines = vec![
            test_line("    rax = *(u64*)(0x140034400);"),
            test_line("    rax ^= rsp;"),
            test_line("    var_250 = rax;"),
            test_line("    eax = 0;"),
            test_line("    rcx = var_250;"),
            test_line("    rcx ^= rsp;"),
            test_line("    return sub_1400019f0(rcx);"),
            test_line("    if (flag) {"),
            test_line("        eax = 1;"),
            test_line("        rcx = var_250;"),
            test_line("        rcx ^= rsp;"),
            test_line("        return sub_1400019f0(rcx);"),
            test_line("    }"),
        ];
        hide_security_cookies(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("0x140034400"), "{text}");
        assert!(!text.contains("rsp"), "{text}");
        assert!(!text.contains("sub_1400019f0"), "{text}");
        assert!(text.contains("    eax = 0;\n    return;\n"), "{text}");
        assert!(text.contains("        eax = 1;\n        return;"), "{text}");
    }

    #[test]
    fn security_cookie_check_before_a_return_is_hidden_even_with_a_frame_restore() {
        let mut lines = vec![
            test_line("    rax = *(u64*)(0x140034400);"),
            test_line("    rax = (rax ^ rsp);"),
            test_line("    var_300 = rax;"),
            test_line("    esi = 0;"),
            test_line("    rcx = var_200;"),
            test_line("    rcx = (rcx ^ rsp);"),
            test_line("    sub_1400019f0(rcx);"),
            test_line("    r11 = &var_310;"),
            test_line("    return;"),
        ];
        hide_security_cookies(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("var_300"), "{text}");
        assert!(!text.contains("sub_1400019f0"), "{text}");
        assert!(!text.contains("var_200"), "{text}");
        assert!(text.contains("    esi = 0;"), "{text}");
        assert!(text.contains("    r11 = &var_310;"), "{text}");
    }

    #[test]
    fn security_cookie_check_with_a_raw_frame_load_is_hidden() {
        // L1 does not seed the register-rebuild set, so the load keeps its raw stack
        // expression instead of the `var_XX` name; the setup must not be dropped alone.
        let mut lines = vec![
            test_line("    rax = *(u64*)(0x140034400);"),
            test_line("    rax = (rax ^ rsp);"),
            test_line("    var_300 = rax;"),
            test_line("    rcx = *(u64*)((rbp + 0x200));"),
            test_line("    rcx = (rcx ^ rsp);"),
            test_line("    sub_1400019f0(rcx);"),
            test_line("    r11 = &var_310;"),
            test_line("    return;"),
        ];
        hide_security_cookies(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("0x140034400"), "{text}");
        assert!(!text.contains("0x200"), "{text}");
        assert!(!text.contains("sub_1400019f0"), "{text}");
        assert!(text.contains("    r11 = &var_310;"), "{text}");
    }

    #[test]
    fn security_cookie_setup_stays_when_the_slot_is_read_elsewhere() {
        let mut lines = vec![
            test_line("    rax = *(u64*)(0x140034400);"),
            test_line("    rax ^= rsp;"),
            test_line("    var_18 = rax;"),
            test_line("    rcx = var_18;"),
            test_line("    rcx ^= rsp;"),
            test_line("    return sub_1400019f0(rcx);"),
            test_line("    *(u64*)(rdi) = var_18;"),
        ];
        hide_security_cookies(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("var_18 = rax;"), "{text}");
        assert!(!text.contains("return sub_1400019f0(rcx);"), "{text}");
    }

    #[test]
    fn split_block_call_arguments_are_recovered_from_the_run_above() {
        let mut lines = vec![
            test_line("    rcx = *(u64*)(0x140036648);"),
            test_line("    edx = 0x464;"),
            test_line("    r9 = r14;"),
            test_line("    sub_140012340();"),
            test_line("    rcx = 0;"),
            test_line("    sub_140045678();"),
            test_line("    rcx = 7;"),
            test_line("    GetProcessHeap();"),
        ];
        let known = BTreeSet::from(["sub_140012340".to_owned(), "sub_140045678".to_owned()]);
        fill_split_call_arguments(&mut lines, &known);
        assert_eq!(
            lines[3].text, "    sub_140012340(*(u64*)(0x140036648), 0x464, r8, r14);",
            "{:?}",
            lines[3].text
        );
        assert_eq!(lines[5].text, "    sub_140045678(0);");
        // An import has no known arity, so a stray register value must not become an argument.
        assert_eq!(lines[7].text, "    GetProcessHeap();");
    }

    #[test]
    fn split_block_call_arguments_stop_at_control_flow() {
        let mut lines = vec![
            test_line("    rcx = 1;"),
            test_line("    if (flag) {"),
            test_line("        rcx = 2;"),
            test_line("    }"),
            test_line("    sub_1400aaaa();"),
            test_line("    rcx = 3;"),
            test_line("    sub_1400bbbb();"),
        ];
        let known = BTreeSet::from(["sub_1400aaaa".to_owned(), "sub_1400bbbb".to_owned()]);
        fill_split_call_arguments(&mut lines, &known);
        // The tracked `rcx` was reset by the branch, so the call keeps its empty list.
        assert_eq!(lines[4].text, "    sub_1400aaaa();");
        // `sub_1400bbbb` follows its own setup line and is filled.
        assert_eq!(lines[6].text, "    sub_1400bbbb(3);");
    }

    #[test]
    fn goto_to_a_trivial_return_label_becomes_a_return() {
        let mut lines = vec![
            test_line("    if (eax == 0) goto loc_140013a2d;"),
            test_line("    value = 1;"),
            test_line("    loc_140013a2d:"),
            test_line("    eax = 0;"),
            test_line("    loc_140013a2f:"),
            test_line("    return;"),
        ];
        fold_trivial_return_labels(&mut lines);
        tidy_labels(&mut lines);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("if (eax == 0) return 0;"), "{text}");
        assert!(!text.contains("goto loc_"), "{text}");
        assert!(!text.contains("loc_140013a2d:"), "{text}");
    }

    #[test]
    fn short_forward_skip_becomes_a_guarded_block() {
        let mut lines = vec![
            test_line("    if (eax == 0) goto loc_20;"),
            test_line("    value = 1;"),
            test_line("    if (flag) {"),
            test_line("        value = 2;"),
            test_line("    }"),
            test_line("    loc_20:"),
            test_line("    return;"),
        ];
        assert_eq!(fold_short_forward_gotos(&mut lines), 1);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("    if (eax != 0) {\n        value = 1;"),
            "{text}"
        );
        assert!(text.contains("        if (flag) {"), "{text}");
        assert!(text.contains("    }\n    loc_20:"), "{text}");
        assert!(!text.contains("goto loc_"), "{text}");
    }

    #[test]
    fn forward_skip_with_an_unbalanced_region_is_kept() {
        let mut lines = vec![
            test_line("    if (eax == 0) goto loc_20;"),
            test_line("    if (flag) {"),
            test_line("        value = 1;"),
            test_line("    loc_20:"),
            test_line("    return;"),
        ];
        assert_eq!(fold_short_forward_gotos(&mut lines), 0);
        assert!(lines[0].text.contains("goto loc_20;"));
    }

    #[test]
    fn long_forward_skip_is_kept_as_a_jump() {
        let mut lines = vec![test_line("    if (eax == 0) goto loc_40;")];
        for index in 0..(MAX_FORWARD_SKIP + 4) {
            lines.push(test_line(&format!("    value = {index};")));
        }
        lines.push(test_line("    loc_40:"));
        lines.push(test_line("    return;"));
        assert_eq!(fold_short_forward_gotos(&mut lines), 0);
        assert!(lines[0].text.contains("goto loc_40;"));
    }

    #[test]
    fn forward_skip_to_a_differently_indented_label_is_kept() {
        let mut lines = vec![
            test_line("    if (eax == 0) goto loc_20;"),
            test_line("    value = 1;"),
            test_line("        loc_20:"),
            test_line("    return;"),
        ];
        assert_eq!(fold_short_forward_gotos(&mut lines), 0);
        assert!(lines[0].text.contains("goto loc_20;"));
    }

    #[test]
    fn backward_conditional_jump_becomes_a_do_while() {
        let mut lines = vec![
            test_line("    loc_20:"),
            test_line("    value += 1;"),
            test_line("    if (value < 10) goto loc_20;"),
            test_line("    return;"),
        ];
        assert_eq!(fold_backward_gotos_into_loops(&mut lines), 1);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("    loc_20:\n    do {\n        value += 1;"),
            "{text}"
        );
        assert!(text.contains("    } while (value < 10);"), "{text}");
        assert!(!text.contains("goto loc_"), "{text}");
    }

    #[test]
    fn backward_unconditional_jump_becomes_an_endless_loop() {
        let mut lines = vec![
            test_line("    loc_20:"),
            test_line("    value += 1;"),
            test_line("    goto loc_20;"),
        ];
        assert_eq!(fold_backward_gotos_into_loops(&mut lines), 1);
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("    do {"), "{text}");
        assert!(text.contains("    } while (1);"), "{text}");
    }

    #[test]
    fn backward_jump_region_with_break_is_kept() {
        let mut lines = vec![
            test_line("    loc_20:"),
            test_line("    if (value) break;"),
            test_line("    if (value < 10) goto loc_20;"),
        ];
        assert_eq!(fold_backward_gotos_into_loops(&mut lines), 0);
        assert!(lines[2].text.contains("goto loc_20;"));
    }

    #[test]
    fn backward_jump_out_of_an_enclosing_block_is_kept() {
        let mut lines = vec![
            test_line("    loc_20:"),
            test_line("    if (flag) {"),
            test_line("        if (value < 10) goto loc_20;"),
            test_line("    }"),
        ];
        // The region closes the enclosing `{`, so the loop body would not be contiguous.
        assert_eq!(fold_backward_gotos_into_loops(&mut lines), 0);
        assert!(lines[2].text.contains("goto loc_20;"));
    }
}
