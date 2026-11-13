// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Stack-variable naming: turn the generated `var_<offset>` slots of one function into
//! model-proposed, reviewable names.
//!
//! The model sees the slot list and a few pseudo-C lines where each slot appears; the reply
//! contract (`<slot> <name> -- <rationale>`) is parsed by [`parse_var_reply`]. Applying a
//! proposal is a plain `Command::RenameVar`, so it is journaled/undoable and the L2 renderer
//! prints the name on every later render.

use std::collections::BTreeSet;

use recraft_common::{Address, Result};
use recraft_core::Session;

use crate::provider::{PromptKind, PromptRequest, ProviderSettings, TokenBudget, system_prompt};

/// One proposed name for a stack variable.
#[derive(Debug, Clone, PartialEq)]
pub struct VarProposal {
    /// Stack offset relative to the frame base (`var_20` → 0x20, `var_n8` → -8).
    pub offset: i64,
    /// The proposed C identifier.
    pub value: String,
    /// Why the model picked it (one sentence).
    pub rationale: String,
    pub confidence: f32,
}

/// One stack slot found in a function's pseudo-C.
#[derive(Debug, Clone, PartialEq)]
pub struct StackSlot {
    pub offset: i64,
    /// The generated token (`var_20` / `var_n8`).
    pub token: String,
    /// Number of pseudo-C lines that mention it.
    pub uses: usize,
    /// A few representative lines (trimmed), for the prompt.
    pub lines: Vec<String>,
}

/// Upper bound on slots per request: a giant frame should not become a giant prompt.
const MAX_VAR_SLOTS: usize = 24;
/// Representative lines kept per slot.
const EXAMPLES_PER_SLOT: usize = 3;
/// A prompt line is cut at this many characters.
const EXAMPLE_CHARS: usize = 140;

/// Parse a generated slot token into its stack offset.
pub fn slot_offset(token: &str) -> Option<i64> {
    if let Some(hex) = token.strip_prefix("var_n") {
        i64::from_str_radix(hex, 16).ok().map(|value| -value)
    } else {
        let hex = token.strip_prefix("var_")?;
        i64::from_str_radix(hex, 16).ok()
    }
}

/// The stack slots of `func`, in first-use order, with a few example lines each.
pub fn stack_slots(session: &mut Session, func: Address) -> Result<Vec<StackSlot>> {
    let pseudo = recraft_decompile::pseudocode_l2(session, func)?;
    let mut slots: Vec<StackSlot> = Vec::new();
    for line in &pseudo.lines {
        let text = line.text.trim();
        if text.is_empty() || text.starts_with("//") {
            continue;
        }
        let mut on_this_line = BTreeSet::new();
        for token in text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')) {
            let Some(offset) = slot_offset(token) else {
                continue;
            };
            if !on_this_line.insert(offset) {
                continue;
            }
            match slots.iter_mut().find(|slot| slot.offset == offset) {
                Some(slot) => {
                    slot.uses += 1;
                    if slot.lines.len() < EXAMPLES_PER_SLOT {
                        slot.lines.push(truncate(text, EXAMPLE_CHARS));
                    }
                }
                None => slots.push(StackSlot {
                    offset,
                    token: token.to_owned(),
                    uses: 1,
                    lines: vec![truncate(text, EXAMPLE_CHARS)],
                }),
            }
        }
    }
    Ok(slots)
}

/// Build the variable-naming request for `func`.
pub fn build_var_prompt(
    session: &mut Session,
    func: Address,
    settings: &ProviderSettings,
) -> Result<PromptRequest> {
    let slots = stack_slots(session, func)?;
    let name = session
        .name_at(func)?
        .unwrap_or_else(|| format!("sub_{:x}", func.get()));
    Ok(build_var_prompt_from_slots(&name, func, &slots, settings))
}

/// The request for an already collected slot list (the session only supplies the text).
pub fn build_var_prompt_from_slots(
    name: &str,
    func: Address,
    slots: &[StackSlot],
    settings: &ProviderSettings,
) -> PromptRequest {
    let mut user = format!(
        "name these stack variables of {name} (0x{:x}):\n",
        func.get()
    );
    for slot in slots.iter().take(MAX_VAR_SLOTS) {
        user.push_str(&format!("- {} ({} uses)\n", slot.token, slot.uses));
        for line in &slot.lines {
            user.push_str(&format!("    {line}\n"));
        }
    }
    user.push_str("\nreply with one line per variable: var_20 new_name -- rationale\n");
    let system = system_prompt(settings, PromptKind::Vars);
    PromptRequest::new(TokenBudget::new(settings.max_tokens), &system, &user)
}

/// Parse a model reply into proposals, one per distinct slot.
pub fn parse_var_reply(text: &str) -> Vec<VarProposal> {
    let mut proposals: Vec<VarProposal> = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches(['-', '*']).trim();
        let Some((head, rationale)) = line.split_once("--") else {
            continue;
        };
        let mut parts = head.split_whitespace();
        let (Some(slot), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        let Some(offset) = slot_offset(slot) else {
            continue;
        };
        if !valid_variable_name(name) || proposals.iter().any(|entry| entry.offset == offset) {
            continue;
        }
        proposals.push(VarProposal {
            offset,
            value: name.to_owned(),
            rationale: rationale.trim().trim_matches('"').to_owned(),
            confidence: 0.8,
        });
    }
    proposals
}

/// A proposed name must be a plain C identifier that does not mimic a generated token.
fn valid_variable_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_')
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return false;
    }
    !["var_", "sub_", "loc_", "arg", "ret"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// Cut `text` at `chars` characters, adding an ellipsis when it was longer.
fn truncate(text: &str, chars: usize) -> String {
    let mut text: Vec<char> = text.chars().collect();
    if text.len() > chars {
        text.truncate(chars);
        text.push('…');
    }
    text.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_tokens_map_to_offsets() {
        assert_eq!(slot_offset("var_20"), Some(0x20));
        assert_eq!(slot_offset("var_n8"), Some(-8));
        assert_eq!(slot_offset("var_zz"), None);
        assert_eq!(slot_offset("v1"), None);
    }

    #[test]
    fn a_reply_yields_one_proposal_per_slot() {
        let proposals = parse_var_reply(
            "var_20 input -- the fgets destination\n\
             var_n8 flags -- bit 0 is tested\n\
             var_20 buffer -- duplicate, first wins\n\
             var_30 3bad -- not an identifier\n\
             noise without a separator",
        );
        assert_eq!(proposals.len(), 2);
        assert_eq!(proposals[0].offset, 0x20);
        assert_eq!(proposals[0].value, "input");
        assert_eq!(proposals[1].offset, -8);
        assert_eq!(proposals[1].value, "flags");
    }

    #[test]
    fn generated_looking_names_are_rejected() {
        assert!(!valid_variable_name("var_8"));
        assert!(!valid_variable_name("sub_100"));
        assert!(!valid_variable_name("9lives"));
        assert!(valid_variable_name("input_buffer"));
    }

    #[test]
    fn the_prompt_lists_slots_and_the_reply_contract() {
        let slots = vec![StackSlot {
            offset: 0x20,
            token: "var_20".to_owned(),
            uses: 3,
            lines: vec!["buf = fgets(&var_20, 0x100, stdin);".to_owned()],
        }];
        let settings = ProviderSettings::default();
        let prompt =
            build_var_prompt_from_slots("main", Address::new(0x140001000), &slots, &settings);
        assert_eq!(prompt.system, crate::VARS_SYSTEM);
        assert!(prompt.user.contains("- var_20 (3 uses)"), "{}", prompt.user);
        assert!(prompt.user.contains("fgets(&var_20"), "{}", prompt.user);
        assert!(
            prompt.user.contains("var_20 new_name -- rationale"),
            "{}",
            prompt.user
        );
        let zh = ProviderSettings {
            prompt_language: crate::PromptLanguage::Zh,
            ..ProviderSettings::default()
        };
        let prompt = build_var_prompt_from_slots("main", Address::new(0x140001000), &slots, &zh);
        assert_eq!(prompt.system, crate::VARS_SYSTEM_ZH);
    }
}
