// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Function boundary heuristics.
//!
//! MSVC (and clang-cl) can split one source function into several `.pdata` entries:
//! hot code plus cold fragments that are only reachable by a conditional branch or by
//! fallthrough from the previous fragment. Treating every `.pdata` entry as a function
//! start produces phantom functions and truncated boundaries. This module merges such
//! fragments before they become function seeds.

use std::collections::BTreeMap;

use recraft_common::{Address, AddressRange, Result};
use recraft_core::Session;
use recraft_decode::FlowKind;

const MAX_FRAGMENT_INSTRUCTIONS: usize = 20_000;

fn find(parent: &mut [usize], mut index: usize) -> usize {
    while parent[index] != index {
        parent[index] = parent[parent[index]];
        index = parent[index];
    }
    index
}

fn union(parent: &mut [usize], left: usize, right: usize) {
    let left = find(parent, left);
    let right = find(parent, right);
    if left != right {
        let (keep, drop) = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        parent[drop] = keep;
    }
}

/// Merge cold fragments into their owning function.
///
/// A `.pdata` entry is merged into the previous one when it directly follows the
/// previous range *and* the previous range ends without a terminator (fallthrough), or
/// when a conditional branch from another entry targets its first instruction. Real
/// functions are entered with `call`/tail-call `jmp`, not conditional branches, so this
/// does not merge ordinary neighbours.
pub fn merge_pdata_functions(
    session: &mut Session,
    ranges: &[AddressRange],
) -> Result<Vec<(Address, Address)>> {
    if ranges.is_empty() {
        return Ok(Vec::new());
    }
    let mut parent: Vec<usize> = (0..ranges.len()).collect();
    let mut ends: BTreeMap<u64, usize> = BTreeMap::new();
    let mut fallthrough = vec![false; ranges.len()];
    let mut conditional_targets: BTreeMap<u64, usize> = BTreeMap::new();

    for (index, range) in ranges.iter().enumerate() {
        ends.insert(range.end.get(), index);
        let mut addr = range.start;
        let mut last_flow = None;
        let mut steps = 0usize;
        while addr < range.end && steps < MAX_FRAGMENT_INSTRUCTIONS {
            let Some(insn) = session.insn_at(addr)? else {
                break;
            };
            let next = insn.next_addr();
            if next <= addr {
                break;
            }
            last_flow = Some(insn.flow());
            if insn.flow() == FlowKind::ConditionalBranch
                && let Some(target) = insn.branch_target()
            {
                conditional_targets.entry(target.get()).or_insert(index);
            }
            addr = next;
            steps += 1;
        }
        fallthrough[index] = !matches!(
            last_flow,
            Some(
                FlowKind::UnconditionalBranch
                    | FlowKind::IndirectBranch
                    | FlowKind::Return
                    | FlowKind::Interrupt
                    | FlowKind::Trap
                    | FlowKind::Invalid
            )
        );
    }

    for (index, range) in ranges.iter().enumerate() {
        // Fallthrough continuation from the preceding range.
        if let Some((end, previous)) = ends
            .range(..=range.start.get())
            .next_back()
            .map(|(&end, &index)| (end, index))
            && end == range.start.get()
            && fallthrough[previous]
        {
            union(&mut parent, previous, index);
        }
        // Direct conditional branch into this fragment.
        if let Some(&source) = conditional_targets.get(&range.start.get())
            && source != index
        {
            union(&mut parent, source, index);
        }
    }

    let mut merged: BTreeMap<usize, (u64, u64)> = BTreeMap::new();
    for (index, range) in ranges.iter().enumerate() {
        let root = find(&mut parent, index);
        merged
            .entry(root)
            .and_modify(|entry| {
                entry.0 = entry.0.min(range.start.get());
                entry.1 = entry.1.max(range.end.get());
            })
            .or_insert((range.start.get(), range.end.get()));
    }
    Ok(merged
        .into_values()
        .map(|(start, end)| (Address::new(start), Address::new(end)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn open(bytes: &[u8]) -> (Session, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("boundaries.rcraft");
        let session = Session::create(&project, bytes, &mut |_, _| {}, &|| false).unwrap();
        (session, dir)
    }

    #[test]
    fn corpus_cold_fragments_are_merged() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/build/x64/corpus.exe");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let (mut session, _dir) = open(&bytes);
        let ranges: Vec<AddressRange> = session
            .image()
            .pdata_functions
            .iter()
            .map(|range| AddressRange::new(range.start, range.end))
            .collect();
        let raw = ranges.len();
        let merged = merge_pdata_functions(&mut session, &ranges).unwrap();
        assert!(
            merged.len() < raw,
            "fragments were not merged ({merged:?} vs {raw} pdata entries)"
        );
        // The merged ranges still have to be sane (non-empty, non-overlapping).
        for window in merged.windows(2) {
            assert!(
                window[0].1 <= window[1].0,
                "overlap after merge: {window:?}"
            );
        }
        assert!(merged.iter().all(|(start, end)| end > start));
    }
}
