// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! ReCraft desktop application (M0 shell).
//!
//! Deliberately small: one session, three panes (navigation, disassembly/pseudo-C,
//! status), keyboard-first commands. All state changes go through `recraft-core`
//! commands, so undo/redo behave exactly like the CLI.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Key, RichText, Sense};

use recraft_common::{Address, AddressRange};
use recraft_core::{
    CodeData, Command, CommentKind, DisasmLine, Flags, FunctionInfo, Session, StructDef,
    StructField, XrefEntry,
};
use recraft_decompile::PseudoLine;

use crate::i18n::{I18n, Lang};

const DISASM_ROWS: usize = 400;
/// Instructions loaded per page when the wheel runs off the end of the disassembly window.
const DISASM_SCROLL_ROWS: usize = 50;
/// Distance from a window edge (in rows) at which the wheel loads the next page.
const DISASM_SCROLL_MARGIN_ROWS: f32 = 3.0;
/// Bytes one raw data row covers in a data window (the listing's `raw` buffer holds 15).
const DATA_ROW_BYTES: u64 = 8;
/// Cap on row-boundary steps when a data window is paged backwards, so a huge segment
/// cannot stall the UI.
const DATA_WALK_LIMIT: usize = 200_000;
/// Column starts of the disassembly listing, in monospace cells. They mirror the field widths
/// [`ReCraftApp::disasm_columns`] formats (14 address + 46 bytes + 9 mnemonic + 36 operands), so
/// the pinned header can be painted cell by cell instead of padded with `{:<14}` — a CJK label is
/// wider than one cell and would shift every later column.
const DISASM_COLUMN_CELLS: [usize; 5] = [0, 14, 60, 69, 105];
const MONO_SIZE: f32 = 12.5;
/// Fraction of the font size by which the monospace CJK fallback is lifted so its
/// baseline lines up with Consolas (see [`ui_fonts`]). The gap is proportional to the
/// size, so the listing zoom keeps it correct at every step; epaint rounds the offset to
/// whole points, which leaves at most 1 px at some sizes.
const MONO_CJK_Y_OFFSET_FACTOR: f32 = -0.25;
const ROW_HEIGHT: f32 = 15.0;
/// Rows the Strings window paints per frame; a full string table would otherwise cost the
/// whole frame. IDA caps its list the same way and reports the truncation in the status bar.
const STRING_LIST_ROWS: usize = 2000;
/// Extensions File → Import offers besides the all-files fallback. PE binaries are
/// `.exe`/`.dll`/`.sys`; ELF binaries are commonly `.elf`/`.so`/`.bin`/`.out` and very often
/// carry no extension at all (`a.out`, `busybox`), so the dialog must also list every file:
/// a filter-only dialog silently hides ELF samples even though the importer detects them by
/// their magic bytes.
const IMPORT_EXTENSIONS: &[&str] = &["exe", "dll", "sys", "elf", "so", "bin", "out"];
const ZOOM_MIN: f32 = 0.75;
const ZOOM_MAX: f32 = 2.0;
const ZOOM_STEP: f32 = 0.1;

/// Display mode of the single code view: graph or text listing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum ViewMode {
    #[default]
    Graph,
    Text,
}

/// Which colour scheme the workspace uses. Persisted with the dock layout.
/// `Light` is a full light theme (light chrome and light views) and is the default.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum ThemeMode {
    Dark,
    #[default]
    Light,
}

/// Navigation-band colours, shared with the legend so they always agree.
const NAV_FUNCTION: Color32 = Color32::from_rgb(0x3a, 0x5a, 0x7a);
const NAV_INSTRUCTION: Color32 = Color32::from_rgb(0x2f, 0x6a, 0x6a);
const NAV_DATA: Color32 = Color32::from_rgb(0x4a, 0x6a, 0x3a);
const NAV_UNEXPLORED: Color32 = Color32::from_rgb(0x5a, 0x5a, 0x5a);
const NAV_EXTERNAL: Color32 = Color32::from_rgb(0x7a, 0x3a, 0x6a);

/// Share of the left column that the Functions window keeps when the Graph overview is
/// docked underneath it (the overview takes the remaining 18 %, like IDA's ~110 px pane).
const GRAPH_OVERVIEW_SPLIT: f32 = 0.82;

/// Semantic palette shared by the chrome, code views and graph view.
#[derive(Clone, Copy)]
struct Palette {
    panel: Color32,
    view: Color32,
    text: Color32,
    addr: Color32,
    /// Pseudo-code line-number gutter (dim, like IDA's listing gutter).
    line_number: Color32,
    bytes: Color32,
    mnemonic: Color32,
    /// Numeric literals inside operands (IDA colours immediates separately).
    number: Color32,
    /// Registers inside operands.
    register: Color32,
    name: Color32,
    comment: Color32,
    selected_row: Color32,
    /// Background of a dragged text selection (distinct from the row cursor).
    text_selection: Color32,
    hover_row: Color32,
    tab_active_bg: Color32,
    tab_focused_bg: Color32,
    tab_inactive_text: Color32,
    grid_line: Color32,
    graph_node: Color32,
    graph_border: Color32,
    graph_selected_border: Color32,
    graph_header: Color32,
    graph_header_text: Color32,
    edge_jump: Color32,
    edge_taken: Color32,
    edge_not_taken: Color32,
    edge_jump_table: Color32,
    /// Raised surface (cards, menus, inputs) above `panel`.
    surface_2: Color32,
    /// Hairline separator, 1 px.
    border: Color32,
    /// Focus / selected outline.
    border_strong: Color32,
    /// Secondary text (labels, hints).
    muted: Color32,
    /// The single accent (links, selection, focus).
    accent: Color32,
}

impl Palette {
    const DARK: Self = Self {
        panel: Color32::from_rgb(0x14, 0x17, 0x1b),
        view: Color32::from_rgb(0x0e, 0x10, 0x13),
        text: Color32::from_rgb(0xe4, 0xe7, 0xeb),
        addr: Color32::from_rgb(0xc9, 0xb4, 0x58),
        line_number: Color32::from_rgb(0x6b, 0x74, 0x82),
        bytes: Color32::from_rgb(0x7c, 0x86, 0x94),
        mnemonic: Color32::from_rgb(0x7e, 0x9e, 0xd0),
        number: Color32::from_rgb(0xb5, 0xce, 0xa8),
        register: Color32::from_rgb(0xc5, 0x86, 0xc0),
        name: Color32::from_rgb(0xd7, 0xd7, 0x87),
        comment: Color32::from_rgb(0x6a, 0x99, 0x55),
        selected_row: Color32::from_rgb(0x26, 0x4f, 0x78),
        text_selection: Color32::from_rgb(0x39, 0x6d, 0xa8),
        hover_row: Color32::from_rgb(0x2a, 0x30, 0x38),
        tab_active_bg: Color32::from_rgb(0x1a, 0x1e, 0x24),
        tab_focused_bg: Color32::from_rgb(0x22, 0x28, 0x31),
        tab_inactive_text: Color32::from_rgb(0x8b, 0x94, 0xa3),
        grid_line: Color32::from_rgb(0x2c, 0x32, 0x3c),
        graph_node: Color32::from_rgb(0x0d, 0x10, 0x12),
        graph_border: Color32::from_rgb(0x4a, 0x8f, 0x8f),
        graph_selected_border: Color32::from_rgb(0xc9, 0xb4, 0x58),
        graph_header: Color32::from_rgb(0x9c, 0xc8, 0xc8),
        graph_header_text: Color32::from_rgb(0x14, 0x2a, 0x2a),
        edge_jump: Color32::from_rgb(0x5a, 0x9a, 0x9a),
        edge_taken: Color32::from_rgb(0x6a, 0xb0, 0x6a),
        edge_not_taken: Color32::from_rgb(0xc0, 0x6a, 0x6a),
        edge_jump_table: Color32::from_rgb(0xa0, 0x8a, 0xc0),
        surface_2: Color32::from_rgb(0x1a, 0x1e, 0x24),
        border: Color32::from_rgb(0x26, 0x2b, 0x33),
        border_strong: Color32::from_rgb(0x33, 0x3a, 0x45),
        muted: Color32::from_rgb(0x98, 0xa2, 0xb3),
        accent: Color32::from_rgb(0x40, 0xcd, 0xe0),
    };

    /// Full light scheme: white views and light grey chrome.
    const LIGHT: Self = Self {
        panel: Color32::from_rgb(0xf5, 0xf7, 0xfa),
        view: Color32::from_rgb(0xff, 0xff, 0xff),
        text: Color32::from_rgb(0x1e, 0x1e, 0x1e),
        addr: Color32::from_rgb(0x1a, 0x3f, 0x8f),
        line_number: Color32::from_rgb(0x70, 0x70, 0x70),
        bytes: Color32::from_rgb(0x60, 0x60, 0x60),
        mnemonic: Color32::from_rgb(0x00, 0x50, 0xa0),
        number: Color32::from_rgb(0x0f, 0x6f, 0x9f),
        register: Color32::from_rgb(0x7a, 0x3e, 0x9d),
        name: Color32::from_rgb(0x80, 0x50, 0x00),
        comment: Color32::from_rgb(0x00, 0x78, 0x00),
        selected_row: Color32::from_rgb(0xcc, 0xe4, 0xff),
        text_selection: Color32::from_rgb(0xa6, 0xcf, 0xf2),
        hover_row: Color32::from_rgb(0xee, 0xf3, 0xf8),
        tab_active_bg: Color32::from_rgb(0xff, 0xff, 0xff),
        tab_focused_bg: Color32::from_rgb(0xe4, 0xec, 0xf4),
        tab_inactive_text: Color32::from_rgb(0x40, 0x48, 0x50),
        grid_line: Color32::from_rgb(0xc0, 0xc6, 0xcc),
        graph_node: Color32::from_rgb(0xff, 0xff, 0xff),
        graph_border: Color32::from_rgb(0x3a, 0x8f, 0x8f),
        graph_selected_border: Color32::from_rgb(0xb0, 0x7a, 0x00),
        graph_header: Color32::from_rgb(0xc8, 0xe4, 0xe4),
        graph_header_text: Color32::from_rgb(0x10, 0x30, 0x30),
        edge_jump: Color32::from_rgb(0x1f, 0x6f, 0x6f),
        edge_taken: Color32::from_rgb(0x1f, 0x7a, 0x1f),
        edge_not_taken: Color32::from_rgb(0xa0, 0x20, 0x20),
        edge_jump_table: Color32::from_rgb(0x6a, 0x3f, 0x9f),
        surface_2: Color32::from_rgb(0xed, 0xf1, 0xf6),
        border: Color32::from_rgb(0xd6, 0xdc, 0xe4),
        border_strong: Color32::from_rgb(0xb7, 0xc0, 0xcc),
        muted: Color32::from_rgb(0x5a, 0x64, 0x72),
        accent: Color32::from_rgb(0x0e, 0x74, 0x90),
    };
}

/// Rounded card used by the AI panel sections.
fn card_frame(pal: Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(pal.surface_2)
        .stroke(egui::Stroke::new(1.0, pal.border))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(12))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HexUnit {
    Byte,
    Word,
}

/// Start of the 16-byte hex row containing `addr`.
fn hex_row_start(addr: Address) -> Address {
    Address::new(addr.get() & !0xf)
}

/// One end of an editor-style text selection in a custom-painted listing: the row index
/// inside the view's rows plus a character index into that row's text.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct TextPoint {
    row: usize,
    char: usize,
}

/// Editor-style text selection: `anchor` is where the drag started, `focus` is where the
/// pointer is now. Either end may be the earlier one, so every use goes through
/// [`Self::ordered`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct TextSelection {
    anchor: TextPoint,
    focus: TextPoint,
}

impl TextSelection {
    /// A caret: both ends on the same character, so nothing is highlighted.
    fn collapsed(point: TextPoint) -> Self {
        Self {
            anchor: point,
            focus: point,
        }
    }

    /// The selection with the earlier end first.
    fn ordered(&self) -> (TextPoint, TextPoint) {
        if self.anchor <= self.focus {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    fn is_empty(&self) -> bool {
        self.anchor == self.focus
    }

    /// Character range `[from, to)` this selection covers in `row`, or `None` when the row
    /// lies outside it. Fully covered rows run from their first to their last character.
    fn row_range(&self, row: usize, char_count: usize) -> Option<(usize, usize)> {
        let (start, end) = self.ordered();
        if row < start.row || row > end.row {
            return None;
        }
        let from = if row == start.row {
            start.char.min(char_count)
        } else {
            0
        };
        let to = if row == end.row {
            end.char.min(char_count)
        } else {
            char_count
        };
        Some((from.min(to), to))
    }
}

/// The text a text selection covers, one listing row per line, with the column padding
/// the views paint on the right trimmed off.
fn selected_text(selection: TextSelection, rows: &[String]) -> Option<String> {
    let (start, end) = selection.ordered();
    if rows.is_empty() || start.row >= rows.len() {
        return None;
    }
    let end_row = end.row.min(rows.len() - 1);
    let mut lines = Vec::with_capacity(end_row - start.row + 1);
    for (row, text) in rows.iter().enumerate().take(end_row + 1).skip(start.row) {
        let chars: Vec<char> = text.chars().collect();
        let (from, to) = selection
            .row_range(row, chars.len())
            .unwrap_or((0, chars.len()));
        let piece: String = chars[from..to].iter().collect();
        lines.push(piece.trim_end().to_owned());
    }
    Some(lines.join("\n"))
}

/// Re-find a text selection in a reloaded listing by address, so paging (or an undo that
/// re-renders the same addresses) keeps the highlight on the same instructions. `None`
/// when either end has scrolled out of the new window.
fn remap_selection(
    selection: TextSelection,
    previous: &[DisasmLine],
    rows: &[DisasmLine],
) -> Option<TextSelection> {
    let remap = |point: TextPoint| {
        let addr = previous.get(point.row)?.addr;
        rows.iter()
            .position(|line| line.addr == addr)
            .map(|row| TextPoint {
                row,
                char: point.char,
            })
    };
    remap(selection.anchor)
        .zip(remap(selection.focus))
        .map(|(anchor, focus)| TextSelection { anchor, focus })
}

/// Address under a pointer in a single-row galley painted at `origin`.
fn galley_char_at(galley: &egui::Galley, origin: egui::Pos2, pos: egui::Pos2) -> usize {
    let local = pos - origin;
    usize::from(galley.cursor_from_pos(local).index)
}

/// Character range of the word at `char`, as IDA's double-click selection picks it: a run of
/// identifier characters, or the run of punctuation next to it. Whitespace selects nothing.
fn word_range_at(text: &str, char: usize) -> (usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return (0, 0);
    }
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let index = char.min(chars.len() - 1);
    if chars[index].is_whitespace() {
        return (index, index);
    }
    let word = is_word(chars[index]);
    let matches = |c: char| {
        if word {
            is_word(c)
        } else {
            !is_word(c) && !c.is_whitespace()
        }
    };
    let mut start = index;
    while start > 0 && matches(chars[start - 1]) {
        start -= 1;
    }
    let mut end = index + 1;
    while end < chars.len() && matches(chars[end]) {
        end += 1;
    }
    (start, end)
}

/// Advance of one monospace cell, i.e. the width every character of a listing row takes.
///
/// Measured from the shaped glyph rather than from the font size so the pinned header lands on
/// the same x as the row text (the advance is fractional and does not round to `size * 0.5`).
fn monospace_cell_width(ui: &egui::Ui, font: &FontId, color: Color32) -> f32 {
    let galley = ui
        .painter()
        .layout_no_wrap("0".to_owned(), font.clone(), color);
    galley
        .rows
        .first()
        .and_then(|row| row.glyphs.first())
        .map_or(font.size * 0.6, |glyph| glyph.advance_width)
}

/// Paint the highlight of a text selection behind a row's text. `range` is the
/// `[from, to)` character range from [`TextSelection::row_range`].
fn paint_text_selection(
    painter: &egui::Painter,
    color: Color32,
    galley: &egui::Galley,
    origin: egui::Pos2,
    (from, to): (usize, usize),
) {
    if from >= to {
        return;
    }
    let count = galley.text().chars().count();
    let x0 = if from == 0 {
        0.0
    } else {
        galley
            .pos_from_cursor(egui::text::CCursor::new(from))
            .left()
    };
    let x1 = if to >= count {
        galley.size().x
    } else {
        galley.pos_from_cursor(egui::text::CCursor::new(to)).left()
    };
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(origin.x + x0, origin.y),
            egui::pos2(origin.x + x1.max(x0 + 1.0), origin.y + galley.size().y),
        ),
        0.0,
        color,
    );
}

/// Paint the highlight of a text selection behind a galley, across every wrapped row the
/// character range covers. [`paint_text_selection`] assumes one screen row; the Output
/// window's log lines wrap at the pane width, so the range can span several layout rows.
fn paint_wrapped_text_selection(
    painter: &egui::Painter,
    color: Color32,
    galley: &egui::Galley,
    origin: egui::Pos2,
    (from, to): (usize, usize),
) {
    let count = galley.text().chars().count();
    let from = from.min(count);
    let to = to.min(count);
    if from >= to {
        return;
    }
    let start = galley.layout_from_cursor(egui::text::CCursor::new(from));
    let end = galley.layout_from_cursor(egui::text::CCursor::new(to));
    for index in start.row..=end.row {
        let Some(placed) = galley.rows.get(index) else {
            break;
        };
        let left = if index == start.row {
            placed.x_offset(start.column)
        } else {
            0.0
        };
        let right = if index == end.row {
            placed.x_offset(end.column)
        } else {
            placed.size.x
        };
        let row_left = origin.x + placed.pos.x;
        let top = origin.y + placed.pos.y;
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(row_left + left, top),
                egui::pos2(row_left + right.max(left + 1.0), top + placed.size.y),
            ),
            0.0,
            color,
        );
    }
}

/// Inclusive byte range picked in the Hex view.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct HexSelection {
    anchor: Address,
    focus: Address,
}

impl HexSelection {
    /// The range with the lower address first.
    fn range(&self) -> (Address, Address) {
        if self.anchor <= self.focus {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    fn contains(&self, addr: Address) -> bool {
        let (start, end) = self.range();
        addr >= start && addr <= end
    }

    fn len(&self) -> u64 {
        let (start, end) = self.range();
        end.get().saturating_sub(start.get()).saturating_add(1)
    }
}

/// Pane that owns Ctrl+C, set by the last click or drag inside that pane.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum CopyFocus {
    #[default]
    None,
    Disasm,
    Pseudo,
    Hex,
    /// A dock list (Strings, Imports, Exports, Functions, Names, Segments).
    List(Pane),
    /// The Output window's log.
    Output,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Disasm,
    Pseudo,
}

/// Docked panes (IDA-style dockable tabs, backed by `egui_dock`).
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
enum Pane {
    View,
    Pseudo,
    Hex,
    Functions,
    Strings,
    Imports,
    Exports,
    Names,
    Segments,
    Structs,
    Ai,
    GraphOverview,
    Output,
}

impl Pane {
    fn all() -> [Pane; 12] {
        [
            // Default document tabs, in tab order.
            Pane::View,
            Pane::Pseudo,
            Pane::Hex,
            Pane::Strings,
            Pane::Structs,
            Pane::Imports,
            Pane::Exports,
            // Hidden by default, reachable from the Views menu.
            Pane::Names,
            Pane::Segments,
            // Panels.
            Pane::Functions,
            Pane::GraphOverview,
            Pane::Output,
        ]
    }

    fn key(self) -> &'static str {
        match self {
            // Disassembly is one view with an internal Graph/Text mode.
            Pane::View => "pane_disassembly",
            Pane::Pseudo => "pane_decompile",
            Pane::Hex => "pane_hex",
            Pane::Functions => "pane_functions",
            Pane::Strings => "pane_strings",
            Pane::Imports => "pane_imports",
            Pane::Exports => "pane_exports",
            Pane::Names => "pane_names",
            Pane::Segments => "pane_segments",
            Pane::Structs => "pane_local_types",
            Pane::Ai => "pane_ai",
            Pane::GraphOverview => "pane_graph_overview",
            Pane::Output => "pane_output",
        }
    }

    /// Keyboard shortcut that focuses this pane (shown in the Views menu tooltip).
    fn shortcut(self) -> Option<&'static str> {
        match self {
            Pane::Functions => Some("F2"),
            Pane::Names => Some("F3"),
            // IDA opens its Strings window with Shift+F12, so the pane uses that chord
            // instead of joining the F-key family below.
            Pane::Strings => Some("Shift+F12"),
            Pane::Imports => Some("F6"),
            Pane::Segments => Some("F8"),
            Pane::Hex => Some("F9"),
            Pane::Ai => Some("F10"),
            Pane::Structs => Some("F11"),
            _ => None,
        }
    }
}

/// Renders `ReCraftApp` panes inside an `egui_dock` tab.
struct DockViewer<'a> {
    app: &'a mut ReCraftApp,
    action: &'a mut Action,
}

impl egui_dock::TabViewer for DockViewer<'_> {
    type Tab = Pane;

    fn id(&mut self, tab: &mut Pane) -> egui::Id {
        egui::Id::new(("recraft-pane", *tab))
    }

    fn title(&mut self, tab: &mut Pane) -> egui::WidgetText {
        self.app.t(tab.key()).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Pane) {
        let pal = self.app.palette();
        ui.visuals_mut().override_text_color = Some(pal.text);
        match tab {
            Pane::View => {
                if self.app.session.is_none() {
                    self.app.welcome(ui, self.action);
                } else if self.app.view_mode == ViewMode::Graph {
                    self.app.graph_view(ui, self.action);
                } else {
                    self.app.disasm_view(ui, self.action);
                }
            }
            Pane::Pseudo => self.app.pseudo_view(ui, self.action),
            Pane::Hex => self.app.hex_view(ui, self.action),
            Pane::Functions => self.app.functions_list(ui, self.action),
            Pane::Strings => self.app.strings_list(ui, self.action),
            Pane::Imports => self.app.imports_list(ui, self.action),
            Pane::Names => self.app.names_list(ui, self.action),
            Pane::Segments => self.app.segments_list(ui, self.action),
            Pane::Ai => self.app.ai_panel(ui, self.action),
            Pane::Structs => self.app.structs_panel(ui, self.action),
            Pane::Exports => self.app.exports_list(ui, self.action),
            Pane::GraphOverview => self.app.graph_overview(ui, self.action),
            Pane::Output => self.app.output_pane(ui, self.action),
        }
    }

    fn allowed_in_windows(&self, _tab: &mut Pane) -> bool {
        true
    }

    fn is_closeable(&self, tab: &Pane) -> bool {
        // Keep the primary views open; auxiliary lists may be closed and reopened from
        // the Views menu.
        !matches!(
            tab,
            Pane::View | Pane::Functions | Pane::Output | Pane::GraphOverview
        )
    }
}

struct ImportRow {
    module: String,
    api: String,
    category: &'static str,
    iat: Address,
    refs: u64,
    first_from: Option<Address>,
}

/// One row of the Strings view: metadata from the project DB plus the decoded text.
#[derive(Clone)]
struct StringRow {
    addr: Address,
    /// Byte length including the terminator, as stored by analysis.
    len: u32,
    enc: i64,
    kind: i64,
    segment: String,
    text: String,
    /// First 15 bytes, for the `db` rows shown when navigating here.
    raw: Vec<u8>,
    /// Resolved symbol name at the address, if any.
    name: Option<String>,
    /// The user replaced the decoded text with their own (see `string_override`).
    overridden: bool,
}

/// One row of the xrefs dialog, laid out like IDA's `xrefs to …` window: direction,
/// type letter, address and the text at the other end of the reference.
#[derive(Debug)]
struct XrefRow {
    /// `true` for a reference into the selected item (`Up`), `false` for one leaving it
    /// (`Down`).
    up: bool,
    /// Single-letter xref type (`c` call, `j` jump, `o` offset, `r` read, `w` write).
    kind: char,
    /// Spelled-out kind (`call`, `jump`, `data`) for the tooltip.
    kind_name: &'static str,
    /// The other end of the reference; following the row navigates here.
    addr: Address,
    /// `main:loc_1400010b9`-style label for the address column.
    label: String,
    /// Instruction (or data) text of the referencing site.
    text: String,
}

/// State of the open `xrefs to …` dialog. Like IDA's dialog it is a snapshot: the rows
/// are built when it opens (and again when an edit makes them stale) and stay put while
/// the listing behind it is navigated.
struct XrefDialog {
    /// Address the dialog was opened for (`xrefs to <target>`).
    target: Address,
    rows: Vec<XrefRow>,
    /// Cursor into `rows`, moved with the arrow keys or a click.
    selected: usize,
    /// `Search` box text; matches the address, label or text columns.
    filter: String,
    /// Whether the `Search` box is visible.
    search: bool,
    /// Focus the search box on the next frame (set when `Search` opens it).
    search_focus: bool,
    /// Whether the type-letter legend is visible.
    help: bool,
    /// Rows must be rebuilt (analysis or an edit changed the project).
    dirty: bool,
}

/// Decode stored string bytes according to the analysis encoding id.
/// The selected text of a pseudo-C listing: whole rows inside the selection, partial rows
/// clipped to the selected characters. Rows are joined with newlines, in reading order.
fn selection_text(lines: &[PseudoLine], selection: TextSelection) -> Option<String> {
    let (from, to) = selection.ordered();
    let mut rows = Vec::new();
    for row in from.row..=to.row {
        let line = lines.get(row)?;
        let count = line.text.chars().count();
        let (start, end) = selection.row_range(row, count)?;
        rows.push(
            line.text
                .chars()
                .skip(start)
                .take(end.saturating_sub(start))
                .collect::<String>(),
        );
    }
    let text = rows.join("\n");
    (!text.trim().is_empty()).then_some(text)
}

/// The first stack-slot token (`var_20`, `var_n8`) of a pseudo-C line, with its offset.
///
/// The Decompile view's context menu offers "rename variable" only when the row actually
/// names a slot; a row without one keeps the menu unchanged.
fn stack_slot_token(text: &str) -> Option<(i64, &str)> {
    for token in text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')) {
        if let Some(hex) = token.strip_prefix("var_n") {
            if let Ok(value) = i64::from_str_radix(hex, 16) {
                return Some((-value, token));
            }
        } else if let Some(hex) = token.strip_prefix("var_") {
            if let Ok(value) = i64::from_str_radix(hex, 16) {
                return Some((value, token));
            }
        }
    }
    None
}

fn decode_string(bytes: &[u8], enc: i64) -> String {
    if enc == recraft_analysis::STRING_ENC_UTF16LE {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|unit| *unit != 0)
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        let end = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end]).into_owned()
    }
}

/// Index of the stored string covering `addr`, plus the byte offset into its text.
///
/// A literal does not have to start a stored string: compiler string pooling makes
/// `"===="` a suffix of an `"O====…"` run, so an address inside a run still names a
/// string. The store reads the literals in address order (`ORDER BY addr`) and drops
/// overlaps, so the only candidate is the last literal that starts at or before `addr`;
/// a binary search keeps the disassembly's data walk (which asks once per row) cheap.
fn string_row_at(strings: &[StringRow], addr: Address) -> Option<(usize, u32)> {
    let after = strings.partition_point(|row| row.addr <= addr);
    let index = after.checked_sub(1)?;
    let row = strings.get(index)?;
    let offset = addr.get().checked_sub(row.addr.get())?;
    (offset < u64::from(row.len)).then_some((index, offset as u32))
}

/// Bytes a literal occupies beyond its text: UTF-16LE literals end in two NUL bytes.
fn string_terminator_len(enc: i64) -> u64 {
    if enc == recraft_analysis::STRING_ENC_UTF16LE {
        2
    } else {
        1
    }
}

/// `; DATA XREF: …` comment for a data row (the first three referencing sites).
fn data_xref_comment(session: &Session, addr: Address) -> Option<String> {
    let xrefs = session.xrefs_to(addr).ok()?;
    if xrefs.is_empty() {
        return None;
    }
    let mut from = Vec::new();
    for xref in xrefs.iter().take(3) {
        let name = session
            .name_at(xref.from)
            .ok()
            .flatten()
            .unwrap_or_else(|| format!("{:#x}", xref.from.get()));
        from.push(name);
    }
    Some(format!("; DATA XREF: {}", from.join(", ")))
}

/// Strings in the PE `HEADER` segment.
///
/// IDA does not map the file header (`is_loaded(IMAGE_BASE) == 0`), so its string list can
/// never contain the DOS stub or the section table; ReCraft maps the header to address the
/// whole file. Hiding this segment keeps the panel comparable with IDA while everything in
/// a real section — including IDA-visible names such as `.text$mn` — is listed.
fn is_header_string(segment: &str) -> bool {
    segment.eq_ignore_ascii_case("HEADER")
}

/// One string as the Strings window shows it.
///
/// A literal may carry control characters — the PE DOS stub is
/// `This program cannot be run in DOS mode.\r\r\n$` — and egui breaks a galley at every
/// `\n`, so such a row used to paint as two lines centred on the row: the first line hung
/// over the header above it and the second line over the row below, both clipped by the row
/// rect. The window is a one-line-per-entry list (as in IDA), so control characters are
/// escaped into visible text instead. Printing the escape also keeps the row's `\t` from
/// breaking the tab-separated columns Ctrl+C copies.
fn string_cell_text(text: &str) -> Cow<'_, str> {
    if !text.chars().any(char::is_control) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                let code = ch as u32;
                if code <= 0xff {
                    out.push_str(&format!("\\x{code:02x}"));
                } else {
                    out.push_str(&format!("\\u{{{code:x}}}"));
                }
            }
            ch => out.push(ch),
        }
    }
    Cow::Owned(out)
}

/// Width of [`string_cell_text`]'s output in monospace cells, so the Strings window can size
/// its string column to the widest literal it holds (IDA sizes those columns the same way).
/// Characters the monospace fallback draws double-width count twice; treating every
/// non-ASCII glyph as wide would overstate Latin-1 text, and counting them all as one would
/// clip CJK.
fn string_cell_units(text: &str) -> usize {
    text.chars()
        .map(|ch| {
            let code = ch as u32;
            let wide = matches!(
                code,
                0x1100..=0x115f
                    | 0x2e80..=0x303e
                    | 0x3041..=0x33ff
                    | 0x3400..=0x4dbf
                    | 0x4e00..=0x9fff
                    | 0xa000..=0xa4cf
                    | 0xac00..=0xd7a3
                    | 0xf900..=0xfaff
                    | 0xfe30..=0xfe6f
                    | 0xff00..=0xff60
                    | 0xffe0..=0xffe6
                    | 0x1f300..=0x1f64f
                    | 0x1f900..=0x1f9ff
                    | 0x20000..=0x3fffd
            );
            if wide { 2 } else { 1 }
        })
        .sum()
}

/// Result of the background analysis worker.
enum AnalysisMessage {
    Done {
        functions: usize,
        strings: usize,
        elapsed_ms: f64,
    },
    Cancelled,
    Failed(String),
}

/// A running background analysis. The worker owns its own `Session` (separate SQLite
/// connection), so the UI stays responsive and can be cancelled cooperatively.
struct AnalysisJob {
    receiver: mpsc::Receiver<AnalysisMessage>,
    cancel: Arc<AtomicBool>,
    started: Instant,
}

/// Result of a background strings rebuild (IDA's *Setup…* → *Rebuild*).
enum StringsMessage {
    Done {
        strings: usize,
        min_len: usize,
        elapsed_ms: f64,
    },
    Failed(String),
}

/// A running strings rebuild: re-extracts the table with a new minimum length and
/// persists it, so the list (and the pseudocode literals that read it) refresh.
struct StringsJob {
    receiver: mpsc::Receiver<StringsMessage>,
}

/// Background LLM request; HTTP calls never run on the UI thread.
enum AiMessage {
    /// Assistant text (the answer shown in the result pane).
    Chunk(String),
    /// Chain-of-thought delta (`reasoning_content`), shown in a collapsible section.
    Reasoning(String),
    /// One tool call the agent made (shown in the tool trace).
    AgentTool(String),
    /// A model turn started (deltas after it belong to the same turn).
    AgentTurnStart,
    /// The deltas since AgentTurnStart were narration before tool calls: drop them.
    AgentRetract,
    /// The agent's conversation after a turn (kept for follow-up questions).
    AgentHistory(Vec<recraft_ai::ChatMessage>),
    Done(std::result::Result<String, String>),
}

/// Which request a background AI job runs; the completion path differs per kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AiJobKind {
    /// Prose explanation shown in the result pane.
    Explain,
    /// Rename proposals merged into the suggestions list.
    Suggest,
    /// Reconstructed C shown (and syntax-coloured) in the result pane.
    Polish,
    /// Project-level agent: the model calls read-only tools, then answers in prose.
    Agent,
    /// Stack-variable names for one function, merged into the variable suggestions list.
    VarSuggest,
}

struct AiJob {
    receiver: mpsc::Receiver<AiMessage>,
    started: Instant,
    kind: AiJobKind,
    prompt: Option<recraft_ai::PromptRequest>,
    /// Function the answer describes (Explain/Polish); the result pane keeps showing it
    /// so an answer never silently follows the cursor to another function.
    target: Option<Address>,
    /// Evidence-based rename proposals a [`AiJobKind::Suggest`] job merges the model's
    /// names into when the reply arrives.
    suggestions: Vec<recraft_ai::Proposal>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditKind {
    Rename,
    Comment,
    /// The Decompile view's trailing comment (IDA's `/`).
    PseudoComment,
    Prototype,
    /// Replacement text for one extracted string (Strings view).
    StringText,
}

/// IDA-style data operations on the current address.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DataOp {
    Code,
    Data,
    Ascii,
    Unknown,
    DefineFunction,
    UndefineFunction,
}

struct EditState {
    kind: EditKind,
    addr: Address,
    text: String,
    just_opened: bool,
}

/// Layout persistence (`%LOCALAPPDATA%\ReCraft\layout.json`). The version is bumped when
/// the default pane set changes so stale layouts fall back to the new default instead of
/// resurrecting the old arrangement. The theme is stored alongside the dock state.
const LAYOUT_VERSION: u32 = 9;

fn default_zoom() -> f32 {
    1.0
}

fn default_ai_sidebar() -> bool {
    true
}

fn default_ai_sidebar_width() -> f32 {
    360.0
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedLayout {
    version: u32,
    dock: egui_dock::DockState<Pane>,
    #[serde(default)]
    theme: ThemeMode,
    #[serde(default = "default_zoom")]
    zoom: f32,
    #[serde(default)]
    view_mode: ViewMode,
    #[serde(default)]
    lang: Lang,
    #[serde(default = "default_ai_sidebar")]
    ai_sidebar: bool,
    #[serde(default = "default_ai_sidebar_width")]
    ai_sidebar_width: f32,
    #[serde(default)]
    ai_fab_pos: Option<[f32; 2]>,
}

fn layout_store_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|dir| PathBuf::from(dir).join("ReCraft").join("layout.json"))
}

fn load_layout_from(path: &Path) -> Option<SavedLayout> {
    let text = std::fs::read_to_string(path).ok()?;
    let saved: SavedLayout = serde_json::from_str(&text).ok()?;
    (saved.version == LAYOUT_VERSION).then_some(saved)
}

fn save_layout_to(path: &Path, saved: &SavedLayout) -> std::result::Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string(saved).map_err(|error| error.to_string())?;
    // `egui` serializes NaN coordinates as null; skip until the dock has been laid out.
    if text.contains("\"x\":null") || text.contains("\"y\":null") {
        return Err("layout not laid out yet".to_owned());
    }
    std::fs::write(path, text).map_err(|error| error.to_string())
}

/// Recent-project list location (per-user, no elevation).
fn recent_store_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|dir| PathBuf::from(dir).join("ReCraft").join("recent.txt"))
}

fn load_recent() -> Vec<PathBuf> {
    let Some(path) = recent_store_path() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .take(8)
        .collect()
}

fn save_recent(recent: &[PathBuf]) {
    let Some(path) = recent_store_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let text = recent
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(path, text);
}

fn push_recent(recent: &mut Vec<PathBuf>, path: &Path) {
    recent.retain(|existing| existing != path);
    recent.insert(0, path.to_path_buf());
    recent.truncate(8);
}

/// Split operand text into candidate hex addresses / symbol tokens.
fn follow_tokens(operands: &str) -> Vec<String> {
    operands
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Characters that can spell a symbol name in pseudo-C. `$`, `@` and `?` cover IDA-style
/// names and MSVC decorations; Unicode letters cover user renames.
fn is_pseudo_symbol_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | '$' | '@' | '?')
}

/// Expand a caret index in a pseudo-C line to the whole symbol token around it.
///
/// The caret index comes from the laid-out text, which reports the closest character
/// boundary: clicking the right half of a name's last letter lands on the following `(`
/// instead. Trying the previous character too keeps the whole name clickable.
fn pseudo_symbol_at(text: &str, char_index: usize) -> Option<&str> {
    fn token_at(text: &str, char_index: usize) -> Option<&str> {
        let (byte, first) = text.char_indices().nth(char_index)?;
        if !is_pseudo_symbol_char(first) {
            return None;
        }
        let start = text[..byte]
            .char_indices()
            .rev()
            .find(|(_, ch)| !is_pseudo_symbol_char(*ch))
            .map_or(0, |(offset, ch)| offset + ch.len_utf8());
        let end = text[byte..]
            .char_indices()
            .find(|(_, ch)| !is_pseudo_symbol_char(*ch))
            .map_or(text.len(), |(offset, _)| byte + offset);
        Some(&text[start..end])
    }
    token_at(text, char_index).or_else(|| {
        char_index
            .checked_sub(1)
            .and_then(|previous| token_at(text, previous))
    })
}

/// Inner text of the C string literal containing `char_index`, without its quotes.
///
/// Escaped quotes (`\"`) stay part of the literal, so the returned slice still matches the
/// quoted text the decompiler recorded for that string. `None` means the caret is outside
/// any literal's quotes.
fn pseudo_string_literal_at(text: &str, char_index: usize) -> Option<&str> {
    fn literal_at(text: &str, char_index: usize) -> Option<&str> {
        let mut open: Option<(usize, usize)> = None;
        let mut escaped = false;
        for (index, (byte, ch)) in text.char_indices().enumerate() {
            let Some((opening, start)) = open else {
                if ch == '"' {
                    open = Some((index, byte));
                }
                continue;
            };
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                if char_index > opening && char_index < index {
                    return Some(&text[start + 1..byte]);
                }
                open = None;
            }
        }
        None
    }
    // As in `pseudo_symbol_at`, the caret reports the closest boundary: clicking the right
    // half of the last character lands on the closing quote. Trying the previous character
    // keeps the whole literal clickable.
    literal_at(text, char_index).or_else(|| {
        char_index
            .checked_sub(1)
            .and_then(|previous| literal_at(text, previous))
    })
}

/// Bytes named by the inner text of a C string literal rendered by the decompiler.
///
/// `quote_c_string` escapes quotes, backslashes, control characters and `\xNN`; reversing
/// those escapes gives the bytes a short literal points at, even when the string table does
/// not cover the address.
fn c_literal_bytes(text: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            let mut buffer = [0u8; 4];
            bytes.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
            continue;
        }
        match chars.next()? {
            'n' => bytes.push(b'\n'),
            'r' => bytes.push(b'\r'),
            't' => bytes.push(b'\t'),
            '\\' => bytes.push(b'\\'),
            '"' => bytes.push(b'"'),
            'x' => {
                let high = chars.next()?.to_digit(16)?;
                let low = chars.next()?.to_digit(16)?;
                bytes.push((high * 16 + low) as u8);
            }
            other => {
                // The decompiler emits no other escapes; keep an unknown one verbatim.
                bytes.push(b'\\');
                let mut buffer = [0u8; 4];
                bytes.extend_from_slice(other.encode_utf8(&mut buffer).as_bytes());
            }
        }
    }
    Some(bytes)
}

/// Address encoded in an IDA-style `sub_<hex>` auto name.
fn sub_symbol_address(symbol: &str) -> Option<Address> {
    let hex = symbol.strip_prefix("sub_")?;
    if hex.is_empty() || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    u64::from_str_radix(hex, 16).ok().map(Address::new)
}

/// Addresses a prose answer quotes (`0x...` with at least four hex digits, de-duplicated).
///
/// The agent and the explanation prompt both cite addresses; making them clickable is how
/// a reader checks a claim without retyping the address into the jump box.
fn address_links(text: &str) -> Vec<Address> {
    let bytes = text.as_bytes();
    let mut seen = BTreeSet::new();
    let mut links = Vec::new();
    let mut index = 0;
    while index + 3 < bytes.len() {
        if bytes[index] == b'0' && (bytes[index + 1] == b'x' || bytes[index + 1] == b'X') {
            let mut end = index + 2;
            while end < bytes.len() && bytes[end].is_ascii_hexdigit() {
                end += 1;
            }
            if end >= index + 6
                && let Ok(value) = u64::from_str_radix(&text[index + 2..end], 16)
                && seen.insert(value)
            {
                links.push(Address::new(value));
            }
            index = end;
            continue;
        }
        index += 1;
    }
    links
}

/// Built-in AI endpoint choices exposed by the settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AiMode {
    DeepSeek,
    OpenCodeGo,
    Custom,
}

/// What the AI result pane holds. Polished C is coloured like the Decompile view;
/// explanation reports are prose and stay plain text.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum AiResultKind {
    #[default]
    Prose,
    Code,
}

impl AiMode {
    fn from_settings(settings: &recraft_ai::ProviderSettings) -> Self {
        if settings.endpoint.contains("api.deepseek.com") {
            Self::DeepSeek
        } else if settings.endpoint.contains("opencode.ai") {
            Self::OpenCodeGo
        } else {
            Self::Custom
        }
    }

    fn preset(self) -> Option<recraft_ai::ProviderPreset> {
        match self {
            Self::DeepSeek => Some(recraft_ai::ProviderPreset::DeepSeek),
            Self::OpenCodeGo => Some(recraft_ai::ProviderPreset::OpenCodeGo),
            Self::Custom => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::DeepSeek => "DeepSeek",
            Self::OpenCodeGo => "OpenCode Go",
            Self::Custom => "Custom OpenAI-compatible",
        }
    }
}

/// Fuzzy command palette state (DESIGN §8.1: menu / command palette).
#[derive(Debug, Clone, Default)]
struct PaletteState {
    query: String,
    selected: usize,
}

/// Commands offered by the palette, in display order.
const PALETTE_COMMANDS: &[(&str, Action)] = &[
    ("ui_open_project", Action::Open),
    ("ui_close_project", Action::CloseProject),
    ("palette_import_binary_file", Action::Import),
    ("palette_analyze", Action::Analyze),
    ("palette_decompile", Action::Decompile),
    ("palette_undo", Action::Undo),
    ("palette_redo", Action::Redo),
    ("palette_toggle_view_mode", Action::ToggleViewMode),
    ("palette_goto_address", Action::GotoDialog),
    ("palette_rename", Action::EditName),
    ("palette_comment", Action::EditComment),
    (
        "palette_pseudo_comment",
        Action::EditPseudoComment { addr: None },
    ),
    ("palette_cross_references", Action::ShowXrefs),
    ("palette_ai_suggest", Action::AiSuggest),
    ("palette_toggle_language", Action::ToggleLang),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    None,
    Open,
    /// Unload the current project and go back to the welcome screen.
    CloseProject,
    Import,
    Analyze,
    /// IDA's F5: show the pseudo-C of the function under the cursor in the Decompile pane.
    Decompile,
    /// IDA's Tab: flip between the listing and the pseudo-C of the cursor's address.
    ToggleDisasmPseudo,
    CancelAnalysis,
    Undo,
    Redo,
    ToggleLang,
    ToggleViewMode,
    GotoDialog,
    EditName,
    EditComment,
    /// IDA's `/`: edit the trailing comment of the Decompile line under the cursor.
    /// `addr` names the row a context menu was opened on; `None` follows the cursor.
    EditPseudoComment {
        addr: Option<Address>,
    },
    /// Rename the stack variable at `offset` in `func` (context menu on a `var_XX` token).
    RenameVar {
        func: Address,
        offset: i64,
    },
    ShowXrefs,
    MoveSelection(i32),
    Page(i32),
    Back,
    Navigate(Address),
    AiSuggest,
    /// Name the stack variables of `func` (Decompile context menu).
    AiRenameVars {
        func: Address,
    },
    AiApply,
    AiClear,
    TogglePalette,
    PaletteCommand(usize),
    Follow,
    OpenRecent(usize),
    AiExplain,
    AiPolish,
    /// Run the polish request again for the current function, ignoring the session cache.
    AiRepolish,
    AiQuery,
    /// Run the project-level agent on the question in the agent box.
    AiAgent,
    /// Forget the agent conversation (next question starts from scratch).
    AiAgentReset,
    /// Ask the agent to dig into the function under the cursor.
    AiAgentDig,
    NavigateToDisasm(Address),
    /// Centre a block in the graph view without leaving graph mode (Graph overview click).
    GraphGoTo(Address),
    /// Open a callee's pseudo-C body in the Decompile pane (double-click on its name).
    EnterFunction(Address),
    SyncDisasm(Address),
    /// Select one pseudo-C row (by line index) and sync the disassembly cursor to its
    /// address. Unlike `SyncDisasm`, this keeps the pseudo-C listing in place.
    SyncPseudoLine {
        addr: Address,
        line: usize,
    },
    Forward,
    ApplyData(DataOp),
    /// Put the focused pane's current selection (or current row) on the clipboard.
    Copy,
    /// Select everything the focused pane can copy (Ctrl+A).
    SelectAll,
    EditPrototype,
    ShowStructs,
    FocusPane(Pane),
    ToggleAiSidebar,
    OpenAiSettings,
    ResetLayout,
    SetTheme(ThemeMode),
    ToggleTheme,
    ZoomIn,
    ZoomOut,
    ZoomReset,
}

/// OpenCode Go's gateway turns a 64k completion allowance into a very long first-token
/// wait. A direct answer (explanation, names, polished C) is normally a few thousand
/// tokens, so cap this gateway only; an explicit thinking request keeps the full budget.
fn direct_answer_token_limit(endpoint: &str, configured: usize) -> usize {
    if endpoint.contains("opencode.ai") {
        configured.min(8192)
    } else {
        configured
    }
}

pub struct ReCraftApp {
    session: Option<Session>,
    project: Option<PathBuf>,
    /// SQLite `data_version` last seen for `session`; a change means another connection
    /// (MCP server, second GUI, background analysis) committed to the project.
    data_version: i64,
    lang: Lang,
    i18n: I18n,
    tab: Tab,
    dock: Option<egui_dock::DockState<Pane>>,
    layout_saved: Option<String>,
    frame_count: u64,
    /// Fonts/theme installed on the first frame (Consolas/Segoe UI when available).
    ui_ready: bool,
    /// Active colour scheme (dark by default; light is IDA's classic look).
    theme: ThemeMode,
    applied_theme: Option<ThemeMode>,
    /// Output-window log (analysis/eval/status messages), oldest first.
    log: Vec<String>,
    /// Dragged text selection in the output window (indices into `log`).
    log_sel: Option<TextSelection>,
    /// `true` while the pointer is still down on an output-window text selection.
    log_drag: bool,
    status: String,
    is_error: bool,
    disasm_start: Address,
    disasm_rows: Vec<DisasmLine>,
    selected: usize,
    disasm_dirty: bool,
    /// Vertical scroll offset of the disassembly view, as reported by the scroll area.
    disasm_offset: f32,
    /// One-shot scroll offset to apply on the next frame (keeps rows in place when the
    /// window is re-anchored or reset).
    disasm_scroll_to: Option<f32>,
    /// Last egui pass that consumed keyboard input, so a multi-pass frame only handles a key
    /// press once.
    key_pass: Option<u64>,
    pseudo_start: Address,
    pseudo: Option<Vec<PseudoLine>>,
    /// Trailing (pseudocode) comments of the rendered function, keyed by the address of
    /// the line they annotate. `pseudo` already carries them at the end of that row; this
    /// map is what the edit dialog pre-fills and what undo/refresh rebuild the rows from.
    pseudo_comments: BTreeMap<Address, String>,
    /// Per row of `pseudo`, the character index where its trailing comment starts (`None`
    /// when the row has no comment). Names and quoted strings inside the comment must not
    /// become jump targets, so the view needs the boundary.
    pseudo_comment_cols: Vec<Option<usize>>,
    /// Quoted string literals of the current listing, with the data address each names.
    /// A double-click on one follows the literal instead of its instruction.
    pseudo_literals: Vec<(String, Address)>,
    /// Literals reached from the decompile view, keyed by data address. Kept separately
    /// because a later refresh re-anchors the listing and drops `pseudo_literals`, while
    /// short literals (`"\r\n"`, `")"`) still need a data row.
    literal_data: Vec<(Address, String)>,
    /// Row clicked in the pseudo-C view (0-based line identity). `None` means follow the
    /// disassembly address.
    pseudo_selected_line: Option<usize>,
    /// Address the Decompile view should centre on the next time it renders. `F5` sets it,
    /// mirroring IDA's decompile shortcut, which keeps the cursor's line in sight.
    pseudo_scroll_to: Option<Address>,
    pseudo_dirty: bool,
    /// Pane that owns Ctrl+C, set by the last click or drag inside that pane.
    copy_focus: CopyFocus,
    /// Dragged text selection in the disassembly listing (indices into `disasm_rows`).
    disasm_sel: Option<TextSelection>,
    /// `true` while the pointer is still down on a disassembly text selection.
    disasm_drag: bool,
    /// Dragged text selection in the Decompile view (indices into `pseudo`).
    pseudo_sel: Option<TextSelection>,
    /// `true` while the pointer is still down on a pseudo-C text selection.
    pseudo_drag: bool,
    /// Byte range picked in the Hex view.
    hex_sel: Option<HexSelection>,
    /// `true` while the pointer is still down on a hex selection.
    hex_drag: bool,
    /// Row picked in a dock list, keyed by the address it shows so a refresh keeps the
    /// highlight on the same item.
    list_sel: Option<(Pane, u64)>,
    functions: Vec<FunctionInfo>,
    /// Names parallel to `functions` (for the side list), refreshed with it.
    function_labels: Vec<String>,
    functions_dirty: bool,
    function_filter: String,
    strings: Vec<StringRow>,
    /// Hide strings in the `HEADER` segment (IDA never sees them: it does not map headers,
    /// so its list cannot contain the DOS stub or the section table).
    strings_hide_header: bool,
    /// Project setting (`strings.min_len`), i.e. IDA's *Minimal string length*.
    strings_min_len: usize,
    /// Draft value while the `Setup…` dialog is open.
    strings_setup: Option<usize>,
    /// Running strings rebuild (a new minimum length re-extracts and persists the table).
    strings_job: Option<StringsJob>,
    strings_dirty: bool,
    string_filter: String,
    names: Vec<(Address, String)>,
    names_filter: String,
    imports: Vec<ImportRow>,
    imports_dirty: bool,
    /// Open IDA-style `xrefs to …` dialog (`X`), frozen on the address it was opened for.
    xref_dialog: Option<XrefDialog>,
    cfg: Option<recraft_analysis::Cfg>,
    cfg_for: Option<Address>,
    cfg_dirty: bool,
    /// Graph-view zoom factor (0.5..=2.0), adjusted by the on-pane +/- controls.
    graph_zoom: f32,
    /// Graph or text mode of the single Code View pane.
    view_mode: ViewMode,
    /// Code-view font/row scale (0.75..=2.0), shared by Text/Pseudocode/Hex views.
    mono_zoom: f32,
    /// Last visible graph-canvas rectangle (for the overview viewport box).
    graph_visible: Option<egui::Rect>,
    /// Split fraction used to re-dock the Graph overview below Functions after it was
    /// taken out of the layout in text mode (kept so a resized overview survives toggles).
    graph_overview_fraction: f32,
    /// Block the graph view should centre on its next frame (set by an overview click).
    graph_center_on: Option<Address>,
    analysis: Option<AnalysisJob>,
    /// AI rename suggestions (never applied implicitly).
    ai_proposals: Vec<recraft_ai::Proposal>,
    /// Proposals the user ticked for a batched apply (targets, not list indices: the list
    /// is re-sorted when a suggestion is applied and would shift indices).
    ai_checked: BTreeSet<Address>,
    /// AI stack-variable names for one function (never applied implicitly).
    ai_var_proposals: Vec<recraft_ai::VarProposal>,
    /// Offsets the user ticked for a batched variable apply.
    ai_var_checked: BTreeSet<i64>,
    /// Function the variable proposals belong to.
    ai_var_function: Option<Address>,
    /// Function the explanation/polish result belongs to, so a stale answer cannot be
    /// mistaken for the function now under the cursor.
    ai_result_target: Option<Address>,
    ai_status: String,
    ai_mode: AiMode,
    ai_api_key: String,
    ai_show_key: bool,
    ai_settings: recraft_ai::ProviderSettings,
    ai_key_id: String,
    ai_cache: recraft_ai::PromptCache,
    ai_cache_fingerprint: Option<String>,
    ai_explanation: Option<String>,
    /// Whether [`Self::ai_explanation`] holds polished C (syntax-coloured) or a plain
    /// explanation report.
    ai_result_kind: AiResultKind,
    /// Streamed chain-of-thought (`reasoning_content`) for the running or last AI call.
    ai_reasoning: String,
    /// Whether the thinking section is expanded; collapsed automatically when a call ends.
    ai_reasoning_open: bool,
    /// UI toggle: request the provider's thinking mode and stream its reasoning.
    ai_thinking: bool,
    ai_query: String,
    ai_answer: Option<recraft_ai::QueryAnswer>,
    /// Question typed into the agent box.
    ai_agent_question: String,
    /// Send the current selection (pseudo-C text or the selected listing row) with the
    /// question, so "what does this do" has something to point at.
    ai_agent_use_selection: bool,
    /// Tool calls the running or last agent made (`name + arguments` per line).
    ai_agent_trace: Vec<String>,
    /// Conversation with the agent, kept so a follow-up question continues it.
    ai_agent_history: Vec<recraft_ai::ChatMessage>,
    /// Character offset in ai_explanation where the current model turn started; a turn
    /// that ends in tool calls retracts the narration it streamed.
    ai_agent_turn_mark: Option<usize>,
    ai_job: Option<AiJob>,
    ai_sidebar: bool,
    ai_sidebar_width: f32,
    ai_fab_pos: Option<egui::Pos2>,
    ai_settings_open: bool,
    /// The About window (Help → About): icon, version, license, project link.
    about_open: bool,
    /// Lazily uploaded 64×64 icon texture for the About window.
    about_icon: Option<egui::TextureHandle>,
    /// Editors for the four system prompts, loaded when the settings window opens.
    prompt_drafts: Vec<(recraft_ai::PromptKind, String)>,
    prompt_drafts_language: recraft_ai::PromptLanguage,
    prompt_drafts_ready: bool,
    struct_name: String,
    struct_size: u64,
    struct_fields: String,
    struct_slot: u8,
    edit: Option<EditState>,
    /// Pending stack-variable rename: function, stack offset and the edited text.
    var_edit: Option<(Address, i64, String)>,
    goto: Option<String>,
    palette: Option<PaletteState>,
    recent: Vec<PathBuf>,
    hex_unit: HexUnit,
    hex_start: Address,
    hex_cursor: Option<Address>,
    hex_synced: Option<Address>,
    history: Vec<Address>,
    history_pos: usize,
}

impl Default for ReCraftApp {
    fn default() -> Self {
        Self {
            session: None,
            project: None,
            data_version: 0,
            lang: Lang::En,
            i18n: I18n::new(),
            tab: Tab::Disasm,
            dock: None,
            layout_saved: None,
            frame_count: 0,
            ui_ready: false,
            theme: ThemeMode::Light,
            applied_theme: None,
            log: Vec::new(),
            log_sel: None,
            log_drag: false,
            status: String::new(),
            is_error: false,
            disasm_start: Address::ZERO,
            disasm_rows: Vec::new(),
            selected: 0,
            disasm_dirty: false,
            disasm_offset: 0.0,
            disasm_scroll_to: None,
            key_pass: None,
            pseudo_start: Address::ZERO,
            pseudo: None,
            pseudo_comments: BTreeMap::new(),
            pseudo_comment_cols: Vec::new(),
            pseudo_literals: Vec::new(),
            literal_data: Vec::new(),
            pseudo_selected_line: None,
            pseudo_scroll_to: None,
            pseudo_dirty: false,
            copy_focus: CopyFocus::None,
            disasm_sel: None,
            disasm_drag: false,
            pseudo_sel: None,
            pseudo_drag: false,
            hex_sel: None,
            hex_drag: false,
            list_sel: None,
            functions: Vec::new(),
            function_labels: Vec::new(),
            functions_dirty: false,
            function_filter: String::new(),
            strings: Vec::new(),
            strings_hide_header: true,
            strings_min_len: recraft_analysis::DEFAULT_MIN_STRING_LEN,
            strings_setup: None,
            strings_job: None,
            strings_dirty: false,
            string_filter: String::new(),
            names: Vec::new(),
            names_filter: String::new(),
            imports: Vec::new(),
            imports_dirty: false,
            xref_dialog: None,
            cfg: None,
            cfg_for: None,
            graph_zoom: 1.0,
            view_mode: ViewMode::Graph,
            mono_zoom: 1.0,
            graph_visible: None,
            graph_overview_fraction: GRAPH_OVERVIEW_SPLIT,
            graph_center_on: None,
            cfg_dirty: false,
            analysis: None,
            ai_proposals: Vec::new(),
            ai_checked: BTreeSet::new(),
            ai_var_proposals: Vec::new(),
            ai_var_checked: BTreeSet::new(),
            ai_var_function: None,
            ai_result_target: None,
            ai_status: String::new(),
            ai_mode: AiMode::Custom,
            ai_api_key: String::new(),
            ai_key_id: String::new(),
            ai_show_key: false,
            ai_settings: recraft_ai::ProviderSettings::default(),
            ai_cache: recraft_ai::PromptCache::new(),
            ai_cache_fingerprint: None,
            ai_explanation: None,
            ai_result_kind: AiResultKind::Prose,
            ai_reasoning: String::new(),
            ai_reasoning_open: false,
            ai_thinking: false,
            ai_query: String::new(),
            ai_answer: None,
            ai_agent_question: String::new(),
            ai_agent_use_selection: true,
            ai_agent_trace: Vec::new(),
            ai_agent_history: Vec::new(),
            ai_agent_turn_mark: None,
            ai_job: None,
            ai_sidebar: true,
            ai_sidebar_width: 360.0,
            ai_fab_pos: None,
            ai_settings_open: false,
            about_open: false,
            about_icon: None,
            prompt_drafts: Vec::new(),
            prompt_drafts_language: recraft_ai::PromptLanguage::En,
            prompt_drafts_ready: false,
            struct_name: String::new(),
            struct_size: 16,
            struct_fields: String::new(),
            struct_slot: 0,
            edit: None,
            var_edit: None,
            goto: None,
            palette: None,
            recent: Vec::new(),
            hex_unit: HexUnit::Byte,
            hex_start: Address::ZERO,
            hex_cursor: None,
            hex_synced: None,
            history: Vec::new(),
            history_pos: 0,
        }
    }
}

impl ReCraftApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Option<PathBuf>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut app = Self {
            recent: load_recent(),
            ai_settings: recraft_ai::ProviderSettings::load(),
            ..Self::default()
        };
        app.ai_mode = AiMode::from_settings(&app.ai_settings);
        app.ai_api_key = app.ai_settings.api_key.clone().unwrap_or_default();
        app.ai_key_id = app.ai_settings.provider_key_id();
        if app.ai_mode.preset().is_some() {
            app.ai_settings.max_tokens = app.ai_settings.max_tokens.max(65536);
        }
        app.restore_layout();
        app.remove_ai_tab_from_dock();
        if let Some(path) = initial {
            app.open_path(&path);
        } else {
            // A layout restored from the last session may leave another document tab (for
            // example Decompile) in front, which is empty and hides the open/import hints.
            app.show_welcome_tab();
            app.set_status(app.t("ui_drop_project"));
        }
        app
    }

    fn t(&self, key: &str) -> String {
        self.i18n.text(self.lang, key)
    }

    fn t_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        self.i18n.text_args(self.lang, key, args)
    }

    fn set_status(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.log(&message);
        self.status = message;
        self.is_error = false;
    }

    fn set_error(&mut self, error: impl std::fmt::Display) {
        let detail = error.to_string();
        let message = self.t_args("ui_error_prefix", &[("error", &detail)]);
        self.log(&message);
        self.status = message;
        self.is_error = true;
    }

    fn log(&mut self, message: &str) {
        self.log.push(message.to_owned());
        if self.log.len() > 500 {
            self.log.drain(..100);
            // The rows moved up: an output-window selection would point at other lines now.
            self.log_sel = None;
            self.log_drag = false;
        }
    }

    /// Open a project or executable (PE/ELF) dropped onto the window, and show a drop
    /// target overlay while a file is hovering over the app.
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let (dropped, hovering) = ctx.input(|input| {
            (
                input
                    .raw
                    .dropped_files
                    .first()
                    .map(|file| file.path().to_path_buf()),
                !input.raw.hovered_files.is_empty(),
            )
        });
        if let Some(path) = dropped {
            self.open_path(&path);
        }
        if hovering {
            let pal = self.palette();
            let title = self.t("ui_drop_to_open");
            let hint = self.t("ui_drop_hint");
            egui::Area::new(egui::Id::new("drop_overlay"))
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -36.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(pal.panel)
                        .stroke(egui::Stroke::new(1.5, pal.mnemonic))
                        .corner_radius(egui::CornerRadius::same(10))
                        .inner_margin(egui::Margin::same(20))
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 4],
                            blur: 18,
                            spread: 0,
                            color: Color32::from_black_alpha(70),
                        })
                        .show(ui, |ui| {
                            ui.set_min_width(250.0);
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new("↓").size(30.0).color(pal.mnemonic));
                                ui.label(RichText::new(title.as_str()).size(18.0).strong());
                                ui.label(RichText::new(hint.as_str()).size(12.0).color(pal.text));
                            });
                        });
                });
        }
    }

    // -------------------------------------------------------------- loading --

    fn open_path(&mut self, path: &Path) {
        let is_project = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("rcraft"));
        if is_project {
            match Session::open(path) {
                Ok(session) => {
                    self.session = Some(session);
                    self.project = Some(path.to_path_buf());
                    self.remember_recent(path);
                    self.after_open();
                    self.set_status(
                        self.t_args("ui_loaded_path", &[("path", &path.display().to_string())]),
                    );
                }
                Err(error) => self.set_error(error),
            }
        } else {
            let project = path.with_extension("rcraft");
            if project.exists() {
                self.open_path(&project);
            } else {
                self.import_binary(path, &project);
            }
        }
        // A failed open leaves the app without a project: keep the welcome page (and the
        // open/import hints it carries) in front instead of an empty document tab restored
        // from the saved layout. A successful open has a session, so this is a no-op.
        self.show_welcome_tab();
    }

    /// Import a PE or ELF executable into a new project. `Session::create` dispatches on
    /// the file's magic bytes, so the format is not decided here.
    fn import_binary(&mut self, source: &Path, project: &Path) {
        let Ok(bytes) = std::fs::read(source) else {
            self.set_error(self.t_args(
                "ui_cannot_read_path",
                &[("path", &source.display().to_string())],
            ));
            return;
        };
        match Session::create(project, &bytes, &mut |_, _| {}, &|| false) {
            Ok(session) => {
                self.session = Some(session);
                self.project = Some(project.to_path_buf());
                self.remember_recent(project);
                let message = format!("imported {} bytes -> {}", bytes.len(), project.display());
                self.after_open();
                self.set_status(message);
                // IDA runs auto-analysis as soon as a binary is loaded; do the same instead
                // of waiting for an explicit Analyze, which keeps `F5` free for decompiling.
                self.start_analysis();
            }
            Err(error) => self.set_error(error),
        }
    }

    /// Track the last opened projects (persisted under `%LOCALAPPDATA%\ReCraft`).
    fn remember_recent(&mut self, path: &Path) {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        push_recent(&mut self.recent, &path);
        save_recent(&self.recent);
    }

    /// Unload the open project and return to the welcome screen.
    ///
    /// Edits are written to the project as they are made, so closing has nothing to save.
    /// A running analysis owns a second SQLite connection, so it is cancelled before the
    /// session is dropped and its result is discarded with the job. Everything derived
    /// from the old project (listing, decompile, lists, history, AI results) is cleared;
    /// layout, theme, language, provider settings and the recent-project list survive
    /// because they are not project state.
    fn close_project(&mut self) {
        let Some(path) = self.project.clone() else {
            return;
        };
        if let Some(job) = self.analysis.as_ref() {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.analysis = None;
        self.ai_job = None;
        self.session = None;
        self.project = None;
        self.data_version = 0;

        self.tab = Tab::Disasm;
        self.disasm_start = Address::ZERO;
        self.disasm_rows.clear();
        self.selected = 0;
        self.disasm_dirty = false;
        self.disasm_offset = 0.0;
        self.disasm_scroll_to = Some(0.0);
        self.disasm_sel = None;
        self.disasm_drag = false;
        self.pseudo_start = Address::ZERO;
        self.pseudo = None;
        self.pseudo_comments.clear();
        self.pseudo_comment_cols.clear();
        self.pseudo_literals.clear();
        self.literal_data.clear();
        self.pseudo_selected_line = None;
        self.pseudo_scroll_to = None;
        self.pseudo_dirty = false;
        self.pseudo_sel = None;
        self.pseudo_drag = false;
        self.hex_start = Address::ZERO;
        self.hex_cursor = None;
        self.hex_synced = None;
        self.hex_sel = None;
        self.hex_drag = false;
        self.list_sel = None;

        self.functions.clear();
        self.function_labels.clear();
        self.functions_dirty = false;
        self.function_filter.clear();
        self.strings.clear();
        self.strings_dirty = false;
        self.string_filter.clear();
        self.names.clear();
        self.names_filter.clear();
        self.imports.clear();
        self.imports_dirty = false;
        self.xref_dialog = None;

        self.cfg = None;
        self.cfg_for = None;
        self.cfg_dirty = false;
        self.graph_visible = None;

        self.history.clear();
        self.history_pos = 0;
        self.edit = None;
        self.goto = None;

        self.ai_proposals.clear();
        self.ai_checked.clear();
        self.ai_result_target = None;
        self.ai_status.clear();
        self.ai_query.clear();
        self.ai_answer = None;
        self.ai_agent_question.clear();
        self.ai_agent_trace.clear();
        self.ai_agent_history.clear();
        self.ai_explanation = None;
        self.ai_result_kind = AiResultKind::Prose;
        self.ai_reasoning.clear();
        self.ai_reasoning_open = false;
        self.ai_cache = recraft_ai::PromptCache::new();
        self.ai_cache_fingerprint = None;

        self.struct_name.clear();
        self.struct_size = 16;
        self.struct_fields.clear();
        self.struct_slot = 0;

        // With the session gone, the Disassembly tab holds the welcome page again; leaving
        // an empty document tab in front would hide the open/import hints.
        self.show_welcome_tab();

        let message = self.t_args("ui_closed_path", &[("path", &path.display().to_string())]);
        self.set_status(message);
    }

    /// Resolve the first mapped address referenced by an instruction's operand text.
    fn follow_target(&self, line: &DisasmLine) -> Option<Address> {
        let session = self.session.as_ref()?;
        for token in follow_tokens(&line.operands) {
            let address = if let Some(hex) = token
                .strip_prefix("0x")
                .or_else(|| token.strip_prefix("0X"))
            {
                u64::from_str_radix(hex, 16).ok().map(Address::new)
            } else {
                session.addr_of_name(&token).ok().flatten()
            };
            if let Some(address) = address
                && session.segment_containing(address).is_some()
            {
                return Some(address);
            }
        }
        None
    }

    /// Function whose pseudo-C listing is on screen: the function containing
    /// [`Self::pseudo_start`], mirroring the anchor `refresh_pseudo` decompiles.
    fn pseudo_function_start(&self) -> Option<Address> {
        self.function_containing(self.pseudo_start)
            .map(|function| function.start)
    }

    fn function_containing(&self, addr: Address) -> Option<&FunctionInfo> {
        self.functions
            .iter()
            .find(|function| addr >= function.start && addr < function.end)
    }

    /// Resolve a symbol printed by the decompiler to the function a double-click enters.
    ///
    /// Callee names follow the decompiler's own naming rules (`resolve_name`): the stored
    /// name at the target, which the Functions refresh mirrors into
    /// [`Self::function_labels`], or the `sub_<hex>` fallback for targets that still have no
    /// name. Anything else (locals, keywords, data symbols) stays unresolved.
    fn pseudo_function_named(&self, symbol: &str) -> Option<Address> {
        // An auto name encodes its address, which stays unambiguous if a user renamed some
        // other function to the same text.
        if let Some(address) = sub_symbol_address(symbol)
            && let Some(function) = self.function_containing(address)
        {
            return Some(function.start);
        }
        let index = self
            .function_labels
            .iter()
            .position(|label| label == symbol)?;
        self.functions.get(index).map(|function| function.start)
    }

    /// Data address named by a rendered string literal. Identical literals at several
    /// addresses are disambiguated by the xref from the instruction under the click.
    fn pseudo_literal_address(&self, literal: &str, from: Address) -> Option<Address> {
        let quoted = format!("\"{literal}\"");
        let mut fallback = None;
        for (text, address) in &self.pseudo_literals {
            if *text != quoted {
                continue;
            }
            let referenced = self.session.as_ref().is_some_and(|session| {
                session
                    .xrefs_to(*address)
                    .is_ok_and(|xrefs| xrefs.iter().any(|xref| xref.from == from))
            });
            if referenced {
                return Some(*address);
            }
            fallback.get_or_insert(*address);
        }
        fallback
    }

    /// Action produced by double-clicking at `char_index` of a pseudo-C row.
    fn pseudo_double_click(&self, text: &str, char_index: usize, addr: Address) -> Action {
        // A quoted literal stands for data, not for the instruction that loads it.
        if let Some(literal) = pseudo_string_literal_at(text, char_index)
            && let Some(target) = self.pseudo_literal_address(literal, addr)
        {
            return Action::NavigateToDisasm(target);
        }
        match pseudo_symbol_at(text, char_index)
            .and_then(|symbol| self.pseudo_function_named(symbol))
        {
            // Stay put for the function's own name (header or recursive call): the
            // listing already shows it.
            Some(start) if self.pseudo_function_start() != Some(start) => {
                Action::EnterFunction(start)
            }
            _ => Action::NavigateToDisasm(addr),
        }
    }

    /// Character index of `pointer` inside one laid-out pseudo-C row. Returns `None` when
    /// the pointer is outside the code area (the line-number gutter or past the text), so
    /// those clicks keep their old meaning.
    fn pseudo_char_index_at(
        pointer: egui::Pos2,
        text_pos: egui::Pos2,
        text_clip: egui::Rect,
        galley: &egui::Galley,
    ) -> Option<usize> {
        if !text_clip.contains(pointer) {
            return None;
        }
        let local = pointer - text_pos;
        if local.x < 0.0 || local.x > galley.size().x {
            return None;
        }
        Some(galley.cursor_from_pos(local).index.0)
    }

    fn after_open(&mut self) {
        let entry = self
            .session
            .as_ref()
            .map(Session::entry_point)
            .unwrap_or_default();
        self.disasm_start = entry;
        self.selected = 0;
        self.disasm_dirty = true;
        self.disasm_scroll_to = Some(0.0);
        self.pseudo_start = entry;
        self.pseudo_dirty = true;
        self.pseudo_scroll_to = None;
        self.pseudo_literals.clear();
        self.literal_data.clear();
        self.functions_dirty = true;
        self.strings_dirty = true;
        self.strings_min_len = self
            .session
            .as_ref()
            .map(recraft_analysis::string_min_len)
            .unwrap_or(recraft_analysis::DEFAULT_MIN_STRING_LEN);
        self.strings_setup = None;
        self.imports_dirty = true;
        self.xref_dialog = None;
        self.cfg_dirty = true;
        self.cfg_for = None;
        self.history.clear();
        self.history.push(entry);
        self.history_pos = 0;
        self.edit = None;
        self.goto = None;
        // Baseline for external-change detection in `poll_external_changes`.
        self.data_version = self
            .session
            .as_ref()
            .and_then(|session| session.data_version().ok())
            .unwrap_or(0);
    }

    /// Pick up edits another SQLite connection committed to the open project (an MCP
    /// server, a second GUI, the background analysis job).
    ///
    /// `PRAGMA data_version` does not move for writes made through this connection, so the
    /// GUI does not reload itself after its own commands. The refresh is deliberately lazy:
    /// the dirty flags are the same ones undo sets, and the panes rebuild on their next
    /// frame, so a burst of external edits costs one reload rather than one per commit.
    fn poll_external_changes(&mut self) {
        if self.analysis.is_some() {
            // The analysis job owns a second connection and already refreshes when it
            // finishes; a concurrent reload could decompile a half-persisted function.
            return;
        }
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let Ok(version) = session.data_version() else {
            return;
        };
        if self.data_version == 0 {
            self.data_version = version;
            return;
        }
        if version == self.data_version {
            return;
        }
        self.data_version = version;
        self.functions_dirty = true;
        self.strings_dirty = true;
        self.imports_dirty = true;
        self.disasm_dirty = true;
        self.pseudo_dirty = true;
        self.cfg_dirty = true;
    }

    fn navigate(&mut self, addr: Address) {
        if self.session.is_none() {
            return;
        }
        self.remember_literal_data(addr);
        self.disasm_start = addr;
        self.selected = 0;
        self.disasm_dirty = true;
        // A jump must show the target, not wherever the previous view happened to be scrolled.
        self.disasm_scroll_to = Some(0.0);
        self.pseudo_start = addr;
        self.pseudo_dirty = true;
        self.history.truncate(self.history_pos + 1);
        self.history_pos = self.history.len();
        self.history.push(addr);
    }

    /// Keep the quoted literal behind `addr`, so the disassembly can keep showing it as
    /// data after the listing re-anchors (short literals have no stored string row).
    fn remember_literal_data(&mut self, addr: Address) {
        if self.literal_data.iter().any(|(known, _)| *known == addr) {
            return;
        }
        let Some((literal, _)) = self
            .pseudo_literals
            .iter()
            .find(|(_, address)| *address == addr)
        else {
            return;
        };
        self.literal_data.push((addr, literal.clone()));
    }

    fn run_data_op(&mut self, op: DataOp, addr: Address, len: usize) {
        let range = AddressRange::new(addr, Address::new(addr.get() + len as u64));
        let function_end = self.suggest_function_end(addr);
        let undefine_start = self
            .current_function()
            .map(|(start, _)| start)
            .unwrap_or(addr);
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let result = match op {
            DataOp::Code | DataOp::Data | DataOp::Ascii | DataOp::Unknown => {
                let kind = match op {
                    DataOp::Code => CodeData::Code,
                    DataOp::Data | DataOp::Ascii => CodeData::Data,
                    DataOp::Unknown => CodeData::Unknown,
                    _ => CodeData::Unknown,
                };
                session.execute(Command::SetCodeData { range, kind })
            }
            DataOp::DefineFunction => session.execute(Command::DefineFunc {
                start: addr,
                end: function_end,
            }),
            DataOp::UndefineFunction => session.execute(Command::UndefineFunc {
                start: undefine_start,
            }),
        };
        match result {
            Ok(_) => {
                self.disasm_dirty = true;
                self.pseudo_dirty = true;
                self.functions_dirty = true;
                self.strings_dirty = true;
                self.imports_dirty = true;
                if let Some(dialog) = self.xref_dialog.as_mut() {
                    dialog.dirty = true;
                }
                self.cfg_dirty = true;
                let op_label = self.t(match op {
                    DataOp::Code => "ui_op_code",
                    DataOp::Data => "ui_op_data",
                    DataOp::Ascii => "ui_op_ascii_data",
                    DataOp::Unknown => "ui_op_undefined",
                    DataOp::DefineFunction => "ui_op_function",
                    DataOp::UndefineFunction => "ui_op_function_removed",
                });
                let addr_text = format!("{:#x}", addr.get());
                self.set_status(
                    self.t_args("ui_data_op_at", &[("op", &op_label), ("addr", &addr_text)]),
                );
            }
            Err(error) => self.set_error(error),
        }
    }

    /// Estimate the end of a new function: stop at the next known function start or at a
    /// return/unconditional jump, whichever comes first (bounded linear scan).
    fn suggest_function_end(&mut self, addr: Address) -> Address {
        let limit = self
            .functions
            .iter()
            .filter(|function| function.start > addr)
            .map(|function| function.start.get())
            .min();
        let Some(session) = self.session.as_mut() else {
            return Address::new(addr.get() + 1);
        };
        let Ok(lines) = session.disasm(addr, 256) else {
            return Address::new(addr.get() + 1);
        };
        let mut end = addr.get() + 1;
        for line in lines {
            let next = line.addr.get() + u64::from(line.len).max(1);
            if line.addr >= addr {
                end = next;
            }
            if limit.is_some_and(|limit| end >= limit) {
                break;
            }
            if line.mnemonic.starts_with("ret") || line.mnemonic == "jmp" {
                break;
            }
        }
        if let Some(limit) = limit {
            end = end.min(limit);
        }
        Address::new(end.max(addr.get() + 1))
    }

    fn back(&mut self) {
        if self.history_pos == 0 || self.history.is_empty() {
            return;
        }
        self.history_pos -= 1;
        let addr = self.history[self.history_pos];
        self.disasm_start = addr;
        self.pseudo_start = addr;
        self.selected = 0;
        self.disasm_dirty = true;
        self.disasm_scroll_to = Some(0.0);
        self.pseudo_dirty = true;
    }

    /// IDA-style forward navigation (the inverse of [`Self::back`]).
    fn forward(&mut self) {
        if self.history_pos + 1 >= self.history.len() {
            return;
        }
        self.history_pos += 1;
        let addr = self.history[self.history_pos];
        self.disasm_start = addr;
        self.pseudo_start = addr;
        self.selected = 0;
        self.disasm_dirty = true;
        self.disasm_scroll_to = Some(0.0);
        self.pseudo_dirty = true;
    }

    fn run_edit(&mut self, edit: EditState) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        // A string's text is data: leading/trailing spaces and newlines are meaningful, so
        // it is taken verbatim. Names, comments and prototypes are trimmed.
        let text = match edit.kind {
            EditKind::StringText => edit.text.clone(),
            _ => edit.text.trim().to_owned(),
        };
        let result = match edit.kind {
            EditKind::Rename => session.execute(Command::Rename {
                addr: edit.addr,
                name: if text.is_empty() { None } else { Some(text) },
            }),
            EditKind::Comment => session.execute(Command::SetComment {
                addr: edit.addr,
                kind: CommentKind::Regular,
                text: if text.is_empty() { None } else { Some(text) },
            }),
            EditKind::PseudoComment => session.execute(Command::SetComment {
                addr: edit.addr,
                kind: CommentKind::Pseudo,
                text: if text.is_empty() { None } else { Some(text) },
            }),
            EditKind::Prototype => session.execute(Command::SetPrototype {
                addr: edit.addr,
                text: if text.is_empty() { None } else { Some(text) },
            }),
            EditKind::StringText => session.execute(Command::SetString {
                addr: edit.addr,
                text: if text.is_empty() { None } else { Some(text) },
                hidden: false,
            }),
        };
        match result {
            Ok(_) => {
                self.disasm_dirty = true;
                self.pseudo_dirty = true;
                // Function labels feed the Functions list and the pseudo-C symbol lookup
                // (double-clicking a callee name), so a rename must refresh them too.
                self.functions_dirty = true;
                // A string edit changes the row's text; the list is cheap to rebuild.
                if edit.kind == EditKind::StringText {
                    self.refresh_strings();
                }
                self.set_status(self.t_args(
                    "ui_changed_addr",
                    &[("addr", &format!("{:#x}", edit.addr.get()))],
                ));
            }
            Err(error) => self.set_error(error),
        }
    }

    /// Delete or restore one string in the list. Runs through the command bus so `Ctrl+Z`
    /// brings the row back and every front end sees the same table.
    fn apply_string_edit(&mut self, addr: Address, text: Option<String>, hidden: bool) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        match session.execute(Command::SetString { addr, text, hidden }) {
            Ok(_) => {
                self.strings_dirty = true;
                self.refresh_strings();
                self.set_status(self.t_args(
                    "ui_changed_addr",
                    &[("addr", &format!("{:#x}", addr.get()))],
                ));
            }
            Err(error) => self.set_error(error),
        }
    }

    // ------------------------------------------------------------- refresh --

    /// Install IDA-like fonts once, then apply the active (dark or light) palette.
    /// Missing font files simply keep egui's defaults.
    fn install_style(&mut self, ctx: &egui::Context) {
        if !self.ui_ready {
            self.ui_ready = true;
            self.install_fonts(ctx);
        }
        if self.applied_theme != Some(self.theme) {
            self.applied_theme = Some(self.theme);
            self.apply_theme(ctx);
        }
    }

    fn palette(&self) -> Palette {
        match self.theme {
            ThemeMode::Dark => Palette::DARK,
            ThemeMode::Light => Palette::LIGHT,
        }
    }

    fn mono_size(&self) -> f32 {
        MONO_SIZE * self.mono_zoom
    }

    fn row_height(&self) -> f32 {
        ROW_HEIGHT * self.mono_zoom
    }

    // ------------------------------------------------------------ clipboard --

    /// Text Ctrl+C puts on the clipboard for the focused pane, plus how many units it
    /// covers (rows, or bytes for the Hex view).
    fn clipboard_payload(&mut self) -> Option<(String, usize)> {
        match self.copy_focus {
            CopyFocus::Disasm => {
                if let Some(selection) = self.disasm_sel.filter(|selection| !selection.is_empty()) {
                    let rows: Vec<String> = self
                        .disasm_rows
                        .iter()
                        .map(|line| self.disasm_line_text(line))
                        .collect();
                    if let Some(text) = selected_text(selection, &rows) {
                        let count = text.lines().count().max(1);
                        return Some((text, count));
                    }
                }
                let line = self.disasm_rows.get(self.selected)?;
                Some((self.disasm_line_copy_text(line), 1))
            }
            CopyFocus::Pseudo => {
                let lines = self.pseudo.as_ref()?;
                if let Some(selection) = self.pseudo_sel.filter(|selection| !selection.is_empty()) {
                    let rows: Vec<String> = lines.iter().map(|line| line.text.clone()).collect();
                    let text = selected_text(selection, &rows)?;
                    let count = text.lines().count().max(1);
                    return Some((text, count));
                }
                let index = self.active_pseudo_line(lines)?;
                Some((lines.get(index)?.text.trim_end().to_owned(), 1))
            }
            CopyFocus::Hex => {
                let (start, count) = match self.hex_sel {
                    Some(selection) => (selection.range().0, selection.len().min(256) as usize),
                    None => (self.hex_cursor?, 1),
                };
                let bytes = self.hex_selection_bytes(start, count);
                if bytes.is_empty() {
                    return None;
                }
                let len = bytes.len();
                Some((Self::hex_dump_text(&bytes, false), len))
            }
            CopyFocus::List(pane) => {
                let (_, key) = self.list_sel.filter(|(selected, _)| *selected == pane)?;
                Some((self.list_row_text(pane, key)?, 1))
            }
            CopyFocus::Output => {
                let selection = self.log_sel.filter(|selection| !selection.is_empty())?;
                let text = selected_text(selection, &self.log)?;
                if text.is_empty() {
                    return None;
                }
                let (from, to) = selection.ordered();
                let rows = to.row.saturating_sub(from.row) + 1;
                Some((text, rows))
            }
            CopyFocus::None => None,
        }
    }

    /// Pane Ctrl+C and Ctrl+A act on: the last pane the user clicked in, or the view being
    /// read when nothing was clicked yet.
    fn effective_copy_focus(&self) -> CopyFocus {
        if self.copy_focus != CopyFocus::None {
            return self.copy_focus;
        }
        if self.tab == Tab::Pseudo && self.pseudo.is_some() {
            CopyFocus::Pseudo
        } else {
            CopyFocus::Disasm
        }
    }

    /// Copy the focused pane's selection (or its current row) to the clipboard.
    fn copy_focused(&mut self, ctx: &egui::Context) {
        self.copy_focus = self.effective_copy_focus();
        let Some((text, count)) = self
            .clipboard_payload()
            .filter(|(text, _)| !text.is_empty())
        else {
            self.set_status(self.t("ui_nothing_to_copy").to_owned());
            return;
        };
        let message = match self.copy_focus {
            CopyFocus::Hex => self.t_args("ui_copied_bytes", &[("count", &count.to_string())]),
            CopyFocus::Output => self.output_copy_message(&text),
            _ => self.t_args("ui_copied_lines", &[("count", &count.to_string())]),
        };
        ctx.copy_text(text);
        self.set_status(message);
    }

    /// Is the Output-window selection a whole run of log rows? A fragment (a word, part of a
    /// line, or a drag that stops at the start of the next row) reports characters instead of
    /// lines: calling a piece of a line "已复制 1 行" reads as if the whole line was copied.
    fn output_selection_is_whole_lines(&self) -> bool {
        let Some(selection) = self.log_sel else {
            return false;
        };
        let (from, to) = selection.ordered();
        if from.char != 0 || from.row >= self.log.len() {
            return false;
        }
        let last = to.row.min(self.log.len().saturating_sub(1));
        to.char >= self.log[last].chars().count()
    }

    /// Status line for a copy out of the Output window (IDA copies silently; this app's
    /// status bar reports what landed on the clipboard).
    fn output_copy_message(&self, text: &str) -> String {
        if self.output_selection_is_whole_lines() {
            let rows = self
                .log_sel
                .map(|selection| {
                    let (from, to) = selection.ordered();
                    to.row.saturating_sub(from.row) + 1
                })
                .unwrap_or(1);
            return self.t_args("ui_copied_lines", &[("count", &rows.to_string())]);
        }
        let chars = text.chars().filter(|character| *character != '\n').count();
        self.t_args("ui_copied_chars", &[("count", &chars.to_string())])
    }

    /// Ctrl+A: select everything the focused pane can copy.
    fn select_all_in_focus(&mut self) {
        match self.effective_copy_focus() {
            CopyFocus::Disasm => {
                let Some(last) = self.disasm_rows.len().checked_sub(1) else {
                    return;
                };
                let end = self
                    .disasm_line_text(&self.disasm_rows[last])
                    .chars()
                    .count();
                self.disasm_sel = Some(TextSelection {
                    anchor: TextPoint { row: 0, char: 0 },
                    focus: TextPoint {
                        row: last,
                        char: end,
                    },
                });
            }
            CopyFocus::Pseudo => {
                let Some(lines) = self.pseudo.as_ref() else {
                    return;
                };
                let Some(last) = lines.len().checked_sub(1) else {
                    return;
                };
                let end = lines[last].text.chars().count();
                self.pseudo_sel = Some(TextSelection {
                    anchor: TextPoint { row: 0, char: 0 },
                    focus: TextPoint {
                        row: last,
                        char: end,
                    },
                });
            }
            CopyFocus::Hex => {
                let start = self.hex_start;
                let count = self.hex_selection_bytes(start, 256).len();
                if count > 0 {
                    self.hex_sel = Some(HexSelection {
                        anchor: start,
                        focus: Address::new(start.get() + count as u64 - 1),
                    });
                }
            }
            CopyFocus::Output => {
                // IDA's Ctrl+A in the Output window selects the whole log, not one line.
                let Some(last) = self.log.len().checked_sub(1) else {
                    return;
                };
                let end = self.log[last].chars().count();
                self.log_sel = Some(TextSelection {
                    anchor: TextPoint { row: 0, char: 0 },
                    focus: TextPoint {
                        row: last,
                        char: end,
                    },
                });
            }
            _ => {}
        }
    }

    /// Bytes at `start`, at most `count`, read straight from the session.
    fn hex_selection_bytes(&mut self, start: Address, count: usize) -> Vec<u8> {
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        let mut buffer = vec![0u8; count.max(1)];
        let read = session.read_at(start, &mut buffer).unwrap_or(0);
        buffer.truncate(read);
        buffer
    }

    /// Hex or ASCII text of `bytes`, 16 per line (the Hex view's copy commands).
    fn hex_dump_text(bytes: &[u8], ascii: bool) -> String {
        bytes
            .chunks(16)
            .map(|chunk| {
                if ascii {
                    chunk
                        .iter()
                        .map(|byte| {
                            if (0x20..=0x7e).contains(byte) {
                                *byte as char
                            } else {
                                '.'
                            }
                        })
                        .collect::<String>()
                } else {
                    chunk
                        .iter()
                        .map(|byte| format!("{byte:02X}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Text of one dock-list row as Ctrl+C copies it: the visible columns, tab separated.
    fn list_row_text(&self, pane: Pane, key: u64) -> Option<String> {
        match pane {
            Pane::Strings => {
                let row = self.strings.iter().find(|row| row.addr.get() == key)?;
                let address = if row.segment.is_empty() {
                    format!("{:016x}", row.addr.get())
                } else {
                    format!("{}:{:016x}", row.segment, row.addr.get())
                };
                let encoding = if row.enc == recraft_analysis::STRING_ENC_UTF16LE {
                    "C16"
                } else {
                    "C"
                };
                Some(format!(
                    "{address}\t{:08x}\t{encoding}\t{}",
                    row.len,
                    string_cell_text(&row.text)
                ))
            }
            Pane::Imports => {
                let import = self.imports.iter().find(|import| import.iat.get() == key)?;
                Some(format!(
                    "{}!{}\t{}\t×{}",
                    import.module, import.api, import.category, import.refs
                ))
            }
            Pane::Exports => {
                let (name, address, forwarder) = self
                    .exports()
                    .into_iter()
                    .find(|(_, address, _)| address.get() == key)?;
                Some(format!(
                    "{name}\t{:#x}\t{}",
                    address.get(),
                    forwarder.unwrap_or_default()
                ))
            }
            Pane::Names => {
                let (address, name) = self
                    .names
                    .iter()
                    .find(|(address, _)| address.get() == key)?;
                Some(format!("{:#012x}\t{name}", address.get()))
            }
            Pane::Functions => {
                let index = self
                    .functions
                    .iter()
                    .position(|function| function.start.get() == key)?;
                let name = self
                    .function_labels
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| format!("sub_{key:x}"));
                let segment = self
                    .session
                    .as_ref()
                    .and_then(|session| session.segment_containing(self.functions[index].start))
                    .map(|segment| segment.name.clone())
                    .unwrap_or_default();
                Some(format!("{name}\t{segment}\t{key:#012x}"))
            }
            Pane::Segments => {
                let segment = self
                    .session
                    .as_ref()?
                    .segments()
                    .iter()
                    .find(|segment| segment.range.start.get() == key)?;
                Some(format!(
                    "{}\t{:#012x}..{:#012x}\t{}",
                    segment.name,
                    segment.range.start.get(),
                    segment.range.end.get(),
                    if segment.perm.is_executable() {
                        "X"
                    } else if segment.perm.is_readable() {
                        "R"
                    } else {
                        "W"
                    }
                ))
            }
            _ => None,
        }
    }

    /// PE exports as (name, address, forwarder), shared by the Exports view and copy.
    fn exports(&self) -> Vec<(String, Address, Option<String>)> {
        let Some(session) = self.session.as_ref() else {
            return Vec::new();
        };
        let image = session.image();
        image
            .exports
            .iter()
            .map(|export| {
                let va = image.image_base.get().saturating_add(u64::from(export.rva));
                let name = export
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("#{}", export.ordinal));
                (name, Address::new(va), export.forwarder.clone())
            })
            .collect()
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let pal = self.palette();
        let mut visuals = match self.theme {
            ThemeMode::Dark => egui::Visuals::dark(),
            ThemeMode::Light => egui::Visuals::light(),
        };
        // Surface ladder (docs/UI.md): view < panel < raised, separated by hairline borders
        // instead of shadows, with one accent for links, selection and focus.
        visuals.panel_fill = pal.panel;
        visuals.window_fill = pal.surface_2;
        visuals.extreme_bg_color = pal.view;
        visuals.faint_bg_color = pal.surface_2;
        visuals.override_text_color = Some(pal.text);
        visuals.window_stroke = egui::Stroke::new(1.0, pal.border_strong);
        visuals.selection.bg_fill = pal.accent.gamma_multiply(0.18);
        visuals.selection.stroke = egui::Stroke::new(1.0, pal.accent);
        visuals.hyperlink_color = pal.accent;
        let widgets = &mut visuals.widgets;
        widgets.noninteractive.bg_fill = pal.panel;
        widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, pal.border);
        widgets.inactive.bg_fill = pal.surface_2;
        widgets.inactive.weak_bg_fill = pal.surface_2;
        widgets.inactive.bg_stroke = egui::Stroke::new(1.0, pal.border);
        widgets.hovered.bg_fill = pal.surface_2;
        widgets.hovered.weak_bg_fill = pal.surface_2;
        widgets.hovered.bg_stroke = egui::Stroke::new(1.0, pal.border_strong);
        widgets.active.bg_fill = pal.accent.gamma_multiply(0.12);
        widgets.active.weak_bg_fill = pal.accent.gamma_multiply(0.12);
        widgets.active.bg_stroke = egui::Stroke::new(1.0, pal.accent);
        widgets.open.bg_stroke = egui::Stroke::new(1.0, pal.border_strong);
        // Hairline borders do the layering; only floating things get a soft shadow.
        visuals.popup_shadow = egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: egui::Color32::from_black_alpha(90),
        };
        visuals.window_shadow = visuals.popup_shadow;
        ctx.set_theme(match self.theme {
            ThemeMode::Dark => egui::Theme::Dark,
            ThemeMode::Light => egui::Theme::Light,
        });
        // Ctrl+= / Ctrl+- / Ctrl+0 are reserved for code-view zoom, not the whole GUI.
        ctx.options_mut(|options| options.zoom_with_keyboard = false);
        ctx.set_visuals(visuals);
        ctx.all_styles_mut(|style| {
            use egui::{FontFamily, FontId, TextStyle};
            style
                .text_styles
                .insert(TextStyle::Body, FontId::proportional(13.0));
            style
                .text_styles
                .insert(TextStyle::Button, FontId::proportional(13.0));
            style
                .text_styles
                .insert(TextStyle::Small, FontId::proportional(11.0));
            style.text_styles.insert(
                TextStyle::Monospace,
                FontId::new(MONO_SIZE, FontFamily::Monospace),
            );
            // Spacing scale 4 / 8 / 12 / 16 and radii 6 / 10 / 12 (docs/UI.md §4).
            style.spacing.item_spacing = egui::vec2(8.0, 4.0);
            style.spacing.button_padding = egui::vec2(10.0, 4.0);
            style.spacing.interact_size.y = 22.0;
            for widget in [
                &mut style.visuals.widgets.noninteractive,
                &mut style.visuals.widgets.inactive,
                &mut style.visuals.widgets.hovered,
                &mut style.visuals.widgets.active,
                &mut style.visuals.widgets.open,
            ] {
                widget.corner_radius = egui::CornerRadius::same(6);
            }
            style.visuals.window_corner_radius = egui::CornerRadius::same(12);
            style.visuals.menu_corner_radius = egui::CornerRadius::same(10);
        });
    }

    fn install_fonts(&mut self, ctx: &egui::Context) {
        if let Some(fonts) = ui_fonts() {
            ctx.set_fonts(fonts);
        }
    }

    fn tick(&mut self) {
        self.poll_analysis();
        self.poll_strings_job();
        self.poll_ai_job();
        if self.disasm_dirty {
            self.refresh_disasm();
        }
        if self.pseudo_dirty {
            self.refresh_pseudo();
        }
        if self.functions_dirty {
            self.refresh_functions();
        }
        if self.strings_dirty {
            self.refresh_strings();
        }
        if self.imports_dirty {
            self.refresh_imports();
        }
        self.refresh_xrefs();
        // A jump, an undo or a restored layout can leave graph mode pointing at an item the
        // graph cannot draw; IDA falls back to text in that case (see the rule below).
        self.enforce_graph_view_mode();
        if self.view_mode == ViewMode::Graph {
            self.refresh_cfg();
        }
        self.sync_hex();
    }

    /// Anchor the hex pane on the current disassembly selection (unless the user picked
    /// an explicit hex position).
    fn sync_hex(&mut self) {
        let Some(anchor) = self.cursor_address() else {
            return;
        };
        if self.hex_synced == Some(anchor) {
            return;
        }
        self.hex_start = hex_row_start(anchor);
        self.hex_cursor = Some(anchor);
        self.hex_synced = Some(anchor);
        // The dump moved: a byte range picked in the old window no longer means anything.
        self.hex_sel = None;
        self.hex_drag = false;
    }

    /// Address under the cursor: the active pseudo-C row while the Decompile view owns the
    /// screen, otherwise the disassembly selection. Hex sync and `F5` both anchor on it, so
    /// the decompile target always matches what the user is looking at.
    fn cursor_address(&self) -> Option<Address> {
        if self.tab == Tab::Pseudo {
            // Follow the active pseudo-C row, not the listing's first address.
            self.pseudo_cursor_address()
                .or_else(|| self.disasm_rows.get(self.selected).map(|line| line.addr))
                .or(Some(self.pseudo_start))
        } else if self.tab == Tab::Disasm && self.view_mode == ViewMode::Graph {
            Some(self.pseudo_start)
        } else {
            self.disasm_rows.get(self.selected).map(|line| line.addr)
        }
    }

    /// Address of the pseudo-C row under the cursor. Returns `None` when the listing has no
    /// row matching the disassembly selection — Graph mode keeps that selection from before
    /// the function on screen, so the listing legitimately disagrees with it.
    fn pseudo_cursor_address(&self) -> Option<Address> {
        self.pseudo.as_ref().and_then(|lines| {
            self.active_pseudo_line(lines)
                .and_then(|index| lines.get(index))
                .map(|line| line.addr)
        })
    }

    /// IDA's `F5`: decompile the function under the cursor, bring the Decompile view to the
    /// front and keep the cursor's line in view.
    fn decompile_cursor_function(&mut self) {
        if self.session.is_none() {
            return;
        }
        let Some(addr) = self.cursor_address() else {
            return;
        };
        let target = self
            .function_containing(addr)
            .map(|function| function.start);
        // Re-decompiling a listing that already shows this function just re-centres it;
        // only a different function retires the current rows and click anchor.
        if target != self.pseudo_function_start() {
            self.pseudo_dirty = true;
            self.pseudo_selected_line = None;
        }
        self.pseudo_start = addr;
        self.pseudo_scroll_to = Some(addr);
        self.tab = Tab::Pseudo;
        self.focus_pane(Pane::Pseudo);
    }

    /// IDA's `Tab`: flip between the listing and the pseudo-C of the address under the
    /// cursor.
    ///
    /// Leaving the listing is exactly `F5`. Leaving the pseudo-C re-anchors the listing on
    /// the cursor's row without retiring the pseudo-C (so its row highlight survives) and
    /// without a history entry: IDA's Tab is a view flip, not a jump. That also means a
    /// second Tab lands back on the same line instead of decompiling again.
    fn toggle_disasm_pseudo(&mut self) {
        if self.session.is_none() {
            return;
        }
        if self.tab != Tab::Pseudo {
            self.decompile_cursor_function();
            return;
        }
        // The row the cursor is on. Graph mode has no row cursor and its anchor is the
        // pseudo-C function, so the anchor is what goes back on screen.
        let addr = self.pseudo_cursor_address().unwrap_or(self.pseudo_start);
        self.disasm_start = addr;
        self.selected = 0;
        self.disasm_dirty = true;
        // The target row opens at the top, like following it from the pseudo-C view.
        self.disasm_scroll_to = Some(0.0);
        // Graph mode anchors on `pseudo_start`, and coming back re-centres on the same row.
        self.pseudo_start = addr;
        self.tab = Tab::Disasm;
        self.focus_pane(Pane::View);
    }

    /// Start the background analysis pass for the open project, unless one is running.
    ///
    /// IDA analyses a freshly loaded binary on its own, so `F5` is free for the decompiler;
    /// importing a PE starts this pass and re-running it later is a menu command.
    fn start_analysis(&mut self) {
        if self.analysis.is_some() {
            return;
        }
        let Some(project) = self.project.clone() else {
            self.set_status(self.t("ui_no_project_2").to_owned());
            return;
        };
        let (sender, receiver) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        std::thread::spawn(move || {
            let message = run_analysis_job(&project, &worker_cancel);
            let _ = sender.send(message);
        });
        self.analysis = Some(AnalysisJob {
            receiver,
            cancel,
            started: Instant::now(),
        });
        self.set_status(self.t("ui_analyzing_2").to_owned());
    }

    fn poll_analysis(&mut self) {
        let Some(job) = self.analysis.as_ref() else {
            return;
        };
        let message = match job.receiver.try_recv() {
            Ok(message) => message,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                AnalysisMessage::Failed("analysis worker stopped unexpectedly".to_owned())
            }
        };
        self.analysis = None;
        match message {
            AnalysisMessage::Done {
                functions,
                strings,
                elapsed_ms,
            } => {
                self.functions_dirty = true;
                self.disasm_dirty = true;
                self.pseudo_dirty = true;
                self.strings_dirty = true;
                self.imports_dirty = true;
                if let Some(dialog) = self.xref_dialog.as_mut() {
                    dialog.dirty = true;
                }
                self.cfg_dirty = true;
                self.cfg_for = None;
                self.set_status(self.t_args(
                    "ui_analysis_done",
                    &[
                        ("functions", &functions.to_string()),
                        ("strings", &strings.to_string()),
                        ("ms", &format!("{elapsed_ms:.0}")),
                    ],
                ));
            }
            AnalysisMessage::Cancelled => {
                self.set_status(self.t("ui_analysis_cancelled").to_owned());
            }
            AnalysisMessage::Failed(error) => {
                self.set_error(recraft_common::Error::internal(error));
            }
        }
    }

    /// Handle `Strings → Setup… → Rebuild`: re-extract the list with `min_len` in the
    /// background, then refresh the panel (and any pseudocode that shows literals).
    fn start_strings_job(&mut self, min_len: usize) {
        if self.strings_job.is_some() {
            return;
        }
        let Some(project) = self.project.clone() else {
            self.set_status(self.t("ui_no_project_2").to_owned());
            return;
        };
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(run_strings_job(&project, min_len));
        });
        self.strings_job = Some(StringsJob { receiver });
        self.set_status(self.t_args("ui_strings_rebuilding", &[("min", &min_len.to_string())]));
    }

    fn poll_strings_job(&mut self) {
        let Some(job) = self.strings_job.as_ref() else {
            return;
        };
        let message = match job.receiver.try_recv() {
            Ok(message) => message,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                StringsMessage::Failed("strings worker stopped unexpectedly".to_owned())
            }
        };
        self.strings_job = None;
        match message {
            StringsMessage::Done {
                strings,
                min_len,
                elapsed_ms,
            } => {
                self.strings_min_len = min_len;
                self.strings_dirty = true;
                self.pseudo_dirty = true;
                self.disasm_dirty = true;
                self.set_status(self.t_args(
                    "ui_strings_rebuilt",
                    &[
                        ("count", &strings.to_string()),
                        ("min", &min_len.to_string()),
                        ("ms", &format!("{elapsed_ms:.0}")),
                    ],
                ));
            }
            StringsMessage::Failed(error) => {
                self.set_error(recraft_common::Error::internal(error));
            }
        }
    }

    /// Poll the running AI request (explain / suggest names / polish).
    fn poll_ai_job(&mut self) {
        let Some(job) = self.ai_job.take() else {
            return;
        };
        // A suggest reply is a list of rename lines, so its stream is not shown as prose.
        let mut received = false;
        loop {
            match job.receiver.try_recv() {
                Ok(AiMessage::Chunk(chunk)) => {
                    received = true;
                    if job.kind != AiJobKind::Suggest {
                        self.ai_explanation
                            .get_or_insert_with(String::new)
                            .push_str(&chunk);
                    }
                    self.ai_status = self.t("ui_streaming").to_owned();
                }
                Ok(AiMessage::Reasoning(chunk)) => {
                    received = true;
                    self.ai_reasoning.push_str(&chunk);
                    self.ai_reasoning_open = true;
                    self.ai_status = self.t("ui_thinking").to_owned();
                }
                Ok(AiMessage::AgentTool(line)) => {
                    received = true;
                    self.ai_agent_trace.push(line);
                    self.ai_status = self.t_args(
                        "ui_agent_tool",
                        &[("count", &self.ai_agent_trace.len().to_string())],
                    );
                }
                Ok(AiMessage::AgentTurnStart) => {
                    received = true;
                    self.ai_agent_turn_mark = Some(
                        self.ai_explanation
                            .as_ref()
                            .map_or(0, |text| text.chars().count()),
                    );
                }
                Ok(AiMessage::AgentRetract) => {
                    received = true;
                    if let Some(mark) = self.ai_agent_turn_mark.take()
                        && let Some(text) = self.ai_explanation.clone()
                    {
                        let head: String = text.chars().take(mark).collect();
                        let tail: String = text.chars().skip(mark).collect();
                        if !tail.trim().is_empty() {
                            // Keep the narration as trace context instead of dropping it.
                            self.ai_agent_trace
                                .push(format!("… {}", recraft_ai::truncate_text(tail.trim(), 160)));
                        }
                        self.ai_explanation = (!head.is_empty()).then_some(head);
                    }
                }
                Ok(AiMessage::AgentHistory(history)) => {
                    self.ai_agent_history = history;
                    received = true;
                }
                Ok(AiMessage::Done(Ok(text))) => {
                    let elapsed = job.started.elapsed().as_millis();
                    // Fold the chain-of-thought once the answer is complete.
                    self.ai_reasoning_open = false;
                    match job.kind {
                        AiJobKind::Polish => {
                            if let Some(prompt) = job.prompt.as_ref() {
                                self.ai_cache.insert(prompt, &text);
                            }
                            self.ai_explanation = Some(text);
                            self.ai_result_kind = AiResultKind::Code;
                            self.ai_result_target = job.target;
                            self.ai_status =
                                self.t_args("ui_polish_done", &[("ms", &elapsed.to_string())]);
                        }
                        AiJobKind::Explain => {
                            self.ai_explanation = Some(text);
                            // The model answers with prose, not pseudo-C: no syntax colours.
                            self.ai_result_kind = AiResultKind::Prose;
                            self.ai_result_target = job.target;
                            self.ai_status =
                                self.t_args("ui_explain_done", &[("ms", &elapsed.to_string())]);
                        }
                        AiJobKind::Agent => {
                            self.ai_explanation = Some(text);
                            self.ai_result_kind = AiResultKind::Prose;
                            self.ai_result_target = None;
                            self.ai_status =
                                self.t_args("ui_agent_done", &[("ms", &elapsed.to_string())]);
                        }
                        AiJobKind::Suggest => {
                            let mut proposals = job.suggestions;
                            let updated = recraft_ai::apply_rename_reply(&mut proposals, &text);
                            let total = proposals.len();
                            self.ai_proposals = proposals;
                            self.ai_checked.clear();
                            self.ai_status = if updated == 0 {
                                self.t("ui_suggest_unparsed").to_owned()
                            } else {
                                self.t_args(
                                    "ui_suggest_done",
                                    &[
                                        ("named", &updated.to_string()),
                                        ("total", &total.to_string()),
                                        ("ms", &elapsed.to_string()),
                                    ],
                                )
                            };
                        }
                        AiJobKind::VarSuggest => {
                            let proposals = recraft_ai::parse_var_reply(&text);
                            let total = proposals.len();
                            self.ai_var_checked =
                                proposals.iter().map(|proposal| proposal.offset).collect();
                            self.ai_var_function = job.target;
                            self.ai_var_proposals = proposals;
                            self.ai_status = if total == 0 {
                                self.t("ui_var_suggest_unparsed").to_owned()
                            } else {
                                self.t_args(
                                    "ui_var_suggest_done",
                                    &[("named", &total.to_string()), ("ms", &elapsed.to_string())],
                                )
                            };
                        }
                    }
                    self.set_status(self.ai_status.clone());
                    return;
                }
                Ok(AiMessage::Done(Err(error))) => {
                    self.ai_reasoning_open = false;
                    let key = match job.kind {
                        AiJobKind::Polish => "ui_polish_failed",
                        AiJobKind::Explain => "ui_explain_failed",
                        AiJobKind::Suggest => "ui_suggest_failed",
                        AiJobKind::Agent => "ui_agent_failed",
                        AiJobKind::VarSuggest => "ui_var_suggest_failed",
                    };
                    self.ai_status = self.t_args(key, &[("error", &error)]);
                    self.set_status(self.ai_status.clone());
                    return;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    if !received
                        && job.kind != AiJobKind::Agent
                        && job.started.elapsed().as_secs() >= 3
                    {
                        self.ai_status = self.t_args(
                            "ui_waiting_for_first_token",
                            &[("seconds", &job.started.elapsed().as_secs().to_string())],
                        );
                    }
                    self.ai_job = Some(job);
                    return;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.ai_status = self.t("ui_ai_worker_stopped").to_owned();
                    return;
                }
            }
        }
    }

    /// Provider settings for one request. The direct-answer path keeps a token ceiling so a
    /// reasoning endpoint does not spend a minute of budget before its first content token;
    /// an explicit thinking request keeps the configured budget.
    fn ai_request_settings(&self) -> recraft_ai::ProviderSettings {
        let mut settings = self.ai_settings.clone();
        if !self.ai_thinking {
            settings.max_tokens =
                direct_answer_token_limit(&settings.endpoint, settings.max_tokens);
        }
        settings
    }

    /// Stream one prompt on a worker thread (HTTP never runs on the UI thread).
    fn spawn_ai_job(
        &mut self,
        ctx: &egui::Context,
        settings: recraft_ai::ProviderSettings,
        prompt: recraft_ai::PromptRequest,
        kind: AiJobKind,
        target: Option<Address>,
        suggestions: Vec<recraft_ai::Proposal>,
    ) {
        let (sender, receiver) = mpsc::channel();
        // Only polished C is cached for the session; explanations and names are cheap to
        // re-request and a rename makes an older answer misleading.
        let job_prompt = (kind == AiJobKind::Polish).then(|| prompt.clone());
        std::thread::spawn(move || {
            let mut provider = recraft_ai::provider_from_settings(&settings);
            let result = provider.complete_stream_with_reasoning(
                &prompt,
                &mut |chunk| {
                    let _ = sender.send(AiMessage::Chunk(chunk.to_owned()));
                },
                &mut |reasoning| {
                    let _ = sender.send(AiMessage::Reasoning(reasoning.to_owned()));
                },
            );
            let _ = sender.send(AiMessage::Done(result.map_err(|error| error.to_string())));
        });
        self.ai_job = Some(AiJob {
            receiver,
            started: Instant::now(),
            kind,
            prompt: job_prompt,
            target,
            suggestions,
        });
        self.ai_status = self.t("ui_calling_provider").to_owned();
        self.set_status(self.ai_status.clone());
        ctx.request_repaint();
    }

    /// Build the explanation prompt for the function under the cursor and stream the model's
    /// answer into the result pane.
    fn start_explain(&mut self, ctx: &egui::Context) {
        if self.ai_job.is_some() {
            return;
        }
        if !self.require_ai_provider() {
            return;
        }
        let Some((start, _)) = self.current_function() else {
            self.ai_status = self.t("ui_put_cursor_inside_function").to_owned();
            return;
        };
        let settings = self.ai_request_settings();
        let Some(session) = self.session.as_mut() else {
            self.set_status(self.t("ui_open_a_project_first").to_owned());
            return;
        };
        let prompt = match recraft_ai::build_explain_prompt(session, start, &settings) {
            Ok(prompt) => prompt,
            Err(error) => {
                self.set_error(error);
                return;
            }
        };
        let prompt = if self.ai_thinking {
            prompt.with_thinking()
        } else {
            prompt
        };
        self.ai_explanation = None;
        // The model answers with prose: the result pane must not colour it as C.
        self.ai_result_kind = AiResultKind::Prose;
        self.ai_reasoning.clear();
        self.ai_reasoning_open = self.ai_thinking;
        self.spawn_ai_job(
            ctx,
            settings,
            prompt,
            AiJobKind::Explain,
            Some(start),
            Vec::new(),
        );
    }

    /// Ask the model to name the evidence-backed `sub_*` functions.
    ///
    /// The local `suggest_names` proposals supply the candidates, their evidence and their
    /// fallback names; the reply only replaces names and rationales, so a partial answer
    /// still leaves applicable proposals on screen.
    fn start_suggest(&mut self, ctx: &egui::Context) {
        if self.ai_job.is_some() {
            return;
        }
        if !self.require_ai_provider() {
            return;
        }
        let settings = self.ai_request_settings();
        let Some(session) = self.session.as_mut() else {
            self.set_status(self.t("ui_open_a_project_first").to_owned());
            return;
        };
        let proposals = match recraft_ai::suggest_names(session, 50) {
            Ok(proposals) => proposals,
            Err(error) => {
                self.set_error(error);
                return;
            }
        };
        if proposals.is_empty() {
            self.ai_status = self
                .t("ui_no_suggestions_yet_run_analyze_then_press_suggest_names")
                .to_owned();
            return;
        }
        let candidates: Vec<Address> = proposals.iter().map(|proposal| proposal.target).collect();
        let prompt = match recraft_ai::build_rename_prompt(session, &candidates, &settings) {
            Ok(prompt) => prompt,
            Err(error) => {
                self.set_error(error);
                return;
            }
        };
        let prompt = if self.ai_thinking {
            prompt.with_thinking()
        } else {
            prompt
        };
        self.ai_checked.clear();
        self.spawn_ai_job(ctx, settings, prompt, AiJobKind::Suggest, None, proposals);
    }

    /// Ask the model to name the stack variables of `func`.
    fn start_var_suggest(&mut self, ctx: &egui::Context, func: Address) {
        if self.ai_job.is_some() {
            return;
        }
        if !self.require_ai_provider() {
            return;
        }
        let settings = self.ai_request_settings();
        let Some(session) = self.session.as_mut() else {
            self.set_status(self.t("ui_open_a_project_first").to_owned());
            return;
        };
        let prompt = match recraft_ai::build_var_prompt(session, func, &settings) {
            Ok(prompt) => prompt,
            Err(error) => {
                self.set_error(error);
                return;
            }
        };
        let prompt = if self.ai_thinking {
            prompt.with_thinking()
        } else {
            prompt
        };
        self.ai_var_proposals.clear();
        self.ai_var_checked.clear();
        self.ai_var_function = Some(func);
        self.spawn_ai_job(
            ctx,
            settings,
            prompt,
            AiJobKind::VarSuggest,
            Some(func),
            Vec::new(),
        );
    }

    /// Apply every ticked variable proposal through the command bus (journaled/undoable).
    fn apply_var_selection(&mut self) -> bool {
        let Some(func) = self.ai_var_function else {
            return false;
        };
        let proposals: Vec<recraft_ai::VarProposal> = self
            .ai_var_proposals
            .iter()
            .filter(|proposal| self.ai_var_checked.contains(&proposal.offset))
            .cloned()
            .collect();
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let mut applied = 0;
        for proposal in &proposals {
            match session.execute(Command::RenameVar {
                func,
                offset: proposal.offset,
                name: Some(proposal.value.clone()),
            }) {
                Ok(_) => applied += 1,
                Err(error) => {
                    self.set_error(error);
                    break;
                }
            }
        }
        if applied > 0 {
            self.pseudo_dirty = true;
            self.ai_status = self.t_args("ui_var_applied", &[("count", &applied.to_string())]);
            self.set_status(self.ai_status.clone());
        }
        applied > 0
    }

    /// The user's current selection as agent context, if there is one.
    ///
    /// An explicit Decompile text selection wins (it is the narrowest thing the user pointed
    /// at); otherwise the selected listing row is used, so "ask about this" always has a
    /// target instead of silently asking a context-free question.
    fn selection_context(&mut self) -> Option<String> {
        let pane = self.focused_pane();
        if (self.tab == Tab::Pseudo || pane == Some(Pane::Pseudo))
            && let (Some(lines), Some(selection)) = (self.pseudo.as_ref(), self.pseudo_sel)
            && !selection.is_empty()
        {
            let text = selection_text(lines, selection)?;
            let (from, to) = selection.ordered();
            let last = to.row.min(lines.len().saturating_sub(1));
            let (Some(first_line), Some(last_line)) = (lines.get(from.row), lines.get(last)) else {
                return None;
            };
            return Some(format!(
                "selected pseudo-C lines {}-{} (0x{:x}..0x{:x}):\n```c\n{text}\n```",
                from.row + 1,
                last + 1,
                first_line.addr.get(),
                last_line.addr.get()
            ));
        }
        if pane == Some(Pane::Hex)
            && let Some(selection) = self.hex_sel
        {
            let (start, end) = selection.range();
            let count = (end.get().saturating_sub(start.get()) + 1).min(256) as usize;
            let bytes = self.hex_selection_bytes(start, count);
            if !bytes.is_empty() {
                return Some(format!(
                    "selected {} bytes at 0x{:x}..0x{:x}:\n```\n{}\n```",
                    bytes.len(),
                    start.get(),
                    end.get(),
                    Self::hex_dump_text(&bytes, false)
                ));
            }
        }
        let line = self.disasm_rows.get(self.selected)?;
        Some(format!(
            "selected instruction at 0x{:x}: `{} {}`",
            line.addr.get(),
            line.mnemonic,
            line.operands
        ))
    }

    /// The docked pane currently under focus, if the dock is up.
    fn focused_pane(&mut self) -> Option<Pane> {
        self.dock
            .as_mut()
            .and_then(|state| state.find_active_focused())
            .map(|(_, tab)| *tab)
    }

    /// Question the "dig deeper" button asks about the function under the cursor.
    ///
    /// The agent already knows how to look things up, so the question only has to name the
    /// starting point; the function's callers/callees are its job to find.
    fn agent_dig_question(&self) -> Option<String> {
        let (start, _) = self.current_function()?;
        let name = self
            .session
            .as_ref()
            .and_then(|session| session.name_at(start).ok().flatten())
            .unwrap_or_else(|| format!("sub_{:x}", start.get()));
        Some(self.t_args(
            "ui_agent_dig_question",
            &[("name", &name), ("addr", &format!("{:#x}", start.get()))],
        ))
    }

    /// Prefill the agent box with the current function and run the loop (history kept).
    fn start_agent_dig(&mut self, ctx: &egui::Context) {
        if self.ai_job.is_some() {
            return;
        }
        let Some(question) = self.agent_dig_question() else {
            self.ai_status = self.t("ui_put_cursor_inside_function").to_owned();
            self.set_status(self.ai_status.clone());
            return;
        };
        self.ai_agent_question = question;
        self.start_agent(ctx);
    }
    /// Run the project-level agent: the model chooses which read-only tools to call.
    ///
    /// The worker opens its own SQLite connection so the UI thread never waits on HTTP or
    /// decompilation; tool calls are streamed back as trace lines, the answer as prose.
    fn start_agent(&mut self, ctx: &egui::Context) {
        if self.ai_job.is_some() {
            return;
        }
        if !self.require_ai_provider() {
            return;
        }
        let mut question = self.ai_agent_question.trim().to_owned();
        if question.is_empty() {
            self.ai_status = self.t("ui_agent_question_empty").to_owned();
            self.set_status(self.ai_status.clone());
            return;
        }
        if self.ai_agent_use_selection
            && let Some(context) = self.selection_context()
        {
            question = format!("{context}\n\nquestion: {question}");
        }
        let Some(path) = self.project.clone() else {
            self.ai_status = self.t("ui_open_a_project_first").to_owned();
            self.set_status(self.ai_status.clone());
            return;
        };
        let settings = self.ai_request_settings();
        let mut history = self.ai_agent_history.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = match recraft_core::Session::open(&path) {
                Ok(mut session) => {
                    let mut provider = recraft_ai::provider_from_settings(&settings);
                    let system =
                        recraft_ai::system_prompt(&settings, recraft_ai::PromptKind::Agent);
                    recraft_ai::run_agent_with_history_system(
                        &mut session,
                        provider.as_mut(),
                        &mut history,
                        &question,
                        &system,
                        recraft_ai::AGENT_MAX_ROUNDS,
                        &mut |event| match event {
                            recraft_ai::AgentEvent::Tool { name, arguments } => {
                                let _ = sender
                                    .send(AiMessage::AgentTool(format!("{name} {arguments}")));
                            }
                            recraft_ai::AgentEvent::TurnStart => {
                                let _ = sender.send(AiMessage::AgentTurnStart);
                            }
                            recraft_ai::AgentEvent::Delta(chunk) => {
                                let _ = sender.send(AiMessage::Chunk(chunk));
                            }
                            recraft_ai::AgentEvent::Retract => {
                                let _ = sender.send(AiMessage::AgentRetract);
                            }
                            recraft_ai::AgentEvent::Answer(_) => {}
                        },
                    )
                    .map_err(|error| error.to_string())
                }
                Err(error) => Err(error.to_string()),
            };
            // A failed turn may still have extended the conversation (tool results arrived
            // before the HTTP error); keep whatever the loop produced.
            let _ = sender.send(AiMessage::AgentHistory(history));
            let _ = sender.send(AiMessage::Done(result));
        });
        self.ai_agent_trace.clear();
        self.ai_explanation = None;
        self.ai_result_kind = AiResultKind::Prose;
        self.ai_result_target = None;
        self.ai_reasoning.clear();
        self.ai_reasoning_open = false;
        self.ai_job = Some(AiJob {
            receiver,
            started: Instant::now(),
            kind: AiJobKind::Agent,
            prompt: None,
            target: None,
            suggestions: Vec::new(),
        });
        self.ai_status = self.t("ui_agent_running").to_owned();
        self.set_status(self.ai_status.clone());
        ctx.request_repaint();
    }

    /// Build the polish prompt for the function under the cursor and start the model
    /// request on a background thread.
    ///
    /// `use_cache` reuses this session's answer for an identical prompt. **Re-polish**
    /// passes `false`: clicking the button again on the same function is an explicit
    /// request for another suggestion, so serving the cached text silently would make
    /// the action look broken. A fresh answer replaces the cached one when it arrives.
    fn start_polish(&mut self, ctx: &egui::Context, use_cache: bool) {
        if self.ai_job.is_some() {
            return;
        }
        self.ai_settings.refresh_enabled();
        if !self.ai_settings.enabled {
            self.ai_status = self.t("ui_enable_provider_for_polish").to_owned();
            return;
        }
        let Some((start, _)) = self.current_function() else {
            self.ai_status = self.t("ui_put_cursor_inside_function").to_owned();
            return;
        };
        let settings = self.ai_request_settings();
        let fingerprint = format!(
            "{}|{}|{}",
            settings.endpoint, settings.model, settings.max_tokens
        );
        if self.ai_cache_fingerprint.as_deref() != Some(fingerprint.as_str()) {
            self.ai_cache = recraft_ai::PromptCache::new();
            self.ai_cache_fingerprint = Some(fingerprint);
        }
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let prompt = match recraft_ai::build_polish_prompt(session, start, &settings) {
            Ok(prompt) => prompt,
            Err(error) => {
                self.set_error(error);
                return;
            }
        };
        // The toggle opts into the model's reasoning pass and streams it into the
        // collapsible thinking section; off keeps the fast direct-answer path.
        let prompt = if self.ai_thinking {
            prompt.with_thinking()
        } else {
            prompt
        };
        self.ai_explanation = None;
        // Polish answers are pseudo-C: colour them like the Decompile view.
        self.ai_result_kind = AiResultKind::Code;
        self.ai_reasoning.clear();
        self.ai_reasoning_open = self.ai_thinking;
        if use_cache && let Some(cached) = self.ai_cache.get(&prompt) {
            let text = cached.to_owned();
            self.ai_explanation = Some(text);
            self.ai_status = self.t_args("ui_polish_cached", &[("ms", "0")]);
            self.set_status(self.ai_status.clone());
            return;
        }
        self.spawn_ai_job(
            ctx,
            settings,
            prompt,
            AiJobKind::Polish,
            Some(start),
            Vec::new(),
        );
    }

    /// Data row for the stored string at `index`, starting at `addr`.
    ///
    /// `addr` is usually the literal's own start; when a literal points inside a longer
    /// run the row shows the tail from the requested address, like IDA's data view, not
    /// the run's own (earlier) start.
    fn string_data_row(&mut self, index: usize, addr: Address) -> Option<DisasmLine> {
        let mut row = self.strings.get(index)?.clone();
        let offset = addr.get().checked_sub(row.addr.get())?;
        let session = self.session.as_mut()?;
        let mut raw = [0u8; 15];
        if offset > 0 {
            let text = if row.enc == recraft_analysis::STRING_ENC_UTF16LE {
                if offset % 2 != 0 {
                    return None;
                }
                let skip = usize::try_from(offset / 2).ok()?;
                row.text.chars().skip(skip).collect()
            } else {
                row.text.get(usize::try_from(offset).ok()?..)?.to_owned()
            };
            row.addr = addr;
            row.len = row.len.saturating_sub(offset as u32);
            row.text = text;
            row.name = session.name_at(row.addr).ok().flatten();
            session.read_at(row.addr, &mut raw).unwrap_or(0);
        } else {
            let copied = row.raw.len().min(15);
            raw[..copied].copy_from_slice(&row.raw[..copied]);
        }
        let comment = data_xref_comment(session, row.addr);
        let escaped = row.text.replace('\\', "\\\\").replace('\'', "\\'");
        let (mnemonic, operands) = if row.enc == recraft_analysis::STRING_ENC_UTF16LE {
            ("dw", format!("'{escaped}',0"))
        } else {
            ("db", format!("'{escaped}',0"))
        };
        Some(DisasmLine {
            addr: row.addr,
            len: row.len.min(254) as u8,
            raw,
            mnemonic: mnemonic.to_owned(),
            operands,
            name: row.name.clone(),
            comment,
            flags: Flags::data(),
        })
    }

    /// Quoted literal cached for `addr`, with the bytes the decompiler's escapes name.
    fn literal_data_bytes(&self, addr: Address) -> Option<(&str, Vec<u8>)> {
        let (_, literal) = self
            .literal_data
            .iter()
            .rev()
            .find(|(known, _)| *known == addr)?;
        let inner = literal.strip_prefix('"')?.strip_suffix('"')?;
        let bytes = c_literal_bytes(inner)?;
        (!bytes.is_empty()).then_some((literal.as_str(), bytes))
    }

    /// Data row for a short literal reached from the decompile view that no stored string
    /// covers. Such literals (`"\r\n"`, `")"`) fall below the extraction minimum but still
    /// name data, so their quoted text is enough to show the row.
    fn literal_data_row(&self, addr: Address) -> Option<DisasmLine> {
        let (literal, bytes) = self.literal_data_bytes(addr)?;
        let session = self.session.as_ref()?;
        let mut raw = [0u8; 15];
        let copied = bytes.len().min(15);
        raw[..copied].copy_from_slice(&bytes[..copied]);
        Some(DisasmLine {
            addr,
            len: (bytes.len() + 1).min(254) as u8,
            raw,
            mnemonic: "db".to_owned(),
            operands: format!("{literal},0"),
            name: session.name_at(addr).ok().flatten(),
            comment: data_xref_comment(session, addr),
            flags: Flags::data(),
        })
    }

    /// Next row boundary after `addr` in the listing's address walk.
    ///
    /// The walk is the one [`Self::data_rows`] renders: one row per stored literal, raw
    /// rows of at most [`DATA_ROW_BYTES`] for the bytes between literals, and instructions
    /// in executable segments. Reaching the end of a segment continues at the next mapped
    /// address, so the listing can scroll across section boundaries.
    fn next_row_start(&mut self, addr: Address) -> Option<Address> {
        let segment = self.session.as_ref()?.segment_containing(addr)?.clone();
        let end = segment.range.end;
        if addr >= end {
            return self.next_segment_start(addr);
        }
        let next = if let Some((index, _)) = string_row_at(&self.strings, addr) {
            let row = self.strings.get(index)?;
            row.addr.get() + u64::from(row.len) + string_terminator_len(row.enc)
        } else if let Some((_, bytes)) = self.literal_data_bytes(addr) {
            addr.get() + bytes.len() as u64 + 1
        } else if segment.perm.is_executable() {
            self.session
                .as_mut()?
                .insn_at(addr)
                .ok()??
                .next_addr()
                .get()
        } else {
            let next_literal = self
                .strings
                .get(self.strings.partition_point(|row| row.addr <= addr))
                .map(|row| row.addr.get())
                .unwrap_or(end.get());
            (addr.get() + DATA_ROW_BYTES).min(next_literal)
        };
        if next >= end.get() {
            return self.next_segment_start(addr);
        }
        (next > addr.get()).then(|| Address::new(next))
    }

    /// First mapped address after `addr` (crossing the gap between two segments).
    fn next_segment_start(&self, addr: Address) -> Option<Address> {
        self.session
            .as_ref()?
            .segments()
            .iter()
            .map(|segment| segment.range.start)
            .filter(|start| *start > addr)
            .min()
    }

    /// Raw `db` row covering the bytes at `addr` up to the next row boundary.
    fn raw_data_row(&mut self, addr: Address) -> Option<DisasmLine> {
        let end = self.session.as_ref()?.segment_containing(addr)?.range.end;
        let next = self.next_row_start(addr)?.min(end);
        let len = next
            .get()
            .saturating_sub(addr.get())
            .min(u64::from(u8::MAX)) as u8;
        let session = self.session.as_mut()?;
        let mut raw = [0u8; 15];
        let read = session.read_at(addr, &mut raw).unwrap_or(0);
        let shown = read.min(usize::from(len));
        let operands = raw[..shown]
            .iter()
            .map(|byte| format!("{byte:#04x}"))
            .collect::<Vec<_>>()
            .join(", ");
        Some(DisasmLine {
            addr,
            len,
            raw,
            mnemonic: "db".to_owned(),
            operands,
            name: session.name_at(addr).ok().flatten(),
            comment: data_xref_comment(session, addr),
            flags: Flags::data(),
        })
    }

    /// Build the listing from `start`: instructions in executable segments, stored literals
    /// as `db '…',0` rows, and the bytes between them as raw `db` rows.
    ///
    /// The window follows the address space instead of listing only one kind of row, so
    /// paging into the padding around a literal shows data rather than decoding the
    /// literal's bytes as instructions (the report "scroll up from a string and the strings
    /// vanish"), the first literal of a section still has its preceding bytes above it, and
    /// the listing stays continuous when it crosses a section boundary.
    fn data_rows(&mut self, start: Address, count: usize) -> Option<Vec<DisasmLine>> {
        let mut rows = Vec::with_capacity(count.min(DISASM_ROWS));
        let mut addr = start;
        while rows.len() < count {
            let Some(segment) = self.session.as_ref()?.segment_containing(addr).cloned() else {
                // Between two segments: continue at the next mapped address, or stop. An
                // unmapped start still shows nothing, as before.
                if rows.is_empty() {
                    break;
                }
                match self.next_segment_start(addr) {
                    Some(next) => addr = next,
                    None => break,
                }
                continue;
            };
            let end = segment.range.end;
            if let Some((index, _)) = string_row_at(&self.strings, addr) {
                let Some(row) = self.string_data_row(index, addr) else {
                    break;
                };
                rows.push(row);
            } else if let Some(row) = self.literal_data_row(addr) {
                rows.push(row);
            } else if segment.perm.is_executable() {
                let remaining = count - rows.len();
                let Ok(mut lines) = self.session.as_mut()?.disasm(addr, remaining) else {
                    break;
                };
                lines.retain(|line| line.addr < end);
                let Some(last) = lines.last() else { break };
                addr = Address::new(last.addr.get() + u64::from(last.len));
                rows.append(&mut lines);
                continue;
            } else {
                let Some(row) = self.raw_data_row(addr) else {
                    break;
                };
                rows.push(row);
            }
            match self.next_row_start(addr) {
                Some(next) => addr = next,
                None => break,
            }
        }
        (!rows.is_empty()).then_some(rows)
    }

    /// Step the listing by `rows` rows of its address walk, returning the new anchor.
    ///
    /// Forward steps follow [`Self::next_row_start`] from the current anchor. Backward
    /// steps walk the row boundaries from the nearest function start (or section start),
    /// so they land on the rows the view renders instead of on instruction boundaries
    /// inside a literal's padding, and they keep going into the previous section.
    fn step_listing(&mut self, rows: i32) -> Option<Address> {
        if rows == 0 {
            return None;
        }
        if rows > 0 {
            let mut addr = self.disasm_start;
            let mut moved = false;
            for _ in 0..rows {
                match self.next_row_start(addr) {
                    Some(next) if next > addr => {
                        addr = next;
                        moved = true;
                    }
                    _ => break,
                }
            }
            return moved.then_some(addr);
        }
        let mut remaining = rows.unsigned_abs() as usize;
        let mut cursor = self.disasm_start;
        let mut result = None;
        // Each pass covers one anchor..cursor span; crossing a section needs the next
        // anchor, hence the (bounded) loop.
        for _ in 0..64 {
            let Some(anchor) = self.backward_anchor(cursor) else {
                break;
            };
            if anchor >= cursor {
                break;
            }
            let mut starts: VecDeque<Address> = VecDeque::with_capacity(remaining.min(64));
            let mut probe = anchor;
            let mut steps = 0usize;
            while probe < cursor && steps < DATA_WALK_LIMIT {
                starts.push_back(probe);
                if starts.len() > remaining {
                    starts.pop_front();
                }
                match self.next_row_start(probe) {
                    Some(next) if next > probe => probe = next,
                    _ => break,
                }
                steps += 1;
            }
            let Some(first) = starts.front().copied() else {
                break;
            };
            remaining = remaining.saturating_sub(starts.len());
            result = Some(first);
            cursor = first;
            if remaining == 0 {
                break;
            }
        }
        result
    }

    /// Where a backward row walk starts: the nearest function start (or the section start)
    /// before `addr`, or — when `addr` already sits on it — the last function of the
    /// preceding section, so scrolling up can cross into earlier code the way IDA does.
    fn backward_anchor(&mut self, addr: Address) -> Option<Address> {
        let segments = self.session.as_ref()?.segments();
        let index = segments
            .iter()
            .position(|segment| addr >= segment.range.start && addr < segment.range.end)?;
        let current = segments.get(index)?;
        if addr > current.range.start {
            if !current.perm.is_executable() {
                return Some(current.range.start);
            }
            let functions = self.session.as_ref()?.functions().ok()?;
            return functions
                .iter()
                .filter(|function| function.start >= current.range.start && function.start < addr)
                .map(|function| function.start)
                .max()
                .or(Some(current.range.start));
        }
        let previous = segments.get(index.checked_sub(1)?)?;
        if !previous.perm.is_executable() {
            return Some(previous.range.start);
        }
        let functions = self.session.as_ref()?.functions().ok()?;
        functions
            .iter()
            .filter(|function| {
                function.start >= previous.range.start && function.start < previous.range.end
            })
            .map(|function| function.start)
            .max()
            .or(Some(previous.range.start))
    }

    fn refresh_disasm(&mut self) {
        if self.strings_dirty {
            self.refresh_strings();
        }
        let start = self.disasm_start;
        // A text selection survives a reload (paging, undo) by address: its row indices are
        // window-relative, so they must be re-found in the new rows.
        let previous = std::mem::take(&mut self.disasm_rows);
        self.disasm_rows = self.data_rows(start, DISASM_ROWS).unwrap_or_default();
        self.disasm_dirty = false;
        if let Some(selection) = self.disasm_sel {
            self.disasm_sel = remap_selection(selection, &previous, &self.disasm_rows);
        }
        if self.selected >= self.disasm_rows.len() {
            self.selected = self.disasm_rows.len().saturating_sub(1);
        }
    }

    fn refresh_pseudo(&mut self) {
        // Line identities only make sense for the listing that produced them.
        self.pseudo_selected_line = None;
        self.pseudo_sel = None;
        self.pseudo_drag = false;
        let Some(session) = self.session.as_mut() else {
            self.pseudo = None;
            self.pseudo_comments.clear();
            self.pseudo_literals.clear();
            self.pseudo_dirty = false;
            return;
        };
        // The Decompile view always shows the structured pseudo-C with the recovered
        // signature and argument names; `pseudocode_l2` itself falls back to the raw
        // listing when the function is not in the analysed set.
        let function = session.functions().ok().and_then(|functions| {
            functions.into_iter().find(|function| {
                self.pseudo_start >= function.start && self.pseudo_start < function.end
            })
        });
        let function_start = function
            .map(|function| function.start)
            .unwrap_or(self.pseudo_start);
        let code = recraft_decompile::pseudocode_l2(session, function_start);
        match code {
            Ok(code) => {
                // Reading the whole function catches comments whose line is no longer
                // emitted (IDA's orphan comments); a fallback listing has no function
                // record, so it reads the span its own rows cover.
                let range = function
                    .map(|function| AddressRange::new(function.start, function.end))
                    .or_else(|| {
                        let first = code.lines.first()?.addr;
                        let last = code.lines.last()?.addr;
                        Some(AddressRange::new(first, Address::new(last.get() + 1)))
                    });
                self.pseudo_comments = range
                    .and_then(|range| session.comments_in(range, CommentKind::Pseudo).ok())
                    .map(|comments| comments.into_iter().collect())
                    .unwrap_or_default();
                let (rows, cols) = Self::with_pseudo_comments(code.lines, &self.pseudo_comments);
                self.pseudo = Some(rows);
                self.pseudo_comment_cols = cols;
                self.pseudo_literals = code.literals;
                self.pseudo_dirty = false;
            }
            Err(error) => {
                self.pseudo = None;
                self.pseudo_comments.clear();
                self.pseudo_comment_cols.clear();
                self.pseudo_literals.clear();
                self.pseudo_dirty = false;
                self.set_status(
                    self.t_args("ui_decompile_error", &[("error", &error.to_string())]),
                );
            }
        }
    }

    /// Append the stored Decompile-view comment to the first row carrying its address, at
    /// the end of that line (`code  // note`) like a disassembly comment. Additional comment
    /// lines follow on their own rows aligned under the marker, so a multi-line note keeps
    /// its shape without moving the annotated line. A comment whose address no longer
    /// appears in the output keeps its rows at the end (IDA's orphan comments).
    ///
    /// Returns the rows plus, per row, the character index where its trailing comment
    /// starts (`None` for uncommented rows): the view uses that boundary to keep names and
    /// quoted strings inside a comment from becoming jump targets.
    fn with_pseudo_comments(
        lines: Vec<PseudoLine>,
        comments: &BTreeMap<Address, String>,
    ) -> (Vec<PseudoLine>, Vec<Option<usize>>) {
        if comments.is_empty() {
            let cols = vec![None; lines.len()];
            return (lines, cols);
        }
        let mut remaining = comments.clone();
        let mut rows = Vec::with_capacity(lines.len() + comments.len());
        let mut cols = Vec::with_capacity(lines.len() + comments.len());
        for mut line in lines {
            let Some(text) = remaining.remove(&line.addr) else {
                cols.push(None);
                rows.push(line);
                continue;
            };
            let mut parts = Self::comment_lines(&text);
            let first = parts.next().unwrap_or_default().trim_end_matches('\r');
            let addr = line.addr;
            let code_chars = line.text.chars().count();
            // Two spaces then the C comment marker, like the disassembly's comment column.
            line.text.push_str("  // ");
            line.text.push_str(first);
            let column = code_chars + 2;
            rows.push(line);
            cols.push(Some(column));
            for extra in parts {
                let mut row = " ".repeat(column);
                row.push_str("// ");
                row.push_str(extra.trim_end_matches('\r'));
                rows.push(PseudoLine { addr, text: row });
                cols.push(Some(column));
            }
        }
        // Orphan comments have no statement to trail: they become comment-only rows.
        for (addr, text) in remaining {
            for extra in Self::comment_lines(&text) {
                let mut row = String::from("// ");
                row.push_str(extra.trim_end_matches('\r'));
                rows.push(PseudoLine { addr, text: row });
                cols.push(Some(0));
            }
        }
        (rows, cols)
    }

    /// Lines of a stored comment, ignoring one trailing newline so a CLI-authored note does
    /// not render a stray empty continuation row.
    fn comment_lines(text: &str) -> std::str::Split<'_, char> {
        text.strip_suffix('\n').unwrap_or(text).split('\n')
    }

    fn refresh_functions(&mut self) {
        let Some(session) = self.session.as_ref() else {
            self.functions.clear();
            self.function_labels.clear();
            self.functions_dirty = false;
            return;
        };
        match session.functions() {
            Ok(functions) => {
                let labels = functions
                    .iter()
                    .map(|function| {
                        session
                            .name_at(function.start)
                            .ok()
                            .flatten()
                            .unwrap_or_else(|| format!("sub_{:x}", function.start.get()))
                    })
                    .collect();
                self.functions = functions;
                self.function_labels = labels;
                self.names = session.names().unwrap_or_default();
                self.functions_dirty = false;
            }
            Err(error) => {
                self.functions.clear();
                self.function_labels.clear();
                self.functions_dirty = false;
                self.set_error(error);
            }
        }
    }

    fn refresh_strings(&mut self) {
        let Some(session) = self.session.as_mut() else {
            self.strings.clear();
            self.strings_dirty = false;
            return;
        };
        match session.strings() {
            Ok(rows) => {
                // User edits live beside the derived table: `text` replaces the decoded
                // bytes, `hidden` rows are already filtered out by `Session::strings`.
                let overrides: std::collections::HashMap<u64, recraft_core::StringOverride> =
                    session
                        .string_overrides()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(addr, entry)| (addr.get(), entry))
                        .collect();
                let mut strings = Vec::with_capacity(rows.len());
                for row in rows {
                    // Read the full stored string (analysis caps it at ~1 KiB of text).
                    let mut bytes = vec![0u8; row.len.min(64 * 1024) as usize];
                    let read = session.read_at(row.addr, &mut bytes).unwrap_or(0);
                    bytes.truncate(read);
                    let edit = overrides.get(&row.addr.get());
                    strings.push(StringRow {
                        addr: row.addr,
                        len: row.len,
                        enc: row.enc,
                        kind: row.kind,
                        segment: session
                            .segment_containing(row.addr)
                            .map(|segment| segment.name.clone())
                            .unwrap_or_default(),
                        text: edit
                            .and_then(|edit| edit.text.clone())
                            .unwrap_or_else(|| decode_string(&bytes, row.enc)),
                        raw: bytes.iter().take(15).copied().collect(),
                        name: session.name_at(row.addr).ok().flatten(),
                        overridden: edit.is_some_and(|edit| edit.text.is_some()),
                    });
                }
                self.strings = strings;
                self.strings_dirty = false;
            }
            Err(error) => {
                self.strings.clear();
                self.strings_dirty = false;
                self.set_error(error);
            }
        }
    }

    fn refresh_imports(&mut self) {
        let Some(session) = self.session.as_mut() else {
            self.imports.clear();
            self.imports_dirty = false;
            return;
        };
        let image = session.image().clone();
        let mut rows = Vec::new();
        for module in &image.imports {
            for function in &module.functions {
                let api = match (&function.name, function.ordinal) {
                    (Some(name), _) if !name.is_empty() => name.clone(),
                    (_, Some(ordinal)) => format!("#{ordinal}"),
                    _ => continue,
                };
                let category = recraft_analysis::categorize(&api);
                let (refs, first_from) = if function.iat_va.get() == 0 {
                    (0, None)
                } else {
                    match session.xrefs_to(function.iat_va) {
                        Ok(xrefs) => (xrefs.len() as u64, xrefs.first().map(|xref| xref.from)),
                        Err(_) => (0, None),
                    }
                };
                rows.push(ImportRow {
                    module: module.name.clone(),
                    api,
                    category,
                    iat: function.iat_va,
                    refs,
                    first_from,
                });
            }
        }
        rows.sort_by(|left, right| {
            (left.module.as_str(), left.api.as_str())
                .cmp(&(right.module.as_str(), right.api.as_str()))
        });
        self.imports = rows;
        self.imports_dirty = false;
    }

    /// Rebuild the open dialog's rows when they are missing or stale.
    fn refresh_xrefs(&mut self) {
        let Some(target) = self.xref_dialog.as_ref().map(|dialog| dialog.target) else {
            return;
        };
        if !self.xref_dialog.as_ref().is_some_and(|dialog| dialog.dirty) {
            return;
        }
        let (rows, failure) = {
            let Some(session) = self.session.as_mut() else {
                self.xref_dialog = None;
                return;
            };
            let mut rows: Vec<XrefRow> = Vec::new();
            let mut failure = None;
            // Incoming references first (`Up`), then the ones leaving the target
            // (`Down`), exactly like IDA's xrefs dialog.
            for (up, entries) in [
                (true, session.xrefs_to(target)),
                (false, session.xrefs_from(target)),
            ] {
                match entries {
                    Ok(entries) => {
                        for xref in entries {
                            rows.push(xref_row(
                                session,
                                &self.functions,
                                &self.function_labels,
                                &self.strings,
                                target,
                                xref,
                                up,
                            ));
                        }
                    }
                    Err(error) => failure = Some(error),
                }
            }
            (rows, failure)
        };
        if let Some(error) = failure {
            self.set_error(error);
        }
        if let Some(dialog) = self.xref_dialog.as_mut() {
            dialog.selected = dialog.selected.min(rows.len().saturating_sub(1));
            dialog.rows = rows;
            dialog.dirty = false;
        }
    }

    fn current_function(&self) -> Option<(Address, Address)> {
        let addr = self.disasm_rows.get(self.selected).map(|line| line.addr)?;
        self.functions
            .iter()
            .find(|function| addr >= function.start && addr < function.end)
            .map(|function| (function.start, function.end))
    }

    fn refresh_cfg(&mut self) {
        let Some((start, end)) = self.current_function() else {
            self.cfg = None;
            self.cfg_for = None;
            self.cfg_dirty = false;
            return;
        };
        if !self.cfg_dirty && self.cfg_for == Some(start) {
            return;
        }
        let Some(session) = self.session.as_mut() else {
            self.cfg = None;
            return;
        };
        match recraft_analysis::build_cfg(session, start, end) {
            Ok(cfg) => {
                self.cfg = Some(cfg);
                self.cfg_for = Some(start);
                self.cfg_dirty = false;
            }
            Err(error) => {
                self.cfg = None;
                self.cfg_for = Some(start);
                self.cfg_dirty = false;
                self.set_error(error);
            }
        }
    }

    /// IDA's rule for graph mode: the graph only exists for instructions that belong to a
    /// function, so anything else (a data row, an address outside every function) keeps the
    /// view in text mode and reports the switch instead of painting an empty canvas.
    ///
    /// Called both when the mode is asked for (`Space`, the toolbar button, the palette) and
    /// on every tick, so jumps, undo and restored layouts cannot leave the pane in graph mode
    /// over an item that has no graph.
    fn enforce_graph_view_mode(&mut self) {
        if self.view_mode != ViewMode::Graph || self.tab != Tab::Disasm {
            return;
        }
        // Without a project the pane shows the welcome page: there is no current item, and
        // the persisted mode is only a preference.
        if self.session.is_none() || self.graph_item_available() {
            return;
        }
        self.view_mode = ViewMode::Text;
        self.set_status(self.t("ui_graph_view_needs_an_instruction").to_owned());
    }

    /// Whether the current item can be drawn as a graph: a code row inside a function.
    fn graph_item_available(&self) -> bool {
        self.disasm_rows
            .get(self.selected)
            .is_some_and(|line| !line.flags.is_data())
            && self.current_function().is_some()
    }

    // ---------------------------------------------------------------- draw --

    /// Focus/Show entries shared by the View and Windows menus.
    fn pane_menu_items(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        for pane in Pane::all() {
            // The overview only exists while a graph is displayed (same rule IDA applies
            // to its Graph overview window), so do not offer to focus it in text mode.
            if pane == Pane::GraphOverview
                && (self.session.is_none() || self.view_mode != ViewMode::Graph)
            {
                continue;
            }
            let button = ui.button(self.t(pane.key()));
            let button = if let Some(shortcut) = pane.shortcut() {
                button.on_hover_text(shortcut)
            } else {
                button
            };
            if button.clicked() {
                *action = Action::FocusPane(pane);
                ui.close();
            }
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let open_label = self.t("ui_open_project");
        let import_label = self.t("ui_import_binary");
        let close_shortcut_label = self.t("ui_close_project_ctrl_w");
        let has_project = self.project.is_some();
        let undo_label = self.t("ui_undo");
        let redo_label = self.t("ui_redo");
        let goto_label = self.t("ui_go_to_address");
        let xrefs_label = self.t("ui_cross_references");
        let back_label = self.t("ui_back");
        let forward_label = self.t("ui_forward");
        let reset_label = self.t("ui_reset_layout");
        let analyze_label = self.t("ui_analyze_functions");
        let cancel_label = self.t("ui_cancel");
        let analyzing_label = self.t("ui_analyzing");
        let file_menu = self.t("menu_file");
        let edit_menu = self.t("menu_edit");
        let jump_menu = self.t("menu_jump");
        let search_menu = self.t("menu_search");
        let view_menu = self.t("menu_view");
        let options_menu = self.t("menu_options");
        let windows_menu = self.t("menu_windows");
        let help_menu = self.t("menu_help");
        let theme_menu = self.t("menu_theme");
        let dark_label = self.t("theme_dark");
        let light_label = self.t("theme_light");
        let about_label = self.t("menu_about");
        let pal = self.palette();
        egui::Panel::top("recraft_top")
            // Chrome sits one surface above the view and breathes a little more than the
            // dense listing (docs/UI.md §4: panel padding 12, bar height ~34).
            .frame(
                egui::Frame::new()
                    .fill(pal.panel)
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .stroke(egui::Stroke::new(1.0, pal.border)),
            )
            .show(ui, |ui| {
                // IDA-style menu bar.
                ui.horizontal(|ui| {
                    ui.menu_button(file_menu.as_str(), |ui| {
                        if ui.button(open_label).clicked() {
                            *action = Action::Open;
                            ui.close();
                        }
                        if ui.button(import_label).clicked() {
                            *action = Action::Import;
                            ui.close();
                        }
                        ui.separator();
                        if ui
                            .add_enabled(
                                has_project,
                                egui::Button::new(close_shortcut_label.as_str()),
                            )
                            .clicked()
                        {
                            *action = Action::CloseProject;
                            ui.close();
                        }
                    });
                    ui.menu_button(edit_menu.as_str(), |ui| {
                        if ui.button(undo_label).clicked() {
                            *action = Action::Undo;
                            ui.close();
                        }
                        if ui.button(redo_label).clicked() {
                            *action = Action::Redo;
                            ui.close();
                        }
                    });
                    ui.menu_button(jump_menu.as_str(), |ui| {
                        if ui.button(goto_label).clicked() {
                            *action = Action::GotoDialog;
                            ui.close();
                        }
                        if ui.button(xrefs_label).clicked() {
                            *action = Action::ShowXrefs;
                            ui.close();
                        }
                        ui.separator();
                        if ui.button(back_label.as_str()).clicked() {
                            *action = Action::Back;
                            ui.close();
                        }
                        if ui.button(forward_label.as_str()).clicked() {
                            *action = Action::Forward;
                            ui.close();
                        }
                    });
                    ui.menu_button(search_menu.as_str(), |ui| {
                        for pane in [Pane::Strings, Pane::Names, Pane::Imports, Pane::Exports] {
                            if ui.button(self.t(pane.key())).clicked() {
                                *action = Action::FocusPane(pane);
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button(view_menu.as_str(), |ui| {
                        self.pane_menu_items(ui, action);
                        ui.separator();
                        if ui.button(self.t("ui_zoom_in_ctrl")).clicked() {
                            *action = Action::ZoomIn;
                            ui.close();
                        }
                        if ui.button(self.t("ui_zoom_out_ctrl")).clicked() {
                            *action = Action::ZoomOut;
                            ui.close();
                        }
                        if ui.button(self.t("ui_reset_zoom_ctrl_0")).clicked() {
                            *action = Action::ZoomReset;
                            ui.close();
                        }
                        ui.separator();
                        if ui.button(reset_label.as_str()).clicked() {
                            *action = Action::ResetLayout;
                            ui.close();
                        }
                    });
                    ui.menu_button(options_menu.as_str(), |ui| {
                        // Like IDA's "Reanalyze program": analysis has no key of its own, it
                        // runs when a binary is imported and can be re-run from here. While a job
                        // runs the same slot reports progress and offers Cancel: the toolbar that
                        // used to hold them is gone.
                        if let Some(job) = self.analysis.as_ref() {
                            let elapsed = job.started.elapsed().as_secs_f64();
                            ui.label(format!("{analyzing_label}… {elapsed:.1}s"));
                            if ui.button(cancel_label.as_str()).clicked() {
                                *action = Action::CancelAnalysis;
                                ui.close();
                            }
                        } else if ui
                            .add_enabled(has_project, egui::Button::new(analyze_label.as_str()))
                            .clicked()
                        {
                            *action = Action::Analyze;
                            ui.close();
                        }
                        ui.separator();
                        ui.menu_button(theme_menu.as_str(), |ui| {
                            if ui
                                .selectable_label(
                                    self.theme == ThemeMode::Dark,
                                    dark_label.as_str(),
                                )
                                .clicked()
                            {
                                *action = Action::SetTheme(ThemeMode::Dark);
                                ui.close();
                            }
                            if ui
                                .selectable_label(
                                    self.theme == ThemeMode::Light,
                                    light_label.as_str(),
                                )
                                .clicked()
                            {
                                *action = Action::SetTheme(ThemeMode::Light);
                                ui.close();
                            }
                        });
                        if ui
                            .selectable_label(self.ai_sidebar, self.t("ui_ai_sidebar"))
                            .clicked()
                        {
                            *action = Action::ToggleAiSidebar;
                            ui.close();
                        }
                        if ui.button(self.t("ui_provider_settings")).clicked() {
                            *action = Action::OpenAiSettings;
                            ui.close();
                        }
                        if ui
                            .button(if self.lang == Lang::En {
                                "中文"
                            } else {
                                "English"
                            })
                            .clicked()
                        {
                            *action = Action::ToggleLang;
                            ui.close();
                        }
                        if ui.button("AI").clicked() {
                            *action = Action::FocusPane(Pane::Ai);
                            ui.close();
                        }
                    });
                    ui.menu_button(windows_menu.as_str(), |ui| {
                        self.pane_menu_items(ui, action);
                        ui.separator();
                        if ui.button(reset_label.as_str()).clicked() {
                            *action = Action::ResetLayout;
                            ui.close();
                        }
                    });
                    ui.menu_button(help_menu.as_str(), |ui| {
                        if ui.button(about_label.as_str()).clicked() {
                            self.about_open = true;
                            ui.close();
                        }
                    });
                });
                ui.separator();
                // Navigation band + legend (IDA's address-space strip).
                self.navigation_band(ui, action);
                ui.horizontal_wrapped(|ui| {
                    for (color, label) in [
                        (NAV_FUNCTION, self.t("ui_regular_function")),
                        (NAV_INSTRUCTION, self.t("ui_instruction")),
                        (NAV_DATA, self.t("ui_data")),
                        (NAV_UNEXPLORED, self.t("ui_unexplored")),
                        (NAV_EXTERNAL, self.t("ui_external_symbol")),
                    ] {
                        ui.colored_label(color, "■");
                        ui.weak(label);
                    }
                });
            });
    }

    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        // One font for the whole bar (IDA's status bar is uniform too). Mixing Body
        // (13 px Segoe UI) and Monospace (12.5 px Consolas) labels in one row leaves
        // their baselines ~3 px apart, because egui centres widget boxes in the row
        // rather than aligning text baselines.
        let pal = self.palette();
        egui::Panel::bottom("recraft_bottom")
            .frame(
                egui::Frame::new()
                    .fill(pal.panel)
                    .inner_margin(egui::Margin::symmetric(12, 4))
                    .stroke(egui::Stroke::new(1.0, pal.border)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.t("status_au_idle")).monospace().weak());
                    ui.separator();
                    let color = if self.is_error {
                        Color32::from_rgb(0xe0, 0x6a, 0x6a)
                    } else {
                        ui.visuals().weak_text_color()
                    };
                    if !self.status.is_empty() {
                        ui.label(RichText::new(self.status.as_str()).monospace().color(color));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(self.t("ui_synchronized_with_hex_view"))
                                .monospace()
                                .weak(),
                        );
                        ui.separator();
                        if let Some((start, _)) = self.current_function() {
                            let name = self
                                .session
                                .as_ref()
                                .and_then(|session| session.name_at(start).ok().flatten())
                                .unwrap_or_else(|| format!("sub_{:x}", start.get()));
                            ui.label(RichText::new(format!("{name}.")).monospace());
                        }
                        if let Some(line) = self.disasm_rows.get(self.selected) {
                            let bytes = line
                                .bytes()
                                .iter()
                                .map(|byte| format!("{byte:02x}"))
                                .collect::<String>();
                            ui.label(
                                RichText::new(format!("{:016x}: {}", line.addr.get(), bytes))
                                    .monospace(),
                            );
                        }
                    });
                });
            });
    }

    /// Bottom hex dump: follows the current selection, click selects a byte, double-click
    /// navigates the disassembly there. Byte/word display toggle.
    fn hex_view(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let pal = self.palette();
        ui.style_mut().text_styles.insert(
            egui::TextStyle::Monospace,
            FontId::monospace(self.mono_size()),
        );
        let byte_label = self.t("ui_bytes");
        let word_label = self.t("ui_words");
        let follow_label = self.t("ui_follow_selection");
        let empty_label = self.t("ui_no_readable_bytes_here");
        let address_label = self.t("ui_address");
        let copy_hex = self.t("ui_copy_hex");
        let copy_ascii = self.t("ui_copy_ascii");
        let mut unit = self.hex_unit;
        let mut hex_sel = self.hex_sel;
        let mut hex_drag = self.hex_drag;
        let mut copy = None;
        ui.horizontal(|ui| {
            ui.strong(self.t("ui_hex"));
            ui.selectable_value(&mut unit, HexUnit::Byte, byte_label);
            ui.selectable_value(&mut unit, HexUnit::Word, word_label);
            ui.separator();
            ui.weak(format!("{:#x}", self.hex_start.get()));
            if let Some(selection) = hex_sel {
                ui.separator();
                ui.weak(self.t_args(
                    "ui_bytes_selected",
                    &[("count", &selection.len().to_string())],
                ));
            }
            if ui.button(follow_label).clicked() {
                self.hex_synced = None;
                self.sync_hex();
            }
        });
        self.hex_unit = unit;
        let start = self.hex_start;
        let cursor = self.hex_cursor;
        let (mut buffer, read) = {
            let Some(session) = self.session.as_mut() else {
                return;
            };
            let mut buffer = vec![0u8; 16 * 16];
            let read = session.read_at(start, &mut buffer).unwrap_or(0);
            (buffer, read)
        };
        buffer.truncate(read);
        if read == 0 {
            ui.weak(empty_label);
            return;
        }
        let rows = buffer.len().div_ceil(16);
        let mut clicked = None;
        let mut navigated = None;
        let mut cells: Vec<(egui::Rect, Address, u64)> = Vec::new();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // IDA-like dense dump: address, one tight byte-cell group, ASCII tail.
                ui.style_mut().spacing.button_padding = egui::vec2(0.0, 0.0);
                egui::Grid::new("hex_grid")
                    .num_columns(3)
                    .striped(false)
                    .min_col_width(0.0)
                    .spacing([8.0, 0.0])
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!("{address_label}  "))
                                .monospace()
                                .strong(),
                        );
                        ui.label(RichText::new(" ").monospace());
                        ui.label(RichText::new("  ASCII").monospace().strong());
                        ui.end_row();
                        for row in 0..rows {
                            let row_addr = Address::new(start.get() + (row as u64) * 16);
                            ui.label(
                                RichText::new(format!("{:016X}  ", row_addr.get()))
                                    .monospace()
                                    .color(pal.addr),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 3.0;
                                match unit {
                                    HexUnit::Byte => {
                                        for column in 0..16 {
                                            let address =
                                                Address::new(row_addr.get() + column as u64);
                                            if let Some(byte) = buffer.get(row * 16 + column) {
                                                let selected = hex_sel.is_some_and(|selection| {
                                                    selection.contains(address)
                                                }) || (hex_sel.is_none()
                                                    && cursor == Some(address));
                                                let response = ui.selectable_label(
                                                    selected,
                                                    RichText::new(format!("{byte:02X}"))
                                                        .monospace(),
                                                );
                                                cells.push((response.rect, address, 1));
                                                if response.clicked() {
                                                    clicked = Some(address);
                                                }
                                                if response.double_clicked() {
                                                    navigated = Some(address);
                                                }
                                                let _ = response.context_menu(|ui| {
                                                    if ui.button(copy_hex.as_str()).clicked() {
                                                        copy = Some(false);
                                                        ui.close();
                                                    }
                                                    if ui.button(copy_ascii.as_str()).clicked() {
                                                        copy = Some(true);
                                                        ui.close();
                                                    }
                                                });
                                            } else {
                                                ui.label(RichText::new("  ").monospace());
                                            }
                                        }
                                    }
                                    HexUnit::Word => {
                                        for word in 0..8 {
                                            let index = row * 16 + word * 2;
                                            let address =
                                                Address::new(row_addr.get() + (word as u64) * 2);
                                            if let (Some(low), Some(high)) =
                                                (buffer.get(index), buffer.get(index + 1))
                                            {
                                                let value = u16::from_le_bytes([*low, *high]);
                                                let selected = hex_sel.is_some_and(|selection| {
                                                    selection.contains(address)
                                                        || selection.contains(Address::new(
                                                            address.get() + 1,
                                                        ))
                                                }) || (hex_sel.is_none()
                                                    && cursor.is_some_and(|cursor| {
                                                        cursor.get() >= address.get()
                                                            && cursor.get() < address.get() + 2
                                                    }));
                                                let response = ui.selectable_label(
                                                    selected,
                                                    RichText::new(format!("{value:04X}"))
                                                        .monospace(),
                                                );
                                                cells.push((response.rect, address, 2));
                                                if response.clicked() {
                                                    clicked = Some(address);
                                                }
                                                if response.double_clicked() {
                                                    navigated = Some(address);
                                                }
                                                let _ = response.context_menu(|ui| {
                                                    if ui.button(copy_hex.as_str()).clicked() {
                                                        copy = Some(false);
                                                        ui.close();
                                                    }
                                                    if ui.button(copy_ascii.as_str()).clicked() {
                                                        copy = Some(true);
                                                        ui.close();
                                                    }
                                                });
                                            } else {
                                                ui.label(RichText::new("    ").monospace());
                                            }
                                        }
                                    }
                                }
                            });
                            let ascii: String = buffer[row * 16..buffer.len().min(row * 16 + 16)]
                                .iter()
                                .map(|byte| {
                                    if (0x20..=0x7e).contains(byte) {
                                        *byte as char
                                    } else {
                                        '.'
                                    }
                                })
                                .collect();
                            ui.label(
                                RichText::new(format!("  {ascii}"))
                                    .monospace()
                                    .color(pal.bytes),
                            );
                            ui.end_row();
                        }
                    });
            });
        if let Some(address) = clicked {
            self.hex_cursor = Some(address);
            // Hex clicks move the disassembly cursor too (IDA-style linkage).
            *action = Action::SyncDisasm(address);
        }
        if let Some(address) = navigated {
            self.hex_synced = None;
            *action = Action::NavigateToDisasm(address);
        }
        // Byte-range selection: a press anchors it on the cell under the pointer, and
        // keeping the button down while moving extends it across cells.
        let (pressed, secondary_pressed, released, down, pointer) = ui.input(|input| {
            (
                input.pointer.primary_pressed(),
                input.pointer.secondary_pressed(),
                input.pointer.primary_released(),
                input.pointer.primary_down(),
                input.pointer.latest_pos(),
            )
        });
        let cell_at = |pos: egui::Pos2| {
            cells
                .iter()
                .find(|(rect, _, _)| rect.contains(pos))
                .map(|(_, address, len)| (*address, *len))
        };
        if (pressed || secondary_pressed)
            && let Some(pos) = pointer
            && let Some((address, len)) = cell_at(pos)
        {
            hex_sel = Some(HexSelection {
                anchor: address,
                focus: Address::new(address.get() + len.saturating_sub(1)),
            });
            hex_drag = pressed;
            self.copy_focus = CopyFocus::Hex;
        } else if hex_drag
            && down
            && let Some(pos) = pointer
        {
            let target = cell_at(pos).or_else(|| {
                let first = cells.first()?;
                let last = cells.last()?;
                if pos.y < first.0.top() {
                    return Some((first.1, first.2));
                }
                if pos.y > last.0.bottom() {
                    return Some((last.1, last.2));
                }
                // Beside a cell or between two of them: take the nearest one.
                cells
                    .iter()
                    .min_by(|a, b| {
                        (a.0.center() - pos)
                            .length_sq()
                            .total_cmp(&(b.0.center() - pos).length_sq())
                    })
                    .map(|(_, address, len)| (*address, *len))
            });
            if let (Some((address, len)), Some(selection)) = (target, hex_sel.as_mut()) {
                selection.focus = Address::new(address.get() + len.saturating_sub(1));
            }
        }
        if released {
            hex_drag = false;
        }
        self.hex_sel = hex_sel;
        self.hex_drag = hex_drag;
        if let Some(ascii) = copy {
            let (start, count) = match self.hex_sel {
                Some(selection) => (selection.range().0, selection.len().min(256) as usize),
                None => (self.hex_cursor.unwrap_or(start), 1),
            };
            let bytes = self.hex_selection_bytes(start, count);
            if bytes.is_empty() {
                self.set_status(self.t("ui_nothing_to_copy").to_owned());
            } else {
                let len = bytes.len();
                ui.ctx().copy_text(Self::hex_dump_text(&bytes, ascii));
                self.set_status(self.t_args("ui_copied_bytes", &[("count", &len.to_string())]));
            }
        }
    }

    /// Build the default dock layout once: Functions + Graph overview on the left,
    /// document tabs (Code View/Hex/Pseudocode/Imports/…) filling the right and an
    /// Output window across the bottom.
    fn ensure_dock(&mut self) {
        if self.dock.is_some() {
            return;
        }
        let mut state = egui_dock::DockState::new(vec![
            Pane::View,
            Pane::Pseudo,
            Pane::Hex,
            Pane::Strings,
            Pane::Structs,
            Pane::Imports,
            Pane::Exports,
        ]);
        {
            let surface = state.main_surface_mut();
            // Output window across the bottom (about 18% like the IDA reference).
            let [top, _output] =
                surface.split_below(egui_dock::NodeIndex::root(), 0.82, vec![Pane::Output]);
            // Functions window with a graph overview underneath it.
            let [left, _docs] = surface.split_left(top, 0.22, vec![Pane::Functions]);
            let [_functions, _overview] =
                surface.split_below(left, GRAPH_OVERVIEW_SPLIT, vec![Pane::GraphOverview]);
        }
        self.dock = Some(state);
    }

    /// IDA only keeps the Graph overview on screen while a graph is displayed: in text
    /// mode the window is not part of the desktop, so the left column gives its space back
    /// to Functions (no tab strip is left behind). Remove/re-add the pane instead of
    /// collapsing it, then put it back under Functions at the size the user last gave it.
    fn sync_graph_overview(&mut self) {
        self.ensure_dock();
        let wanted = self.session.is_some() && self.view_mode == ViewMode::Graph;
        let present = self
            .dock
            .as_ref()
            .is_some_and(|state| state.find_tab(&Pane::GraphOverview).is_some());
        if wanted && !present {
            self.add_graph_overview();
        } else if !wanted && present {
            self.remove_graph_overview();
        }
    }

    /// Re-dock the overview below Functions (the slot IDA's default desktop uses).
    fn add_graph_overview(&mut self) {
        let fraction = self.graph_overview_fraction.clamp(0.5, 0.95);
        let Some(state) = self.dock.as_mut() else {
            return;
        };
        if state.find_tab(&Pane::GraphOverview).is_some() {
            return;
        }
        let Some(functions) = state.find_tab(&Pane::Functions) else {
            return;
        };
        state.split(
            functions.node_path(),
            egui_dock::Split::Below,
            fraction,
            egui_dock::Node::leaf_with(vec![Pane::GraphOverview]),
        );
    }

    /// Take the overview out of the dock, remembering the height the user chose.
    fn remove_graph_overview(&mut self) {
        let Some(state) = self.dock.as_mut() else {
            return;
        };
        let Some(path) = state.find_tab(&Pane::GraphOverview) else {
            return;
        };
        if let Some(parent) = path.node.parent()
            && let Ok(egui_dock::Node::Vertical(split)) =
                state.node(egui_dock::NodePath::new(path.surface, parent))
        {
            self.graph_overview_fraction = split.fraction;
        }
        state.remove_tab(path);
        // The next graph gets its own viewport; a frame from the old one must not leak.
        self.graph_visible = None;
    }

    /// Draw the dock area. The state is temporarily moved out so the tab viewer can borrow
    /// the rest of the app mutably (egui_dock needs `&mut DockState` for tab drag/drop).
    fn dock_area(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let pal = self.palette();
        self.ensure_dock();
        let Some(mut state) = self.dock.take() else {
            return;
        };
        {
            let mut viewer = DockViewer { app: self, action };
            egui::CentralPanel::default().show(ui, |ui| {
                let mut style = egui_dock::Style::from_egui(ui.style().as_ref());
                // IDA-ish chrome: themed tab bars, subtle separators, compact tabs.
                style.tab_bar.bg_fill = pal.panel;
                style.tab_bar.hline_color = pal.grid_line;
                style.tab_bar.height = 22.0;
                style.tab.active.bg_fill = pal.tab_active_bg;
                style.tab.focused.bg_fill = pal.tab_focused_bg;
                style.tab.inactive.bg_fill = pal.panel;
                style.tab.active.text_color = pal.text;
                style.tab.inactive.text_color = pal.tab_inactive_text;
                style.tab.tab_body.bg_fill = pal.view;
                style.separator.width = 2.0;
                style.separator.color_idle = pal.grid_line;
                style.separator.color_hovered = pal.edge_jump;
                style.buttons.close_tab_bg_fill = Color32::TRANSPARENT;
                style.buttons.close_tab_color = pal.tab_inactive_text;
                style.buttons.close_tab_active_color = pal.text;
                egui_dock::DockArea::new(&mut state)
                    .style(style)
                    .draggable_tabs(true)
                    .show_close_buttons(true)
                    .show_add_buttons(false)
                    .show_leaf_close_all_buttons(false)
                    .show_leaf_collapse_buttons(false)
                    .show_inside(ui, &mut viewer);
            });
        }
        self.dock = Some(state);
    }

    /// Focus an existing tab, or dock it into the focused leaf when it was closed.
    fn focus_pane(&mut self, pane: Pane) {
        self.ensure_dock();
        let Some(state) = self.dock.as_mut() else {
            return;
        };
        if state.find_tab(&pane).is_none() {
            state.push_to_focused_leaf(pane);
        }
        // A pane that was closed comes back as its leaf's active tab; marking the leaf
        // focused too makes the reopened view the one keyboard navigation follows.
        if let Some(path) = state.find_tab(&pane) {
            let node_path = path.node_path();
            if state.set_active_tab(path).is_ok() {
                state.set_focused_node_and_surface(node_path);
            }
        }
    }

    /// Bring the welcome page to the front. The welcome page is the Disassembly pane's
    /// no-project state, but it is only visible while that tab is active: an empty document
    /// tab restored from the saved layout (or left in front by Ctrl+W) would hide the only
    /// place the app offers to open a project.
    ///
    /// Called whenever the app can be left without a project (startup without a path, a
    /// failed open, [`Self::close_project`]), not on every tick, so the user can still
    /// switch to another tab afterwards.
    fn show_welcome_tab(&mut self) {
        if self.session.is_some() {
            return;
        }
        self.focus_pane(Pane::View);
    }

    /// The AI assistant is sidebar-only, so remove any stale AI tab loaded from an older
    /// layout.
    fn remove_ai_tab_from_dock(&mut self) {
        self.ensure_dock();
        let Some(state) = self.dock.as_mut() else {
            return;
        };
        if let Some(path) = state.find_tab(&Pane::Ai) {
            let ai_was_active = state
                .find_active_focused()
                .is_some_and(|(_, tab)| *tab == Pane::Ai);
            state.remove_tab(path);
            if ai_was_active && let Some(view) = state.find_tab(&Pane::View) {
                let _ = state.set_active_tab(view);
            }
        }
    }

    /// Persist the dock layout and theme to `%LOCALAPPDATA%\ReCraft\layout.json` when they
    /// change.
    fn persist_layout_if_changed(&mut self) {
        self.frame_count = self.frame_count.wrapping_add(1);
        if self.frame_count % 30 != 0 {
            return;
        }
        self.save_layout_now();
    }

    /// Save immediately (language toggle, explicit layout reset) instead of waiting for
    /// the periodic checkpoint.
    fn save_layout_now(&mut self) {
        if cfg!(test) {
            return;
        }
        let Some(state) = self.dock.as_ref() else {
            return;
        };
        let theme = self.theme;
        let zoom = self.mono_zoom;
        let view_mode = self.view_mode;
        let saved = SavedLayout {
            version: LAYOUT_VERSION,
            dock: state.clone(),
            theme,
            zoom,
            view_mode,
            lang: self.lang,
            ai_sidebar: self.ai_sidebar,
            ai_sidebar_width: self.ai_sidebar_width,
            ai_fab_pos: self.ai_fab_pos.map(|pos| [pos.x, pos.y]),
        };
        let Ok(text) = serde_json::to_string(&saved) else {
            return;
        };
        if self.layout_saved.as_deref() == Some(text.as_str()) {
            return;
        }
        let Some(path) = layout_store_path() else {
            return;
        };
        if save_layout_to(&path, &saved).is_ok() {
            self.layout_saved = Some(text);
        }
    }

    fn restore_layout(&mut self) {
        if let Some(path) = layout_store_path()
            && let Some(saved) = load_layout_from(&path)
        {
            self.theme = saved.theme;
            self.mono_zoom = saved.zoom.clamp(ZOOM_MIN, ZOOM_MAX);
            self.view_mode = saved.view_mode;
            self.lang = saved.lang;
            self.ai_sidebar = saved.ai_sidebar;
            self.ai_sidebar_width = saved.ai_sidebar_width.clamp(300.0, 620.0);
            self.ai_fab_pos = saved.ai_fab_pos.map(|[x, y]| egui::pos2(x, y));
            self.dock = Some(saved.dock);
        }
    }

    fn segments_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let Some(session) = self.session.as_ref() else {
            ui.weak(self.t("ui_no_project_2"));
            return;
        };
        let picked = self
            .list_sel
            .filter(|(pane, _)| *pane == Pane::Segments)
            .map(|(_, key)| key);
        let copy_row = self.t("ui_copy_row");
        let mut picked_now = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for segment in session.segments() {
                    let label = format!(
                        "{:<8} {:#012x} .. {:#012x}  {}",
                        segment.name,
                        segment.range.start.get(),
                        segment.range.end.get(),
                        if segment.perm.is_executable() {
                            "X"
                        } else if segment.perm.is_readable() {
                            "R"
                        } else {
                            "W"
                        }
                    );
                    let key = segment.range.start.get();
                    let response = ui.selectable_label(picked == Some(key), label);
                    if response.clicked() || response.double_clicked() {
                        *action = Action::NavigateToDisasm(segment.range.start);
                    }
                    if response.clicked() || response.secondary_clicked() {
                        picked_now = Some(key);
                    }
                    let _ = response.context_menu(|ui| {
                        if ui.button(copy_row.as_str()).clicked() {
                            *action = Action::Copy;
                            ui.close();
                        }
                    });
                }
            });
        if let Some(key) = picked_now {
            self.list_sel = Some((Pane::Segments, key));
            self.copy_focus = CopyFocus::List(Pane::Segments);
        }
    }

    /// PE exports list (document tab, like IDA's Exports window).
    fn exports_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        if self.session.is_none() {
            ui.weak(self.t("ui_no_project_2"));
            return;
        }
        let exports = self.exports();
        if exports.is_empty() {
            ui.weak(self.t("ui_no_exports"));
            return;
        }
        let picked = self
            .list_sel
            .filter(|(pane, _)| *pane == Pane::Exports)
            .map(|(_, key)| key);
        let copy_row = self.t("ui_copy_row");
        let mut picked_now = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("exports_grid")
                    .num_columns(3)
                    .striped(true)
                    .spacing([12.0, 2.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new(self.t("ui_name")).strong());
                        ui.label(RichText::new(self.t("ui_address")).strong());
                        ui.label(RichText::new(self.t("ui_forwarder")).strong());
                        ui.end_row();
                        for (name, address, forwarder) in &exports {
                            let key = address.get();
                            let response = ui.selectable_label(picked == Some(key), name);
                            ui.label(RichText::new(format!("{:#x}", address.get())).monospace());
                            ui.label(RichText::new(forwarder.as_deref().unwrap_or("")).weak());
                            ui.end_row();
                            if response.clicked() || response.double_clicked() {
                                *action = Action::NavigateToDisasm(*address);
                            }
                            if response.clicked() || response.secondary_clicked() {
                                picked_now = Some(key);
                            }
                            let _ = response.context_menu(|ui| {
                                if ui.button(copy_row.as_str()).clicked() {
                                    *action = Action::Copy;
                                    ui.close();
                                }
                            });
                        }
                    });
            });
        if let Some(key) = picked_now {
            self.list_sel = Some((Pane::Exports, key));
            self.copy_focus = CopyFocus::List(Pane::Exports);
        }
    }

    /// Miniature CFG map under the Functions window (IDA's "Graph overview").
    fn graph_overview(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let pal = self.palette();
        let Some((start, end)) = self.current_function() else {
            ui.weak(self.t("ui_no_function_selected"));
            return;
        };
        if (self.cfg_for != Some(start) || self.cfg_dirty)
            && let Some(session) = self.session.as_mut()
            && let Ok(cfg) = recraft_analysis::build_cfg(session, start, end)
        {
            self.cfg = Some(cfg);
            self.cfg_for = Some(start);
            self.cfg_dirty = false;
        }
        let Some(cfg) = self.cfg.clone() else {
            ui.weak(self.t("ui_no_graph"));
            return;
        };
        if cfg.blocks.is_empty() {
            return;
        }
        let (local, canvas) = graph_layout(&cfg, 1.0);
        let width = ui.available_width().max(80.0);
        let height = ui.available_height().max(80.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 2.0, pal.view);
        // Fit the whole CFG into the pane like IDA: small graphs are scaled up (up to
        // 100 %) instead of staying a stamp in the corner of a mostly empty pane.
        let inner = rect.shrink(4.0);
        let scale = (inner.width() / canvas.x)
            .min(inner.height() / canvas.y)
            .clamp(0.01, 1.0);
        let free = egui::vec2(
            (inner.width() - canvas.x * scale).max(0.0),
            (inner.height() - canvas.y * scale).max(0.0),
        );
        let offset = inner.min + free * 0.5;
        let map = |r: egui::Rect| {
            egui::Rect::from_min_size(
                egui::pos2(offset.x + r.min.x * scale, offset.y + r.min.y * scale),
                egui::vec2((r.width() * scale).max(2.0), (r.height() * scale).max(2.0)),
            )
        };
        for block in &cfg.blocks {
            for &(target, kind) in &block.edges {
                if let Some(target_rect) = local.get(target) {
                    painter.line_segment(
                        [map(local[block.index]).center(), map(*target_rect).center()],
                        egui::Stroke::new(1.0, graph_edge_color(pal, kind)),
                    );
                }
            }
        }
        for (index, block) in cfg.blocks.iter().enumerate() {
            let node = map(local[index]);
            // IDA's overview draws plain boxes: white blocks on the graph colour with a
            // hairline frame (no per-block text and no "current block" tint — the dashed
            // viewport is what tells you where you are).
            painter.rect_filled(node, 1.0, pal.graph_node);
            painter.rect_stroke(
                node,
                1.0,
                egui::Stroke::new(1.0, pal.text),
                egui::StrokeKind::Inside,
            );
            let response = ui.interact(
                node,
                ui.id().with(("overview", block.start.get())),
                Sense::click(),
            );
            if response.clicked() {
                *action = Action::GraphGoTo(block.start);
            }
        }
        // The visible viewport is a dashed frame, exactly like IDA's overview, and only
        // meaningful while the graph is larger than the pane.
        if let Some(visible) = self.graph_visible
            && (visible.width() < canvas.x - 0.5 || visible.height() < canvas.y - 0.5)
        {
            paint_dashed_rect(
                &painter,
                map(visible),
                egui::Stroke::new(1.0, pal.text),
                4.0,
                3.0,
            );
        }
    }

    /// Output window: analysis/status log lines (IDA's Output pane).
    ///
    /// The rows were pinned at 11 px with the default 3 px item spacing, so one line's
    /// descenders sat directly on the next line's ascenders: at 100 % zoom the first rows of
    /// a line have very little ink and were visually swallowed by the line above (the
    /// report's "已加载 那行顶部被挡住了"; the pixels were all there — a flat-top glyph such as
    /// 口 renders its top stroke — the rows just had no air between them). The window uses
    /// the listing's monospace size and a real gap between rows.
    ///
    /// Like IDA's Output window the log is selectable text: a press anchors a caret, a drag
    /// extends it across rows (also past the window edges), Shift+click extends, and Ctrl+A
    /// selects the whole log — not just the clicked line. Ctrl+C copies the selection with
    /// one line per log row. The selection is app-owned (`log_sel`) rather than egui's label
    /// selection because egui's Ctrl+A only reaches the label under the cursor.
    fn output_pane(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let pal = self.palette();
        if self.log.is_empty() {
            ui.weak(self.t("ui_no_messages_yet"));
            return;
        }
        let font = FontId::monospace(self.mono_size());
        let row_gap = (self.row_height() * 0.25).max(4.0);
        let copy_menu = self.t("ui_copy");
        let select_all_menu = self.t("ui_select_all");
        let mut text_sel = self.log_sel;
        let mut dragging = self.log_drag;
        let mut took_focus = false;
        let pointer = ui.input(|input| input.pointer.latest_pos());
        let primary_down = ui.input(|input| input.pointer.primary_down());
        let extend_selection = ui.input(|input| input.modifiers.shift);
        let mut first_top: Option<f32> = None;
        let mut last_bottom: Option<f32> = None;
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(3.0);
                ui.spacing_mut().item_spacing.y = row_gap;
                let width = ui.available_width().max(200.0);
                for (index, line) in self.log.iter().enumerate() {
                    let color = if line.starts_with("error:") {
                        Color32::from_rgb(0xe0, 0x6a, 0x6a)
                    } else {
                        pal.text
                    };
                    let galley = ui
                        .painter()
                        .layout(line.clone(), font.clone(), color, width);
                    // The hit area is the whole row width (plus the gap) so a drag that
                    // starts or ends in the empty space right of a line still lands on it.
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(width, galley.size().y + row_gap),
                        Sense::click_and_drag(),
                    );
                    first_top.get_or_insert(rect.top());
                    last_bottom = Some(rect.bottom());
                    let text_origin = rect.left_top();
                    if let Some(selection) = text_sel
                        && let Some(range) =
                            selection.row_range(index, galley.text().chars().count())
                    {
                        paint_wrapped_text_selection(
                            ui.painter(),
                            pal.text_selection,
                            &galley,
                            text_origin,
                            range,
                        );
                    }
                    ui.painter().galley(text_origin, galley.clone(), color);
                    // Text selection: a press anchors it, a drag extends it across rows, and
                    // a plain click collapses it to a caret where it landed (Shift extends).
                    if response.drag_started()
                        && let Some(pos) = ui.input(|input| input.pointer.press_origin())
                    {
                        let point = TextPoint {
                            row: index,
                            char: galley_char_at(&galley, text_origin, pos),
                        };
                        text_sel = Some(TextSelection {
                            anchor: point,
                            focus: point,
                        });
                        dragging = true;
                        took_focus = true;
                    }
                    if dragging
                        && primary_down
                        && let Some(pos) = pointer
                        && pos.y >= rect.top()
                        && pos.y <= rect.bottom()
                    {
                        let point = TextPoint {
                            row: index,
                            char: galley_char_at(&galley, text_origin, pos),
                        };
                        if let Some(selection) = text_sel.as_mut() {
                            selection.focus = point;
                        }
                    }
                    if response.clicked()
                        && !response.double_clicked()
                        && let Some(pos) = response.interact_pointer_pos()
                    {
                        let point = TextPoint {
                            row: index,
                            char: galley_char_at(&galley, text_origin, pos),
                        };
                        text_sel = Some(match text_sel {
                            Some(selection) if extend_selection => TextSelection {
                                focus: point,
                                ..selection
                            },
                            _ => TextSelection::collapsed(point),
                        });
                    }
                    // IDA's double click selects the word under the cursor (a run of
                    // identifier characters, or the punctuation run it sits in).
                    if response.double_clicked()
                        && let Some(pos) = response.interact_pointer_pos()
                    {
                        let hit = galley_char_at(&galley, text_origin, pos);
                        let (from, to) = word_range_at(galley.text(), hit);
                        text_sel = Some(TextSelection {
                            anchor: TextPoint {
                                row: index,
                                char: from,
                            },
                            focus: TextPoint {
                                row: index,
                                char: to,
                            },
                        });
                    }
                    if response.clicked()
                        || response.secondary_clicked()
                        || response.double_clicked()
                    {
                        took_focus = true;
                    }
                    response.context_menu(|ui| {
                        if ui.button(copy_menu.as_str()).clicked() {
                            *action = Action::Copy;
                            ui.close();
                        }
                        if ui.button(select_all_menu.as_str()).clicked() {
                            *action = Action::SelectAll;
                            ui.close();
                        }
                    });
                }
            });
        // Dragging above or below the window keeps selecting to that edge, like the listing.
        if dragging
            && primary_down
            && let Some(pos) = pointer
        {
            if first_top.is_some_and(|top| pos.y < top) {
                if let Some(selection) = text_sel.as_mut() {
                    selection.focus = TextPoint { row: 0, char: 0 };
                }
            } else if last_bottom.is_some_and(|bottom| pos.y > bottom)
                && let Some(last) = self.log.len().checked_sub(1)
            {
                let end = self.log[last].chars().count();
                if let Some(selection) = text_sel.as_mut() {
                    selection.focus = TextPoint {
                        row: last,
                        char: end,
                    };
                }
            }
        }
        if !primary_down {
            dragging = false;
        }
        self.log_sel = text_sel;
        self.log_drag = dragging;
        if took_focus {
            self.copy_focus = CopyFocus::Output;
        }
    }

    fn functions_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let pal = self.palette();
        if self.functions.is_empty() {
            ui.weak(self.t("ui_no_functions_yet_press_analyze_f5"));
        } else {
            let hint = self.t("ui_name_or_address");
            ui.horizontal(|ui| {
                ui.weak(self.t("ui_filter"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.function_filter)
                        .desired_width(f32::INFINITY)
                        .hint_text(hint),
                );
            });
        }
        let filter = self.function_filter.to_lowercase();
        // Segment column, precomputed once per list refresh (IDA's Functions window has
        // "Function name" and "Segment" columns).
        let segment_names: Vec<String> = {
            let Some(session) = self.session.as_ref() else {
                return;
            };
            self.functions
                .iter()
                .map(|function| {
                    session
                        .segment_containing(function.start)
                        .map(|segment| segment.name.clone())
                        .unwrap_or_default()
                })
                .collect()
        };
        let mut shown = 0usize;
        let picked = self
            .list_sel
            .filter(|(pane, _)| *pane == Pane::Functions)
            .map(|(_, key)| key);
        let copy_row = self.t("ui_copy_row");
        let mut picked_now = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("functions_grid")
                    .num_columns(3)
                    .striped(true)
                    .spacing([10.0, 2.0])
                    .show(ui, |ui| {
                        ui.label("");
                        ui.label(RichText::new(self.t("ui_function_name")).strong());
                        ui.label(RichText::new(self.t("ui_segment")).strong());
                        ui.end_row();
                        for (index, function) in self.functions.iter().enumerate() {
                            let name = self
                                .function_labels
                                .get(index)
                                .map(String::as_str)
                                .unwrap_or("");
                            if !function_matches(&filter, function.start, name) {
                                continue;
                            }
                            shown += 1;
                            ui.label(RichText::new("ƒ").color(pal.name));
                            let key = function.start.get();
                            let response = ui
                                .selectable_label(picked == Some(key), name)
                                .on_hover_text(format!(
                                    "{:#012x} .. {:#012x}",
                                    function.start.get(),
                                    function.end.get()
                                ));
                            ui.label(
                                RichText::new(
                                    segment_names.get(index).cloned().unwrap_or_default(),
                                )
                                .monospace()
                                .weak(),
                            );
                            ui.end_row();
                            if response.clicked() {
                                *action = Action::Navigate(function.start);
                            }
                            if response.double_clicked() {
                                *action = Action::NavigateToDisasm(function.start);
                            }
                            if response.clicked() || response.secondary_clicked() {
                                picked_now = Some(key);
                            }
                            let _ = response.context_menu(|ui| {
                                if ui.button(copy_row.as_str()).clicked() {
                                    *action = Action::Copy;
                                    ui.close();
                                }
                            });
                        }
                    });
                if shown == 0 && !self.functions.is_empty() {
                    ui.weak(self.t("ui_no_match"));
                }
            });
        if let Some(key) = picked_now {
            self.list_sel = Some((Pane::Functions, key));
            self.copy_focus = CopyFocus::List(Pane::Functions);
        }
    }

    fn strings_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let pal = self.palette();
        if self.strings.is_empty() {
            ui.weak(self.t("ui_no_strings_yet_run_analyze_f5"));
        }
        let hint = self.t("ui_substring");
        let hide_label = self.t("ui_hide_header_strings");
        let setup_label = self.t("ui_setup");
        // Editing used to live only in the row context menu; a visible button makes it
        // discoverable for the selected row (the menu is still there).
        let edit_label = self.t("ui_edit_string");
        let selected_row = self
            .list_sel
            .filter(|(pane, _)| *pane == Pane::Strings)
            .map(|(_, key)| key);
        let mut edit_selected = None;
        ui.horizontal(|ui| {
            ui.weak(self.t("ui_filter"));
            ui.add(
                egui::TextEdit::singleline(&mut self.string_filter)
                    .desired_width(260.0)
                    .hint_text(hint),
            );
            ui.checkbox(&mut self.strings_hide_header, hide_label);
            // IDA's *Setup…*: the minimum length is a project setting applied on rebuild.
            if ui.button(setup_label).clicked() {
                self.strings_setup = Some(self.strings_min_len);
            }
            if ui
                .add_enabled(
                    selected_row.is_some(),
                    egui::Button::new(edit_label.as_str()),
                )
                .on_hover_text(self.t("ui_edit_string_hint"))
                .clicked()
            {
                edit_selected = selected_row;
            }
            ui.weak(format!("{}", self.strings.len()));
        });
        if let Some(key) = edit_selected
            && let Some(row) = self.strings.iter().find(|row| row.addr.get() == key)
        {
            self.edit = Some(EditState {
                kind: EditKind::StringText,
                addr: row.addr,
                text: row.text.clone(),
                just_opened: true,
            });
        }
        let filter = self.string_filter.to_lowercase();
        let hide_header = self.strings_hide_header;
        // Rows the filter and the header switch keep, in list order. Built once so the
        // column width below and the painted rows can never disagree.
        let mut visible: Vec<usize> = Vec::new();
        let mut more = false;
        for (index, row) in self.strings.iter().enumerate() {
            if hide_header && is_header_string(&row.segment) {
                continue;
            }
            if !filter.is_empty() && !string_cell_text(&row.text).to_lowercase().contains(&filter) {
                continue;
            }
            if visible.len() == STRING_LIST_ROWS {
                more = true;
                break;
            }
            visible.push(index);
        }
        let mono_size = self.mono_size();
        let row_h = self.row_height().max(16.0);
        let scale = mono_size / MONO_SIZE;
        let addr_w = 280.0 * scale;
        let len_w = 90.0 * scale;
        let type_w = 70.0 * scale;
        let string_x = addr_w + len_w + type_w;
        let selected = self
            .disasm_rows
            .get(self.selected)
            .map(|line| line.addr)
            .unwrap_or(self.disasm_start);
        let mono = FontId::monospace(mono_size);
        let widest = visible
            .iter()
            .map(|&index| string_cell_text(&self.strings[index].text))
            .max_by_key(|text| string_cell_units(text));
        // Lay the widest literal out once (egui caches the galley) so the column matches the
        // font exactly, CJK fallback included.
        let widest_w = widest
            .map(|text| {
                ui.painter()
                    .layout_no_wrap(text.into_owned(), mono.clone(), pal.text)
                    .size()
                    .x
            })
            .unwrap_or(0.0);
        // IDA's Strings window sizes its columns to their content and scrolls sideways when
        // the widest literal does not fit, so a long string is reachable instead of being
        // cut at the pane edge.
        let content_width = (string_x + widest_w + 12.0).max(ui.available_width());
        // Header row, aligned with the row offsets below and painted after the list so it
        // can follow this frame's horizontal offset.
        let (header, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), Sense::hover());
        let picked = self
            .list_sel
            .filter(|(pane, _)| *pane == Pane::Strings)
            .map(|(_, key)| key);
        let copy_row = self.t("ui_copy_row");
        let edit_row_label = self.t("ui_edit_string");
        let hide_row_label = self.t("ui_delete_string");
        let restore_row_label = self.t("ui_restore_string");
        let mut picked_now = None;
        let mut edit_row = None;
        let mut hide_row = None;
        let mut restore_row = None;
        let scroll = egui::ScrollArea::both()
            .id_salt("strings_rows")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (shown, &index) in visible.iter().enumerate() {
                    let row = &self.strings[index];
                    let key = row.addr.get();
                    let is_selected = row.addr == selected || picked == Some(key);
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(content_width, row_h), Sense::click());
                    if is_selected {
                        ui.painter().rect_filled(rect, 1.0, pal.selected_row);
                    } else if response.hovered() || shown % 2 == 1 {
                        ui.painter().rect_filled(rect, 1.0, pal.hover_row);
                    }
                    let painter = ui.painter_at(rect);
                    let address = if row.segment.is_empty() {
                        format!("{:016x}", row.addr.get())
                    } else {
                        format!("{}:{:016x}", row.segment, row.addr.get())
                    };
                    painter.text(
                        egui::pos2(rect.left() + 4.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &address,
                        mono.clone(),
                        pal.addr,
                    );
                    painter.text(
                        egui::pos2(rect.left() + addr_w, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        format!("{:08x}", row.len),
                        mono.clone(),
                        pal.bytes,
                    );
                    painter.text(
                        egui::pos2(rect.left() + addr_w + len_w, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        if row.enc == recraft_analysis::STRING_ENC_UTF16LE {
                            "C16"
                        } else {
                            "C"
                        },
                        mono.clone(),
                        pal.mnemonic,
                    );
                    painter.text(
                        egui::pos2(rect.left() + string_x, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        string_cell_text(&row.text),
                        mono.clone(),
                        pal.text,
                    );
                    let response = response.on_hover_text(format!(
                        "{} · {} bytes · enc {} · kind {}",
                        string_cell_text(&row.text),
                        row.len,
                        row.enc,
                        row.kind
                    ));
                    if response.clicked() {
                        *action = Action::SyncDisasm(row.addr);
                    }
                    if response.double_clicked() {
                        *action = Action::NavigateToDisasm(row.addr);
                    }
                    if response.clicked() || response.secondary_clicked() {
                        picked_now = Some(key);
                    }
                    let _ = response.context_menu(|ui| {
                        if ui.button(copy_row.as_str()).clicked() {
                            *action = Action::Copy;
                            ui.close();
                        }
                        if ui.button(edit_row_label.as_str()).clicked() {
                            edit_row = Some((row.addr, row.text.clone()));
                            ui.close();
                        }
                        if ui.button(hide_row_label.as_str()).clicked() {
                            hide_row = Some(row.addr);
                            ui.close();
                        }
                        if row.overridden && ui.button(restore_row_label.as_str()).clicked() {
                            restore_row = Some(row.addr);
                            ui.close();
                        }
                    });
                }
                if more {
                    ui.weak(self.t("ui_first_2000_matches"));
                }
            });
        if let Some((addr, text)) = edit_row {
            self.edit = Some(EditState {
                kind: EditKind::StringText,
                addr,
                text,
                just_opened: true,
            });
        }
        if let Some(addr) = hide_row {
            self.apply_string_edit(addr, None, true);
        }
        if let Some(addr) = restore_row {
            self.apply_string_edit(addr, None, false);
        }
        let painter = ui.painter_at(header);
        let left = header.left() - scroll.state.offset.x;
        for (x, text) in [
            (4.0, self.t("ui_address")),
            (addr_w, self.t("ui_length")),
            (addr_w + len_w, self.t("ui_type")),
            (string_x, self.t("ui_string")),
        ] {
            painter.text(
                egui::pos2(left + x, header.center().y),
                egui::Align2::LEFT_CENTER,
                text,
                mono.clone(),
                pal.text,
            );
        }
        if let Some(key) = picked_now {
            self.list_sel = Some((Pane::Strings, key));
            self.copy_focus = CopyFocus::List(Pane::Strings);
        }
    }

    fn imports_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        if self.imports.is_empty() {
            ui.weak(self.t("ui_no_imports_or_open_a_project_first"));
        }
        let picked = self
            .list_sel
            .filter(|(pane, _)| *pane == Pane::Imports)
            .map(|(_, key)| key);
        let copy_row = self.t("ui_copy_row");
        let mut picked_now = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for import in &self.imports {
                    let label = format!(
                        "{}!{}  [{}]  ×{}",
                        import.module, import.api, import.category, import.refs
                    );
                    let key = import.iat.get();
                    let response = ui.selectable_label(picked == Some(key), label);
                    let target = import.first_from.unwrap_or(import.iat);
                    if response.clicked() {
                        *action = Action::Navigate(target);
                    }
                    if response.double_clicked() {
                        *action = Action::NavigateToDisasm(target);
                    }
                    if response.clicked() || response.secondary_clicked() {
                        picked_now = Some(key);
                    }
                    let _ = response.context_menu(|ui| {
                        if ui.button(copy_row.as_str()).clicked() {
                            *action = Action::Copy;
                            ui.close();
                        }
                    });
                }
            });
        if let Some(key) = picked_now {
            self.list_sel = Some((Pane::Imports, key));
            self.copy_focus = CopyFocus::List(Pane::Imports);
        }
    }

    fn names_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        if self.names.is_empty() {
            ui.weak(self.t("ui_no_names_yet_open_a_project_and_run_analyze"));
        }
        let hint = self.t("ui_name_or_address");
        ui.horizontal(|ui| {
            ui.weak(self.t("ui_filter"));
            ui.add(
                egui::TextEdit::singleline(&mut self.names_filter)
                    .desired_width(f32::INFINITY)
                    .hint_text(hint),
            );
        });
        let filter = self.names_filter.to_lowercase();
        let picked = self
            .list_sel
            .filter(|(pane, _)| *pane == Pane::Names)
            .map(|(_, key)| key);
        let copy_row = self.t("ui_copy_row");
        let mut picked_now = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (addr, name) in &self.names {
                    if !filter.is_empty()
                        && !name.to_lowercase().contains(&filter)
                        && !format!("{:#x}", addr.get()).contains(&filter)
                    {
                        continue;
                    }
                    let label = format!("{:#012x}  {name}", addr.get());
                    let response = ui.selectable_label(picked == Some(addr.get()), label);
                    if response.clicked() || response.double_clicked() {
                        *action = Action::NavigateToDisasm(*addr);
                    }
                    if response.clicked() || response.secondary_clicked() {
                        picked_now = Some(addr.get());
                    }
                    let _ = response.context_menu(|ui| {
                        if ui.button(copy_row.as_str()).clicked() {
                            *action = Action::Copy;
                            ui.close();
                        }
                    });
                }
            });
        if let Some(key) = picked_now {
            self.list_sel = Some((Pane::Names, key));
            self.copy_focus = CopyFocus::List(Pane::Names);
        }
    }
    /// Structures window: list user layouts, define one, bind it to a function argument.
    fn structs_panel(&mut self, ui: &mut egui::Ui, _action: &mut Action) {
        let structs: Vec<StructDef> = {
            let Some(session) = self.session.as_ref() else {
                ui.weak(self.t("ui_no_project_2"));
                return;
            };
            session.structs().unwrap_or_default()
        };
        if structs.is_empty() {
            ui.weak(self.t("ui_no_structures_yet_define_one_below"));
        }
        egui::ScrollArea::vertical()
            .id_salt("structs_list")
            .max_height(140.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for def in &structs {
                    let fields = def
                        .fields
                        .iter()
                        .map(|field| format!("{:#x}:{}:{}", field.offset, field.name, field.kind))
                        .collect::<Vec<_>>()
                        .join(", ");
                    ui.label(format!("{}  ({} bytes)  {}", def.name, def.size, fields));
                }
            });
        ui.separator();
        ui.strong(self.t("ui_define_structure"));
        egui::Grid::new("struct_form")
            .num_columns(2)
            .spacing([8.0, 2.0])
            .show(ui, |ui| {
                ui.label(self.t("ui_name_2"));
                ui.add(egui::TextEdit::singleline(&mut self.struct_name).desired_width(260.0));
                ui.end_row();
                ui.label(self.t("ui_size"));
                ui.add(egui::DragValue::new(&mut self.struct_size).range(1..=65536));
                ui.end_row();
                ui.label(self.t("ui_fields"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.struct_fields)
                        .desired_width(260.0)
                        .hint_text("0:tag:u32, 16:value:i32"),
                );
                ui.end_row();
            });
        if ui.button(self.t("ui_define_update")).clicked() {
            self.define_struct_from_form();
        }
        ui.separator();
        ui.strong(self.t("ui_apply_to_selected_function_argument"));
        ui.horizontal(|ui| {
            ui.label(self.t("ui_slot"));
            ui.add(egui::DragValue::new(&mut self.struct_slot).range(0..=7));
            ui.label(self.t("ui_type_2"));
            ui.add(egui::TextEdit::singleline(&mut self.struct_name).desired_width(140.0));
            if ui.button(self.t("ui_apply")).clicked() {
                self.apply_struct_to_slot();
            }
        });
        ui.weak(self.t("ui_both_operations_are_journaled_ctrl_z_undoes_them"));
    }

    fn define_struct_from_form(&mut self) {
        let name = self.struct_name.trim().to_owned();
        if name.is_empty() {
            self.set_status(self.t("ui_structure_name_is_required").to_owned());
            return;
        }
        let mut fields = Vec::new();
        for spec in self.struct_fields.split(',') {
            let spec = spec.trim();
            if spec.is_empty() {
                continue;
            }
            match parse_struct_field_spec(spec) {
                Ok(field) => fields.push(field),
                Err(error) => {
                    self.set_error(recraft_common::Error::unsupported(error));
                    return;
                }
            }
        }
        let def = StructDef {
            name: name.clone(),
            size: self.struct_size,
            fields,
        };
        let Some(session) = self.session.as_mut() else {
            return;
        };
        match session.execute(Command::DefineStruct { def }) {
            Ok(_) => {
                self.disasm_dirty = true;
                self.pseudo_dirty = true;
                self.functions_dirty = true;
                self.set_status(self.t_args("ui_structure_defined", &[("name", &name)]));
            }
            Err(error) => self.set_error(error),
        }
    }

    fn apply_struct_to_slot(&mut self) {
        let Some((start, _)) = self.current_function() else {
            self.set_status(
                self.t("ui_put_the_cursor_inside_a_function_first")
                    .to_owned(),
            );
            return;
        };
        let name = self.struct_name.trim().to_owned();
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let struct_name = if name.is_empty() { None } else { Some(name) };
        match session.execute(Command::ApplyType {
            func: start,
            slot: self.struct_slot,
            struct_name,
        }) {
            Ok(_) => {
                self.disasm_dirty = true;
                self.pseudo_dirty = true;
                self.set_status(self.t_args(
                    "ui_argument_bound",
                    &[
                        ("slot", &self.struct_slot.to_string()),
                        ("addr", &format!("{:#x}", start.get())),
                    ],
                ));
            }
            Err(error) => self.set_error(error),
        }
    }

    fn ai_panel(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        // The panel can be shorter than its content (provider settings + result), so keep
        // the whole body scrollable instead of clipping the status/result below the fold.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| self.ai_panel_inner(ui, action));
    }

    fn ai_panel_inner(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let pal = self.palette();
        let suggest = self.t("ui_suggest_names");
        let apply_label = self.t("ui_apply_selected");
        let explain_label = self.t("ui_explain");
        let dig_label = self.t("ui_agent_dig");
        let dig_hint = self.t("ui_agent_dig_hint");
        let polish_label = self.t("ui_polish");
        let repolish_label = self.t("ui_repolish");
        let repolish_hint = self.t("ui_repolish_hint");
        let clear = self.t("ui_clear");
        let thinking_mode = self.t("ui_thinking_mode");
        let thinking_hint = self.t("ui_thinking_hint");
        let has_selection = !self.ai_checked.is_empty();
        let apply_text = if has_selection {
            format!("{apply_label} ({})", self.ai_checked.len())
        } else {
            apply_label.clone()
        };

        ui.horizontal(|ui| {
            ui.label(RichText::new("AI").size(19.0).strong().color(pal.mnemonic));
            ui.label(
                RichText::new(self.t("ai_assistant"))
                    .size(12.0)
                    .color(pal.muted),
            );
            if let Some(job) = self.ai_job.as_ref() {
                ui.spinner();
                ui.weak(format!(
                    "{} {:.1}s",
                    self.t("ai_working"),
                    job.started.elapsed().as_secs_f64()
                ));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new("⚙").size(15.0))
                    .on_hover_text(self.t("ui_provider_settings"))
                    .clicked()
                {
                    *action = Action::OpenAiSettings;
                }
                if ui
                    .button(RichText::new("»").size(17.0).strong())
                    .on_hover_text(self.t("ui_collapse"))
                    .clicked()
                {
                    *action = Action::ToggleAiSidebar;
                }
            });
        });
        ui.add_space(6.0);

        // Status is shown as a compact banner so slow requests never look like a dead
        // button and errors stay readable above the controls.
        if !self.ai_status.is_empty() {
            card_frame(pal).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    let dot = if self.ai_job.is_some() {
                        pal.mnemonic
                    } else if self.is_error {
                        Color32::from_rgb(0xc0, 0x50, 0x50)
                    } else {
                        pal.comment
                    };
                    ui.label(RichText::new("●").color(dot));
                    ui.label(RichText::new(&self.ai_status).color(pal.text));
                });
            });
            ui.add_space(8.0);
        }

        card_frame(pal).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new(self.t("ai_actions"))
                    .strong()
                    .size(13.0)
                    .color(pal.text),
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button(suggest).clicked() {
                    *action = Action::AiSuggest;
                }
                if ui.button(explain_label).clicked() {
                    *action = Action::AiExplain;
                }
                // Explain stays the fast one-request path; this hands the same function to
                // the agent when the reader wants callers/behaviour, with history kept.
                if ui
                    .add_enabled(
                        self.ai_job.is_none() && self.current_function().is_some(),
                        egui::Button::new(RichText::new(dig_label).size(11.0)),
                    )
                    .on_hover_text(dig_hint)
                    .clicked()
                {
                    *action = Action::AiAgentDig;
                }
                if ui
                    .add(
                        egui::Button::new(RichText::new(polish_label).strong().color(pal.text))
                            .fill(pal.selected_row),
                    )
                    .clicked()
                {
                    *action = Action::AiPolish;
                }
                // A polish answer can be cached, so pressing Polish again may return the
                // same text. Offer an explicit re-run that skips the cache once a polish
                // result is on screen (an explanation is prose and has nothing to redo).
                if self.ai_result_kind == AiResultKind::Code
                    && self.ai_explanation.is_some()
                    && ui
                        .add_enabled(
                            self.ai_job.is_none(),
                            egui::Button::new(repolish_label.as_str()),
                        )
                        .on_hover_text(repolish_hint.as_str())
                        .clicked()
                {
                    *action = Action::AiRepolish;
                }
                if ui
                    .add_enabled(has_selection, egui::Button::new(apply_text.as_str()))
                    .clicked()
                {
                    *action = Action::AiApply;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(clear).clicked() {
                        *action = Action::AiClear;
                    }
                });
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.ai_thinking, thinking_mode.as_str())
                    .on_hover_text(thinking_hint.as_str());
            });
        });

        // Project-level agent: unlike the one-shot buttons above, the model decides which
        // read-only tools to call (list functions, read a function, follow xrefs, ...).
        card_frame(pal).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new(self.t("ai_agent_title"))
                    .strong()
                    .size(13.0)
                    .color(pal.text),
            );
            ui.add_space(4.0);
            let question_hint = self.t("ui_agent_question_hint");
            let run_label = self.t("ui_run_agent");
            let agent_hint = self.t("ui_agent_hint");
            let new_label = self.t("ui_agent_new_chat");
            let new_hint = self.t("ui_agent_new_chat_hint");
            let has_history = !self.ai_agent_history.is_empty();
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 140.0).max(80.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.ai_agent_question)
                        .desired_width(width)
                        .hint_text(question_hint),
                );
                let submitted =
                    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                let run = ui
                    .add_enabled(self.ai_job.is_none(), egui::Button::new(run_label))
                    .on_hover_text(agent_hint)
                    .clicked();
                if (run || submitted) && self.ai_job.is_none() {
                    *action = Action::AiAgent;
                }
                if ui
                    .add_enabled(
                        has_history && self.ai_job.is_none(),
                        egui::Button::new(RichText::new(new_label).size(11.0)),
                    )
                    .on_hover_text(new_hint)
                    .clicked()
                {
                    *action = Action::AiAgentReset;
                }
            });
            // The question can carry whatever the user pointed at: the pseudo-C selection or
            // the selected listing row. The hint says which one would be sent.
            if self.selection_context().is_some() {
                let use_label = self.t("ui_agent_use_selection");
                let hint = self.t("ui_agent_selection_hint");
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.ai_agent_use_selection, use_label);
                    ui.weak(hint);
                });
            }
            // Tool calls are the evidence for the answer: without them a confident prose
            // answer is unverifiable.
            if !self.ai_agent_trace.is_empty() {
                let title = self.t_args(
                    "ui_agent_trace",
                    &[("count", &self.ai_agent_trace.len().to_string())],
                );
                egui::CollapsingHeader::new(RichText::new(title).size(11.0).color(pal.comment))
                    .id_salt("ai_agent_trace")
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("ai_agent_trace_scroll")
                            .max_height(140.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for line in &self.ai_agent_trace {
                                    ui.label(
                                        RichText::new(line).monospace().size(10.0).color(pal.muted),
                                    );
                                }
                            });
                    });
            }
        });

        // Chain-of-thought stream (only when the toggle asked for thinking). Expanded while
        // the model reasons, folded as soon as the answer completes.
        if !self.ai_reasoning.is_empty() {
            // Long reasoning passes can reach tens of thousands of characters; render only
            // the tail so live text layout stays cheap while the count shows the full size.
            const THINKING_DISPLAY_CHARS: usize = 12_000;
            let total = self.ai_reasoning.chars().count();
            let reasoning = if total > THINKING_DISPLAY_CHARS {
                self.ai_reasoning
                    .chars()
                    .skip(total - THINKING_DISPLAY_CHARS)
                    .collect::<String>()
            } else {
                self.ai_reasoning.clone()
            };
            let title = self.t_args("ui_thinking_process", &[("chars", &total.to_string())]);
            let open = self.ai_reasoning_open;
            let streaming = self.ai_job.is_some();
            let response =
                egui::CollapsingHeader::new(RichText::new(title).size(12.0).color(pal.comment))
                    .id_salt("ai_reasoning")
                    .open(Some(open))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("ai_reasoning_scroll")
                            .max_height(160.0)
                            .auto_shrink([false, false])
                            .stick_to_bottom(streaming)
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(&reasoning)
                                        .monospace()
                                        .size(10.0)
                                        .color(pal.comment),
                                );
                            });
                    });
            if response.header_response.clicked() {
                self.ai_reasoning_open = !open;
            }
            ui.add_space(8.0);
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(self.t("ai_result"))
                    .strong()
                    .size(13.0)
                    .color(pal.text),
            );
            if self.ai_explanation.is_some() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(self.t("ui_copy")).clicked()
                        && let Some(text) = self.ai_explanation.as_ref()
                    {
                        ui.ctx().copy_text(text.clone());
                    }
                });
            }
        });
        // Which function this answer belongs to. Without it an explanation kept looking
        // current after the cursor moved to another function.
        if let Some(target) = self.ai_result_target {
            let name = self
                .session
                .as_ref()
                .and_then(|session| session.name_at(target).ok().flatten())
                .unwrap_or_else(|| format!("sub_{:x}", target.get()));
            let label = format!("{name} ({:#x})", target.get());
            let stale = self.current_function().map(|(start, _)| start) != Some(target);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(self.t_args("ui_ai_result_for", &[("target", &label)]))
                        .size(11.0)
                        .color(pal.comment),
                );
                if ui.link(self.t("ui_jump_to_function")).clicked() {
                    *action = Action::Navigate(target);
                }
                if stale {
                    ui.label(
                        RichText::new(self.t("ui_ai_result_stale"))
                            .size(11.0)
                            .color(Color32::from_rgb(0xc0, 0x90, 0x40)),
                    );
                }
            });
            ui.add_space(4.0);
        }
        ui.add_space(4.0);
        let result_kind = self.ai_result_kind;
        match self.ai_explanation.clone() {
            Some(text) => {
                card_frame(pal).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    egui::ScrollArea::vertical()
                        .id_salt("ai_explanation")
                        .max_height(240.0)
                        .auto_shrink([false, false])
                        .stick_to_bottom(self.ai_job.is_some())
                        .show(ui, |ui| {
                            let mono = FontId::monospace(11.0);
                            let mut job = LayoutJob::default();
                            append_ai_result(&mut job, &text, result_kind, pal, &mono);
                            // Wrap long lines at the pane width, like the plain label did.
                            job.wrap.max_width = ui.available_width();
                            ui.label(job);
                        });
                });
                // Addresses quoted in the answer are the evidence: make them clickable so
                // the reader can check each one instead of hunting for the address.
                let links: Vec<Address> = address_links(&text)
                    .into_iter()
                    .filter(|addr| {
                        self.functions
                            .iter()
                            .any(|function| *addr >= function.start && *addr < function.end)
                    })
                    .take(10)
                    .collect();
                if !links.is_empty() {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(self.t("ui_answer_addresses"))
                                .size(11.0)
                                .color(pal.comment),
                        );
                        for addr in links {
                            if ui
                                .link(RichText::new(format!("{:#x}", addr.get())).size(11.0))
                                .clicked()
                            {
                                *action = Action::Navigate(addr);
                            }
                        }
                    });
                }
            }
            None => {
                card_frame(pal).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.weak(self.t("ai_no_result"));
                });
            }
        }

        ui.add_space(8.0);
        ui.label(
            RichText::new(self.t("ai_ask_database"))
                .strong()
                .size(13.0)
                .color(pal.text),
        );
        ui.add_space(4.0);
        let query_hint = self.t("ui_ask_urls_callers_of_createfilew_text_http");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.ai_query)
                    .desired_width((ui.available_width() - 80.0).max(80.0))
                    .hint_text(query_hint),
            );
            if ui.button(self.t("ui_query")).clicked() {
                *action = Action::AiQuery;
            }
        });
        if let Some(answer) = self.ai_answer.clone() {
            ui.add_space(4.0);
            let count = answer.items.len().to_string();
            let truncated = if answer.truncated {
                self.t("ui_truncated_suffix")
            } else {
                String::new()
            };
            ui.weak(self.t_args(
                "ui_query_matches",
                &[
                    ("intent", &answer.intent),
                    ("count", &count),
                    ("truncated", &truncated),
                ],
            ));
            card_frame(pal).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                egui::ScrollArea::vertical()
                    .id_salt("ai_answer")
                    .max_height(160.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for item in answer.items.iter().take(20) {
                            let label = format!("[{}] {}  {}", item.kind, item.addr, item.text);
                            let response = ui.selectable_label(false, label);
                            if response.clicked()
                                && let Some(addr) = parse_query_addr(&item.addr)
                            {
                                *action = Action::NavigateToDisasm(addr);
                            }
                            if !item.functions.is_empty() {
                                ui.weak(format!("  {}", item.functions.join(", ")));
                            }
                        }
                    });
            });
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} ({})",
                    self.t("ai_suggestions"),
                    self.ai_proposals.len()
                ))
                .strong()
                .size(13.0)
                .color(pal.text),
            );
        });
        ui.add_space(4.0);
        if self.ai_proposals.is_empty() {
            card_frame(pal).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.weak(self.t("ui_no_suggestions_yet_run_analyze_then_press_suggest_names"));
            });
        } else {
            card_frame(pal).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                // Review first, apply later: a local heuristic fallback name is labelled as
                // such, so "the model named 12 of 50" cannot be read as a model failure.
                ui.horizontal_wrapped(|ui| {
                    if ui.button(self.t("ui_check_all")).clicked() {
                        self.ai_checked = self
                            .ai_proposals
                            .iter()
                            .map(|proposal| proposal.target)
                            .collect();
                    }
                    if ui.button(self.t("ui_check_none")).clicked() {
                        self.ai_checked.clear();
                    }
                    if ui.button(self.t("ui_auto_apply")).clicked() {
                        // One-click naming: check everything, then let the normal apply path
                        // journal each rename (undo takes them back one at a time).
                        self.ai_checked = self
                            .ai_proposals
                            .iter()
                            .map(|proposal| proposal.target)
                            .collect();
                        *action = Action::AiApply;
                    }
                });
                for (index, proposal) in self.ai_proposals.iter().enumerate() {
                    let mut checked = self.ai_checked.contains(&proposal.target);
                    let source = match proposal.source {
                        recraft_ai::ProposalSource::Model => self.t("ui_source_model"),
                        recraft_ai::ProposalSource::Heuristic => self.t("ui_source_heuristic"),
                        recraft_ai::ProposalSource::External => self.t("ui_source_external"),
                    };
                    let label = format!(
                        "{:.0}%  {:#012x}  →  {}",
                        proposal.confidence * 100.0,
                        proposal.target.get(),
                        proposal.value
                    );
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut checked, "").clicked() {
                            if checked {
                                self.ai_checked.insert(proposal.target);
                            } else {
                                self.ai_checked.remove(&proposal.target);
                            }
                        }
                        let response = ui.add_sized(
                            [ui.available_width(), 20.0],
                            egui::Button::selectable(
                                false,
                                RichText::new(label).monospace().color(pal.text),
                            ),
                        );
                        if response.clicked() {
                            if self.ai_checked.contains(&proposal.target) {
                                self.ai_checked.remove(&proposal.target);
                            } else {
                                self.ai_checked.insert(proposal.target);
                            }
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!("[{}]", source))
                                .size(10.0)
                                .color(pal.mnemonic),
                        );
                        ui.label(
                            RichText::new(&proposal.rationale)
                                .size(11.0)
                                .color(pal.muted),
                        );
                    });
                    if let Some(evidence) = proposal.evidence.first()
                        && ui
                            .link(format!(
                                "{} {:#x}",
                                self.t("ai_evidence"),
                                evidence.addr.get()
                            ))
                            .clicked()
                    {
                        *action = Action::Navigate(evidence.addr);
                    }
                    if index + 1 < self.ai_proposals.len() {
                        ui.separator();
                    }
                }
            });
        }

        if !self.ai_var_proposals.is_empty() {
            let mut apply_now = false;
            let rows: Vec<(i64, String, String)> = self
                .ai_var_proposals
                .iter()
                .map(|proposal| {
                    (
                        proposal.offset,
                        proposal.value.clone(),
                        proposal.rationale.clone(),
                    )
                })
                .collect();
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("{} ({})", self.t("ui_var_suggestions"), rows.len()))
                    .strong()
                    .size(13.0)
                    .color(pal.text),
            );
            ui.add_space(4.0);
            card_frame(pal).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                if let Some(func) = self.ai_var_function {
                    ui.weak(format!("{:#x}", func.get()));
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button(self.t("ui_check_all")).clicked() {
                        self.ai_var_checked = rows.iter().map(|row| row.0).collect();
                    }
                    if ui.button(self.t("ui_check_none")).clicked() {
                        self.ai_var_checked.clear();
                    }
                    if ui.button(self.t("ui_apply")).clicked() {
                        apply_now = true;
                    }
                });
                for (offset, value, rationale) in &rows {
                    let mut checked = self.ai_var_checked.contains(offset);
                    let slot = if *offset < 0 {
                        format!("var_n{:x}", -offset)
                    } else {
                        format!("var_{offset:x}")
                    };
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut checked, "").clicked() {
                            if checked {
                                self.ai_var_checked.insert(*offset);
                            } else {
                                self.ai_var_checked.remove(offset);
                            }
                        }
                        ui.label(
                            RichText::new(format!("{slot}  →  {value}"))
                                .monospace()
                                .color(pal.text),
                        );
                    });
                    ui.label(RichText::new(rationale).size(11.0).color(pal.muted));
                }
            });
            if apply_now {
                self.apply_var_selection();
            }
        }

        ui.add_space(8.0);
    }

    fn ai_sidebar_ui(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        if !self.ai_sidebar {
            let screen = ui.ctx().content_rect();
            let mut pos = self
                .ai_fab_pos
                .unwrap_or(egui::pos2(screen.right() - 64.0, screen.center().y - 20.0));
            pos.x = pos.x.clamp(screen.left() + 8.0, screen.right() - 60.0);
            pos.y = pos.y.clamp(screen.top() + 8.0, screen.bottom() - 48.0);
            egui::Area::new(egui::Id::new("ai_sidebar_fab"))
                .fixed_pos(pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    let (rect, response) = ui
                        .allocate_exact_size(egui::vec2(52.0, 40.0), egui::Sense::click_and_drag());
                    let painter = ui.painter();
                    // Soft stacked glow instead of a flat drop shadow.
                    for step in (1..=4).rev() {
                        painter.rect_filled(
                            rect.expand(step as f32 * 2.0),
                            20.0,
                            Color32::from_black_alpha(8),
                        );
                    }
                    let fill = if response.hovered() {
                        self.palette().mnemonic.gamma_multiply(1.15)
                    } else {
                        self.palette().mnemonic
                    };
                    painter.rect_filled(rect, 20.0, fill);
                    painter.rect_stroke(
                        rect,
                        20.0,
                        egui::Stroke::new(1.0, Color32::from_white_alpha(110)),
                        egui::StrokeKind::Inside,
                    );
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "AI",
                        egui::FontId::proportional(14.0),
                        Color32::WHITE,
                    );
                    let response = response
                        .on_hover_text(self.t("ui_open_in_sidebar"))
                        .on_hover_cursor(egui::CursorIcon::Grab);
                    if response.dragged() {
                        pos += response.drag_delta();
                        pos.x = pos.x.clamp(screen.left() + 8.0, screen.right() - 60.0);
                        pos.y = pos.y.clamp(screen.top() + 8.0, screen.bottom() - 48.0);
                        self.ai_fab_pos = Some(pos);
                    }
                    if response.drag_stopped() {
                        self.layout_saved = None;
                        self.save_layout_now();
                    }
                    if response.clicked() {
                        *action = Action::ToggleAiSidebar;
                    }
                });
            return;
        }
        let response = egui::Panel::right("ai_sidebar")
            .resizable(true)
            .default_size(self.ai_sidebar_width)
            .min_size(300.0)
            .max_size(620.0)
            .show(ui, |ui| {
                ui.add_space(3.0);
                self.ai_panel(ui, action);
            });
        self.ai_sidebar_width = response.response.rect.width().clamp(300.0, 620.0);
    }

    /// Help → About: icon, version, license and the project link. The icon is the same raw
    /// RGBA the window uses, uploaded as a texture on first open.
    fn about_window(&mut self, ctx: &egui::Context) {
        if !self.about_open {
            return;
        }
        let icon = self.icon_texture(ctx).id();
        let mut open = self.about_open;
        let title = self.t("menu_about");
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.image((icon, egui::vec2(64.0, 64.0)));
                    ui.vertical(|ui| {
                        ui.heading("ReCraft");
                        ui.weak(format!("v{}", env!("CARGO_PKG_VERSION")));
                        ui.label(self.t("ui_about_tagline"));
                        ui.weak(self.t("ui_about_license"));
                    });
                });
                ui.separator();
                ui.hyperlink_to(
                    "github.com/qux-bbb/ReCraft",
                    "https://github.com/qux-bbb/ReCraft",
                );
            });
        self.about_open = open;
    }

    /// The 64×64 application icon as an egui texture (created once, then reused).
    fn icon_texture(&mut self, ctx: &egui::Context) -> &egui::TextureHandle {
        /// Same bytes `main.rs` hands to the window icon (generated by the icon script).
        const ICON_RGBA: &[u8] = include_bytes!("../../../assets/recraft-64.rgba");
        self.about_icon.get_or_insert_with(|| {
            let image = egui::ColorImage::from_rgba_unmultiplied([64, 64], ICON_RGBA);
            ctx.load_texture("app_icon", image, egui::TextureOptions::LINEAR)
        })
    }

    fn ai_settings_window(&mut self, ctx: &egui::Context) {
        if !self.ai_settings_open {
            self.prompt_drafts_ready = false;
            return;
        }
        let mut open = self.ai_settings_open;
        egui::Window::new(self.t("ui_provider_settings"))
            .open(&mut open)
            .resizable(true)
            .default_width(560.0)
            .show(ctx, |ui| self.ai_settings_ui(ui));
        self.ai_settings_open = open;
    }

    /// Return whether an LLM provider is configured, updating the AI status if not.
    fn require_ai_provider(&mut self) -> bool {
        self.ai_settings.refresh_enabled();
        if self.ai_settings.enabled {
            true
        } else {
            self.ai_status = self.t("ui_configure_provider_first").to_owned();
            false
        }
    }

    /// Switch the AI provider as one unit: endpoint/model preset, API-key field and
    /// session key. Keys are remembered per provider so switching back restores the right
    /// secret; the previous key is never sent to the newly selected endpoint.
    fn switch_ai_mode(&mut self, mode: AiMode) {
        let previous = self.ai_mode;
        if previous == mode {
            return;
        }
        if !self.ai_api_key.trim().is_empty() {
            self.ai_settings.api_key = Some(self.ai_api_key.trim().to_owned());
        }
        self.ai_settings.remember_current_key();
        self.ai_mode = mode;
        match mode.preset() {
            Some(preset) => preset.apply(&mut self.ai_settings),
            None => {
                self.ai_settings.kind = recraft_ai::ProviderKind::OpenAiCompatible;
                if previous.preset().is_some() || self.ai_settings.endpoint.is_empty() {
                    let defaults = recraft_ai::ProviderSettings::default();
                    self.ai_settings.endpoint = "https://api.openai.com".to_owned();
                    self.ai_settings.model = defaults.model;
                }
            }
        }
        let key = self
            .ai_settings
            .saved_api_keys
            .get(&self.ai_settings.provider_key_id())
            .cloned();
        self.ai_api_key = key.clone().unwrap_or_default();
        self.ai_settings.api_key = key;
        self.ai_settings.api_key_env = recraft_ai::ProviderSettings::default().api_key_env;
        self.ai_settings.refresh_enabled();
        self.ai_cache = recraft_ai::PromptCache::new();
        self.ai_cache_fingerprint = None;
        self.ai_key_id = self.ai_settings.provider_key_id();
        self.ai_status = if self.ai_settings.api_key_present() {
            self.t("ui_provider_switched_key_restored").to_owned()
        } else {
            self.t("ui_provider_switched_enter_key").to_owned()
        };
    }

    /// Detect a raw endpoint edit in Custom mode. Preserve the key under the old
    /// endpoint and restore the key remembered for the new endpoint, if any.
    fn sync_provider_key_identity(&mut self) {
        let id = self.ai_settings.provider_key_id();
        if id == self.ai_key_id {
            return;
        }
        if let Some(key) =
            (!self.ai_api_key.trim().is_empty()).then(|| self.ai_api_key.trim().to_owned())
        {
            self.ai_settings
                .saved_api_keys
                .insert(self.ai_key_id.clone(), key);
        }
        let key = self.ai_settings.saved_api_keys.get(&id).cloned();
        self.ai_api_key = key.clone().unwrap_or_default();
        self.ai_settings.api_key = key;
        self.ai_settings.refresh_enabled();
        self.ai_key_id = id;
        self.ai_cache = recraft_ai::PromptCache::new();
        self.ai_cache_fingerprint = None;
        self.ai_status = if self.ai_settings.api_key_present() {
            self.t("ui_provider_switched_key_restored").to_owned()
        } else {
            self.t("ui_provider_switched_enter_key").to_owned()
        };
    }

    /// Provider settings editor: built-in DeepSeek / OpenCode Go modes, model picker and a
    /// session-only API key (persisted settings never contain the key).
    fn ai_settings_ui(&mut self, ui: &mut egui::Ui) {
        let title = self.t("ui_provider_settings");
        egui::CollapsingHeader::new(title)
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(self.t("ui_mode"));
                    egui::ComboBox::from_id_salt("ai_mode")
                        .selected_text(self.ai_mode.label())
                        .show_ui(ui, |ui| {
                            for mode in [AiMode::DeepSeek, AiMode::OpenCodeGo, AiMode::Custom] {
                                if ui
                                    .selectable_label(self.ai_mode == mode, mode.label())
                                    .clicked()
                                {
                                    self.switch_ai_mode(mode);
                                }
                            }
                        });
                });
                if let Some(preset) = self.ai_mode.preset() {
                    ui.horizontal(|ui| {
                        ui.label(self.t("ui_endpoint"));
                        ui.weak(preset.endpoint());
                    });
                    ui.horizontal(|ui| {
                        ui.label(self.t("ui_model"));
                        egui::ComboBox::from_id_salt("ai_model")
                            .selected_text(self.ai_settings.model.clone())
                            .show_ui(ui, |ui| {
                                for model in preset.models() {
                                    ui.selectable_value(
                                        &mut self.ai_settings.model,
                                        (*model).to_owned(),
                                        *model,
                                    );
                                }
                            });
                    });
                } else if self.ai_mode == AiMode::Custom {
                    ui.horizontal(|ui| {
                        ui.label(self.t("ui_endpoint"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ai_settings.endpoint)
                                .desired_width(f32::INFINITY)
                                .hint_text("https://api.example.com"),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(self.t("ui_model"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ai_settings.model)
                                .desired_width(220.0),
                        );
                        ui.label(self.t("ui_key_env"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ai_settings.api_key_env)
                                .desired_width(180.0),
                        );
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label(self.t("ui_endpoint"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ai_settings.endpoint)
                                .desired_width(320.0)
                                .hint_text("https://api.example.com/v1"),
                        );
                    });
                }
                ui.horizontal(|ui| {
                    ui.label(self.t("ui_api_key"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.ai_api_key)
                            .password(!self.ai_show_key)
                            .desired_width(320.0)
                            .hint_text("sk-…"),
                    );
                    let reveal = if self.ai_show_key {
                        self.t("ui_hide")
                    } else {
                        self.t("ui_show")
                    };
                    if ui.button(reveal).clicked() {
                        self.ai_show_key = !self.ai_show_key;
                    }
                    if ui.button(self.t("ui_copy")).clicked() {
                        ui.ctx().copy_text(self.ai_api_key.clone());
                    }
                });
                ui.weak(self.t("ui_the_key_is_encrypted_for_this_user_dpapi_and_reused_on_startu"));
                ui.horizontal(|ui| {
                    ui.label(self.t("ui_max_tokens"));
                    ui.add(
                        egui::DragValue::new(&mut self.ai_settings.max_tokens)
                            .range(1024..=65536)
                            .speed(512),
                    );
                    ui.weak(self.t("ui_reasoning_models_usually_need_32768_65536"));
                });
                if self.ai_mode == AiMode::OpenCodeGo {
                    ui.weak(self.t("ui_opencode_polish_budget_cap"));
                }
                self.sync_provider_key_identity();
                if ui.button(self.t("ui_save_settings")).clicked() {
                    self.ai_settings.api_key = (!self.ai_api_key.trim().is_empty())
                        .then(|| self.ai_api_key.trim().to_owned());
                    self.ai_settings.remember_current_key();
                    self.ai_settings.refresh_enabled();
                    match self.ai_settings.save() {
                        Ok(()) => {
                            self.ai_status = format!(
                                "{}{}",
                                self.t("ui_settings_saved"),
                                self.ai_settings.describe()
                            );
                        }
                        Err(error) => self.set_error(error),
                    }
                }
                let env_key = std::env::var(&self.ai_settings.api_key_env)
                    .is_ok_and(|value| !value.is_empty());
                let key_state = if !self.ai_api_key.trim().is_empty() {
                    self.t("ui_api_key_saved")
                } else if env_key {
                    self.t("ui_env_key_detected")
                } else {
                    self.t("ui_no_api_key")
                };
                ui.weak(format!(
                    "{key_state} · {}",
                    if self.ai_settings.enabled {
                        self.t("ui_enabled")
                    } else {
                        self.t("ui_not_configured")
                    }
                ));
            });
        self.ai_settings.api_key =
            (!self.ai_api_key.trim().is_empty()).then(|| self.ai_api_key.trim().to_owned());
        self.ai_settings.refresh_enabled();
        self.prompt_section(ui);
    }

    /// Fill the prompt editors from the settings: one per kind, in [`PromptKind::ALL`] order.
    fn reload_prompt_drafts(&mut self) {
        self.prompt_drafts = recraft_ai::PromptKind::ALL
            .iter()
            .map(|kind| (*kind, recraft_ai::system_prompt(&self.ai_settings, *kind)))
            .collect();
        self.prompt_drafts_language = self.ai_settings.prompt_language;
        self.prompt_drafts_ready = true;
    }

    /// Built-in system prompts: pick the prompt language, edit any of the four, save the
    /// override or restore the shipped text. Edits are user data in `ai-settings.json`; the
    /// cache key hashes the system text, so a changed prompt never hits an old reply.
    fn prompt_section(&mut self, ui: &mut egui::Ui) {
        let title = self.t("ui_prompts");
        let language_label = self.t("ui_prompt_language");
        let hint = self.t("ui_prompt_language_hint");
        let save_label = self.t("ui_save_prompt");
        let reset_label = self.t("ui_reset_prompt");
        let labels = [
            self.t("ui_prompt_explain"),
            self.t("ui_prompt_rename"),
            self.t("ui_prompt_polish"),
            self.t("ui_prompt_agent"),
            self.t("ui_prompt_vars"),
        ];
        if !self.prompt_drafts_ready
            || self.prompt_drafts_language != self.ai_settings.prompt_language
        {
            self.reload_prompt_drafts();
        }
        egui::CollapsingHeader::new(title)
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(language_label.as_str());
                    egui::ComboBox::from_id_salt("ai_prompt_language")
                        .selected_text(self.ai_settings.prompt_language.label())
                        .show_ui(ui, |ui| {
                            for language in [
                                recraft_ai::PromptLanguage::En,
                                recraft_ai::PromptLanguage::Zh,
                            ] {
                                if ui
                                    .selectable_label(
                                        self.ai_settings.prompt_language == language,
                                        language.label(),
                                    )
                                    .clicked()
                                    && self.ai_settings.prompt_language != language
                                {
                                    self.ai_settings.prompt_language = language;
                                    self.reload_prompt_drafts();
                                }
                            }
                        });
                });
                ui.weak(hint.as_str());
                for (index, kind) in recraft_ai::PromptKind::ALL.iter().copied().enumerate() {
                    let overridden =
                        self.ai_settings
                            .prompt_overrides
                            .contains_key(&recraft_ai::prompt_key(
                                kind,
                                self.ai_settings.prompt_language,
                            ));
                    egui::CollapsingHeader::new(labels[index].as_str())
                        .id_salt(kind.key())
                        .show(ui, |ui| {
                            {
                                let text = &mut self.prompt_drafts[index].1;
                                ui.add(
                                    egui::TextEdit::multiline(text)
                                        .desired_rows(7)
                                        .desired_width(f32::INFINITY),
                                );
                            }
                            let mut save = false;
                            let mut reset = false;
                            ui.horizontal(|ui| {
                                save = ui.button(save_label.as_str()).clicked();
                                reset = ui
                                    .add_enabled(
                                        overridden,
                                        egui::Button::new(reset_label.as_str()),
                                    )
                                    .clicked();
                            });
                            if !(save || reset) {
                                return;
                            }
                            if reset {
                                self.ai_settings.set_prompt_override(kind, None);
                                self.prompt_drafts[index].1 =
                                    self.ai_settings.builtin_prompt(kind).to_owned();
                            } else {
                                let text = self.prompt_drafts[index].1.clone();
                                self.ai_settings.set_prompt_override(kind, Some(text));
                            }
                            self.ai_settings.api_key = (!self.ai_api_key.trim().is_empty())
                                .then(|| self.ai_api_key.trim().to_owned());
                            self.ai_settings.remember_current_key();
                            self.ai_settings.refresh_enabled();
                            match self.ai_settings.save() {
                                Ok(()) => {
                                    self.ai_status = self.t("ui_settings_saved").to_owned();
                                }
                                Err(error) => self.set_error(error),
                            }
                        });
                }
            });
    }

    /// Apply every ticked AI proposal through the command bus (journaled/undoable).
    ///
    /// Each rename is its own journal record, so undo takes them back one at a time; the
    /// status says how many landed when a later proposal is refused (e.g. a duplicate
    /// name), instead of pretending the whole batch succeeded.
    fn apply_ai_selection(&mut self) -> bool {
        let proposals: Vec<recraft_ai::Proposal> = self
            .ai_proposals
            .iter()
            .filter(|proposal| self.ai_checked.contains(&proposal.target))
            .cloned()
            .collect();
        if proposals.is_empty() {
            return false;
        }
        let mut applied: Vec<Address> = Vec::new();
        let mut failure = None;
        if let Some(session) = self.session.as_mut() {
            for proposal in &proposals {
                match recraft_ai::apply_proposals(session, std::slice::from_ref(proposal)) {
                    Ok(_) => applied.push(proposal.target),
                    Err(error) => {
                        failure = Some(error.to_string());
                        break;
                    }
                }
            }
        }
        if applied.is_empty() {
            if let Some(error) = failure {
                self.ai_status = self.t_args("ui_ai_apply_failed", &[("error", &error)]);
                self.set_status(self.ai_status.clone());
            }
            return false;
        }
        self.functions_dirty = true;
        self.disasm_dirty = true;
        self.ai_proposals
            .retain(|proposal| !applied.contains(&proposal.target));
        self.ai_checked.clear();
        self.ai_status = match failure {
            Some(error) => self.t_args(
                "ui_ai_applied_partial",
                &[("count", &applied.len().to_string()), ("error", &error)],
            ),
            None => self.t_args(
                "ui_ai_applied_many",
                &[("count", &applied.len().to_string())],
            ),
        };
        self.set_status(self.ai_status.clone());
        true
    }

    /// IDA-style graph view: basic-block nodes with teal frames and headers, curved
    /// coloured edges with arrowheads, entry marker, zoom controls and a status line.
    fn graph_view(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        self.tab = Tab::Disasm;
        let pal = self.palette();
        let Some(cfg) = self.cfg.clone() else {
            ui.weak(self.t("ui_no_control_flow_graph_run_analyze_f5_and_put_the_cursor_insid"));
            return;
        };
        if cfg.blocks.is_empty() {
            ui.weak(self.t("ui_function_has_no_decodable_blocks"));
            return;
        }
        let edge_color = |kind: recraft_analysis::EdgeKind| graph_edge_color(pal, kind);
        ui.horizontal(|ui| {
            for (label, color) in [
                (
                    self.t("graph_edge_fallthrough"),
                    edge_color(recraft_analysis::EdgeKind::Fallthrough),
                ),
                (
                    self.t("graph_edge_taken"),
                    edge_color(recraft_analysis::EdgeKind::Taken),
                ),
                (
                    self.t("graph_edge_not_taken"),
                    edge_color(recraft_analysis::EdgeKind::NotTaken),
                ),
                (
                    self.t("graph_edge_jump"),
                    edge_color(recraft_analysis::EdgeKind::Unconditional),
                ),
                (
                    self.t("graph_edge_jump_table"),
                    edge_color(recraft_analysis::EdgeKind::JumpTable),
                ),
            ] {
                ui.colored_label(color, label);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("+")
                    .on_hover_text(self.t("graph_zoom_in"))
                    .clicked()
                {
                    self.graph_zoom = (self.graph_zoom + 0.1).min(2.0);
                }
                if ui
                    .small_button("-")
                    .on_hover_text(self.t("graph_zoom_out"))
                    .clicked()
                {
                    self.graph_zoom = (self.graph_zoom - 0.1).max(0.5);
                }
                if ui
                    .small_button(self.t("graph_fit"))
                    .on_hover_text(self.t("graph_zoom_reset"))
                    .clicked()
                {
                    self.graph_zoom = 1.0;
                }
                ui.monospace(format!("{:.2}%", self.graph_zoom * 100.0));
            });
        });
        let zoom = self.graph_zoom;
        let header_h = 18.0 * zoom;
        let line_h = 14.0 * zoom;
        let text_size = 11.0 * zoom;
        let pad = 10.0 * zoom;
        let max_lines = 10usize;
        let (local, canvas) = graph_layout(&cfg, zoom);
        let canvas_w = canvas.x;
        let canvas_h = canvas.y;
        let selected = self
            .disasm_rows
            .get(self.selected)
            .map(|line| line.addr.get());
        let mono = FontId::monospace(text_size);
        let mut visible_rect = None;
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let (response, painter) = ui.allocate_painter(
                    egui::vec2(
                        canvas_w.max(ui.available_width()),
                        canvas_h.max(ui.available_height()),
                    ),
                    Sense::hover(),
                );
                let origin = response.rect.min.to_vec2();
                let clip = ui.clip_rect();
                visible_rect = Some(egui::Rect::from_min_max(
                    clip.min - origin,
                    clip.max - origin,
                ));
                // An overview click asks the graph to centre one block without leaving
                // graph mode (IDA pans the graph instead of switching to the listing).
                if let Some(addr) = self.graph_center_on.take()
                    && let Some(index) = cfg.blocks.iter().position(|block| {
                        block.start.get() <= addr.get() && addr.get() < block.end.get()
                    })
                {
                    ui.scroll_to_rect(local[index].translate(origin), Some(egui::Align::Center));
                }
                // Edges first, so frames paint on top.
                for block in &cfg.blocks {
                    for &(target, kind) in &block.edges {
                        let Some(target_rect) = local.get(target) else {
                            continue;
                        };
                        let from = local[block.index].right_center() + origin;
                        let to = target_rect.left_center() + origin;
                        let color = edge_color(kind);
                        let bend = ((to.x - from.x).abs() * 0.4).max(40.0);
                        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                            [
                                from,
                                egui::pos2(from.x + bend, from.y),
                                egui::pos2(to.x - bend, to.y),
                                to,
                            ],
                            false,
                            Color32::TRANSPARENT,
                            egui::Stroke::new((1.2 * zoom).max(1.0), color),
                        ));
                        // Arrowhead pointing into the target block.
                        let dir = (to - egui::pos2(to.x - bend, to.y)).normalized();
                        let size = 5.0 * zoom;
                        let normal = egui::vec2(-dir.y, dir.x);
                        let tip = to;
                        let base = to - dir * size;
                        painter.add(egui::epaint::PathShape::convex_polygon(
                            vec![tip, base + normal * size * 0.6, base - normal * size * 0.6],
                            color,
                            egui::Stroke::NONE,
                        ));
                    }
                }
                for (index, block) in cfg.blocks.iter().enumerate() {
                    let rect = local[index].translate(origin);
                    let is_entry = index == cfg.entry;
                    let current = selected
                        .is_some_and(|addr| addr >= block.start.get() && addr < block.end.get());
                    // IDA-style frame: dark body, teal border, light header bar.
                    painter.rect_filled(rect, 4.0, pal.graph_node);
                    let border = if current {
                        pal.graph_selected_border
                    } else {
                        pal.graph_border
                    };
                    painter.rect_stroke(
                        rect,
                        4.0,
                        egui::Stroke::new(if current { 2.0 } else { 1.0 }, border),
                        egui::StrokeKind::Inside,
                    );
                    let header =
                        egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), header_h));
                    painter.rect_filled(header, 4.0, pal.graph_header);
                    painter.text(
                        header.min + egui::vec2(6.0, 1.0),
                        egui::Align2::LEFT_TOP,
                        format!("{:#x}", block.start.get()),
                        FontId::monospace(text_size * 0.92),
                        pal.graph_header_text,
                    );
                    if is_entry {
                        painter.text(
                            header.right_center() - egui::vec2(4.0, 0.0),
                            egui::Align2::RIGHT_CENTER,
                            self.t("graph_entry").as_str(),
                            FontId::proportional(text_size * 0.85),
                            pal.comment,
                        );
                    }
                    for (line_index, insn) in block.insns.iter().take(max_lines).enumerate() {
                        let mut job = LayoutJob::default();
                        let (mnemonic, operands) = match insn.text.split_once(' ') {
                            Some((mnemonic, rest)) => (mnemonic, rest),
                            None => (insn.text.as_str(), ""),
                        };
                        append(&mut job, mnemonic, pal.mnemonic, &mono);
                        append(&mut job, " ", pal.text, &mono);
                        if let Some((operands, comment)) = operands.split_once(';') {
                            append_operands(&mut job, operands, pal, &mono);
                            append(&mut job, &format!("; {comment}"), pal.comment, &mono);
                        } else {
                            append_operands(&mut job, operands, pal, &mono);
                        }
                        let galley = painter.layout_job(job);
                        painter.galley(
                            rect.min + egui::vec2(pad, header_h + pad + line_index as f32 * line_h),
                            galley,
                            pal.text,
                        );
                    }
                    if block.insns.len() > max_lines {
                        painter.text(
                            rect.left_bottom() + egui::vec2(pad, -pad - line_h * 0.2),
                            egui::Align2::LEFT_BOTTOM,
                            format!("… {} more", block.insns.len() - max_lines),
                            FontId::proportional(text_size * 0.85),
                            Color32::from_gray(0x90),
                        );
                    }
                    let response = ui.interact(rect, ui.id().with(("cfg", index)), Sense::click());
                    if response.clicked() {
                        *action = Action::Navigate(block.start);
                    }
                    if response.double_clicked() {
                        *action = Action::NavigateToDisasm(block.start);
                    }
                    if response.hovered() {
                        let tooltip = block
                            .insns
                            .iter()
                            .map(|insn| insn.text.as_str())
                            .collect::<Vec<_>>()
                            .join("\n");
                        response.on_hover_text(tooltip);
                    }
                }
            });
        self.graph_visible = visible_rect;
        ui.separator();
        ui.horizontal(|ui| {
            ui.weak(format!("{:.2}%", zoom * 100.0));
            ui.separator();
            ui.weak(format!("{} blocks", cfg.blocks.len()));
            ui.separator();
            ui.weak(self.t("ui_synchronized_with_hex_view"));
        });
    }

    /// First-run experience: quick actions, recent projects and a shortcut cheat sheet.
    fn welcome(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let pal = self.palette();
        ui.add_space(28.0);
        ui.label(
            RichText::new(self.t("ui_recraft_reassembly_decompilation_workbench"))
                .size(24.0)
                .strong()
                .color(pal.text),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                self.t("ui_drop_a_rcraft_project_or_a_pe_file_anywhere_in_this_window_or"),
            )
            .size(13.0)
            .color(pal.muted),
        );
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            // One accent action (import) next to the secondary one, per docs/UI.md §3.
            let import = ui.add(
                egui::Button::new(
                    RichText::new(self.t("ui_import_binary"))
                        .size(13.0)
                        .color(egui::Color32::from_rgb(0x0b, 0x12, 0x14)),
                )
                .fill(pal.accent)
                .stroke(egui::Stroke::NONE)
                .corner_radius(egui::CornerRadius::same(6))
                .min_size(egui::vec2(120.0, 28.0)),
            );
            if import.clicked() {
                *action = Action::Import;
            }
            let open = ui.add(
                egui::Button::new(RichText::new(self.t("ui_open_project")).size(13.0))
                    .fill(pal.surface_2)
                    .stroke(egui::Stroke::new(1.0, pal.border_strong))
                    .corner_radius(egui::CornerRadius::same(6))
                    .min_size(egui::vec2(120.0, 28.0)),
            );
            if open.clicked() {
                *action = Action::Open;
            }
        });
        if !self.recent.is_empty() {
            ui.add_space(20.0);
            ui.label(
                RichText::new(self.t("ui_recent_projects"))
                    .size(11.0)
                    .strong()
                    .color(pal.muted),
            );
            ui.add_space(6.0);
            let recent = self.recent.clone();
            for (index, path) in recent.iter().enumerate() {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                let card = card_frame(pal)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width().clamp(240.0, 520.0));
                        let response = ui
                            .horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(name).size(13.0).strong());
                                    ui.label(
                                        RichText::new(path.display().to_string())
                                            .size(11.0)
                                            .color(pal.muted),
                                    );
                                });
                            })
                            .response;
                        if response.interact(egui::Sense::click()).clicked() {
                            *action = Action::OpenRecent(index);
                        }
                    });
                card.response
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                ui.add_space(6.0);
            }
        }
        ui.add_space(20.0);
        ui.label(
            RichText::new(
                self.t("ui_shortcuts_ctrl_p_palette_g_goto_n_rename_comment_x_xrefs_spac"),
            )
            .size(11.0)
            .color(pal.muted),
        );
    }

    /// Column text of one disassembly row exactly as the listing paints it. Text selection
    /// and copy both run off this, so they cannot drift from the rendering.
    fn disasm_columns(&self, line: &DisasmLine) -> (String, String, String, String, String) {
        let bytes = line
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x} "))
            .collect::<String>();
        (
            format!("{:#012x}  ", line.addr.get()),
            format!("{bytes:<46}"),
            format!("{:<9}", line.mnemonic),
            format!("{:<36}", line.operands),
            line.comment.clone().unwrap_or_default(),
        )
    }

    /// One disassembly row as the listing paints it (address, bytes, mnemonic, operands,
    /// comment).
    fn disasm_line_text(&self, line: &DisasmLine) -> String {
        let (address, bytes, mnemonic, operands, comment) = self.disasm_columns(line);
        format!("{address}{bytes}{mnemonic}{operands}{comment}")
    }

    /// Instruction text of one row, without the address/bytes columns: what Ctrl+C copies
    /// when the user did not drag a text selection.
    fn disasm_line_copy_text(&self, line: &DisasmLine) -> String {
        let mut text = format!("{} {}", line.mnemonic, line.operands);
        text.truncate(text.trim_end().len());
        if let Some(comment) = line
            .comment
            .as_deref()
            .map(str::trim)
            .filter(|comment| !comment.is_empty())
        {
            text.push_str("  ; ");
            text.push_str(comment.trim_start_matches(';').trim_start());
        }
        text
    }

    fn disasm_view(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        self.tab = Tab::Disasm;
        let pal = self.palette();
        if self.disasm_rows.is_empty() {
            ui.weak(self.t("ui_no_disassembly_import_a_pe_file_first"));
            return;
        }
        let mono = FontId::monospace(self.mono_size());
        let follow_menu = self.t("ui_follow_operand");
        let rename_menu = self.t("ui_rename");
        let comment_menu = self.t("ui_comment");
        let xrefs_menu = self.t("ui_cross_references");
        let copy_menu = self.t("ui_copy_address");
        let copy_text_menu = self.t("ui_copy");
        let undo_menu = self.t("ui_undo");
        let redo_menu = self.t("ui_redo");
        let mut clicked_row = None;
        let mut drag_row = None;
        let mut took_focus = false;
        let mut follow = false;
        // Editor-style text selection over the listing, kept across frames so the highlight
        // survives repaints while the pointer moves.
        let mut text_sel = self.disasm_sel;
        let mut dragging = self.disasm_drag;
        let pointer = ui.input(|input| input.pointer.latest_pos());
        let primary_down = ui.input(|input| input.pointer.primary_down());
        let extend_selection = ui.input(|input| input.modifiers.shift);
        let mut first_top: Option<f32> = None;
        let mut last_bottom: Option<f32> = None;
        // Column header, reserved above the listing instead of being the first row *inside* it:
        // as a scroll-area item the wheel carried it off the top with the instructions. IDA pins
        // its column header, and the Strings list here already reserves its header the same way.
        let headers = [
            self.t("ui_address"),
            self.t("ui_bytes"),
            self.t("ui_mnemonic"),
            self.t("ui_operands"),
            self.t("ui_comment_2"),
        ];
        // The CJK fallback makes a header line taller than a listing row (`row_height()` is 15 px
        // at 100 %), so the strip is sized to the text. Clipping the text to a row-high strip cut
        // the top of the glyphs.
        let head_line = headers
            .iter()
            .map(|label| {
                ui.painter()
                    .layout_no_wrap(label.clone(), mono.clone(), pal.text)
                    .size()
                    .y
            })
            .fold(0.0f32, f32::max);
        let header_height = self.row_height().max(head_line + 2.0);
        let (header_rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), header_height),
            Sense::hover(),
        );
        let mut area = egui::ScrollArea::both().auto_shrink([false, false]);
        if let Some(offset) = self.disasm_scroll_to.take() {
            area = area.vertical_scroll_offset(offset);
        }
        let scroll = area.show(ui, |ui| {
            let width = ui.available_width().max(1000.0);
            for (index, line) in self.disasm_rows.iter().enumerate() {
                // IDA renders names as inline labels above the instruction instead of a
                // dedicated "Name" column.
                let label_height = self.disasm_label_height(line);
                let row_height = self.row_height() + label_height;
                // One full-row hit area, like IDA: the whole line is selectable and
                // double-click follows the operand instead of only the address cell. The
                // drag sense is what turns the listing into selectable text.
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(width, row_height), Sense::click_and_drag());
                first_top.get_or_insert(rect.top());
                last_bottom = Some(rect.bottom());
                if index == self.selected {
                    ui.painter().rect_filled(rect, 1.0, pal.selected_row);
                } else if response.hovered() {
                    ui.painter().rect_filled(rect, 1.0, pal.hover_row);
                }
                if let Some(name) = line.name.as_deref() {
                    ui.painter().text(
                        rect.left_top() + egui::vec2(4.0, 1.0),
                        egui::Align2::LEFT_TOP,
                        format!("{name}:"),
                        mono.clone(),
                        pal.name,
                    );
                }
                let (address, byte_column, mnemonic, operands, comment) = self.disasm_columns(line);
                let mut job = LayoutJob::default();
                append(&mut job, &address, pal.addr, &mono);
                append(&mut job, &byte_column, pal.bytes, &mono);
                append(&mut job, &mnemonic, pal.mnemonic, &mono);
                append_operands(&mut job, &operands, pal, &mono);
                append(&mut job, &comment, pal.comment, &mono);
                let galley = ui.painter().layout_job(job);
                let text_origin = rect.left_top() + egui::vec2(4.0, label_height + 1.0);
                // The dragged selection sits behind the glyphs so the code stays readable.
                if let Some(selection) = text_sel
                    && let Some(range) = selection.row_range(index, galley.text().chars().count())
                {
                    paint_text_selection(
                        ui.painter(),
                        pal.text_selection,
                        &galley,
                        text_origin,
                        range,
                    );
                }
                ui.painter()
                    .galley(text_origin, galley.clone(), Color32::WHITE);
                // Text selection: a press anchors it, a drag extends it across rows, and a
                // plain click collapses it to a caret where it landed (Shift extends).
                if response.drag_started()
                    && let Some(pos) = ui.input(|input| input.pointer.press_origin())
                {
                    let point = TextPoint {
                        row: index,
                        char: galley_char_at(&galley, text_origin, pos),
                    };
                    text_sel = Some(TextSelection {
                        anchor: point,
                        focus: point,
                    });
                    dragging = true;
                    drag_row = Some(index);
                }
                if dragging
                    && primary_down
                    && let Some(pos) = pointer
                    && pos.y >= rect.top()
                    && pos.y <= rect.bottom()
                {
                    let point = TextPoint {
                        row: index,
                        char: galley_char_at(&galley, text_origin, pos),
                    };
                    if let Some(selection) = text_sel.as_mut() {
                        selection.focus = point;
                    }
                }
                if response.clicked()
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    let point = TextPoint {
                        row: index,
                        char: galley_char_at(&galley, text_origin, pos),
                    };
                    text_sel = Some(match text_sel {
                        Some(selection) if extend_selection => TextSelection {
                            focus: point,
                            ..selection
                        },
                        _ => TextSelection::collapsed(point),
                    });
                }
                if response.clicked() || response.secondary_clicked() {
                    clicked_row = Some(index);
                    took_focus = true;
                }
                if response.double_clicked() {
                    clicked_row = Some(index);
                    follow = true;
                }
                response.context_menu(|ui| {
                    if ui.button(copy_text_menu.as_str()).clicked() {
                        *action = Action::Copy;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(follow_menu.as_str()).clicked() {
                        *action = Action::Follow;
                        ui.close();
                    }
                    if ui.button(rename_menu.as_str()).clicked() {
                        *action = Action::EditName;
                        ui.close();
                    }
                    if ui.button(comment_menu.as_str()).clicked() {
                        *action = Action::EditComment;
                        ui.close();
                    }
                    if ui.button(xrefs_menu.as_str()).clicked() {
                        *action = Action::ShowXrefs;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(self.t("ui_mark_as_code_c")).clicked() {
                        *action = Action::ApplyData(DataOp::Code);
                        ui.close();
                    }
                    if ui.button(self.t("ui_mark_as_data_d")).clicked() {
                        *action = Action::ApplyData(DataOp::Data);
                        ui.close();
                    }
                    if ui.button(self.t("ui_mark_as_string_a")).clicked() {
                        *action = Action::ApplyData(DataOp::Ascii);
                        ui.close();
                    }
                    if ui.button(self.t("ui_undefine_u")).clicked() {
                        *action = Action::ApplyData(DataOp::Unknown);
                        ui.close();
                    }
                    if ui.button(self.t("ui_create_function_p")).clicked() {
                        *action = Action::ApplyData(DataOp::DefineFunction);
                        ui.close();
                    }
                    if ui.button(self.t("ui_remove_function")).clicked() {
                        *action = Action::ApplyData(DataOp::UndefineFunction);
                        ui.close();
                    }
                    if ui.button(self.t("ui_edit_prototype")).clicked() {
                        *action = Action::EditPrototype;
                        ui.close();
                    }
                    if ui.button(self.t("ui_structures")).clicked() {
                        *action = Action::ShowStructs;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(copy_menu.as_str()).clicked() {
                        ui.ctx().copy_text(format!("{:#x}", line.addr.get()));
                        ui.close();
                    }
                    if ui.button(undo_menu.as_str()).clicked() {
                        *action = Action::Undo;
                        ui.close();
                    }
                    if ui.button(redo_menu.as_str()).clicked() {
                        *action = Action::Redo;
                        ui.close();
                    }
                });
            }
        });
        // Pinned header: painted over the reserved strip after the list so it can follow this
        // frame's horizontal offset and keep its columns lined up with the rows.
        //
        // One label per column instead of one padded string: a CJK label is wider than one
        // monospace cell, so `{:<14}` padding would push every later column to the right. Only the
        // horizontal edge is clipped (the pane's), so a tall glyph box is never cut vertically.
        let cell = monospace_cell_width(ui, &mono, pal.text);
        let clip = egui::Rect::from_x_y_ranges(header_rect.x_range(), ui.clip_rect().y_range());
        let painter = ui.painter().with_clip_rect(clip);
        let left = header_rect.left() + 4.0 - scroll.state.offset.x;
        let top = header_rect.top() + ((header_height - head_line) * 0.5).max(1.0);
        for (cells, label) in DISASM_COLUMN_CELLS.iter().zip(&headers) {
            painter.text(
                egui::pos2(left + *cells as f32 * cell, top),
                egui::Align2::LEFT_TOP,
                label,
                mono.clone(),
                pal.text,
            );
        }
        self.follow_disasm_scroll(ui, &scroll);
        // Dragging above or below the window keeps selecting to that edge.
        if dragging
            && primary_down
            && let Some(pos) = pointer
        {
            if first_top.is_some_and(|top| pos.y < top) {
                if let Some(selection) = text_sel.as_mut() {
                    selection.focus = TextPoint { row: 0, char: 0 };
                }
            } else if last_bottom.is_some_and(|bottom| pos.y > bottom)
                && let Some(last) = self.disasm_rows.len().checked_sub(1)
            {
                let end = self
                    .disasm_line_text(&self.disasm_rows[last])
                    .chars()
                    .count();
                if let Some(selection) = text_sel.as_mut() {
                    selection.focus = TextPoint {
                        row: last,
                        char: end,
                    };
                }
            }
        }
        if !primary_down {
            dragging = false;
        }
        self.disasm_sel = text_sel;
        self.disasm_drag = dragging;
        if let Some(index) = clicked_row.or(drag_row) {
            self.selected = index;
            self.copy_focus = CopyFocus::Disasm;
        }
        if took_focus {
            self.copy_focus = CopyFocus::Disasm;
        }
        if follow {
            *action = Action::Follow;
        }
        if let Some(line) = self.disasm_rows.get(self.selected) {
            ui.separator();
            ui.horizontal(|ui| {
                ui.weak(format!("100.00%   {:#x}", line.addr.get()));
                if let Some(name) = line.name.as_deref() {
                    ui.separator();
                    ui.label(RichText::new(name).color(pal.name));
                }
                if let Some((start, _)) = self.current_function() {
                    ui.separator();
                    ui.weak(format!("fn {:#x}", start.get()));
                }
                ui.separator();
                ui.weak(self.t("ui_synchronized_with_hex_view"));
            });
        }
    }

    /// Extra height a row needs for the inline name label IDA draws above an instruction.
    fn disasm_label_height(&self, line: &DisasmLine) -> f32 {
        if line.name.is_some() {
            self.mono_size() + 1.0
        } else {
            0.0
        }
    }

    /// Pixel pitch of one rendered row: the allocated height plus the item spacing the layout
    /// inserts after it (kept in sync with `disasm_view`).
    fn disasm_row_pitch(&self, ui: &egui::Ui, line: &DisasmLine) -> f32 {
        self.row_height() + self.disasm_label_height(line) + ui.spacing().item_spacing.y
    }

    /// Let the wheel keep scrolling past the ends of the loaded window.
    ///
    /// The pane holds a fixed window of instructions, so a plain `ScrollArea` can only scroll
    /// *inside* it: at the top edge the wheel does nothing and earlier code is unreachable
    /// (the "cannot page up" report). When the wheel keeps turning against an edge, load one
    /// more page of instructions above/below and shift the scroll offset by the pixel height of
    /// the rows that were added/removed, so the text on screen does not move.
    fn follow_disasm_scroll(
        &mut self,
        ui: &egui::Ui,
        scroll: &egui::scroll_area::ScrollAreaOutput<()>,
    ) {
        self.disasm_offset = scroll.state.offset.y;
        let wheel = ui.input(|input| input.smooth_scroll_delta().y);
        if wheel == 0.0 || !ui.rect_contains_pointer(scroll.inner_rect) {
            return;
        }
        let margin = self.row_height() * DISASM_SCROLL_MARGIN_ROWS;
        let at_top = scroll.state.offset.y <= margin;
        let at_bottom =
            scroll.state.offset.y + scroll.inner_rect.height() + margin >= scroll.content_size.y;
        let direction = if wheel > 0.0 && at_top {
            -1
        } else if wheel < 0.0 && at_bottom {
            1
        } else {
            return;
        };
        if let Some(offset) = self.extend_disasm_window(ui, direction) {
            self.disasm_scroll_to = Some(offset);
            ui.ctx().request_repaint();
        }
    }

    /// Load one page of rows above (`-1`) or below (`+1`) the window and return the scroll
    /// offset that keeps the visible rows in place.
    fn extend_disasm_window(&mut self, ui: &egui::Ui, direction: i32) -> Option<f32> {
        let anchor = self.disasm_start;
        let target = self.step_listing(direction * DISASM_SCROLL_ROWS as i32)?;
        if target == anchor {
            // Already at the start/end of the image.
            return None;
        }
        let offset = self.disasm_offset;
        let cursor = self.disasm_rows.get(self.selected).map(|line| line.addr);
        let removed_px: f32 = if direction > 0 {
            self.disasm_rows
                .iter()
                .take_while(|line| line.addr < target)
                .map(|line| self.disasm_row_pitch(ui, line))
                .sum()
        } else {
            0.0
        };
        self.disasm_start = target;
        self.refresh_disasm();
        let added_px: f32 = if direction < 0 {
            self.disasm_rows
                .iter()
                .take_while(|line| line.addr < anchor)
                .map(|line| self.disasm_row_pitch(ui, line))
                .sum()
        } else {
            0.0
        };
        // Keep the cursor on the same instruction (the rows above it changed index).
        if let Some(addr) = cursor
            && let Some(index) = self.disasm_rows.iter().position(|line| line.addr == addr)
        {
            self.selected = index;
        }
        let offset = if direction < 0 {
            offset + added_px
        } else {
            (offset - removed_px).max(0.0)
        };
        self.disasm_offset = offset;
        Some(offset)
    }

    /// IDA-style navigation band: segment map of the image with a marker for the current
    /// disassembly position; clicking jumps proportionally.
    /// IDA-style navigation band: segment background, function-density overlay, external
    /// symbol ticks, current-position marker and a hover tooltip. Colours match the legend.
    fn navigation_band(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let pal = self.palette();
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let segments = session.segments();
        if segments.is_empty() {
            return;
        }
        let base = segments
            .iter()
            .map(|segment| segment.range.start.get())
            .min()
            .unwrap_or(0);
        let end = segments
            .iter()
            .map(|segment| segment.range.end.get())
            .max()
            .unwrap_or(base + 1)
            .max(base + 1);
        let span = (end - base) as f32;
        let width = ui.available_width().max(120.0);
        let (rect, mut response) = ui.allocate_exact_size(egui::vec2(width, 12.0), Sense::click());
        let painter = ui.painter();
        painter.rect_filled(rect, 2.0, pal.view);
        let x_of = |addr: u64| {
            rect.left()
                + ((addr.saturating_sub(base) as f32 / span) * rect.width())
                    .clamp(0.0, rect.width())
        };
        // Segment background: executable = instructions, readable = data, rest unexplored.
        for segment in segments {
            let color = if segment.perm.is_executable() {
                NAV_INSTRUCTION
            } else if segment.perm.is_readable() {
                NAV_DATA
            } else {
                NAV_UNEXPLORED
            };
            let x0 = x_of(segment.range.start.get());
            let x1 = x_of(segment.range.end.get()).max(x0 + 1.0);
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(x0, rect.top()),
                    egui::pos2(x1.min(rect.right()), rect.bottom()),
                ),
                0.0,
                color,
            );
        }
        // Function density: upper strip per discovered function.
        let function_bottom = rect.top() + rect.height() * 0.55;
        for function in &self.functions {
            let x0 = x_of(function.start.get());
            let x1 = x_of(function.end.get()).max(x0 + 1.0);
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(x0, rect.top()),
                    egui::pos2(x1.min(rect.right()), function_bottom),
                ),
                0.0,
                NAV_FUNCTION,
            );
        }
        // External symbols (imports/IAT) as thin magenta ticks.
        for import in &self.imports {
            let x = x_of(import.iat.get());
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(x - 0.5, rect.top()),
                    egui::pos2(x + 0.5, rect.bottom()),
                ),
                0.0,
                NAV_EXTERNAL,
            );
        }
        let current = self.disasm_start.get().clamp(base, end);
        let marker = rect.left()
            + (((current.saturating_sub(base)) as f32 / span) * rect.width())
                .clamp(0.0, rect.width());
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(marker - 1.0, rect.top()),
                egui::pos2(marker + 1.0, rect.bottom()),
            ),
            0.0,
            pal.addr,
        );
        if let Some(position) = response.hover_pos() {
            let fraction = ((position.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
            let addr = Address::new(base + (fraction * span as f64) as u64);
            let segment = session
                .segment_containing(addr)
                .map(|segment| segment.name.as_str())
                .unwrap_or("?");
            let function = self
                .functions
                .iter()
                .enumerate()
                .find(|(_, function)| addr >= function.start && addr < function.end)
                .map(|(index, function)| {
                    self.function_labels
                        .get(index)
                        .filter(|name| !name.is_empty())
                        .cloned()
                        .unwrap_or_else(|| format!("sub_{:x}", function.start.get()))
                });
            let label = match function {
                Some(name) => format!("{:#x} · {segment} · {name}", addr.get()),
                None => format!("{:#x} · {segment}", addr.get()),
            };
            response = response.on_hover_text(label);
        }
        if response.clicked()
            && let Some(position) = response.interact_pointer_pos()
        {
            let fraction = ((position.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
            *action = Action::SyncDisasm(Address::new(base + (fraction * span as f64) as u64));
        }
    }

    /// Geometry of one pseudo-C row: the line number stays at the viewport's left edge,
    /// while the code text follows the horizontal scroll offset in `row.left()`.
    fn pseudo_row_geometry(
        viewport: egui::Rect,
        row: egui::Rect,
        gutter: f32,
    ) -> (egui::Pos2, egui::Pos2, egui::Rect) {
        let number_pos = egui::pos2(viewport.left() + gutter - 6.0, row.top() + 1.0);
        let text_pos = egui::pos2(row.left() + gutter + 4.0, row.top() + 1.0);
        let text_clip = egui::Rect::from_min_max(
            egui::pos2((viewport.left() + gutter).min(viewport.right()), row.top()),
            egui::pos2(viewport.right(), row.bottom()),
        );
        (number_pos, text_pos, text_clip)
    }

    /// Row to paint as the current pseudo-C line. The clicked row wins while it still
    /// belongs to the disassembly address; otherwise the first line at that address wins,
    /// so address-keyed groups never light up as a multi-row selection.
    fn active_pseudo_line(&self, lines: &[PseudoLine]) -> Option<usize> {
        let synced = self.disasm_rows.get(self.selected).map(|line| line.addr);
        self.pseudo_selected_line
            .filter(|&index| {
                lines
                    .get(index)
                    .is_some_and(|line| synced == Some(line.addr))
            })
            .or_else(|| lines.iter().position(|line| synced == Some(line.addr)))
    }

    fn pseudo_view(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        self.tab = Tab::Pseudo;
        // Nothing has been decompiled yet *and* there is no project to decompile: "no
        // decompilation for this address" would be the wrong explanation.
        if self.session.is_none() && self.pseudo.is_none() {
            ui.weak(self.t("ui_open_a_project_first"));
            return;
        }
        let pal = self.palette();
        let mono = FontId::monospace(self.mono_size());
        ui.horizontal(|ui| {
            ui.weak(self.t("ui_pseudo_click_hint"));
        });
        if self.pseudo_dirty {
            return;
        }
        let Some(lines) = self.pseudo.as_ref() else {
            ui.weak(self.t("ui_no_decompilation_for_this_address"));
            return;
        };
        let active_line = self.active_pseudo_line(lines);
        // Function whose body this listing shows: its own name is not a jump target.
        let current_function = self.pseudo_function_start();
        // F5 asks for the cursor's line to be centred. Row indices belong to one listing, so
        // the pending address is resolved here, once the rows it names exist.
        let scroll_line = self
            .pseudo_scroll_to
            .and_then(|addr| lines.iter().position(|line| line.addr == addr));
        // Fixed line-number gutter. The listing scrolls horizontally underneath the
        // gutter (the text is clipped at its right edge), while the numbers stay put.
        let digits = lines.len().max(1).to_string().len().max(2);
        let number_width = ui
            .painter()
            .layout_no_wrap("0".repeat(digits), mono.clone(), pal.line_number)
            .size()
            .x;
        let gutter = number_width + 12.0;
        let mut clicked = None;
        let mut double_clicked = None;
        let mut took_focus = false;
        let copy_menu = self.t("ui_copy");
        let comment_menu = self.t("ui_edit_pseudo_comment");
        let rename_var_menu = self.t("ui_rename_variable");
        let ai_var_menu = self.t("ui_var_suggest");
        // Editor-style text selection over the pseudo-C rows (the gutter stays out of it).
        let mut text_sel = self.pseudo_sel;
        let mut dragging = self.pseudo_drag;
        let drag_pointer = ui.input(|input| input.pointer.latest_pos());
        let primary_down = ui.input(|input| input.pointer.primary_down());
        let extend_selection = ui.input(|input| input.modifiers.shift);
        let mut first_top: Option<f32> = None;
        let mut last_bottom: Option<f32> = None;
        let width = gutter + ui.available_width().max(1100.0);
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let viewport = ui.clip_rect();
                ui.spacing_mut().item_spacing.y = 1.0;
                for (index, line) in lines.iter().enumerate() {
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(width, self.row_height()),
                        Sense::click_and_drag(),
                    );
                    first_top.get_or_insert(rect.top());
                    last_bottom = Some(rect.bottom());
                    if scroll_line == Some(index) {
                        // Centre the row vertically only. `scroll_to_rect` centres both axes,
                        // so the target spans the visible width: a row wider than the pane
                        // would otherwise pan the listing (and its line numbers) sideways.
                        ui.scroll_to_rect(
                            egui::Rect::from_min_max(
                                egui::pos2(viewport.left(), rect.top()),
                                egui::pos2(viewport.right(), rect.bottom()),
                            ),
                            Some(egui::Align::Center),
                        );
                    }
                    if rect.bottom() < viewport.top() || rect.top() > viewport.bottom() {
                        continue;
                    }
                    // The row highlight spans the whole visible row, gutter included.
                    let row_rect = egui::Rect::from_min_max(
                        egui::pos2(viewport.left(), rect.top()),
                        egui::pos2(viewport.right(), rect.bottom()),
                    );
                    if active_line == Some(index) {
                        ui.painter().rect_filled(row_rect, 1.0, pal.selected_row);
                    } else if response.hovered() {
                        ui.painter().rect_filled(row_rect, 1.0, pal.hover_row);
                    }
                    let (number_pos, text_pos, text_clip) =
                        Self::pseudo_row_geometry(viewport, rect, gutter);
                    ui.painter().text(
                        number_pos,
                        egui::Align2::RIGHT_TOP,
                        (index + 1).to_string(),
                        mono.clone(),
                        pal.line_number,
                    );
                    let mut job = LayoutJob::default();
                    append_pseudo_line(&mut job, &line.text, pal, &mono);
                    let galley = ui.painter().layout_job(job);
                    if let Some(selection) = text_sel
                        && let Some(range) =
                            selection.row_range(index, galley.text().chars().count())
                    {
                        paint_text_selection(
                            ui.painter(),
                            pal.text_selection,
                            &galley,
                            text_pos,
                            range,
                        );
                    }
                    ui.painter().with_clip_rect(text_clip).galley(
                        text_pos,
                        galley.clone(),
                        pal.text,
                    );
                    // A press anchors a text selection, a drag extends it across rows and a
                    // plain click collapses it to a caret (Shift extends the last one).
                    if response.drag_started()
                        && let Some(pos) = ui.input(|input| input.pointer.press_origin())
                    {
                        let point = TextPoint {
                            row: index,
                            char: galley_char_at(&galley, text_pos, pos),
                        };
                        text_sel = Some(TextSelection {
                            anchor: point,
                            focus: point,
                        });
                        dragging = true;
                        took_focus = true;
                    }
                    if dragging
                        && primary_down
                        && let Some(pos) = drag_pointer
                        && pos.y >= rect.top()
                        && pos.y <= rect.bottom()
                    {
                        let point = TextPoint {
                            row: index,
                            char: galley_char_at(&galley, text_pos, pos),
                        };
                        if let Some(selection) = text_sel.as_mut() {
                            selection.focus = point;
                        }
                    }
                    if response.clicked()
                        && let Some(pos) = response.interact_pointer_pos()
                    {
                        let point = TextPoint {
                            row: index,
                            char: galley_char_at(&galley, text_pos, pos),
                        };
                        text_sel = Some(match text_sel {
                            Some(selection) if extend_selection => TextSelection {
                                focus: point,
                                ..selection
                            },
                            _ => TextSelection::collapsed(point),
                        });
                        took_focus = true;
                    }
                    // Symbol under the pointer, so callee names can advertise the
                    // double-click that opens their body. A word inside a string literal
                    // is data, not a call, so it keeps the plain address tooltip.
                    let pointer = response
                        .interact_pointer_pos()
                        .or_else(|| response.hover_pos());
                    let char_index = pointer.and_then(|pointer| {
                        Self::pseudo_char_index_at(pointer, text_pos, text_clip, &galley)
                    });
                    let in_literal = char_index.is_some_and(|char_index| {
                        pseudo_string_literal_at(&line.text, char_index).is_some()
                    });
                    // A trailing comment is annotation, not C: a name or a quoted string
                    // inside it must not become a jump target. Comment-only rows (orphan
                    // comments, decompiler-emitted `//` lines) are covered by the prefix
                    // check; a trailing comment starts at the recorded column.
                    let comment_start = self.pseudo_comment_cols.get(index).copied().flatten();
                    let in_comment = line.text.trim_start().starts_with("//")
                        || comment_start.is_some_and(|start| {
                            char_index.is_some_and(|char_index| char_index >= start)
                        });
                    let symbol = if in_literal || in_comment {
                        None
                    } else {
                        char_index.and_then(|char_index| pseudo_symbol_at(&line.text, char_index))
                    };
                    let callee = symbol.and_then(|symbol| self.pseudo_function_named(symbol));
                    let response = match (callee, symbol) {
                        (Some(start), Some(name)) if Some(start) != current_function => {
                            let addr = format!("{:#x}", start.get());
                            response
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .on_hover_text(self.t_args(
                                    "ui_double_click_to_enter_function",
                                    &[("name", name), ("addr", &addr)],
                                ))
                        }
                        _ if response.hovered() => {
                            response.on_hover_text(format!("{:#x}", line.addr.get()))
                        }
                        _ => response,
                    };
                    // Shift+click only extends the text selection; syncing the row would
                    // re-anchor the listing and drop it again.
                    if response.clicked() && !extend_selection {
                        clicked = Some((index, line.addr));
                    }
                    if response.secondary_clicked() {
                        took_focus = true;
                    }
                    if response.double_clicked() {
                        double_clicked = Some(if in_comment {
                            Action::NavigateToDisasm(line.addr)
                        } else {
                            match char_index {
                                Some(char_index) => {
                                    self.pseudo_double_click(&line.text, char_index, line.addr)
                                }
                                None => Action::NavigateToDisasm(line.addr),
                            }
                        });
                    }
                    let _ = response.context_menu(|ui| {
                        if ui.button(copy_menu.as_str()).clicked() {
                            *action = Action::Copy;
                            ui.close();
                        }
                        if ui.button(comment_menu.as_str()).clicked() {
                            *action = Action::EditPseudoComment {
                                addr: Some(line.addr),
                            };
                            ui.close();
                        }
                        if let Some((offset, _)) = stack_slot_token(&line.text)
                            && let Some(func) = current_function
                            && ui.button(rename_var_menu.as_str()).clicked()
                        {
                            *action = Action::RenameVar { func, offset };
                            ui.close();
                        }
                        if let Some(func) = current_function
                            && ui.button(ai_var_menu.as_str()).clicked()
                        {
                            *action = Action::AiRenameVars { func };
                            ui.close();
                        }
                    });
                }
            });
        // Dragging above or below the listing keeps selecting to that edge.
        if dragging
            && primary_down
            && let Some(pos) = drag_pointer
        {
            if first_top.is_some_and(|top| pos.y < top) {
                if let Some(selection) = text_sel.as_mut() {
                    selection.focus = TextPoint { row: 0, char: 0 };
                }
            } else if last_bottom.is_some_and(|bottom| pos.y > bottom)
                && let Some(last) = lines.len().checked_sub(1)
            {
                let end = lines[last].text.chars().count();
                if let Some(selection) = text_sel.as_mut() {
                    selection.focus = TextPoint {
                        row: last,
                        char: end,
                    };
                }
            }
        }
        if !primary_down {
            dragging = false;
        }
        self.pseudo_sel = text_sel;
        self.pseudo_drag = dragging;
        if took_focus {
            self.copy_focus = CopyFocus::Pseudo;
        }
        // Single click selects that exact row and syncs the disassembly cursor without
        // leaving pseudo-C; double-click opens a callee's body, and otherwise jumps into
        // the disassembly tab.
        if let Some((line, addr)) = clicked {
            *action = Action::SyncPseudoLine { addr, line };
        }
        if let Some(double_click) = double_clicked {
            *action = double_click;
        }
        // The scroll request is one-shot: a later re-render must not yank the view back.
        self.pseudo_scroll_to = None;
    }

    // ------------------------------------------------------------- windows --

    /// IDA-style stack-variable rename. Empty text restores the generated `var_<offset>`
    /// name; anything else is a C identifier stored as user data and applied by the L2
    /// renderer (`var_20` → `input`, `var_20.field` → `input.field`).
    fn var_rename_window(&mut self, ctx: &egui::Context) {
        let Some((func, offset, mut text)) = self.var_edit.take() else {
            return;
        };
        let slot = if offset < 0 {
            format!("var_n{:x}", -offset)
        } else {
            format!("var_{offset:x}")
        };
        let mut commit = false;
        let mut cancel = false;
        let title = self.t("ui_rename_variable");
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("{:#x}  {slot}", func.get()));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .desired_width(320.0)
                        .hint_text(self.t("ui_empty_deletes")),
                );
                ui.horizontal(|ui| {
                    if ui.button(self.t("ui_ok")).clicked() {
                        commit = true;
                    }
                    if ui.button(self.t("ui_cancel")).clicked() {
                        cancel = true;
                    }
                });
                if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                    commit = true;
                }
            });
        if cancel {
            return;
        }
        if commit {
            let name = text.trim();
            let Some(session) = self.session.as_mut() else {
                return;
            };
            match session.execute(Command::RenameVar {
                func,
                offset,
                name: (!name.is_empty()).then(|| name.to_owned()),
            }) {
                Ok(_) => {
                    self.pseudo_dirty = true;
                    self.set_status(self.t_args(
                        "ui_changed_addr",
                        &[("addr", &format!("{:#x}", func.get()))],
                    ));
                }
                Err(error) => self.set_error(error),
            }
        } else {
            self.var_edit = Some((func, offset, text));
        }
    }

    fn edit_window(&mut self, ctx: &egui::Context, _action: &mut Action) {
        let Some(mut edit) = self.edit.take() else {
            return;
        };
        let mut commit = false;
        let mut cancel = false;
        let title = match edit.kind {
            EditKind::Rename => self.t("ui_rename_2").to_owned(),
            EditKind::Comment => self.t("ui_comment_2").to_owned(),
            EditKind::PseudoComment => self.t("ui_pseudo_comment").to_owned(),
            EditKind::Prototype => self.t("ui_prototype").to_owned(),
            EditKind::StringText => self.t("ui_edit_string").to_owned(),
        };
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("{:#x}", edit.addr.get()));
                // A pseudocode comment may span several lines; the disassembly comment
                // stays single-line because it is painted on one listing row. A string
                // literal may contain newlines too, so it gets the same multi-line field.
                let text_edit = match edit.kind {
                    EditKind::PseudoComment | EditKind::StringText => {
                        egui::TextEdit::multiline(&mut edit.text)
                            .desired_width(420.0)
                            .desired_rows(3)
                    }
                    _ => egui::TextEdit::singleline(&mut edit.text).desired_width(360.0),
                };
                let response = ui.add(text_edit.hint_text(self.t("ui_empty_deletes")));
                if edit.just_opened {
                    response.request_focus();
                    edit.just_opened = false;
                }
                ui.horizontal(|ui| {
                    if ui.button(self.t("ui_ok")).clicked() {
                        commit = true;
                    }
                    if ui.button(self.t("ui_cancel")).clicked() {
                        cancel = true;
                    }
                });
                // A multi-line field consumes Enter, so it commits with Ctrl+Enter instead.
                let submit = match edit.kind {
                    EditKind::PseudoComment | EditKind::StringText => {
                        ui.input(|input| input.modifiers.command && input.key_pressed(Key::Enter))
                    }
                    _ => response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)),
                };
                if submit {
                    commit = true;
                }
            });
        if commit {
            self.run_edit(edit);
        } else if cancel {
            self.edit = None;
        } else {
            self.edit = Some(edit);
        }
    }

    /// Command palette (Ctrl+P / F1): fuzzy-filtered list of UI commands.
    fn palette_window(&mut self, ctx: &egui::Context, action: &mut Action) {
        let Some(mut state) = self.palette.take() else {
            return;
        };
        let lang = self.lang;
        let query = state.query.to_lowercase();
        let filtered: Vec<usize> = PALETTE_COMMANDS
            .iter()
            .enumerate()
            .filter(|(_, (key, _))| {
                let en = self.i18n.text(Lang::En, key).to_lowercase();
                let zh = self.i18n.text(Lang::Zh, key);
                query.is_empty() || en.contains(&query) || zh.contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
        let labels: Vec<(usize, String)> = filtered
            .iter()
            .map(|&index| (index, self.i18n.text(lang, PALETTE_COMMANDS[index].0)))
            .collect();
        let title = self.i18n.text(lang, "ui_command_palette");
        let hint = self.i18n.text(lang, "ui_type_command");
        state.selected = state.selected.min(labels.len().saturating_sub(1));
        let mut chosen: Option<usize> = None;
        let mut open = true;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .open(&mut open)
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.query)
                        .desired_width(400.0)
                        .hint_text(hint),
                );
                if state.query.is_empty() && !response.has_focus() {
                    response.request_focus();
                }
                if ui.input(|input| input.key_pressed(Key::ArrowDown)) {
                    state.selected = (state.selected + 1).min(labels.len().saturating_sub(1));
                }
                if ui.input(|input| input.key_pressed(Key::ArrowUp)) {
                    state.selected = state.selected.saturating_sub(1);
                }
                if ui.input(|input| input.key_pressed(Key::Enter))
                    && let Some(&(index, _)) = labels.get(state.selected)
                {
                    chosen = Some(index);
                }
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for (position, (index, label)) in labels.iter().enumerate() {
                            if ui
                                .selectable_label(position == state.selected, label)
                                .clicked()
                            {
                                chosen = Some(*index);
                            }
                        }
                    });
            });
        if let Some(index) = chosen {
            *action = Action::PaletteCommand(index);
        } else if open {
            self.palette = Some(state);
        }
    }

    fn goto_window(&mut self, ctx: &egui::Context, action: &mut Action) {
        let Some(mut buffer) = self.goto.take() else {
            return;
        };
        let mut submit = None;
        let mut open = true;
        egui::Window::new(self.t("ui_go_to_address_symbol"))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut buffer)
                        .desired_width(320.0)
                        .hint_text("0x140001000, 4198400, start"),
                );
                if ui.input(|input| input.key_pressed(Key::Enter)) {
                    submit = Some(buffer.clone());
                }
                ui.horizontal(|ui| {
                    if ui.button(self.t("ui_go")).clicked() {
                        submit = Some(buffer.clone());
                    }
                    if ui.button(self.t("ui_cancel")).clicked() {
                        open = false;
                    }
                });
                let _ = response;
            });
        if let Some(raw) = submit {
            if let Some(session) = self.session.as_ref() {
                match resolve_address(session, &raw) {
                    Ok(addr) => {
                        *action = Action::Navigate(addr);
                        self.goto = None;
                        return;
                    }
                    Err(error) => self.set_error(error),
                }
            }
        }
        if open {
            self.goto = Some(buffer);
        } else {
            self.goto = None;
        }
    }

    /// IDA-style *Setup…* for the Strings window: the minimum length is the option that
    /// controls the list, and `Rebuild` applies it (IDA likewise rebuilds on demand).
    fn strings_setup_window(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.strings_setup else {
            return;
        };
        let mut rebuild = false;
        let mut open = true;
        egui::Window::new(self.t("ui_strings_setup_title"))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(self.t("ui_min_string_length"));
                    ui.add(
                        egui::DragValue::new(&mut draft)
                            .range(
                                recraft_analysis::MIN_STRING_LEN_FLOOR
                                    ..=recraft_analysis::MIN_STRING_LEN_CEILING,
                            )
                            .speed(0.2),
                    );
                });
                ui.weak(self.t("ui_min_string_length_hint"));
                ui.horizontal(|ui| {
                    if ui.button(self.t("ui_rebuild")).clicked() {
                        rebuild = true;
                    }
                    if ui.button(self.t("ui_cancel")).clicked() {
                        open = false;
                    }
                });
            });
        if rebuild {
            self.start_strings_job(recraft_analysis::clamp_min_string_len(draft));
            self.strings_setup = None;
        } else {
            self.strings_setup = open.then_some(draft);
        }
    }

    /// IDA's dialog title: `xrefs to <name>`, falling back to the bare address.
    fn xrefs_title(&self, target: Address) -> String {
        let name = self
            .session
            .as_ref()
            .and_then(|session| session.name_at(target).ok().flatten())
            .or_else(|| {
                string_literal_at(&self.strings, target).map(|text| quote_literal(&text, 40))
            });
        match name {
            Some(name) => format!("{} {name} ({:#x})", self.t("ui_xrefs_to"), target.get()),
            None => format!("{} {:#x}", self.t("ui_xrefs_to"), target.get()),
        }
    }

    /// IDA-style `xrefs to …` dialog (`X`): a snapshot of the references around the address
    /// it was opened on. `OK`/Enter follows the selected row, `Cancel`/Esc closes,
    /// `Search` filters the columns and `Help` explains the type letters.
    fn xrefs_window(&mut self, ctx: &egui::Context, action: &mut Action) {
        let Some(mut dialog) = self.xref_dialog.take() else {
            return;
        };
        let pal = self.palette();
        let mono = FontId::monospace(self.mono_size());
        let scale = self.mono_size() / MONO_SIZE;
        let row_h = self.row_height().max(16.0);
        let dir_w = 72.0 * scale;
        let type_w = 40.0 * scale;
        let addr_w = 300.0 * scale;
        let text_x = dir_w + type_w + addr_w;
        let title = self.xrefs_title(dialog.target);
        let up_label = self.t("ui_up");
        let down_label = self.t("ui_down");
        let dir_head = self.t("ui_direction");
        let type_head = self.t("ui_type");
        let addr_head = self.t("ui_address");
        let text_head = self.t("ui_text");
        let ok_label = self.t("ui_ok");
        let cancel_label = self.t("ui_cancel");
        let search_label = self.t("ui_search");
        let help_label = self.t("ui_help");
        let no_refs = self.t("ui_no_references");
        let no_match = self.t("ui_no_match");
        let search_hint = self.t("ui_xrefs_search_hint");
        let help_text = self.t("ui_xrefs_help");
        let mut open = true;
        let mut close = false;
        let mut follow = None;
        let mut copied = false;
        // IDA's dialog opens centered; a user-moved position is remembered per id.
        let default_size = egui::vec2(760.0, 380.0);
        let default_pos = ctx.content_rect().center() - default_size / 2.0;
        egui::Window::new(title)
            .id(egui::Id::new("recraft_xrefs_dialog"))
            .collapsible(false)
            .resizable(true)
            .default_pos(default_pos)
            .default_size(default_size)
            .min_width(480.0)
            .open(&mut open)
            .show(ctx, |ui| {
                let mut typing = false;
                if dialog.search {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut dialog.filter)
                            .desired_width(320.0)
                            .hint_text(search_hint.as_str()),
                    );
                    if dialog.search_focus {
                        response.request_focus();
                        dialog.search_focus = false;
                    }
                    typing = response.has_focus();
                }
                let visible = xref_matches(&dialog.rows, &dialog.filter);
                if let Some(&first) = visible.first()
                    && !visible.contains(&dialog.selected)
                {
                    dialog.selected = first;
                }
                // Ctrl+C copies the selected row while the search box does not own the keys.
                if !typing
                    && ui.input(|input| {
                        input
                            .events
                            .iter()
                            .any(|event| matches!(event, egui::Event::Copy))
                    })
                    && let Some(row) = dialog.rows.get(dialog.selected)
                {
                    ctx.copy_text(xref_row_text(row, up_label.as_str(), down_label.as_str()));
                    copied = true;
                }
                // IDA's keys: arrows move the cursor, Enter is OK, Esc is Cancel. The
                // search box owns the arrows/Enter while it is being typed into.
                let mut move_by = 0i32;
                let mut scroll_to_selected = false;
                if !typing {
                    if ui.input(|input| input.key_pressed(Key::ArrowDown)) {
                        move_by = 1;
                    }
                    if ui.input(|input| input.key_pressed(Key::ArrowUp)) {
                        move_by = -1;
                    }
                    if ui.input(|input| input.key_pressed(Key::Enter))
                        && let Some(row) = dialog.rows.get(dialog.selected)
                    {
                        follow = Some(row.addr);
                    }
                }
                if ui.input(|input| input.key_pressed(Key::Escape)) {
                    close = true;
                }
                if move_by != 0
                    && !visible.is_empty()
                    && let Some(position) =
                        visible.iter().position(|index| *index == dialog.selected)
                {
                    let next = (position as i32 + move_by).clamp(0, visible.len() as i32 - 1);
                    dialog.selected = visible[next as usize];
                    scroll_to_selected = true;
                }
                let width = ui.available_width().max(420.0);
                let char_w = ui
                    .painter()
                    .layout_no_wrap("0".to_owned(), mono.clone(), pal.text)
                    .size()
                    .x
                    .max(1.0);
                let (header, _) = ui.allocate_exact_size(egui::vec2(width, row_h), Sense::hover());
                let painter = ui.painter_at(header);
                for (x, text) in [
                    (4.0, dir_head.as_str()),
                    (dir_w, type_head.as_str()),
                    (dir_w + type_w, addr_head.as_str()),
                    (text_x, text_head.as_str()),
                ] {
                    painter.text(
                        egui::pos2(header.left() + x, header.center().y),
                        egui::Align2::LEFT_CENTER,
                        text,
                        mono.clone(),
                        pal.bytes,
                    );
                }
                ui.separator();
                if visible.is_empty() {
                    // An empty table and a filtered-away table read differently, as in IDA.
                    ui.weak(if dialog.rows.is_empty() {
                        no_refs.as_str()
                    } else {
                        no_match.as_str()
                    });
                }
                let text_chars = (((width - text_x) / char_w).floor() as usize).max(16);
                let label_chars = (((addr_w - 8.0) / char_w).floor() as usize).max(8);
                // Fill the dialog: the list scrolls while the button row stays pinned below.
                let list_height = (ui.available_height() - 36.0).max(80.0);
                let mut clicked_row = None;
                egui::ScrollArea::vertical()
                    .id_salt("recraft_xrefs_rows")
                    .auto_shrink([false, false])
                    .max_height(list_height)
                    .show(ui, |ui| {
                        for &index in &visible {
                            let row = &dialog.rows[index];
                            let (rect, response) =
                                ui.allocate_exact_size(egui::vec2(width, row_h), Sense::click());
                            if index == dialog.selected {
                                ui.painter().rect_filled(rect, 1.0, pal.selected_row);
                                if scroll_to_selected {
                                    ui.scroll_to_rect(rect, Some(egui::Align::Center));
                                }
                            } else if response.hovered() {
                                ui.painter().rect_filled(rect, 1.0, pal.hover_row);
                            }
                            let painter = ui.painter_at(rect);
                            let y = rect.center().y;
                            painter.text(
                                egui::pos2(rect.left() + 4.0, y),
                                egui::Align2::LEFT_CENTER,
                                if row.up {
                                    up_label.as_str()
                                } else {
                                    down_label.as_str()
                                },
                                mono.clone(),
                                pal.line_number,
                            );
                            painter.text(
                                egui::pos2(rect.left() + dir_w, y),
                                egui::Align2::LEFT_CENTER,
                                row.kind.to_string(),
                                mono.clone(),
                                pal.mnemonic,
                            );
                            painter.text(
                                egui::pos2(rect.left() + dir_w + type_w, y),
                                egui::Align2::LEFT_CENTER,
                                elide_chars(&row.label, label_chars),
                                mono.clone(),
                                pal.addr,
                            );
                            let mut job = LayoutJob::default();
                            append_operands(
                                &mut job,
                                &elide_chars(&row.text, text_chars),
                                pal,
                                &mono,
                            );
                            let galley = painter.layout_job(job);
                            painter.galley(
                                egui::pos2(rect.left() + text_x, y - galley.size().y * 0.5),
                                galley,
                                pal.text,
                            );
                            let response = response.on_hover_text(format!(
                                "{} {}\n{}\n{}",
                                if row.up {
                                    up_label.as_str()
                                } else {
                                    down_label.as_str()
                                },
                                row.kind_name,
                                row.label,
                                row.text,
                            ));
                            if response.clicked() {
                                clicked_row = Some(index);
                            }
                            if response.double_clicked() {
                                clicked_row = Some(index);
                                follow = Some(row.addr);
                            }
                        }
                    });
                if let Some(index) = clicked_row {
                    dialog.selected = index;
                }
                ui.separator();
                let total = visible.len();
                let line_no = visible
                    .iter()
                    .position(|index| *index == dialog.selected)
                    .map_or(0, |position| position + 1);
                let status = self.t_args(
                    "ui_line_n_of_m",
                    &[
                        ("line", &line_no.to_string()),
                        ("total", &total.to_string()),
                    ],
                );
                ui.horizontal(|ui| {
                    // IDA's status line: `Line N of M`; nothing to number when empty.
                    ui.weak(if total == 0 { "" } else { status.as_str() });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(help_label.as_str()).clicked() {
                            dialog.help = !dialog.help;
                        }
                        if ui.button(search_label.as_str()).clicked() {
                            dialog.search = !dialog.search;
                            dialog.search_focus = dialog.search;
                            if !dialog.search {
                                dialog.filter.clear();
                            }
                        }
                        if ui.button(cancel_label.as_str()).clicked() {
                            close = true;
                        }
                        if ui.button(ok_label.as_str()).clicked()
                            && let Some(row) = dialog.rows.get(dialog.selected)
                        {
                            follow = Some(row.addr);
                        }
                    });
                });
                if dialog.help {
                    ui.weak(help_text.as_str());
                }
            });
        if let Some(addr) = follow {
            *action = Action::NavigateToDisasm(addr);
            self.xref_dialog = None;
        } else if open && !close {
            self.xref_dialog = Some(dialog);
        } else {
            self.xref_dialog = None;
        }
        if copied {
            self.set_status(self.t_args("ui_copied_lines", &[("count", "1")]));
        }
    }

    // ------------------------------------------------------------- actions --

    /// Take Tab away from egui's own widget-focus navigation.
    ///
    /// egui 0.36 queues a focus move in `begin_pass` for Tab (= next) and Shift+Tab (= the
    /// previous widget); the first focusable widget that renders claims it, which is the menu
    /// bar here. The app binds Tab instead, so the queued move is cleared at the top of the
    /// frame, before any widget can take it. Text fields keep their focus, and a `Tab` inside
    /// them is still delivered to the field itself.
    fn claim_tab_from_focus_navigation(&self, ctx: &egui::Context) {
        if ctx.input(|input| input.key_pressed(Key::Tab)) {
            ctx.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
        }
    }

    fn handle_keys(&mut self, ctx: &egui::Context) -> Action {
        // egui may run several layout passes for one frame (`request_discard`); the same key
        // event is visible in each pass, so only the first pass may act on it or shortcuts
        // (paging, undo, ...) fire twice per key press.
        let pass = ctx.cumulative_pass_nr();
        if self.key_pass == Some(pass) {
            return Action::None;
        }
        self.key_pass = Some(pass);
        if self.palette.is_some() {
            if ctx.input(|input| input.key_pressed(Key::Escape)) {
                return Action::TogglePalette;
            }
            // The palette's text field owns the keyboard while it is open.
            return Action::None;
        }
        if self.xref_dialog.is_some() {
            // The xrefs dialog handles its own keys (arrows/Enter/Esc); nothing behind it
            // may react to them.
            return Action::None;
        }
        if self.edit.is_some() || self.goto.is_some() {
            return Action::None;
        }
        // A focused text field owns the keyboard, so typing there cannot trigger shortcuts.
        // Tab is the exception: egui's focus ring is cancelled for it (see
        // [`Self::claim_tab_from_focus_navigation`]), so a Tab pressed while a button or a
        // list has the focus still has to flip the views.
        let tab = ctx.input(|input| input.key_pressed(Key::Tab));
        if ctx.egui_wants_keyboard_input() && !(tab && !ctx.text_edit_focused()) {
            return Action::None;
        }
        // The desktop integration turns Ctrl+C into `Event::Copy`. A selected label (the AI
        // panel, the hint lines) owns it through egui's own copy, so the panes only claim the
        // event when no label text is selected.
        let label_selection = ctx
            .plugin_opt::<egui::text_selection::LabelSelectionState>()
            .is_some_and(|plugin| plugin.lock().has_selection());
        if !label_selection
            && ctx.input(|input| {
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Copy))
            })
        {
            return Action::Copy;
        }
        // Ctrl+A selects the focused pane's whole text. The chord is read off the key event
        // itself, so it also works when the platform does not resend the modifier state.
        let select_all = ctx.input(|input| {
            input.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::Key {
                        key: Key::A,
                        pressed: true,
                        modifiers,
                        ..
                    } if modifiers.command
                )
            })
        });
        if select_all {
            return Action::SelectAll;
        }
        let mut action = Action::None;
        ctx.input(|input| {
            if (input.modifiers.command && input.key_pressed(Key::P)) || input.key_pressed(Key::F1)
            {
                action = Action::TogglePalette;
            }
            if input.modifiers.command && input.key_pressed(Key::Z) {
                action = Action::Undo;
            }
            if input.modifiers.command && input.key_pressed(Key::Y) {
                action = Action::Redo;
            }
            if input.modifiers.command && input.key_pressed(Key::O) {
                action = Action::Open;
            }
            if input.modifiers.command && input.key_pressed(Key::W) {
                action = Action::CloseProject;
            }
            if input.modifiers.command && input.key_pressed(Key::T) {
                action = Action::ToggleTheme;
            }
            if input.modifiers.command
                && (input.key_pressed(Key::Plus) || input.key_pressed(Key::Equals))
            {
                action = Action::ZoomIn;
            }
            if input.modifiers.command && input.key_pressed(Key::Minus) {
                action = Action::ZoomOut;
            }
            if input.modifiers.command && input.key_pressed(Key::Num0) {
                action = Action::ZoomReset;
            }
            if input.key_pressed(Key::F5) {
                action = Action::Decompile;
            }
            // IDA opens its Strings window with Shift+F12; this is the one pane shortcut
            // that matches IDA, so IDA muscle memory finds the list where it expects it.
            if input.modifiers.shift && input.key_pressed(Key::F12) {
                action = Action::FocusPane(Pane::Strings);
            }
            // Pane focus shortcuts (F2/F3, F6, F8-F11). Like IDA, analysis has no key of
            // its own: it runs when the binary is loaded, and re-running it is a menu
            // command.
            if input.key_pressed(Key::F2) {
                action = Action::FocusPane(Pane::Functions);
            } else if input.key_pressed(Key::F3) {
                action = Action::FocusPane(Pane::Names);
            } else if input.key_pressed(Key::F6) {
                action = Action::FocusPane(Pane::Imports);
            } else if input.key_pressed(Key::F8) {
                action = Action::FocusPane(Pane::Segments);
            } else if input.key_pressed(Key::F9) {
                action = Action::FocusPane(Pane::Hex);
            } else if input.key_pressed(Key::F10) {
                action = Action::FocusPane(Pane::Ai);
            } else if input.key_pressed(Key::F11) {
                action = Action::FocusPane(Pane::Structs);
            }
            if input.key_pressed(Key::G) {
                action = Action::GotoDialog;
            } else if input.key_pressed(Key::N) {
                action = Action::EditName;
            } else if input.key_pressed(Key::Slash) && self.tab == Tab::Pseudo {
                // IDA's `/` edits the pseudocode comment; `;` is offered as an alias
                // because it is the comment shortcut everywhere else in the app.
                action = Action::EditPseudoComment { addr: None };
            } else if input.key_pressed(Key::Semicolon) {
                action = if self.tab == Tab::Pseudo {
                    Action::EditPseudoComment { addr: None }
                } else {
                    Action::EditComment
                };
            } else if self.tab == Tab::Disasm && input.key_pressed(Key::Space) {
                // IDA binds Space to the disassembly view, so the Decompile pane (and the
                // side lists) leave it alone; the toolbar button and the palette entry can
                // still bring the graph/text switch back to the front from anywhere.
                action = Action::ToggleViewMode;
            } else if input.key_pressed(Key::X) {
                action = Action::ShowXrefs;
            } else if input.modifiers.command {
                // leave Ctrl/Cmd chords (copy, paste, …) to the platform.
            } else if input.key_pressed(Key::Tab) && !input.modifiers.any() {
                // IDA's Tab flips the two code views; the auxiliary panes keep their keys.
                action = Action::ToggleDisasmPseudo;
            } else if self.tab == Tab::Disasm && input.key_pressed(Key::C) {
                action = Action::ApplyData(DataOp::Code);
            } else if self.tab == Tab::Disasm && input.key_pressed(Key::D) {
                action = Action::ApplyData(DataOp::Data);
            } else if self.tab == Tab::Disasm && input.key_pressed(Key::A) {
                action = Action::ApplyData(DataOp::Ascii);
            } else if self.tab == Tab::Disasm && input.key_pressed(Key::U) {
                action = Action::ApplyData(DataOp::Unknown);
            } else if self.tab == Tab::Disasm && input.key_pressed(Key::P) {
                action = Action::ApplyData(DataOp::DefineFunction);
            } else if input.key_pressed(Key::Enter) && self.tab == Tab::Disasm {
                action = Action::Follow;
            } else if input.key_pressed(Key::Escape) {
                action = Action::Back;
            } else if input.key_pressed(Key::ArrowDown) {
                action = Action::MoveSelection(1);
            } else if input.key_pressed(Key::ArrowUp) {
                action = Action::MoveSelection(-1);
            } else if input.key_pressed(Key::PageDown) {
                action = Action::Page(1);
            } else if input.key_pressed(Key::PageUp) {
                action = Action::Page(-1);
            }
        });
        action
    }

    fn apply_action(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::None => {}
            Action::Copy => {
                self.copy_focused(ctx);
            }
            Action::SelectAll => {
                self.select_all_in_focus();
            }
            Action::Open => {
                // Opening a project and importing a binary are the same call: `open_path`
                // dispatches on the extension. The dialog used to list only `.rcraft`, so an
                // ELF had to be dragged in or reached through Import — a dead end for anyone
                // who clicked Open first.
                let executables = self.t("ui_filter_executables");
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("ReCraft project", &["rcraft"])
                    .add_filter(executables.as_str(), IMPORT_EXTENSIONS)
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.open_path(&path);
                }
            }
            Action::CloseProject => self.close_project(),
            Action::Import => {
                // PE and ELF share one entry point: the loader picks the format from the
                // magic bytes, so the dialog only has to stop hiding ELF samples.
                let executables = self.t("ui_filter_executables");
                let all_files = self.t("ui_filter_all_files");
                let picked = rfd::FileDialog::new()
                    .add_filter(executables.as_str(), IMPORT_EXTENSIONS)
                    .add_filter(all_files.as_str(), &["*"])
                    .pick_file();
                if let Some(path) = picked {
                    self.open_path(&path);
                }
            }
            Action::Follow => {
                let Some(line) = self.disasm_rows.get(self.selected).cloned() else {
                    return;
                };
                if let Some(target) = self.follow_target(&line) {
                    self.navigate(target);
                    self.tab = Tab::Disasm;
                    self.view_mode = ViewMode::Text;
                    self.focus_pane(Pane::View);
                } else {
                    self.set_status(self.t("ui_no_followable_target").to_owned());
                }
            }
            Action::OpenRecent(index) => {
                if let Some(path) = self.recent.get(index).cloned() {
                    self.open_path(&path);
                }
            }
            Action::NavigateToDisasm(addr) => {
                self.navigate(addr);
                self.tab = Tab::Disasm;
                self.view_mode = ViewMode::Text;
                self.focus_pane(Pane::View);
            }
            Action::GraphGoTo(addr) => {
                // IDA's overview is a navigation aid for the graph, not a way out of it:
                // follow the block but keep the graph on screen and centre the target.
                self.navigate(addr);
                self.tab = Tab::Disasm;
                self.view_mode = ViewMode::Graph;
                self.graph_center_on = Some(addr);
                self.focus_pane(Pane::View);
            }
            Action::EnterFunction(start) => {
                // Like IDA's decompiler: the listing follows the callee and the Decompile
                // pane stays in front. Entering the function already on screen must not
                // append a duplicate history entry.
                if self.pseudo_function_start() == Some(start) {
                    return;
                }
                self.navigate(start);
                self.tab = Tab::Pseudo;
                self.focus_pane(Pane::Pseudo);
                let name = self
                    .functions
                    .iter()
                    .position(|function| function.start == start)
                    .and_then(|index| self.function_labels.get(index).cloned())
                    .unwrap_or_else(|| format!("sub_{:x}", start.get()));
                let addr = format!("{:#x}", start.get());
                self.set_status(
                    self.t_args("ui_entered_function", &[("name", &name), ("addr", &addr)]),
                );
            }
            Action::SyncDisasm(addr) => {
                self.disasm_start = addr;
                self.selected = 0;
                self.disasm_dirty = true;
                self.disasm_scroll_to = Some(0.0);
                self.pseudo_start = addr;
                self.pseudo_selected_line = None;
                self.pseudo_dirty = true;
            }
            Action::SyncPseudoLine { addr, line } => {
                self.pseudo_selected_line = Some(line);
                self.disasm_start = addr;
                self.selected = 0;
                self.disasm_dirty = true;
                self.disasm_scroll_to = Some(0.0);
            }
            Action::ShowStructs => {
                self.focus_pane(Pane::Structs);
            }
            Action::FocusPane(pane) => {
                // The AI assistant has no dock tab; focusing it opens the sidebar.
                if pane == Pane::Ai {
                    self.ai_sidebar = true;
                    self.remove_ai_tab_from_dock();
                    self.layout_saved = None;
                    self.save_layout_now();
                    return;
                }
                self.focus_pane(pane);
            }
            Action::ToggleAiSidebar => {
                self.ai_sidebar = !self.ai_sidebar;
                self.remove_ai_tab_from_dock();
                self.layout_saved = None;
                self.save_layout_now();
            }
            Action::OpenAiSettings => {
                self.ai_settings_open = true;
            }
            Action::ResetLayout => {
                self.dock = None;
                self.ensure_dock();
                self.layout_saved = None;
                if let Some(path) = layout_store_path() {
                    let _ = std::fs::remove_file(path);
                }
                self.set_status(self.t("ui_layout_reset").to_owned());
            }
            Action::SetTheme(theme) => {
                self.theme = theme;
                self.layout_saved = None;
                self.set_status(match theme {
                    ThemeMode::Dark => self.t("ui_dark_theme").to_owned(),
                    ThemeMode::Light => self.t("ui_light_theme").to_owned(),
                });
            }
            Action::ToggleTheme => {
                self.theme = match self.theme {
                    ThemeMode::Dark => ThemeMode::Light,
                    ThemeMode::Light => ThemeMode::Dark,
                };
                self.layout_saved = None;
                self.set_status(match self.theme {
                    ThemeMode::Dark => self.t("ui_dark_theme").to_owned(),
                    ThemeMode::Light => self.t("ui_light_theme").to_owned(),
                });
            }
            Action::ZoomIn => {
                self.mono_zoom = (self.mono_zoom + ZOOM_STEP).min(ZOOM_MAX);
                self.layout_saved = None;
            }
            Action::ZoomOut => {
                self.mono_zoom = (self.mono_zoom - ZOOM_STEP).max(ZOOM_MIN);
                self.layout_saved = None;
            }
            Action::ZoomReset => {
                self.mono_zoom = 1.0;
                self.layout_saved = None;
            }
            Action::ApplyData(op) => {
                let Some(line) = self.disasm_rows.get(self.selected).cloned() else {
                    return;
                };
                self.run_data_op(op, line.addr, usize::from(line.len).max(1));
            }
            Action::EditPrototype => {
                let Some((start, _)) = self.current_function() else {
                    self.set_status(
                        self.t("ui_put_the_cursor_inside_a_function_first")
                            .to_owned(),
                    );
                    return;
                };
                let text = self
                    .session
                    .as_ref()
                    .and_then(|session| session.prototype_at(start).ok().flatten())
                    .unwrap_or_default();
                self.edit = Some(EditState {
                    kind: EditKind::Prototype,
                    addr: start,
                    text,
                    just_opened: true,
                });
            }
            Action::Analyze => {
                self.start_analysis();
            }
            Action::Decompile => self.decompile_cursor_function(),
            Action::ToggleDisasmPseudo => self.toggle_disasm_pseudo(),
            Action::CancelAnalysis => {
                if let Some(job) = self.analysis.as_ref() {
                    job.cancel.store(true, Ordering::Relaxed);
                    self.set_status(self.t("ui_cancelling").to_owned());
                }
            }
            Action::AiSuggest => self.start_suggest(ctx),
            Action::AiRenameVars { func } => self.start_var_suggest(ctx, func),
            Action::AiAgent => self.start_agent(ctx),
            Action::AiAgentDig => self.start_agent_dig(ctx),
            Action::AiAgentReset => {
                self.ai_agent_history.clear();
                self.ai_agent_trace.clear();
                self.ai_explanation = None;
                self.ai_result_kind = AiResultKind::Prose;
                self.ai_result_target = None;
                self.ai_status = self.t("ui_agent_new").to_owned();
                self.set_status(self.ai_status.clone());
            }
            Action::AiApply => {
                self.apply_ai_selection();
            }
            Action::AiClear => {
                self.ai_proposals.clear();
                self.ai_checked.clear();
                self.ai_result_target = None;
                self.ai_status.clear();
                self.ai_explanation = None;
                self.ai_result_kind = AiResultKind::Prose;
                self.ai_reasoning.clear();
                self.ai_reasoning_open = false;
                self.ai_answer = None;
                self.ai_agent_trace.clear();
            }
            Action::AiQuery => {
                if !self.require_ai_provider() {
                    return;
                }
                let text = self.ai_query.trim().to_owned();
                if text.is_empty() {
                    self.ai_status = self.t("ui_type_a_question_first").to_owned();
                    return;
                }
                let Some(session) = self.session.as_mut() else {
                    self.set_status(self.t("ui_open_a_project_first").to_owned());
                    return;
                };
                match recraft_ai::answer_query(session, &text) {
                    Ok(answer) => {
                        let count = answer.items.len().to_string();
                        self.ai_status = self.t_args(
                            "ui_query_matches",
                            &[
                                ("intent", &answer.intent),
                                ("count", &count),
                                ("truncated", ""),
                            ],
                        );
                        self.ai_answer = Some(answer);
                    }
                    Err(error) => self.set_error(error),
                }
            }
            Action::AiExplain => self.start_explain(ctx),
            Action::AiPolish => self.start_polish(ctx, true),
            Action::AiRepolish => self.start_polish(ctx, false),
            Action::TogglePalette => {
                self.palette = if self.palette.is_some() {
                    None
                } else {
                    Some(PaletteState::default())
                };
            }
            Action::PaletteCommand(index) => {
                self.palette = None;
                if let Some((_, command)) = PALETTE_COMMANDS.get(index) {
                    let command = *command;
                    self.apply_action(ctx, command);
                }
            }
            Action::Undo => {
                let result = self.session.as_mut().map(Session::undo);
                match result {
                    Some(Ok(Some(_))) => {
                        self.disasm_dirty = true;
                        self.pseudo_dirty = true;
                        // Deleting a string is undoable; the list is cheap to rebuild.
                        self.strings_dirty = true;
                        self.set_status(self.t("ui_undo_2").to_owned());
                    }
                    Some(Ok(None)) => self.set_status(self.t("ui_nothing_to_undo").to_owned()),
                    Some(Err(error)) => self.set_error(error),
                    None => {}
                }
            }
            Action::Redo => {
                let result = self.session.as_mut().map(Session::redo);
                match result {
                    Some(Ok(Some(_))) => {
                        self.disasm_dirty = true;
                        self.pseudo_dirty = true;
                        self.strings_dirty = true;
                        self.set_status(self.t("ui_redo_2").to_owned());
                    }
                    Some(Ok(None)) => self.set_status(self.t("ui_nothing_to_redo").to_owned()),
                    Some(Err(error)) => self.set_error(error),
                    None => {}
                }
            }
            Action::ToggleLang => {
                self.lang = self.lang.toggle();
                self.save_layout_now();
            }
            Action::ToggleViewMode => {
                self.tab = Tab::Disasm;
                self.focus_pane(Pane::View);
                self.view_mode = match self.view_mode {
                    ViewMode::Graph => ViewMode::Text,
                    ViewMode::Text => ViewMode::Graph,
                };
                // Asking for the graph on a data row stays in text, like IDA; the status
                // line says why (the button and the palette reach this from any pane).
                self.enforce_graph_view_mode();
            }
            Action::GotoDialog => {
                if self.session.is_some() {
                    self.goto = Some(String::new());
                }
            }
            Action::ShowXrefs => {
                let Some(addr) = self.disasm_rows.get(self.selected).map(|line| line.addr) else {
                    return;
                };
                if self.session.is_none() {
                    return;
                }
                self.xref_dialog = Some(XrefDialog {
                    target: addr,
                    rows: Vec::new(),
                    selected: 0,
                    filter: String::new(),
                    search: false,
                    search_focus: false,
                    help: false,
                    dirty: true,
                });
                self.refresh_xrefs();
            }
            Action::EditName | Action::EditComment => {
                let Some(line) = self.disasm_rows.get(self.selected) else {
                    return;
                };
                let addr = line.addr;
                let existing = match action {
                    Action::EditName => line.name.clone().unwrap_or_default(),
                    _ => line.comment.clone().unwrap_or_default(),
                };
                self.edit = Some(EditState {
                    kind: if matches!(action, Action::EditName) {
                        EditKind::Rename
                    } else {
                        EditKind::Comment
                    },
                    addr,
                    text: existing,
                    just_opened: true,
                });
            }
            Action::RenameVar { func, offset } => {
                let existing = self
                    .session
                    .as_ref()
                    .and_then(|session| session.var_name(func, offset).ok().flatten())
                    .unwrap_or_default();
                self.var_edit = Some((func, offset, existing));
            }
            Action::EditPseudoComment { addr } => {
                if self.pseudo.is_none() {
                    return;
                }
                // A context menu names its row; the shortcut follows the cursor's row and
                // finally the function anchor, so the dialog always targets a rendered row.
                let addr = addr
                    .or_else(|| self.pseudo_cursor_address())
                    .unwrap_or(self.pseudo_start);
                self.edit = Some(EditState {
                    kind: EditKind::PseudoComment,
                    addr,
                    text: self.pseudo_comments.get(&addr).cloned().unwrap_or_default(),
                    just_opened: true,
                });
            }
            Action::MoveSelection(delta) => {
                if self.disasm_rows.is_empty() {
                    return;
                }
                let next = self.selected as i64 + i64::from(delta);
                if next < 0 {
                    self.shift_window(-1);
                } else if next as usize >= self.disasm_rows.len() {
                    self.shift_window(1);
                } else {
                    self.selected = next as usize;
                }
            }
            Action::Page(delta) => self.shift_window(delta * 20),
            Action::Back => self.back(),
            Action::Forward => self.forward(),
            Action::Navigate(addr) => self.navigate(addr),
        }
        let _ = ctx;
    }

    /// Move the loaded window by `rows` listing rows (negative = earlier code).
    ///
    /// Stepping follows the listing's own row boundaries (see [`Self::step_listing`]): the
    /// old implementation subtracted `rows * first_row.len()` bytes, which landed inside
    /// instructions and made the view decode garbage as soon as the instruction sizes
    /// varied, and paging a string window through the instruction walker decoded the
    /// literals as code.
    fn shift_window(&mut self, rows: i32) {
        if self.session.is_none() || rows == 0 {
            return;
        }
        let Some(next) = self.step_listing(rows) else {
            return;
        };
        self.disasm_start = next;
        self.refresh_disasm();
        // Keep the cursor index so it stays on the same screen line, like IDA paging.
        self.selected = self.selected.min(self.disasm_rows.len().saturating_sub(1));
    }

    fn update_inner(&mut self, root: &mut egui::Ui) {
        let ctx = root.ctx().clone();
        self.install_style(&ctx);
        self.handle_dropped_files(&ctx);
        // egui queues its own widget-focus move for Tab at the start of the frame, and the
        // first focusable widget that renders claims it; every later Tab would then walk the
        // menu bar instead of the app's binding. Drop the move before any widget runs.
        self.claim_tab_from_focus_navigation(&ctx);
        self.tick();
        // `PRAGMA data_version` is cheap, but polling it every frame is needless: a burst
        // of external edits only needs one reload, and ~5 checks per second is plenty for
        // an IDE-style workbench.
        if self.frame_count % 10 == 0 {
            self.poll_external_changes();
        }
        // Background AI requests must wake the UI when they complete; otherwise the
        // polished text only appears after an unrelated mouse/keyboard event.
        if self.ai_job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let title = match self.project.as_ref() {
            Some(path) => format!(
                "ReCraft - {}  {}",
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("project"),
                path.display()
            ),
            None => "ReCraft".to_owned(),
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        let mut action = self.handle_keys(&ctx);
        self.top_bar(root, &mut action);
        self.bottom_bar(root);
        self.ai_sidebar_ui(root, &mut action);
        self.sync_graph_overview();
        self.dock_area(root, &mut action);
        self.ai_settings_window(&ctx);
        self.about_window(&ctx);
        self.edit_window(&ctx, &mut action);
        self.var_rename_window(&ctx);
        self.goto_window(&ctx, &mut action);
        self.strings_setup_window(&ctx);
        self.palette_window(&ctx, &mut action);
        self.xrefs_window(&ctx, &mut action);
        self.apply_action(&ctx, action);
        self.persist_layout_if_changed();
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

impl eframe::App for ReCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.update_inner(ui);
    }
}

/// Edge colour shared by the graph view and its overview thumbnail.
fn graph_edge_color(pal: Palette, kind: recraft_analysis::EdgeKind) -> Color32 {
    match kind {
        recraft_analysis::EdgeKind::Fallthrough => pal.edge_jump,
        recraft_analysis::EdgeKind::Taken => pal.edge_taken,
        recraft_analysis::EdgeKind::NotTaken => pal.edge_not_taken,
        recraft_analysis::EdgeKind::Unconditional => pal.edge_jump,
        recraft_analysis::EdgeKind::JumpTable => pal.edge_jump_table,
    }
}

/// Draw a dashed rectangle (egui has no dashed stroke, and IDA's overview viewport frame
/// is dotted). The painter clips the segments to the pane.
fn paint_dashed_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    dash: f32,
    gap: f32,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 || dash <= 0.0 {
        return;
    }
    for (from, to) in [
        (rect.left_top(), rect.right_top()),
        (rect.right_top(), rect.right_bottom()),
        (rect.right_bottom(), rect.left_bottom()),
        (rect.left_bottom(), rect.left_top()),
    ] {
        let length = (to - from).length();
        if length <= 0.0 {
            continue;
        }
        let direction = (to - from) / length;
        let mut start = 0.0;
        while start < length {
            let end = (start + dash).min(length);
            painter.line_segment([from + direction * start, from + direction * end], stroke);
            start = end + gap;
        }
    }
}

/// Lay out the CFG the same way the graph view does, so the overview minimap can reuse
/// the exact node geometry at any scale.
fn graph_layout(cfg: &recraft_analysis::Cfg, zoom: f32) -> (Vec<egui::Rect>, egui::Vec2) {
    const MAX_LINES: usize = 10;
    let node_w = 300.0 * zoom;
    let header_h = 18.0 * zoom;
    let line_h = 14.0 * zoom;
    let gap_x = 120.0 * zoom;
    let gap_y = 24.0 * zoom;
    let pad = 10.0 * zoom;
    let layer_of = recraft_analysis::layers(cfg);
    let column_count = layer_of.iter().copied().max().unwrap_or(0) + 1;
    let mut column_y = vec![24.0 * zoom; column_count];
    let mut rects = Vec::with_capacity(cfg.blocks.len());
    for (index, block) in cfg.blocks.iter().enumerate() {
        let layer = layer_of[index];
        let shown = block.insns.len().min(MAX_LINES);
        let height = header_h + shown as f32 * line_h + pad * 2.0 + 4.0 * zoom;
        let x = 24.0 * zoom + layer as f32 * (node_w + gap_x);
        let y = column_y[layer];
        column_y[layer] = y + height + gap_y;
        rects.push(egui::Rect::from_min_size(
            egui::pos2(x, y),
            egui::vec2(node_w, height),
        ));
    }
    let width = 48.0 * zoom + column_count as f32 * (node_w + gap_x);
    let height = column_y.iter().copied().fold(24.0f32 * zoom, f32::max) + 24.0 * zoom;
    (rects, egui::vec2(width, height))
}

fn append(job: &mut LayoutJob, text: &str, color: Color32, mono: &FontId) {
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: mono.clone(),
            color,
            ..Default::default()
        },
    );
}

/// Semantic class of an operand token, so the text view can colour operands the way
/// IDA does (registers, immediates, symbol names, strings).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OperandKind {
    Register,
    Number,
    Name,
    String,
    Other,
}

/// Whether `token` names an x86/x64 register, including the 8/16/32-bit aliases and
/// the `r8b`…`r15d`, `xmm`/`ymm`/`zmm` families.
fn is_register(token: &str) -> bool {
    is_register_lower(&token.to_ascii_lowercase())
}

/// [`is_register`] for a caller that already lowercased the token.
fn is_register_lower(lower: &str) -> bool {
    const NAMED: &[&str] = &[
        "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rbp", "rsp", "rip", "eax", "ebx", "ecx", "edx",
        "esi", "edi", "ebp", "esp", "eip", "ax", "bx", "cx", "dx", "si", "di", "bp", "sp", "ip",
        "al", "ah", "bl", "bh", "cl", "ch", "dl", "dh", "sil", "dil", "bpl", "spl", "cs", "ds",
        "es", "fs", "gs", "ss",
    ];
    if NAMED.contains(&lower) {
        return true;
    }
    for prefix in ["xmm", "ymm", "zmm"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            return !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit());
        }
    }
    let Some(rest) = lower.strip_prefix('r') else {
        return false;
    };
    let digits = rest
        .strip_suffix('b')
        .or_else(|| rest.strip_suffix('w'))
        .or_else(|| rest.strip_suffix('d'))
        .unwrap_or(rest);
    !digits.is_empty()
        && digits.len() <= 2
        && digits.chars().all(|ch| ch.is_ascii_digit())
        && digits
            .parse::<u8>()
            .is_ok_and(|number| (8..=15).contains(&number))
}

fn classify_operand(token: &str) -> OperandKind {
    if token.len() >= 2
        && ((token.starts_with('"') && token.ends_with('"'))
            || (token.starts_with('\'') && token.ends_with('\'')))
    {
        return OperandKind::String;
    }
    let lower = token.to_ascii_lowercase();
    if is_register_lower(&lower) {
        return OperandKind::Register;
    }
    const NAME_PREFIXES: [&str; 12] = [
        "sub_", "loc_", "off_", "unk_", "byte_", "word_", "dword_", "qword_", "asc_", "stru_",
        "flt_", "dbl_",
    ];
    if NAME_PREFIXES.iter().any(|prefix| lower.starts_with(prefix)) {
        return OperandKind::Name;
    }
    let numeric = token.starts_with('#')
        || token.starts_with('-')
        || token.starts_with('+')
        || token
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_digit());
    if numeric
        && token.chars().all(|ch| {
            ch.is_ascii_hexdigit() || matches!(ch, 'x' | 'X' | 'h' | 'H' | '#' | '-' | '+')
        })
    {
        return OperandKind::Number;
    }
    OperandKind::Other
}

/// Append an operand string, colouring registers/numbers/symbols/strings separately while
/// keeping punctuation and padding as normal text.
fn append_operands(job: &mut LayoutJob, text: &str, pal: Palette, mono: &FontId) {
    let mut token = String::new();
    let mut quote: Option<char> = None;
    for ch in text.chars() {
        if let Some(open) = quote {
            token.push(ch);
            if ch == open {
                append(job, &token, pal.comment, mono);
                token.clear();
                quote = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            if !token.is_empty() {
                append_operand_token(job, &token, pal, mono);
                token.clear();
            }
            quote = Some(ch);
            token.push(ch);
        } else if ch.is_alphanumeric() || matches!(ch, '_' | '.' | '$' | '@') {
            token.push(ch);
        } else {
            if !token.is_empty() {
                append_operand_token(job, &token, pal, mono);
                token.clear();
            }
            append(job, &ch.to_string(), pal.text, mono);
        }
    }
    if !token.is_empty() {
        append_operand_token(job, &token, pal, mono);
    }
}

fn append_operand_token(job: &mut LayoutJob, token: &str, pal: Palette, mono: &FontId) {
    let color = match classify_operand(token) {
        OperandKind::Register => pal.register,
        OperandKind::Number => pal.number,
        OperandKind::Name => pal.name,
        OperandKind::String => pal.comment,
        OperandKind::Other => pal.text,
    };
    append(job, token, color, mono);
}

/// Syntax class of one pseudo-C token. The decompiler emits a small C subset, so only
/// these classes can occur; everything else keeps the plain text colour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PseudoKind {
    Keyword,
    Type,
    Register,
    Name,
    Number,
    String,
    Comment,
}

/// Pseudo-C keywords printed by the emitter. `__asm` is an intrinsic marker, not a C
/// keyword, but it reads like one in the listing.
fn is_pseudo_keyword(word: &str) -> bool {
    matches!(
        word,
        "__asm"
            | "break"
            | "case"
            | "continue"
            | "default"
            | "do"
            | "else"
            | "for"
            | "goto"
            | "if"
            | "return"
            | "sizeof"
            | "switch"
            | "while"
    )
}

/// Type names the decompiler prints: the set `structuring::is_type_name` recognises plus
/// the pointer/scalar aliases `l2` uses for recovered signatures.
fn is_pseudo_type(word: &str) -> bool {
    matches!(
        word,
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
            | "f32"
            | "f64"
            | "__m128"
            | "__int128"
            | "bool"
            | "char"
            | "char16_t"
            | "double"
            | "float"
            | "int"
            | "long"
            | "short"
            | "signed"
            | "size_t"
            | "unsigned"
            | "void"
            | "wchar_t"
    )
}

/// Whether a pseudo-C word names a function, label or global rather than a local.
/// Auto names carry the decompiler's prefixes; imported/API names carry an uppercase
/// letter (`SendMessageW`, `_CxxThrowException`), which the generated locals never do.
fn is_pseudo_symbol(word: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "sub_", "loc_", "off_", "unk_", "byte_", "word_", "dword_", "qword_", "xmmword_",
        "ymmword_", "zmmword_", "asc_", "stru_", "flt_", "dbl_", "ord_",
    ];
    PREFIXES.iter().any(|prefix| word.starts_with(prefix))
        || (word.len() > 1 && word.starts_with('_'))
        || word.chars().any(|ch| ch.is_ascii_uppercase())
}

fn pseudo_word_kind(word: &str) -> Option<PseudoKind> {
    if is_pseudo_keyword(word) {
        Some(PseudoKind::Keyword)
    } else if is_pseudo_type(word) {
        Some(PseudoKind::Type)
    } else if is_register(word) {
        Some(PseudoKind::Register)
    } else if is_pseudo_symbol(word) {
        Some(PseudoKind::Name)
    } else {
        None
    }
}

fn pseudo_color(kind: PseudoKind, pal: Palette) -> Color32 {
    match kind {
        PseudoKind::Keyword | PseudoKind::Type => pal.mnemonic,
        PseudoKind::Register => pal.register,
        PseudoKind::Name => pal.name,
        PseudoKind::Number => pal.number,
        PseudoKind::String | PseudoKind::Comment => pal.comment,
    }
}

/// Index just past the string or character literal opening at `start` (unterminated
/// literals run to the end of the line).
fn pseudo_quoted_end(text: &str, start: usize) -> usize {
    let quote = text.as_bytes()[start];
    let mut index = start + 1;
    while index < text.len() {
        match text.as_bytes()[index] {
            b'\\' => index += 2,
            byte if byte == quote => return index + 1,
            _ => index += 1,
        }
    }
    text.len()
}

/// Index just past the number starting at `start` (`0x` hex, decimal, or a float with
/// an exponent), including integer/float suffixes.
fn pseudo_number_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut index = start;
    if bytes.get(index) == Some(&b'0') && matches!(bytes.get(index + 1), Some(b'x' | b'X')) {
        index += 2;
        while index < bytes.len() && (bytes[index].is_ascii_hexdigit() || bytes[index] == b'_') {
            index += 1;
        }
    } else {
        while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.') {
            index += 1;
        }
        if matches!(bytes.get(index), Some(b'e' | b'E')) {
            let mut exponent = index + 1;
            if matches!(bytes.get(exponent), Some(b'+' | b'-')) {
                exponent += 1;
            }
            if bytes.get(exponent).is_some_and(u8::is_ascii_digit) {
                index = exponent;
                while index < bytes.len() && bytes[index].is_ascii_digit() {
                    index += 1;
                }
            }
        }
    }
    while matches!(
        bytes.get(index),
        Some(b'u' | b'U' | b'l' | b'L' | b'f' | b'F')
    ) {
        index += 1;
    }
    index
}

/// Append one pseudo-C line, colouring its C syntax (keywords and types blue, registers
/// purple, symbol names yellow, numbers and string literals green) while keeping the
/// row's text byte-for-byte identical to `text`.
fn append_pseudo_line(job: &mut LayoutJob, text: &str, pal: Palette, mono: &FontId) {
    let mut plain = 0;
    let mut index = 0;
    let bytes = text.as_bytes();
    while index < bytes.len() {
        let byte = bytes[index];
        let (end, kind) = if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            (end, Some(PseudoKind::Comment))
        } else if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let end = text[index + 2..]
                .find("*/")
                .map_or(text.len(), |offset| index + 2 + offset + 2);
            (end, Some(PseudoKind::Comment))
        } else if byte == b'"' || byte == b'\'' {
            (pseudo_quoted_end(text, index), Some(PseudoKind::String))
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            let mut end = index + 1;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            let mut kind = pseudo_word_kind(&text[index..end]);
            // Lower-case CRT/API names (`wcsnlen`, `memcpy`) carry no auto prefix and no
            // upper-case letter, so recognise them by the call syntax that follows.
            if kind.is_none() && text[end..].trim_start().starts_with('(') {
                kind = Some(PseudoKind::Name);
            }
            (end, kind)
        } else if byte.is_ascii_digit() {
            (pseudo_number_end(text, index), Some(PseudoKind::Number))
        } else {
            index += text[index..].chars().next().map_or(1, char::len_utf8);
            continue;
        };
        if let Some(kind) = kind {
            if plain < index {
                append(job, &text[plain..index], pal.text, mono);
            }
            append(job, &text[index..end], pseudo_color(kind, pal), mono);
            plain = end;
        }
        index = end;
    }
    if plain < text.len() {
        append(job, &text[plain..], pal.text, mono);
    }
}

/// Lay out the AI result pane. A polish answer is pseudo-C and gets the same syntax
/// colours as the Decompile view; an explanation report is prose and stays plain text.
fn append_ai_result(
    job: &mut LayoutJob,
    text: &str,
    kind: AiResultKind,
    pal: Palette,
    mono: &FontId,
) {
    match kind {
        AiResultKind::Code => append_pseudo_line(job, text, pal, mono),
        AiResultKind::Prose => append(job, text, pal.text, mono),
    }
}

/// Default screen position/size for detached tool windows.
/// Parse `offset:name:kind` (hex offsets) for the structures editor.
fn parse_struct_field_spec(spec: &str) -> std::result::Result<StructField, String> {
    let mut parts = spec.split(':');
    let offset = parts.next().ok_or("missing offset")?;
    let name = parts.next().ok_or("missing field name")?;
    let kind = parts.next().ok_or("missing field kind")?;
    if parts.next().is_some() {
        return Err(format!("too many ':' in field spec: {spec}"));
    }
    let offset = u64::from_str_radix(offset.trim().trim_start_matches("0x"), 16)
        .map_err(|_| format!("bad offset: {offset}"))?;
    if name.trim().is_empty() || kind.trim().is_empty() {
        return Err(format!("empty name/kind in field spec: {spec}"));
    }
    Ok(StructField {
        offset,
        name: name.trim().to_owned(),
        kind: kind.trim().to_owned(),
    })
}

/// Parse the `addr` string of an offline query result (`0x...` hex or decimal).
fn parse_query_addr(raw: &str) -> Option<Address> {
    let raw = raw.trim();
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok().map(Address::new)
    } else {
        raw.parse::<u64>().ok().map(Address::new)
    }
}

/// Function-list filter: match name or address hex substring (case-insensitive input).
fn function_matches(filter: &str, start: Address, name: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    name.to_lowercase().contains(filter) || format!("{:#x}", start.get()).contains(filter)
}

fn xref_kind_name(kind: i64) -> &'static str {
    match kind {
        recraft_analysis::XREF_CALL => "call",
        recraft_analysis::XREF_JUMP => "jump",
        recraft_analysis::XREF_DATA => "data",
        _ => "other",
    }
}

/// Single-letter xref type, as IDA's xrefs dialog shows it: `c` call, `j` jump,
/// `o` address taken, `r` data read, `w` data write.
///
/// One data xref is stored per instruction, so the offset/read/write split is derived
/// from the instruction text at the referencing site.
fn xref_type_letter(kind: i64, site_text: &str) -> char {
    match kind {
        recraft_analysis::XREF_CALL => 'c',
        recraft_analysis::XREF_JUMP => 'j',
        recraft_analysis::XREF_DATA => data_xref_letter(site_text),
        _ => '?',
    }
}

fn data_xref_letter(site_text: &str) -> char {
    let mut parts = site_text.splitn(2, ' ');
    let mnemonic = parts.next().unwrap_or_default().to_ascii_lowercase();
    if mnemonic == "lea" {
        return 'o';
    }
    // A store writes its (first) memory operand; everything else reads.
    const STORES: [&str; 13] = [
        "mov", "movzx", "movsx", "movsxd", "movaps", "movups", "movdqa", "movdqu", "movss",
        "movsd", "movq", "xchg", "cmpxchg",
    ];
    let destination = parts
        .next()
        .unwrap_or_default()
        .split(',')
        .next()
        .unwrap_or_default();
    if destination.contains('[') && STORES.contains(&mnemonic.as_str()) {
        'w'
    } else {
        'r'
    }
}

/// Index of the function containing `addr` (`functions` is sorted by start address).
fn function_at(functions: &[FunctionInfo], addr: Address) -> Option<usize> {
    let index = functions.partition_point(|function| function.start <= addr);
    let index = index.checked_sub(1)?;
    (addr < functions[index].end).then_some(index)
}

/// Address-column label the way IDA writes xrefs: the symbol at a function or data
/// start, `main:loc_1400010b9` for an unnamed location inside a function, and
/// `loc_`/`unk_` for anything else.
fn xref_label(
    addr: Address,
    name: Option<&str>,
    function: Option<(&str, Address)>,
    code: bool,
) -> String {
    match (function, name) {
        (Some((function, start)), _) if addr == start => name.unwrap_or(function).to_owned(),
        (Some((function, _)), Some(name)) => format!("{function}:{name}"),
        (Some((function, _)), None) => format!("{function}:loc_{:x}", addr.get()),
        (None, Some(name)) => name.to_owned(),
        (None, None) if code => format!("loc_{:x}", addr.get()),
        (None, None) => format!("unk_{:x}", addr.get()),
    }
}

/// Build one row of the Xrefs table. `current` is the selected address (the item whose
/// references are listed); `xref` points into it for `up` rows and out of it otherwise.
fn xref_row(
    session: &mut Session,
    functions: &[FunctionInfo],
    function_labels: &[String],
    strings: &[StringRow],
    current: Address,
    xref: XrefEntry,
    up: bool,
) -> XrefRow {
    // The other end: the referencing instruction for `Up`, the referenced item for `Down`.
    let site = if up { xref.from } else { xref.to };
    let name = session.name_at(site).ok().flatten();
    let function = function_at(functions, site).and_then(|index| {
        function_labels
            .get(index)
            .map(|label| (label.as_str(), functions[index].start))
    });
    let code = is_code_at(session, site);
    let label = xref_label(site, name.as_deref(), function, code);
    let mut text = site_text(session, strings, site).unwrap_or_else(|| label.clone());
    // The type letter describes how the *referencing* instruction touches the target:
    // for `Down` rows that is the selected address, not the target shown in `addr`.
    let letter = if up {
        xref_type_letter(xref.kind, &text)
    } else {
        let referencing = site_text(session, strings, current).unwrap_or_default();
        xref_type_letter(xref.kind, &referencing)
    };
    // IDA appends the referenced string to the referencing line:
    // `lea rdx, Str2  ; 'flag{...}'`.
    if up && let Some(literal) = string_literal_at(strings, current) {
        text.push_str("  ; ");
        text.push_str(&quote_literal(&literal, 64));
    }
    XrefRow {
        up,
        kind: letter,
        kind_name: xref_kind_name(xref.kind),
        addr: site,
        label,
        text,
    }
}

/// Text at `addr` for the text column: the instruction, the stored string, or a short
/// data preview. `None` when the address holds nothing readable.
fn site_text(session: &mut Session, strings: &[StringRow], addr: Address) -> Option<String> {
    if is_code_at(session, addr)
        && let Some(line) = session.disasm(addr, 1).ok()?.into_iter().next()
    {
        let text = format!("{} {}", line.mnemonic, line.operands);
        return Some(text.trim_end().to_owned());
    }
    if let Some(literal) = string_literal_at(strings, addr) {
        return Some(format!("db {}", quote_literal(&literal, 64)));
    }
    data_preview(session, addr)
}

/// Whether `addr` holds code. Analysis does not persist a code flag for every decoded
/// instruction, so the containing segment's permission decides; an explicit data
/// definition still wins over an executable segment.
fn is_code_at(session: &mut Session, addr: Address) -> bool {
    let defined_data = session
        .flags_at(addr)
        .map(|flags| flags.is_data())
        .unwrap_or(false);
    !defined_data
        && session
            .segment_containing(addr)
            .is_some_and(|segment| segment.perm.is_executable())
}

/// `db 48 65 6c 6c 6f` preview of up to eight bytes at `addr`.
fn data_preview(session: &mut Session, addr: Address) -> Option<String> {
    let mut bytes = [0u8; 8];
    let read = session.read_at(addr, &mut bytes).ok()?;
    if read == 0 {
        return None;
    }
    let hex = bytes[..read]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    Some(format!("db {hex}"))
}

/// Text of the stored string covering `addr`. The character offset into a pooled run is
/// applied, so a substring literal shows only its own tail.
fn string_literal_at(strings: &[StringRow], addr: Address) -> Option<String> {
    let (index, offset) = string_row_at(strings, addr)?;
    let row = strings.get(index)?;
    let skip = if row.enc == recraft_analysis::STRING_ENC_UTF16LE {
        usize::try_from(offset / 2).ok()?
    } else {
        usize::try_from(offset).ok()?
    };
    Some(row.text.chars().skip(skip).collect())
}

/// Quote a literal the way the listing does (`'…'`), escaping it and eliding long text.
fn quote_literal(text: &str, max_chars: usize) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace(['\r', '\n'], " ");
    format!("'{}'", elide_chars(&escaped, max_chars))
}

/// Shorten `text` to `max_chars` characters, ending in `…` when it was cut.
fn elide_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// One xref row as Ctrl+C copies it: direction, type, address label and the text at the
/// other end, tab separated like IDA's dialog columns.
fn xref_row_text(row: &XrefRow, up: &str, down: &str) -> String {
    format!(
        "{}\t{}\t{}\t{}",
        if row.up { up } else { down },
        row.kind,
        row.label,
        row.text
    )
}

/// Indices of the rows matching the dialog's search text, which (like IDA's Search)
/// applies to the address, label and text columns.
fn xref_matches(rows: &[XrefRow], filter: &str) -> Vec<usize> {
    let filter = filter.trim().to_lowercase();
    rows.iter()
        .enumerate()
        .filter(|(_, row)| {
            filter.is_empty()
                || row.label.to_lowercase().contains(&filter)
                || row.text.to_lowercase().contains(&filter)
                || format!("{:#x}", row.addr.get()).contains(&filter)
        })
        .map(|(index, _)| index)
        .collect()
}

/// Background worker: its own `Session` (separate SQLite connection) keeps the UI
/// responsive; `cancel` is polled by the analysis pass.
fn run_analysis_job(project: &Path, cancel: &Arc<AtomicBool>) -> AnalysisMessage {
    let started = Instant::now();
    let outcome = (|| -> recraft_common::Result<(usize, usize)> {
        let mut session = Session::open(project)?;
        let options = recraft_analysis::AnalysisOptions {
            cancel: Some(cancel.clone()),
            seed_range: None,
        };
        let result = recraft_analysis::analyze_with_options(&mut session, &options)?;
        recraft_analysis::persist(&mut session, &result)?;
        let min_len = recraft_analysis::string_min_len(&session);
        let strings = recraft_analysis::extract_strings(&mut session, min_len, 200_000)?;
        recraft_analysis::persist_strings(&mut session, &strings)?;
        // Names learned elsewhere reach a new project without an extra command: match the
        // built-in CRT signatures once, right after the analysis that finds the functions.
        recraft_decompile::signature::apply_builtin(&mut session, false)?;
        Ok((result.functions.len(), strings.len()))
    })();
    match outcome {
        Ok((functions, strings)) => AnalysisMessage::Done {
            functions,
            strings,
            elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
        },
        Err(recraft_common::Error::Cancelled) => AnalysisMessage::Cancelled,
        Err(error) => AnalysisMessage::Failed(error.to_string()),
    }
}

/// Background worker for `Strings → Setup… → Rebuild`: re-extract with the requested
/// minimum length, persist both the table and the setting (so reopening the project keeps
/// it), and report the new row count.
fn run_strings_job(project: &Path, min_len: usize) -> StringsMessage {
    let started = Instant::now();
    let outcome = (|| -> recraft_common::Result<(usize, usize)> {
        let mut session = Session::open(project)?;
        let min_len = recraft_analysis::clamp_min_string_len(min_len);
        let strings = recraft_analysis::extract_strings(&mut session, min_len, 200_000)?;
        recraft_analysis::persist_strings(&mut session, &strings)?;
        recraft_analysis::set_string_min_len(&mut session, min_len)?;
        Ok((strings.len(), min_len))
    })();
    match outcome {
        Ok((strings, min_len)) => StringsMessage::Done {
            strings,
            min_len,
            elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
        },
        Err(error) => StringsMessage::Failed(error.to_string()),
    }
}

fn resolve_address(session: &Session, raw: &str) -> Result<Address, String> {
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("entry") {
        return Ok(session.entry_point());
    }
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16)
            .map(Address::new)
            .map_err(|_| format!("invalid hex address: {raw}"));
    }
    match session.addr_of_name(raw) {
        Ok(Some(addr)) => return Ok(addr),
        Ok(None) => {}
        Err(error) => return Err(error.to_string()),
    }
    raw.parse::<u64>()
        .map(Address::new)
        .map_err(|_| format!("invalid address or unknown symbol: {raw}"))
}

/// IDA-like font stack: Consolas for code, Segoe UI for chrome, and a CJK face as the
/// last fallback in both families.
///
/// `None` means no Windows font file could be read, in which case egui's built-in
/// fonts stay installed (missing files must not blank out the UI).
fn ui_fonts() -> Option<egui::FontDefinitions> {
    let mut fonts = egui::FontDefinitions::default();
    let mut loaded = false;
    for (name, path) in [
        ("consolas", "C:\\Windows\\Fonts\\consola.ttf"),
        ("consolas_bold", "C:\\Windows\\Fonts\\consolab.ttf"),
        ("segoeui", "C:\\Windows\\Fonts\\segoeui.ttf"),
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.to_owned(), egui::FontData::from_owned(bytes).into());
            loaded = true;
        }
    }

    // CJK fallback (IDA's UI is English but ReCraft ships a Chinese UI too).
    //
    // epaint does not align a fallback face on the Latin baseline: it centres the
    // fallback's line box inside the row
    // (`pos.y = face_ascent + 0.5 * (row_height - face_height)`, epaint's
    // `text_layout.rs`). Microsoft YaHei's ascent is 1.06 em against Consolas' 0.74 em,
    // so in monospace text every Chinese glyph would sit ~3 px lower than the Latin
    // text around it. Segoe UI and YaHei happen to share ascent and row height at the
    // sizes the UI uses, so only the monospace fallback needs the correction.
    // Guarded by `cjk_monospace_baseline_matches_latin`.
    for path in [
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
    ] {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        // The tweak lives on `FontData`, so the two families need one entry each. They
        // share a single leaked buffer instead of reading (and holding) the file twice.
        let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        fonts
            .font_data
            .insert("cjk".to_owned(), egui::FontData::from_static(bytes).into());
        let mut aligned = egui::FontData::from_static(bytes);
        aligned.tweak.y_offset_factor = MONO_CJK_Y_OFFSET_FACTOR;
        fonts
            .font_data
            .insert("cjk_mono".to_owned(), aligned.into());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("cjk".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("cjk_mono".to_owned());
        loaded = true;
        break;
    }

    for (name, family) in [
        ("consolas", egui::FontFamily::Monospace),
        ("segoeui", egui::FontFamily::Proportional),
    ] {
        if fonts.font_data.contains_key(name) {
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, name.to_owned());
        }
    }

    loaded.then_some(fonts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn the_modern_theme_lands_the_documented_tokens() {
        // docs/UI.md §1/§3/§4: surface ladder, one accent, hairline borders, radii 6/10/12.
        let ctx = egui::Context::default();
        let mut app = ReCraftApp {
            theme: ThemeMode::Dark,
            ..ReCraftApp::default()
        };
        app.apply_theme(&ctx);
        let pal = Palette::DARK;
        let mut seen_dark = false;
        ctx.all_styles_mut(|style| {
            let visuals = &style.visuals;
            if visuals.panel_fill != pal.panel {
                return; // the light style is built from the light palette
            }
            seen_dark = true;
            assert_eq!(visuals.extreme_bg_color, pal.view);
            assert_eq!(visuals.window_fill, pal.surface_2);
            assert_eq!(visuals.hyperlink_color, pal.accent);
            assert_eq!(visuals.window_stroke.color, pal.border_strong);
            assert_eq!(
                visuals.widgets.inactive.corner_radius,
                egui::CornerRadius::same(6)
            );
            assert_eq!(visuals.window_corner_radius, egui::CornerRadius::same(12));
            // Selection is an accent tint, not the dense-list row colour.
            assert_ne!(visuals.selection.bg_fill, pal.selected_row);
            assert_eq!(visuals.selection.stroke.color, pal.accent);
        });
        assert!(seen_dark, "the dark palette was not applied");
        // The three surface levels must be distinct or the layers disappear.
        assert_ne!(pal.view, pal.panel);
        assert_ne!(pal.panel, pal.surface_2);
        assert_ne!(pal.border, pal.border_strong);
    }
    /// WCAG relative luminance of one sRGB channel.
    fn channel_luminance(value: u8) -> f32 {
        let channel = f32::from(value) / 255.0;
        if channel <= 0.039_28 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    fn relative_luminance(color: egui::Color32) -> f32 {
        0.2126 * channel_luminance(color.r())
            + 0.7152 * channel_luminance(color.g())
            + 0.0722 * channel_luminance(color.b())
    }

    /// WCAG contrast ratio between two opaque colours.
    fn contrast_ratio(a: egui::Color32, b: egui::Color32) -> f32 {
        let (la, lb) = (relative_luminance(a), relative_luminance(b));
        let (lighter, darker) = if la >= lb { (la, lb) } else { (lb, la) };
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn the_palette_meets_the_documented_contrast_floor() {
        // docs/UI.md §6.5: body text ≥ 4.5:1, accent (links/selection) ≥ 3:1.
        for pal in [Palette::DARK, Palette::LIGHT] {
            for background in [pal.panel, pal.view, pal.surface_2] {
                assert!(
                    contrast_ratio(pal.text, background) >= 4.5,
                    "text on {background:?} is {:.2}:1",
                    contrast_ratio(pal.text, background)
                );
                assert!(
                    contrast_ratio(pal.muted, background) >= 4.5,
                    "muted on {background:?} is {:.2}:1",
                    contrast_ratio(pal.muted, background)
                );
                assert!(
                    contrast_ratio(pal.accent, background) >= 3.0,
                    "accent on {background:?} is {:.2}:1",
                    contrast_ratio(pal.accent, background)
                );
            }
        }
    }
    fn sample_pe() -> Option<Vec<u8>> {
        for path in [
            "C:/Windows/System32/notepad.exe",
            "C:/Windows/System32/kernel32.dll",
        ] {
            if let Ok(bytes) = std::fs::read(path)
                && !bytes.is_empty()
            {
                return Some(bytes);
            }
        }
        None
    }

    fn put_u16(file: &mut [u8], offset: usize, value: u16) {
        file[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(file: &mut [u8], offset: usize, value: u32) {
        file[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(file: &mut [u8], offset: usize, value: u64) {
        file[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    /// Minimal ELF64 x86-64 executable: one `PT_LOAD` plus the matching `.text` section
    /// (`mov eax, 1; ret` at the entry point). The loader takes the address space from
    /// `SHF_ALLOC` sections, so the section is not optional even though the program header
    /// alone would describe the mapping. Built here like the PE fixture so the importer test
    /// needs no Linux toolchain.
    fn sample_elf() -> Vec<u8> {
        const TEXT_OFFSET: usize = 0x1000;
        const TEXT_VA: u64 = 0x401_000;
        const SHOFF: usize = 0x100;
        let mut file = vec![0u8; TEXT_OFFSET + 0x20];

        file[..4].copy_from_slice(b"\x7fELF");
        file[4] = 2; // ELFCLASS64
        file[5] = 1; // ELFDATA2LSB
        file[6] = 1; // EV_CURRENT
        put_u16(&mut file, 16, 2); // ET_EXEC
        put_u16(&mut file, 18, 0x3e); // EM_X86_64
        put_u32(&mut file, 20, 1);
        put_u64(&mut file, 24, TEXT_VA); // e_entry
        put_u64(&mut file, 32, 0x40); // e_phoff
        put_u64(&mut file, 40, SHOFF as u64); // e_shoff
        put_u16(&mut file, 52, 64); // e_ehsize
        put_u16(&mut file, 54, 56); // e_phentsize
        put_u16(&mut file, 56, 1); // e_phnum
        put_u16(&mut file, 58, 64); // e_shentsize
        put_u16(&mut file, 60, 2); // e_shnum (null + .text)
        put_u16(&mut file, 62, 0); // e_shstrndx: no section names

        put_u32(&mut file, 0x40, 1); // PT_LOAD
        put_u32(&mut file, 0x44, 5); // PF_R | PF_X
        put_u64(&mut file, 0x48, TEXT_OFFSET as u64);
        put_u64(&mut file, 0x50, TEXT_VA);
        put_u64(&mut file, 0x58, TEXT_VA);
        put_u64(&mut file, 0x60, 0x10);
        put_u64(&mut file, 0x68, 0x10);
        put_u64(&mut file, 0x70, 0x1000);

        put_u32(&mut file, SHOFF + 64, 0); // sh_name
        put_u32(&mut file, SHOFF + 68, 1); // SHT_PROGBITS
        put_u64(&mut file, SHOFF + 72, 0x6); // SHF_ALLOC | SHF_EXECINSTR
        put_u64(&mut file, SHOFF + 80, TEXT_VA); // sh_addr
        put_u64(&mut file, SHOFF + 88, TEXT_OFFSET as u64); // sh_offset
        put_u64(&mut file, SHOFF + 96, 0x10); // sh_size

        file[TEXT_OFFSET..TEXT_OFFSET + 6].copy_from_slice(&[0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3]);
        file
    }

    /// Guardrail for the CJK fallback tweak: epaint centres a fallback face inside the
    /// row instead of aligning baselines, so a CJK face with a taller ascent drifts
    /// below the Latin text of the same line. Monospace text (output pane, listing,
    /// status bar) must stay aligned. Skips when the Windows fonts are unavailable.
    ///
    /// `Glyph::pos.y` is the face's own baseline; epaint shifts the rendered bitmap by
    /// `round(size * y_offset_factor)`, so that is the visible correction.
    #[test]
    fn cjk_monospace_baseline_matches_latin() {
        use egui::emath::GuiRounding as _;

        let Some(fonts) = ui_fonts() else {
            return;
        };
        assert_eq!(
            fonts.font_data["cjk_mono"].tweak.y_offset_factor, MONO_CJK_Y_OFFSET_FACTOR,
            "the monospace CJK fallback must carry the baseline tweak"
        );
        let ctx = egui::Context::default();
        ctx.set_fonts(fonts);
        let mut output = ctx.run_ui(Default::default(), |_| {});
        output.textures_delta.clear();
        // 11 px is the output pane, 12/12.5 the listing, 20 the zoomed-in listing.
        for size in [11.0, 12.0, MONO_SIZE, 13.0, 20.0] {
            let mut job = egui::text::LayoutJob::default();
            job.append(
                "已改 0x14",
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::monospace(size),
                    ..Default::default()
                },
            );
            let galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));
            let baseline = |ch: char| {
                galley.rows[0]
                    .glyphs
                    .iter()
                    .find(|glyph| glyph.chr == ch)
                    .map(|glyph| glyph.pos.y)
                    .expect("glyph is in the galley")
            };
            let applied = (size * MONO_CJK_Y_OFFSET_FACTOR).round_ui();
            let offset = baseline('已') + applied - baseline('0');
            assert!(
                offset.abs() <= 1.0,
                "monospace {size}px: CJK baseline is {offset}px off the Latin baseline \
                 after the tweak; re-tune MONO_CJK_Y_OFFSET_FACTOR"
            );
        }
    }

    /// Pane the dock shows in its focused leaf: the view that owns the screen right now.
    fn active_pane(app: &mut ReCraftApp) -> Option<Pane> {
        app.dock
            .as_mut()
            .and_then(|state| state.find_active_focused())
            .map(|(_, tab)| *tab)
    }

    /// Headless layout smoke test: real `Session` + real widget code, no window.
    /// Verifies the grid actually emits shapes and that a frame stays fast enough
    /// for interactive use (this is our stand-in for a visual check in CI).
    #[test]
    fn headless_layout_renders_disassembly_grid() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ui-smoke.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();

        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };

        let mut max_frame_ms = 0.0f64;
        let mut max_shapes = 0usize;
        for frame in 0..3 {
            input.time = Some(frame as f64 / 60.0);
            let started = Instant::now();
            let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            max_frame_ms = max_frame_ms.max(started.elapsed().as_secs_f64() * 1000.0);
            let shapes = output.shapes.len();
            max_shapes = max_shapes.max(shapes);
            // Without a renderer nobody uploads the font/texture deltas.
            output.textures_delta.clear();
        }

        assert_eq!(app.disasm_rows.len(), DISASM_ROWS);
        assert!(
            max_shapes > 100,
            "expected a populated frame, got {max_shapes} shapes"
        );
        // Headless software rendering on a shared CI runner is several times slower than
        // on a dev box (652 ms observed on windows-latest), so the budget is set to catch
        // pathological regressions — multi-second stalls — rather than micro-timing.
        assert!(
            max_frame_ms < 2000.0,
            "first frames should stay interactive, took {max_frame_ms:.1} ms"
        );
    }

    /// Render every side-panel tab against a real analysed project (notepad), so the
    /// strings/imports/xrefs views are exercised without a window.
    #[test]
    fn headless_side_panels_render() {
        let Ok(bytes) = std::fs::read("C:/Windows/System32/notepad.exe") else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ui-panels.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let strings = recraft_analysis::extract_strings(&mut session, 4, 200_000).unwrap();
        recraft_analysis::persist_strings(&mut session, &strings).unwrap();

        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        // Exercise the collapsible thinking section in the AI pane.
        app.ai_thinking = true;
        app.ai_reasoning_open = true;
        app.ai_reasoning = "check the loop bound and the buffer size".to_owned();

        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };

        for pane in [
            Pane::Functions,
            Pane::Strings,
            Pane::Imports,
            Pane::Names,
            Pane::Segments,
            Pane::Ai,
            Pane::Structs,
            Pane::Hex,
            Pane::Pseudo,
            Pane::View,
            Pane::Exports,
            Pane::GraphOverview,
            Pane::Output,
        ] {
            app.focus_pane(pane);
            for frame in 0..2 {
                input.time = Some(frame as f64 / 60.0);
                let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
                output.textures_delta.clear();
            }
            assert!(
                !app.disasm_rows.is_empty(),
                "disassembly empty while rendering {pane:?}"
            );
            assert!(
                app.dock
                    .as_ref()
                    .and_then(|state| state.find_tab(&pane))
                    .is_some(),
                "pane {pane:?} missing from the dock"
            );
        }
        assert!(!app.strings.is_empty(), "strings panel data missing");
        assert!(!app.imports.is_empty(), "imports panel data missing");

        // The xrefs dialog (`X`) renders over the dock instead of owning a tab.
        app.apply_action(&ctx, Action::ShowXrefs);
        for frame in 0..2 {
            input.time = Some(frame as f64 / 60.0);
            let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            output.textures_delta.clear();
        }
        assert!(app.xref_dialog.is_some(), "xrefs dialog did not open");
        app.xref_dialog = None;

        // Code View: build and render the CFG for the entry function.
        app.tab = Tab::Disasm;
        app.view_mode = ViewMode::Graph;
        app.cfg_dirty = true;
        app.cfg_for = None;
        for frame in 0..2 {
            input.time = Some(frame as f64 / 60.0);
            let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            output.textures_delta.clear();
        }
        let cfg = app
            .cfg
            .as_ref()
            .expect("CFG not built for the selected function");
        assert!(!cfg.blocks.is_empty(), "CFG has no blocks");

        // The Decompile view renders the structured pseudo-C (signature + argument names)
        // without an L1/L2 toggle.
        app.tab = Tab::Pseudo;
        app.pseudo_dirty = true;
        for frame in 0..2 {
            input.time = Some(frame as f64 / 60.0);
            let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            output.textures_delta.clear();
        }
        let pseudo = app.pseudo.as_ref().expect("pseudo-C was not produced");
        assert!(
            pseudo
                .iter()
                .any(|line| line.text.contains("structure recovery")),
            "structure header missing"
        );
        assert!(
            pseudo.iter().any(|line| !line.text.contains("_l1")
                && line.text.ends_with('{')
                && (line.text.starts_with("void ")
                    || line.text.starts_with("int32 ")
                    || line.text.starts_with("int64 "))),
            "structured signature missing"
        );

        // AI proposal diff panel: render suggestions, then apply one through the bus.
        let target = app.functions.first().expect("functions").start;
        app.ai_proposals = vec![recraft_ai::Proposal {
            kind: recraft_ai::ProposalKind::Rename,
            target,
            value: "ai_suggested_name".to_owned(),
            confidence: 0.5,
            rationale: "unit test evidence".to_owned(),
            evidence: vec![recraft_ai::Evidence {
                addr: target,
                kind: "api_tag",
                text: "test".to_owned(),
            }],
            source: recraft_ai::ProposalSource::External,
        }];
        app.ai_checked.insert(target);
        app.focus_pane(Pane::Ai);
        for frame in 0..2 {
            input.time = Some(frame as f64 / 60.0);
            let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            output.textures_delta.clear();
        }
        assert_eq!(app.ai_proposals.len(), 1, "AI panel lost the proposal");
        assert!(app.apply_ai_selection(), "AI apply failed");
        let session = app.session.as_ref().unwrap();
        assert_eq!(
            session.name_at(target).unwrap().as_deref(),
            Some("ai_suggested_name")
        );
        assert!(app.ai_proposals.is_empty(), "applied proposal not removed");
    }

    /// Ticking several proposals applies all of them, and the panel reports the count
    /// instead of pretending only one landed.
    #[test]
    fn ai_apply_batch_renames_every_checked_proposal() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ai-batch.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_functions();
        let targets: Vec<Address> = app
            .functions
            .iter()
            .take(2)
            .map(|function| function.start)
            .collect();
        if targets.len() < 2 {
            return;
        }
        app.ai_proposals = targets
            .iter()
            .enumerate()
            .map(|(index, target)| recraft_ai::Proposal {
                kind: recraft_ai::ProposalKind::Rename,
                target: *target,
                value: format!("batch_name_{index}"),
                confidence: 0.8,
                rationale: "batch test".to_owned(),
                evidence: Vec::new(),
                source: recraft_ai::ProposalSource::Model,
            })
            .collect();
        for target in &targets {
            app.ai_checked.insert(*target);
        }
        assert!(app.apply_ai_selection(), "batch apply failed");
        let session = app.session.as_ref().unwrap();
        for (index, target) in targets.iter().enumerate() {
            assert_eq!(
                session.name_at(*target).unwrap().as_deref(),
                Some(format!("batch_name_{index}").as_str())
            );
        }
        assert!(app.ai_proposals.is_empty(), "applied proposals not removed");
        assert!(app.ai_checked.is_empty(), "checks not cleared");
        assert!(app.ai_status.contains('2'), "{}", app.ai_status);
    }

    #[test]
    fn x_shortcut_opens_the_xrefs_dialog() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ui-xrefs.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        let ctx = egui::Context::default();
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let mut first = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
        first.textures_delta.clear();
        // F7 is not a shortcut for the xrefs dialog any more; only `X` opens it.
        input.events.push(egui::Event::Key {
            key: egui::Key::F7,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let mut second = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
        second.textures_delta.clear();
        assert!(
            app.xref_dialog.is_none(),
            "F7 must not open the xrefs dialog"
        );
        input.events.clear();
        input.events.push(egui::Event::Key {
            key: egui::Key::X,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let mut third = ctx.run_ui(input, |ui| app.update_inner(ui));
        third.textures_delta.clear();
        let dialog = app
            .xref_dialog
            .as_ref()
            .expect("X should open the xrefs dialog");
        assert_eq!(
            Some(dialog.target),
            app.disasm_rows.get(app.selected).map(|line| line.addr),
            "the dialog must open on the selected address"
        );
    }

    #[test]
    fn xref_labels_follow_ida_style() {
        let main = Address::new(0x140001000);
        let inside = Address::new(0x1400010b9);
        assert_eq!(
            xref_label(main, Some("main"), Some(("main", main)), true),
            "main"
        );
        assert_eq!(
            xref_label(inside, None, Some(("main", main)), true),
            "main:loc_1400010b9"
        );
        assert_eq!(
            xref_label(inside, Some("decode_loop"), Some(("main", main)), true),
            "main:decode_loop"
        );
        assert_eq!(
            xref_label(Address::new(0x140003000), None, None, false),
            "unk_140003000"
        );
        assert_eq!(
            xref_label(Address::new(0x140003000), Some("g_flag"), None, false),
            "g_flag"
        );
    }

    #[test]
    fn data_xref_letters_split_offsets_reads_and_writes() {
        assert_eq!(data_xref_letter("lea rdx, [rip+0x3000]"), 'o');
        assert_eq!(data_xref_letter("mov rax, [rbx]"), 'r');
        assert_eq!(data_xref_letter("mov [rsp+8], rax"), 'w');
        assert_eq!(data_xref_letter("cmp [rbx], eax"), 'r');
        assert_eq!(data_xref_letter("db 48 65"), 'r');
    }

    #[test]
    fn elide_chars_keeps_short_text() {
        assert_eq!(elide_chars("flag{abc}", 32), "flag{abc}");
        assert_eq!(elide_chars("abcdef", 4), "abc…");
    }

    #[test]
    fn xref_search_matches_every_column() {
        let rows = [
            XrefRow {
                up: true,
                kind: 'o',
                kind_name: "data",
                addr: Address::new(0x140001321),
                label: "sub_1400012dc:loc_140001321".to_owned(),
                text: "lea rcx, [0x14002e7f0]  ; 'ETW0'".to_owned(),
            },
            XrefRow {
                up: false,
                kind: 'c',
                kind_name: "call",
                addr: Address::new(0x140001000),
                label: "sub_140001000".to_owned(),
                text: "push rbp".to_owned(),
            },
        ];
        assert_eq!(xref_matches(&rows, ""), vec![0, 1]);
        assert_eq!(xref_matches(&rows, "etw0"), vec![0], "text column");
        assert_eq!(
            xref_matches(&rows, "0x140001321"),
            vec![0],
            "address column"
        );
        assert_eq!(
            xref_matches(&rows, "sub_140001000"),
            vec![1],
            "label column"
        );
        assert!(xref_matches(&rows, "no-such-row").is_empty());
    }

    /// The xrefs dialog mirrors IDA's: `Up` rows carry the referencing instruction plus
    /// the referenced literal, `Down` rows describe the target.
    #[test]
    fn xrefs_table_matches_ida_layout() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ui-xref-table.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let strings = recraft_analysis::extract_strings(&mut session, 4, 200_000).unwrap();
        recraft_analysis::persist_strings(&mut session, &strings).unwrap();

        // A string taken with `lea`: IDA lists it as `Up  o  main:loc_…  lea rdx, Str2`.
        let mut picked = None;
        for xref in session
            .all_xrefs(Some(recraft_analysis::XREF_DATA))
            .unwrap()
        {
            if !strings.iter().any(|row| row.addr == xref.to) {
                continue;
            }
            let Ok(lines) = session.disasm(xref.from, 1) else {
                continue;
            };
            if lines.first().is_some_and(|line| line.mnemonic == "lea") {
                picked = Some(xref);
                break;
            }
        }
        let Some(xref) = picked else {
            return;
        };
        let target = xref.to;

        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        // `after_open` only marks the side lists dirty; load them before building rows so
        // function labels and string literals resolve.
        app.refresh_functions();
        app.refresh_strings();
        assert!(!app.functions.is_empty(), "analysis produced no functions");
        let ctx = egui::Context::default();
        app.disasm_start = target;
        app.disasm_dirty = true;
        app.refresh_disasm();
        app.selected = 0;
        app.apply_action(&ctx, Action::ShowXrefs);

        let dialog = app.xref_dialog.as_ref().expect("X did not open the dialog");
        assert_eq!(dialog.target, target, "dialog must freeze on the selection");
        let row = dialog
            .rows
            .iter()
            .find(|row| row.up && row.addr == xref.from)
            .expect("missing Up row for the lea reference");
        assert_eq!(row.kind, 'o', "lea references are offsets: {row:?}");
        assert!(
            row.label.contains(':'),
            "address label should be `func:loc_…`, got {:?}",
            row.label
        );
        assert!(row.text.starts_with("lea"), "text: {:?}", row.text);
        assert!(
            row.text.contains('\'') && row.text.ends_with('\''),
            "referenced literal should be appended: {:?}",
            row.text
        );

        // The same reference seen from the instruction: a `Down` row onto the literal.
        app.disasm_start = xref.from;
        app.disasm_dirty = true;
        app.refresh_disasm();
        app.selected = 0;
        app.apply_action(&ctx, Action::ShowXrefs);
        let dialog = app
            .xref_dialog
            .as_ref()
            .expect("reopening the dialog failed");
        assert_eq!(dialog.target, xref.from);
        let down = dialog
            .rows
            .iter()
            .find(|row| !row.up && row.addr == target)
            .expect("missing Down row for the lea target");
        assert_eq!(down.kind, 'o', "lea is still an offset: {down:?}");
        assert!(
            down.text.starts_with("db '"),
            "Down text should show the literal: {:?}",
            down.text
        );

        // `Down` rows point at the callee and describe it.
        let call = app
            .session
            .as_ref()
            .unwrap()
            .all_xrefs(Some(recraft_analysis::XREF_CALL))
            .unwrap()
            .into_iter()
            .next()
            .expect("no call xref in the sample");
        let down = xref_row(
            app.session.as_mut().unwrap(),
            &[],
            &[],
            &[],
            call.from,
            call,
            false,
        );
        assert!(!down.up);
        assert_eq!(down.kind, 'c');
        assert_eq!(down.addr, call.to);
        assert!(!down.text.is_empty(), "Down row has no text");

        // The dialog is a window over the listing, not a dock tab.
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let mut shapes = 0;
        for frame in 0..2 {
            input.time = Some(frame as f64 / 60.0);
            let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            output.textures_delta.clear();
            shapes += output.shapes.len();
        }
        assert!(shapes > 0, "the xrefs dialog drew nothing");
        assert!(app.xref_dialog.is_some(), "the dialog should stay open");

        // Enter is IDA's OK: follow the selected row and close.
        input.events.clear();
        input.events.push(egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        input.time = Some(0.1);
        let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
        output.textures_delta.clear();
        assert!(app.xref_dialog.is_none(), "Enter should close the dialog");
        assert_eq!(
            app.disasm_start, target,
            "Enter should follow the selected row"
        );

        // Esc closes it too, like IDA's Cancel.
        app.apply_action(&ctx, Action::ShowXrefs);
        input.events.clear();
        input.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        input.time = Some(0.2);
        let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
        output.textures_delta.clear();
        assert!(app.xref_dialog.is_none(), "Esc should close the dialog");
    }

    #[test]
    fn follow_tokens_extracts_addresses_and_symbols() {
        assert_eq!(
            follow_tokens("call 0x140001000"),
            vec!["call", "0x140001000"]
        );
        let tokens = follow_tokens("mov rax, qword ptr [rip + 0x2a0]");
        assert!(tokens.contains(&"0x2a0".to_owned()));
        assert!(follow_tokens("call sub_140001234").contains(&"sub_140001234".to_owned()));
        assert!(follow_tokens("ret").len() == 1);
    }

    #[test]
    fn recent_projects_dedupe_and_cap() {
        let mut recent = Vec::new();
        for index in 0..10 {
            push_recent(&mut recent, &PathBuf::from(format!("C:/x/p{index}.rcraft")));
        }
        assert_eq!(recent.len(), 8);
        assert_eq!(recent[0], PathBuf::from("C:/x/p9.rcraft"));
        push_recent(&mut recent, &PathBuf::from("C:/x/p5.rcraft"));
        assert_eq!(recent.len(), 8);
        assert_eq!(recent[0], PathBuf::from("C:/x/p5.rcraft"));
        assert_eq!(
            recent
                .iter()
                .filter(|path| path.as_path() == std::path::Path::new("C:/x/p5.rcraft"))
                .count(),
            1
        );
    }

    #[test]
    fn follow_navigates_to_operand_target() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("follow.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_disasm();
        // Drive the resolver from a synthetic call line: the sample PE's entry stub may
        // contain no absolute references of its own.
        let target = app.session.as_ref().expect("session").entry_point();
        let mut line = app
            .disasm_rows
            .first()
            .cloned()
            .expect("at least one disassembly row");
        line.operands = format!("call {:#x}", target.get());
        app.disasm_rows[0] = line;
        let resolved = app.follow_target(&app.disasm_rows[0]);
        assert_eq!(resolved, Some(target), "call operand should resolve");
        app.selected = 0;
        app.disasm_start = Address::ZERO;
        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::Follow);
        assert_eq!(
            app.disasm_start, target,
            "follow should navigate to the operand"
        );
        assert!(matches!(app.tab, Tab::Disasm));
    }

    #[test]
    fn welcome_panel_renders_without_a_session() {
        let mut app = ReCraftApp::default();
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
        output.textures_delta.clear();
        assert!(app.session.is_none());
    }

    #[test]
    fn closing_a_project_unloads_it_and_returns_to_the_welcome_screen() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("close.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project.clone()),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_disasm();
        app.names = vec![(app.disasm_start, "kept_name".to_owned())];
        app.pseudo = Some(vec![PseudoLine {
            addr: app.disasm_start,
            text: "int main(void)".to_owned(),
        }]);
        app.cfg_for = Some(app.disasm_start);
        app.history.push(Address::new(0x140001234));
        app.ai_explanation = Some("stale answer".to_owned());
        app.ai_proposals = vec![recraft_ai::Proposal {
            kind: recraft_ai::ProposalKind::Rename,
            target: app.disasm_start,
            value: "stale".to_owned(),
            confidence: 1.0,
            rationale: "old project".to_owned(),
            evidence: Vec::new(),
            source: recraft_ai::ProposalSource::External,
        }];
        assert!(!app.disasm_rows.is_empty());
        assert!(!app.history.is_empty());

        app.apply_action(&egui::Context::default(), Action::CloseProject);

        assert!(app.session.is_none(), "the session must be dropped");
        assert!(app.project.is_none(), "no project stays open");
        assert!(app.disasm_rows.is_empty(), "the listing is cleared");
        assert!(app.pseudo.is_none(), "the decompile view is cleared");
        assert!(app.names.is_empty(), "stored names are cleared");
        assert!(app.history.is_empty(), "navigation history is cleared");
        assert_eq!(app.history_pos, 0);
        assert!(app.cfg_for.is_none());
        assert!(app.ai_explanation.is_none());
        assert!(app.ai_proposals.is_empty());
        assert!(!app.is_error, "closing is not an error");
        assert!(
            app.status.contains(&project.display().to_string()),
            "the status bar should name the closed project: {}",
            app.status
        );

        // The next frame must render the welcome screen without touching a stale session.
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
        output.textures_delta.clear();
        assert!(app.session.is_none());
        assert!(app.disasm_rows.is_empty());
    }

    /// The welcome page is the Disassembly pane's no-project state, so it is only visible
    /// while that tab is in front. A layout restored from the last session can leave an
    /// empty Decompile tab there; with no project the app must bring the open/import hints
    /// back instead of showing a blank document.
    #[test]
    fn no_project_brings_the_welcome_tab_to_the_front() {
        let mut app = ReCraftApp::default();
        app.ensure_dock();
        app.focus_pane(Pane::Pseudo);
        assert_eq!(active_pane(&mut app), Some(Pane::Pseudo));

        app.show_welcome_tab();
        assert_eq!(
            active_pane(&mut app),
            Some(Pane::View),
            "no project: the welcome page must own the screen"
        );

        // The rule fires when the project goes away, not every frame: a tab the user picks
        // afterwards stays put.
        app.focus_pane(Pane::Hex);
        app.tick();
        assert_eq!(active_pane(&mut app), Some(Pane::Hex));
    }

    #[test]
    fn the_aarch64_elf_fixture_imports_from_the_open_path() {
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/aarch64/tiny.elf");
        let Ok(bytes) = std::fs::read(&fixture) else {
            eprintln!("skipping: {} is missing", fixture.display());
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("tiny.elf");
        std::fs::write(&source, &bytes).unwrap();
        let mut app = ReCraftApp::default();
        app.ensure_dock();

        app.open_path(&source);

        assert!(
            !app.is_error,
            "importing the ELF fixture must not report an error"
        );
        assert!(app.session.is_some(), "the fixture must load");
        let project = app.project.clone().expect("a project path is set");
        assert_eq!(
            project.extension().and_then(|value| value.to_str()),
            Some("rcraft")
        );
    }

    #[test]
    fn a_failed_open_keeps_the_welcome_page_in_front() {
        let dir = tempfile::tempdir().unwrap();
        let broken = dir.path().join("broken.rcraft");
        std::fs::write(&broken, b"not a project").unwrap();
        let mut app = ReCraftApp::default();
        app.ensure_dock();
        app.focus_pane(Pane::Pseudo);

        app.open_path(&broken);

        assert!(app.session.is_none(), "a broken project must not load");
        assert!(app.is_error, "the failure must be reported");
        assert_eq!(
            active_pane(&mut app),
            Some(Pane::View),
            "with no project the welcome page stays in front"
        );
    }

    #[test]
    fn closing_a_project_brings_the_welcome_tab_to_the_front() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("welcome-after-close.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project),
            ..ReCraftApp::default()
        };
        app.ensure_dock();
        app.focus_pane(Pane::Pseudo);

        // An open project keeps whatever tab the user put in front.
        app.show_welcome_tab();
        assert_eq!(
            active_pane(&mut app),
            Some(Pane::Pseudo),
            "the welcome rule must not steal the tab while a project is open"
        );

        app.apply_action(&egui::Context::default(), Action::CloseProject);
        assert_eq!(
            active_pane(&mut app),
            Some(Pane::View),
            "closing the project must show the welcome page again"
        );
    }

    #[test]
    fn closing_a_project_cancels_the_background_analysis() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("close-analysis.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let (_sender, receiver) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project),
            analysis: Some(AnalysisJob {
                receiver,
                cancel: cancel.clone(),
                started: Instant::now(),
            }),
            ..ReCraftApp::default()
        };

        app.apply_action(&egui::Context::default(), Action::CloseProject);

        assert!(
            cancel.load(Ordering::Relaxed),
            "the worker must be told to stop before its connection is orphaned"
        );
        assert!(
            app.analysis.is_none(),
            "the finished job must not report into the next project"
        );
        assert!(app.session.is_none());
    }

    #[test]
    fn closing_without_a_project_keeps_the_status() {
        let mut app = ReCraftApp::default();
        app.set_status("welcome");
        app.close_project();
        assert_eq!(app.status, "welcome");
    }

    #[test]
    fn ctrl_w_closes_the_open_project() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ctrl-w.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project),
            ..ReCraftApp::default()
        };
        app.after_open();
        let ctx = egui::Context::default();
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let mut first = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
        first.textures_delta.clear();
        let command = egui::Modifiers {
            command: true,
            ..egui::Modifiers::NONE
        };
        input.events.push(egui::Event::ModifiersChanged(command));
        input.events.push(egui::Event::Key {
            key: egui::Key::W,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: command,
        });
        let mut second = ctx.run_ui(input, |ui| app.update_inner(ui));
        second.textures_delta.clear();
        assert!(
            app.session.is_none() && app.project.is_none(),
            "Ctrl+W must close the project"
        );
        assert!(app.disasm_rows.is_empty());
    }

    #[test]
    fn palette_close_project_command_closes_the_project() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("palette-close.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project),
            ..ReCraftApp::default()
        };
        app.after_open();
        let index = PALETTE_COMMANDS
            .iter()
            .position(|(_, action)| *action == Action::CloseProject)
            .expect("the palette offers Close project");
        app.apply_action(&egui::Context::default(), Action::PaletteCommand(index));
        assert!(app.session.is_none());
        assert!(app.project.is_none());
    }

    /// IDA's `F5` decompiles the function under the cursor; analysis now starts on import,
    /// so the key must never start it again.
    #[test]
    fn f5_decompiles_instead_of_analyzing() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("f5-key.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project),
            ..ReCraftApp::default()
        };
        app.after_open();
        let ctx = egui::Context::default();
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let mut first = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
        first.textures_delta.clear();
        input.events.push(egui::Event::Key {
            key: egui::Key::F5,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let mut second = ctx.run_ui(input, |ui| app.update_inner(ui));
        second.textures_delta.clear();
        assert!(
            matches!(app.tab, Tab::Pseudo),
            "F5 must open the Decompile view"
        );
        assert!(
            app.analysis.is_none(),
            "F5 must not start analysis; importing already did"
        );
    }

    /// `F5` follows the cursor: the pane re-anchors on the cursor's function and the listing
    /// centres the cursor's line once its rows exist.
    #[test]
    fn f5_centres_the_cursor_line_in_the_decompile_view() {
        let Ok(bytes) = std::fs::read("C:/Windows/System32/notepad.exe") else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("f5-scroll.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project),
            ..ReCraftApp::default()
        };
        app.after_open();
        // Text mode puts the cursor on a disassembly row; Graph mode anchors on the graph's
        // function instead. `cursor_address` covers both.
        app.view_mode = ViewMode::Text;
        app.tick();
        let address = app
            .disasm_rows
            .get(1)
            .expect("the entry listing must have rows")
            .addr;
        app.selected = 1;
        app.apply_action(&egui::Context::default(), Action::Decompile);
        assert!(matches!(app.tab, Tab::Pseudo));
        assert_eq!(app.pseudo_start, address, "the pane must follow the cursor");
        assert_eq!(
            app.pseudo_scroll_to,
            Some(address),
            "F5 must ask the listing to centre the cursor line"
        );
        app.tick();
        assert!(
            app.pseudo.as_ref().is_some_and(|lines| !lines.is_empty()),
            "the function under the cursor must be decompiled"
        );
        // Rendering consumes the one-shot scroll request.
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
        output.textures_delta.clear();
        assert!(
            app.pseudo_scroll_to.is_none(),
            "the scroll request is one-shot"
        );
    }

    /// `Tab` is IDA's view flip; the auxiliary panes keep their own keys, so the binding is
    /// only read off the tab. `Space` is the disassembly view's own Graph/Text switch, so the
    /// pseudo-C view must leave it alone instead of jumping back to the listing.
    #[test]
    fn tab_key_flips_the_two_code_views() {
        let ctx = egui::Context::default();
        let mut app = ReCraftApp::default();
        let run = |app: &mut ReCraftApp, key: Key| -> Action {
            let mut action = Action::None;
            let input = egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..egui::RawInput::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                let _ = ui;
                action = app.handle_keys(&ctx);
            });
            output.textures_delta.clear();
            action
        };
        app.tab = Tab::Disasm;
        assert_eq!(
            run(&mut app, Key::Tab),
            Action::ToggleDisasmPseudo,
            "Tab must flip the code views, not edit the listing"
        );
        app.tab = Tab::Pseudo;
        assert_eq!(run(&mut app, Key::Tab), Action::ToggleDisasmPseudo);
        app.tab = Tab::Disasm;
        assert_eq!(
            run(&mut app, Key::Space),
            Action::ToggleViewMode,
            "Space stays the Graph/Text switch"
        );
        app.tab = Tab::Pseudo;
        assert_eq!(
            run(&mut app, Key::Space),
            Action::None,
            "Space belongs to the disassembly view, not to the pseudo-C view"
        );
        // IDA edits pseudocode comments with `/`; `;` is accepted as an alias because it is
        // the comment key everywhere else in the app. The disassembly keeps `;` for its own
        // (separately stored) comment slot.
        assert_eq!(
            run(&mut app, Key::Slash),
            Action::EditPseudoComment { addr: None },
            "/ edits the pseudocode comment"
        );
        assert_eq!(
            run(&mut app, Key::Semicolon),
            Action::EditPseudoComment { addr: None },
            "; is an alias inside the Decompile view"
        );
        app.tab = Tab::Disasm;
        assert_eq!(run(&mut app, Key::Semicolon), Action::EditComment);
        assert_eq!(
            run(&mut app, Key::Slash),
            Action::None,
            "/ is not a disassembly shortcut"
        );
    }

    /// `scroll_to_rect(Align::Center)` centres both axes, so centring a row wider than the
    /// pane used to drag the pseudo-C listing (and its line numbers) sideways whenever `F5`
    /// or `Tab` re-centred it. The view's scroll target spans the visible width instead,
    /// which leaves the horizontal offset alone.
    #[test]
    fn centring_the_pseudo_c_row_does_not_pan_sideways() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(360.0, 240.0));
        let mut offset = egui::Vec2::ZERO;
        for _ in 0..4 {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                ..egui::RawInput::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                ui.style_mut().scroll_animation = egui::style::ScrollAnimation::none();
                // The view allocates rows wider than the pane (it keeps 1100 pt of code) and
                // then centres the cursor row.
                let scroll = egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let viewport = ui.clip_rect();
                        for index in 0..200 {
                            let (rect, _) =
                                ui.allocate_exact_size(egui::vec2(1100.0, 16.0), Sense::hover());
                            if index == 150 {
                                ui.scroll_to_rect(
                                    egui::Rect::from_min_max(
                                        egui::pos2(viewport.left(), rect.top()),
                                        egui::pos2(viewport.right(), rect.bottom()),
                                    ),
                                    Some(egui::Align::Center),
                                );
                            }
                        }
                    });
                offset = scroll.state.offset;
            });
            output.textures_delta.clear();
        }
        assert!(
            offset.y > 0.0,
            "the row must be centred vertically, got {offset:?}"
        );
        assert_eq!(
            offset.x, 0.0,
            "centring a row must not pan the listing sideways, got {offset:?}"
        );
    }

    /// From the listing, `Tab` is exactly `F5`: the cursor's function opens in Decompile.
    #[test]
    fn tab_from_the_listing_decompiles_the_cursor_function() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("tab-open.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.view_mode = ViewMode::Text;
        app.tick();
        app.tab = Tab::Disasm;
        let cursor = app
            .disasm_rows
            .first()
            .map(|row| row.addr)
            .expect("the entry listing must have rows");
        app.apply_action(&egui::Context::default(), Action::ToggleDisasmPseudo);
        assert!(
            matches!(app.tab, Tab::Pseudo),
            "Tab opens the Decompile view"
        );
        assert_eq!(app.pseudo_start, cursor, "the pane must follow the cursor");
        assert_eq!(
            app.pseudo_scroll_to,
            Some(cursor),
            "Tab must centre the cursor line like F5"
        );
    }

    /// Inside the pseudo-C, `Tab` goes back to the listing row it came from: the row keeps
    /// its highlight, no history entry is added and a second `Tab` re-centres instead of
    /// decompiling the same function again.
    #[test]
    fn tab_from_the_pseudo_c_returns_to_its_listing_row() {
        let Ok(bytes) = std::fs::read("C:/Windows/System32/notepad.exe") else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("tab-toggle.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::Decompile);
        app.tick();
        let lines = app
            .pseudo
            .clone()
            .expect("the entry function must decompile");
        // Click a row in the middle of the listing, then leave with Tab. A frame runs
        // between the click and the key press, so the listing is reloaded by then.
        let index = lines.len() / 2;
        let addr = lines[index].addr;
        app.apply_action(&ctx, Action::SyncPseudoLine { addr, line: index });
        app.tick();
        let history = app.history.len();
        app.apply_action(&ctx, Action::ToggleDisasmPseudo);
        assert!(
            matches!(app.tab, Tab::Disasm),
            "Tab must leave the pseudo-C view"
        );
        assert_eq!(
            app.disasm_start, addr,
            "the listing must re-anchor on the row's address"
        );
        assert_eq!(
            app.history.len(),
            history,
            "Tab is a view flip, not a jump; it must not touch the history"
        );
        app.tick();
        assert_eq!(
            app.disasm_rows.first().map(|row| row.addr),
            Some(addr),
            "the row address must open the listing"
        );
        assert_eq!(
            app.active_pseudo_line(&lines),
            Some(index),
            "the row stays the active line across the flip"
        );
        let history = app.history.len();
        app.apply_action(&ctx, Action::ToggleDisasmPseudo);
        assert!(matches!(app.tab, Tab::Pseudo));
        assert_eq!(
            app.pseudo_scroll_to,
            Some(addr),
            "coming back must re-centre the same line"
        );
        assert!(
            !app.pseudo_dirty,
            "the function is already decompiled; Tab must not decompile it again"
        );
        assert_eq!(app.history.len(), history, "the way back is a flip too");
    }

    /// egui moves keyboard focus with Tab on its own, and the first press hands the ring to
    /// a focusable widget (the menu bar first). The app binds Tab, so every press must flip
    /// the views and no widget may keep the keyboard focus.
    #[test]
    fn tab_keeps_flipping_without_moving_the_keyboard_focus() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("tab-repeat.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project),
            ..ReCraftApp::default()
        };
        app.after_open();
        let ctx = egui::Context::default();
        let frame = |app: &mut ReCraftApp, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                events,
                ..egui::RawInput::default()
            };
            let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
            output.textures_delta.clear();
        };
        let tab = || {
            vec![egui::Event::Key {
                key: Key::Tab,
                physical_key: Some(Key::Tab),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]
        };
        // One frame without events, so the menu bar, the dock and the panes are laid out and
        // their widgets are known to egui.
        frame(&mut app, Vec::new());
        frame(&mut app, tab());
        assert!(
            matches!(app.tab, Tab::Pseudo),
            "the first Tab opens the Decompile view"
        );
        frame(&mut app, tab());
        assert!(
            matches!(app.tab, Tab::Disasm),
            "the second Tab must flip back, not move the focus ring"
        );
        assert!(
            ctx.memory(|memory| memory.focused()).is_none(),
            "Tab must not leave the keyboard focus on a widget"
        );
    }

    /// IDA analyses a binary as soon as it is loaded, so importing a PE must start the
    /// background pass without the user pressing anything.
    #[test]
    fn importing_a_pe_starts_analysis() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("sample.exe");
        std::fs::write(&source, &bytes).unwrap();
        let mut app = ReCraftApp::default();
        app.open_path(&source);
        assert!(app.session.is_some() && app.project.is_some());
        assert!(
            app.analysis.is_some(),
            "importing must start analysis, like IDA's auto-analysis"
        );
        // Drain the worker so the temporary project outlives its connection.
        let deadline = Instant::now() + std::time::Duration::from_secs(120);
        while app.analysis.is_some() && Instant::now() < deadline {
            app.poll_analysis();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(app.analysis.is_none(), "analysis must finish in time");
        assert!(
            app.functions_dirty,
            "a finished analysis must refresh the derived lists"
        );
    }

    /// The Import dialog used to filter on PE extensions only (`exe`/`dll`/`sys`), so an ELF
    /// binary — usually extensionless — could not be picked at all. Import itself is
    /// format-agnostic: `Session::create` sniffs the magic bytes.
    #[test]
    fn importing_an_elf_binary_starts_analysis() {
        let dir = tempfile::tempdir().unwrap();
        // ELF binaries commonly carry no extension (`a.out`, `busybox`).
        let source = dir.path().join("sample");
        std::fs::write(&source, sample_elf()).unwrap();
        let mut app = ReCraftApp::default();
        app.open_path(&source);
        let session = app
            .session
            .as_ref()
            .expect("an ELF import must open a session");
        assert_eq!(session.format(), recraft_core::Format::Elf);
        assert_eq!(session.arch(), recraft_core::Arch::X64);
        assert!(app.project.is_some());
        assert!(
            app.analysis.is_some(),
            "importing must start analysis, like IDA's auto-analysis"
        );
        let deadline = Instant::now() + std::time::Duration::from_secs(120);
        while app.analysis.is_some() && Instant::now() < deadline {
            app.poll_analysis();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(app.analysis.is_none(), "analysis must finish in time");
        assert!(
            app.functions_dirty,
            "a finished analysis must refresh the derived lists"
        );
    }

    /// Guardrail against the Import dialog drifting back to PE-only filters. The all-files
    /// fallback lives in [`ReCraftApp::apply_action`] and cannot be inspected headlessly.
    #[test]
    fn import_dialog_offers_elf_files_alongside_pe_ones() {
        for extension in ["exe", "dll", "sys"] {
            assert!(
                IMPORT_EXTENSIONS.contains(&extension),
                "the import filter lost the PE extension .{extension}"
            );
        }
        for extension in ["elf", "so"] {
            assert!(
                IMPORT_EXTENSIONS.contains(&extension),
                "the import filter must offer ELF .{extension} files"
            );
        }
    }

    #[test]
    fn string_address_renders_data_rows() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("strings.rcraft");
        let _ = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let _ = run_analysis_job(&project, &cancel);
        let session = Session::open(&project).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        app.refresh_strings();
        let row = app
            .strings
            .iter()
            .find(|row| !is_header_string(&row.segment) && !row.text.is_empty())
            .cloned()
            .expect("sample has at least one string");
        app.navigate(row.addr);
        app.apply_action(
            &egui::Context::default(),
            Action::NavigateToDisasm(row.addr),
        );
        app.tick();
        assert_eq!(
            app.view_mode,
            ViewMode::Text,
            "string navigation opens text mode"
        );
        let line = app.disasm_rows.first().expect("data rows");
        assert_eq!(line.addr, row.addr);
        assert_eq!(line.mnemonic, "db");
        assert!(
            line.operands.contains(&row.text),
            "operand should quote the string, got {:?}",
            line.operands
        );
        assert!(line.flags.is_data(), "string rows are data, not code");
    }

    /// IDA's graph view exists for instructions inside a function only: asking for it on a
    /// data row (or on an address outside every function) reports the switch and stays in
    /// text, and a jump onto data leaves graph mode the same way instead of painting an
    /// empty canvas.
    #[test]
    fn graph_view_falls_back_to_text_for_items_it_cannot_draw() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("graph-fallback.rcraft");
        let _ = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let _ = run_analysis_job(&project, &cancel);
        let session = Session::open(&project).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        app.refresh_strings();

        // Code inside a function keeps graph mode and builds its CFG.
        let function = app
            .functions
            .first()
            .expect("analysis found a function")
            .start;
        app.apply_action(
            &egui::Context::default(),
            Action::NavigateToDisasm(function),
        );
        app.tick();
        app.view_mode = ViewMode::Graph;
        app.tick();
        assert_eq!(app.view_mode, ViewMode::Graph, "code keeps graph mode");
        assert!(app.cfg.is_some(), "graph mode builds the function's CFG");

        // A jump onto a string's data row drops out of graph mode on the next tick.
        let data = app
            .strings
            .iter()
            .find(|row| !is_header_string(&row.segment) && !row.text.is_empty())
            .map(|row| row.addr)
            .expect("sample has at least one string");
        app.navigate(data);
        app.tick();
        assert_eq!(
            app.view_mode,
            ViewMode::Text,
            "graph mode cannot stay on a data row"
        );
        assert_eq!(app.status, app.t("ui_graph_view_needs_an_instruction"));
        assert!(!app.is_error, "the fallback is a notice, not an error");

        // Space on a data row must not flip the mode either, only report why.
        app.apply_action(&egui::Context::default(), Action::ToggleViewMode);
        assert_eq!(
            app.view_mode,
            ViewMode::Text,
            "Space cannot open a graph here"
        );
        assert_eq!(app.status, app.t("ui_graph_view_needs_an_instruction"));
    }

    #[test]
    fn overview_click_centres_the_graph_without_leaving_it() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("overview-click.rcraft");
        let _ = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let _ = run_analysis_job(&project, &cancel);
        let session = Session::open(&project).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        let function = app
            .functions
            .first()
            .expect("analysis found a function")
            .start;
        app.apply_action(
            &egui::Context::default(),
            Action::NavigateToDisasm(function),
        );
        app.view_mode = ViewMode::Graph;
        app.tick();
        assert!(app.cfg.is_some(), "graph mode builds the CFG");

        app.apply_action(&egui::Context::default(), Action::GraphGoTo(function));
        assert_eq!(
            app.view_mode,
            ViewMode::Graph,
            "the overview stays in graph mode"
        );
        assert!(app.tab == Tab::Disasm);
        assert_eq!(app.graph_center_on, Some(function));

        // Rendering the graph consumes the pending centre request without a mode change.
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
        output.textures_delta.clear();
        assert!(
            app.graph_center_on.is_none(),
            "the graph view must consume the pending centre"
        );
        assert_eq!(app.view_mode, ViewMode::Graph);
    }

    #[test]
    fn clicking_a_string_literal_in_pseudo_c_opens_its_data() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/x64/corpus.exe");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("literal.rcraft");
        let _ = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let _ = run_analysis_job(&project, &cancel);
        let session = Session::open(&project).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        let index = app
            .function_labels
            .iter()
            .position(|label| label == "corpus_banner")
            .expect("corpus_banner is an export");
        let start = app.functions[index].start;
        app.navigate(start);
        app.tick();
        let line = app
            .pseudo
            .as_ref()
            .unwrap()
            .iter()
            .find(|line| line.text.contains("return \"ReCraft analysis corpus\";"))
            .cloned()
            .expect("the banner function returns its string");
        let caret = line.text.find("analysis").unwrap();
        let action = app.pseudo_double_click(&line.text, caret, line.addr);
        app.apply_action(&egui::Context::default(), action);
        app.tick();
        assert!(matches!(app.tab, Tab::Disasm));
        let row = app.disasm_rows.first().expect("string data rows");
        assert_eq!(row.mnemonic, "db", "the string must render as data");
        assert!(row.operands.contains("ReCraft analysis corpus"), "{row:?}");
        assert!(row.flags.is_data(), "string rows are data, not code");
    }

    #[test]
    fn jumping_inside_a_stored_string_shows_the_tail_as_data() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/x64/corpus.exe");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("interior.rcraft");
        let _ = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let _ = run_analysis_job(&project, &cancel);
        let session = Session::open(&project).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        let row = app
            .strings
            .iter()
            .find(|row| row.text == "ReCraft analysis corpus")
            .cloned()
            .expect("the corpus banner is a stored string");
        app.disasm_start = Address::new(row.addr.get() + 1);
        app.disasm_dirty = true;
        app.tick();
        let line = app
            .disasm_rows
            .first()
            .expect("a string tail must render as a data row");
        assert_eq!(
            line.addr,
            Address::new(row.addr.get() + 1),
            "the row starts at the address the literal named"
        );
        assert_eq!(line.mnemonic, "db");
        assert!(
            line.operands.contains("eCraft analysis corpus"),
            "the tail from the interior address is shown, got {:?}",
            line.operands
        );
        assert!(line.flags.is_data());
    }

    /// Scrolling a data window follows the address space and never decodes data as code.
    ///
    /// Two reports: (1) double-clicking a string in the Strings view showed it as a data
    /// row, but one wheel notch up anchored the window in the padding *before* the literal
    /// (notepad's `.rdata` literals are separated by zero bytes that decode as
    /// `add [rax], al`), so the next refresh decoded the literals as instructions; (2)
    /// paging the literal list instead stopped at the first literal, so the view could not
    /// scroll up at all. The window is now a row walk over the address space — literals,
    /// the bytes between them, and code once it reaches an executable segment — and paging
    /// moves along that walk.
    #[test]
    fn scrolling_a_data_window_follows_the_address_space() {
        let Ok(bytes) = std::fs::read("C:/Windows/System32/notepad.exe") else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("string-scroll.rcraft");
        let _ = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let _ = run_analysis_job(&project, &cancel);
        let session = Session::open(&project).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        app.refresh_strings();
        // The first literal the Strings pane shows is the top of its list — the place the
        // second report scrolled up from and found nothing above.
        let row = app
            .strings
            .iter()
            .find(|row| !is_header_string(&row.segment))
            .cloned()
            .expect("notepad has a visible literal");
        let segment = app
            .session
            .as_ref()
            .unwrap()
            .segment_containing(row.addr)
            .cloned()
            .expect("the literal is mapped");
        assert!(
            segment.range.start < row.addr,
            "notepad has data above its first visible literal"
        );
        // The old path: the instruction walker cannot stop on the literal, it stops in the
        // padding right before it, where no string covers the address.
        let walked = app.session.as_mut().unwrap().rewind(row.addr, 1).unwrap();
        assert_ne!(walked, row.addr, "the walker must move off the literal");
        assert!(
            string_row_at(&app.strings, walked).is_none(),
            "expected the rewind to land in padding, got {:#x}",
            walked.get()
        );

        app.apply_action(
            &egui::Context::default(),
            Action::NavigateToDisasm(row.addr),
        );
        app.tick();
        assert!(
            app.disasm_rows
                .first()
                .is_some_and(|line| line.flags.is_data()),
            "a string address shows a data row"
        );
        assert_eq!(
            app.disasm_rows.first().map(|line| line.addr),
            Some(row.addr)
        );
        assert_eq!(
            app.disasm_rows.first().map(|line| line.mnemonic.as_str()),
            Some("db")
        );

        // One row up is the padding before the literal, rendered as data — the exact row
        // that used to come back as `add [rax], al`.
        app.shift_window(-1);
        assert!(
            app.disasm_rows.iter().all(|line| line.flags.is_data()),
            "scrolling left the data rows"
        );
        let first = app.disasm_rows.first().cloned().expect("rows");
        assert!(
            first.addr < row.addr,
            "one row up must move above the literal"
        );
        assert!(first.flags.is_data(), "padding decoded as code: {first:?}");
        assert_eq!(
            first.addr.get() + u64::from(first.len),
            row.addr.get(),
            "the raw row must end where the literal starts"
        );
        assert!(
            app.disasm_rows.iter().any(|line| line.addr == row.addr),
            "the literal that was on screen must stay in the window"
        );

        // The wheel walks the same rows (`extend_disasm_window`), keeps scrolling up through
        // the section instead of stopping at the first literal, and crosses into the
        // preceding section rather than stopping at the section start.
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            ..egui::RawInput::default()
        };
        let wheel_up = |app: &mut ReCraftApp| {
            let mut offset = None;
            let mut output = ctx.run_ui(input.clone(), |ui| {
                offset = app.extend_disasm_window(ui, -1);
            });
            output.textures_delta.clear();
            offset
        };
        let mut moved = 0usize;
        let mut crossed = false;
        for _ in 0..400 {
            let before = app.disasm_start;
            if wheel_up(&mut app).is_none() {
                break;
            }
            assert!(app.disasm_start < before, "the wheel must move upwards");
            if app.disasm_start >= segment.range.start {
                assert!(
                    app.disasm_rows.iter().all(|line| line.flags.is_data()),
                    "the wheel decoded data as code at {:#x}",
                    app.disasm_start.get()
                );
            } else {
                crossed = true;
                break;
            }
            moved += 1;
        }
        assert!(moved > 0, "the wheel must scroll up from the first literal");
        assert!(
            crossed,
            "scrolling up must reach the start of the section and cross into the code above it"
        );
        assert!(
            app.disasm_start < segment.range.start,
            "the window must have crossed the section start, got {:#x}",
            app.disasm_start.get()
        );
        assert!(
            app.disasm_rows.iter().any(|line| !line.flags.is_data()),
            "the section above the data must decode as instructions"
        );

        // PageUp walks the same rows and lands on the literal again on the way down.
        app.navigate(row.addr);
        app.tick();
        app.shift_window(-20);
        assert!(app.disasm_start < row.addr);
        assert!(app.disasm_rows.iter().all(|line| line.flags.is_data()));
        app.shift_window(20);
        assert_eq!(
            app.disasm_rows.first().map(|line| line.addr),
            Some(row.addr),
            "a page down must return to the literal"
        );

        // Past the last literal the window keeps walking data instead of decoding it.
        let last = app
            .strings
            .iter()
            .rev()
            .find(|row| !is_header_string(&row.segment))
            .cloned()
            .expect("a last literal");
        app.navigate(last.addr);
        app.tick();
        let before = app.disasm_start;
        app.shift_window(1);
        assert!(app.disasm_start > before, "the window must move down");
        assert!(app.disasm_rows.iter().all(|line| line.flags.is_data()));
    }

    /// A literal that sits on the first byte of its section has nothing above it *inside*
    /// the section, so scrolling up must cross into the section before it (the corpus's
    /// first literal is exactly there — the report's "cannot scroll up" case).
    #[test]
    fn scrolling_up_from_the_first_byte_of_a_section_crosses_into_the_previous_one() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/x64/corpus.exe");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("section-start.rcraft");
        let _ = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let _ = run_analysis_job(&project, &cancel);
        let session = Session::open(&project).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        app.refresh_strings();
        // A literal whose address is the first byte of a section that follows code.
        let Some(row) = app
            .strings
            .iter()
            .find(|row| {
                app.session
                    .as_ref()
                    .and_then(|session| session.segment_containing(row.addr))
                    .is_some_and(|segment| segment.range.start == row.addr)
            })
            .cloned()
        else {
            return;
        };
        let segment = app
            .session
            .as_ref()
            .unwrap()
            .segment_containing(row.addr)
            .cloned()
            .expect("mapped");
        let previous = app
            .session
            .as_ref()
            .unwrap()
            .segments()
            .iter()
            .filter(|candidate| candidate.range.end <= segment.range.start)
            .max_by_key(|candidate| candidate.range.end)
            .cloned();
        let Some(previous) = previous else {
            return;
        };
        if !previous.perm.is_executable() {
            return;
        }
        app.apply_action(
            &egui::Context::default(),
            Action::NavigateToDisasm(row.addr),
        );
        app.tick();
        assert!(
            app.disasm_rows
                .first()
                .is_some_and(|line| line.flags.is_data()),
            "a string address shows a data row"
        );
        assert_eq!(
            app.disasm_rows.first().map(|line| line.addr),
            Some(row.addr)
        );

        app.shift_window(-1);
        assert!(
            app.disasm_start < row.addr,
            "the first byte of the section must still scroll up"
        );
        assert!(
            app.disasm_rows
                .first()
                .is_some_and(|line| !line.flags.is_data()),
            "the window must cross into the code section above it"
        );
        assert!(
            app.disasm_rows.iter().any(|line| !line.flags.is_data()),
            "the section above must decode as instructions"
        );
        assert!(
            app.disasm_start >= previous.range.start,
            "the window must stay inside the preceding section"
        );
    }

    #[test]
    fn hex_view_aligns_and_follows_selection() {
        assert_eq!(hex_row_start(Address::new(0x1234)).get(), 0x1230);
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("hex.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_disasm();
        app.tick();
        let selected = app.disasm_rows[app.selected].addr;
        assert_eq!(app.hex_start, hex_row_start(selected));
        assert_eq!(app.hex_cursor, Some(selected));

        let target = Address::new(selected.get() + 0x40);
        app.navigate(target);
        app.tick();
        assert_eq!(app.hex_start, hex_row_start(target));
        assert_eq!(app.hex_cursor, Some(target));
    }

    #[test]
    fn hex_panel_renders_bytes_and_words() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("hex-render.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_disasm();
        app.tick();
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let run = |app: &mut ReCraftApp| {
            let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            output.textures_delta.clear();
        };
        run(&mut app);
        app.hex_unit = HexUnit::Word;
        run(&mut app);
    }

    #[test]
    fn ai_panel_renders_without_a_project() {
        let mut app = ReCraftApp::default();
        app.focus_pane(Pane::Ai);
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
        output.textures_delta.clear();
        assert!(app.ai_job.is_none());
    }

    #[test]
    fn ai_panel_renders_a_coloured_polish_result() {
        // Exercises the code path that lays out a long, wrapped, syntax-coloured result
        // (a panic here would only show up at runtime, not in the tokenizer tests).
        let mut app = ReCraftApp {
            ai_explanation: Some(
                "if (rcx != 0) {\n    sub_140001000(\"name\", 0x10);\n}\nreturn 0;".to_owned(),
            ),
            ai_result_kind: AiResultKind::Code,
            ..ReCraftApp::default()
        };
        app.focus_pane(Pane::Ai);
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
        output.textures_delta.clear();
        assert_eq!(app.ai_result_kind, AiResultKind::Code);
    }

    #[test]
    fn ai_actions_require_a_provider() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ai.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_disasm();
        app.refresh_functions();
        app.tick();
        if app.functions.is_empty() || app.disasm_rows.is_empty() {
            return;
        }
        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::AiExplain);
        assert!(
            app.ai_explanation.is_none()
                && (app.ai_status.to_lowercase().contains("provider")
                    || app.ai_status.to_lowercase().contains("configure")),
            "explain should ask for a provider: {:?} / {}",
            app.ai_explanation,
            app.ai_status
        );
        app.apply_action(&ctx, Action::AiPolish);
        assert!(
            app.ai_status.to_lowercase().contains("provider")
                || app.ai_status.to_lowercase().contains("enable")
                || app.ai_status.to_lowercase().contains("configure"),
            "polish should ask for a provider: {}",
            app.ai_status
        );
        app.apply_action(&ctx, Action::AiSuggest);
        assert!(
            app.ai_status.to_lowercase().contains("provider")
                || app.ai_status.to_lowercase().contains("configure"),
            "suggest names should ask for a provider: {}",
            app.ai_status
        );
        assert!(app.ai_job.is_none());
    }

    #[test]
    fn ai_explain_and_suggest_ask_the_model() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ai-jobs.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_disasm();
        app.refresh_functions();
        app.tick();
        if app.functions.is_empty() || app.disasm_rows.is_empty() {
            return;
        }
        // A closed local port fails fast, so a job can be observed without a model.
        app.ai_settings.endpoint = "http://127.0.0.1:1".to_owned();
        app.ai_settings.model = "test-model".to_owned();
        app.ai_settings.api_key = Some("test-key".to_owned());
        app.ai_settings.refresh_enabled();
        assert!(app.ai_settings.enabled);
        let ctx = egui::Context::default();

        // Explain asks the model instead of formatting the local report.
        app.apply_action(&ctx, Action::AiExplain);
        assert!(app.ai_job.is_some(), "explain must call the provider");
        assert_eq!(app.ai_result_kind, AiResultKind::Prose);
        assert!(
            app.ai_explanation.is_none(),
            "the explanation streams in later"
        );
        app.ai_job = None;

        // Suggest names asks for names shaped like the local proposals (same targets).
        let candidates = recraft_ai::suggest_names(app.session.as_mut().unwrap(), 50).unwrap();
        if candidates.is_empty() {
            return;
        }
        app.apply_action(&ctx, Action::AiSuggest);
        assert!(
            app.ai_job.is_some(),
            "suggest names must call the provider: {}",
            app.ai_status
        );
        assert!(
            app.ai_proposals.is_empty(),
            "proposals appear with the model's reply"
        );
        app.ai_job = None;
    }

    #[test]
    fn ai_explain_job_result_is_prose() {
        let (sender, receiver) = mpsc::channel();
        let target = Address::new(0x140001234);
        let mut app = ReCraftApp {
            ai_job: Some(AiJob {
                receiver,
                started: Instant::now(),
                kind: AiJobKind::Explain,
                prompt: None,
                target: Some(target),
                suggestions: Vec::new(),
            }),
            ..ReCraftApp::default()
        };
        sender
            .send(AiMessage::Done(Ok(
                "This function reads a configuration file.".to_owned(),
            )))
            .unwrap();
        app.tick();
        assert_eq!(
            app.ai_explanation.as_deref(),
            Some("This function reads a configuration file.")
        );
        assert_eq!(
            app.ai_result_kind,
            AiResultKind::Prose,
            "an explanation is prose, not pseudo-C"
        );
        assert!(
            app.ai_status.starts_with("explanation"),
            "{}",
            app.ai_status
        );
        assert_eq!(
            app.ai_result_target,
            Some(target),
            "the answer must stay attached to the function it describes"
        );
        assert!(app.ai_job.is_none());
    }

    /// Narration streamed before a tool call is dropped from the answer and kept as trace
    /// context, so a Chinese answer never starts with "I'll look for…".
    #[test]
    fn agent_narration_is_retracted_into_the_trace() {
        let (sender, receiver) = mpsc::channel();
        let mut app = ReCraftApp {
            ai_job: Some(AiJob {
                receiver,
                started: Instant::now(),
                kind: AiJobKind::Agent,
                prompt: None,
                target: None,
                suggestions: Vec::new(),
            }),
            ..ReCraftApp::default()
        };
        sender.send(AiMessage::AgentTurnStart).unwrap();
        sender
            .send(AiMessage::Chunk("I'll look for logging APIs.".to_owned()))
            .unwrap();
        sender.send(AiMessage::AgentRetract).unwrap();
        sender.send(AiMessage::AgentTurnStart).unwrap();
        sender
            .send(AiMessage::Chunk("答案：会写日志。".to_owned()))
            .unwrap();
        sender
            .send(AiMessage::Done(Ok("答案：会写日志。".to_owned())))
            .unwrap();
        app.tick();
        assert_eq!(app.ai_explanation.as_deref(), Some("答案：会写日志。"));
        assert!(
            app.ai_agent_trace
                .iter()
                .any(|line| line.starts_with('…') && line.contains("logging")),
            "narration missing from the trace: {:?}",
            app.ai_agent_trace
        );
    }
    /// The agent job streams tool-call lines into the trace and its final answer into the
    /// result pane as prose (no pseudo-C colouring).
    #[test]
    fn agent_job_trace_and_answer_are_applied() {
        let (sender, receiver) = mpsc::channel();
        let mut app = ReCraftApp {
            ai_job: Some(AiJob {
                receiver,
                started: Instant::now(),
                kind: AiJobKind::Agent,
                prompt: None,
                target: None,
                suggestions: Vec::new(),
            }),
            ..ReCraftApp::default()
        };
        sender
            .send(AiMessage::AgentTool(
                "list_functions {\"filter\":\"sub_\"}".to_owned(),
            ))
            .unwrap();
        sender
            .send(AiMessage::AgentHistory(vec![
                recraft_ai::ChatMessage::user("哪些函数在写注册表？"),
            ]))
            .unwrap();
        sender
            .send(AiMessage::Done(Ok("这些函数负责注册表访问。".to_owned())))
            .unwrap();
        app.tick();
        assert_eq!(app.ai_agent_trace.len(), 1);
        assert_eq!(
            app.ai_agent_history.len(),
            1,
            "conversation kept for a follow-up"
        );
        assert!(app.ai_agent_trace[0].starts_with("list_functions"));
        assert_eq!(
            app.ai_explanation.as_deref(),
            Some("这些函数负责注册表访问。")
        );
        assert_eq!(app.ai_result_kind, AiResultKind::Prose);
        assert!(app.ai_job.is_none());
    }
    #[test]
    fn ai_suggest_job_merges_model_names() {
        let (sender, receiver) = mpsc::channel();
        let target = Address::new(0x140001000);
        let mut app = ReCraftApp {
            ai_job: Some(AiJob {
                receiver,
                started: Instant::now(),
                kind: AiJobKind::Suggest,
                prompt: None,
                target: None,
                suggestions: vec![recraft_ai::Proposal {
                    kind: recraft_ai::ProposalKind::Rename,
                    target,
                    value: "file_140001000".to_owned(),
                    confidence: 0.5,
                    rationale: "calls CreateFileW".to_owned(),
                    evidence: Vec::new(),
                    source: recraft_ai::ProposalSource::Heuristic,
                }],
            }),
            ..ReCraftApp::default()
        };
        sender
            .send(AiMessage::Done(Ok(format!(
                "{:#x} open_config -- opens the configuration file",
                target.get()
            ))))
            .unwrap();
        app.tick();
        assert_eq!(app.ai_proposals.len(), 1);
        assert_eq!(app.ai_proposals[0].value, "open_config");
        assert!(app.ai_proposals[0].confidence >= 0.7);
        assert_eq!(
            app.ai_proposals[0].source,
            recraft_ai::ProposalSource::Model
        );
        assert!(!app.ai_status.is_empty(), "no completion status");
        assert!(
            !app.ai_status.to_lowercase().contains("failed"),
            "{}",
            app.ai_status
        );
        assert!(
            app.ai_explanation.is_none(),
            "a rename reply is not shown as the result"
        );
        assert!(app.ai_job.is_none());
    }

    #[test]
    fn repolish_bypasses_the_session_cache() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("repolish.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_disasm();
        app.refresh_functions();
        app.tick();
        if app.functions.is_empty() || app.disasm_rows.is_empty() {
            return;
        }
        // Any cache miss spawns a worker for this endpoint, so the cache decision is
        // observable without a real provider round trip.
        app.ai_settings.endpoint = "http://127.0.0.1:1".to_owned();
        app.ai_settings.model = "test-model".to_owned();
        app.ai_settings.api_key = Some("test-key".to_owned());
        app.ai_settings.max_tokens = 4096;
        app.ai_settings.refresh_enabled();
        assert!(app.ai_settings.enabled);
        app.ai_cache_fingerprint = Some(format!(
            "{}|{}|{}",
            app.ai_settings.endpoint, app.ai_settings.model, app.ai_settings.max_tokens
        ));
        let (start, _) = app.current_function().expect("cursor inside a function");
        let prompt =
            recraft_ai::build_polish_prompt(app.session.as_mut().unwrap(), start, &app.ai_settings)
                .unwrap();
        app.ai_cache.insert(&prompt, "cached polish");
        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::AiPolish);
        assert_eq!(app.ai_explanation.as_deref(), Some("cached polish"));
        assert!(
            app.ai_job.is_none(),
            "a repeated polish may reuse the cache"
        );
        app.apply_action(&ctx, Action::AiRepolish);
        assert!(
            app.ai_explanation.is_none(),
            "a fresh answer streams from scratch"
        );
        assert!(
            app.ai_job.is_some(),
            "re-polish must ask the provider again"
        );
        app.ai_job = None;
    }

    #[test]
    fn function_filter_matches_names_and_addresses() {
        let start = Address::new(0x140001000);
        assert!(function_matches("", start, "start"));
        assert!(function_matches("star", start, "start"));
        assert!(function_matches("start", start, "start"));
        assert!(function_matches("140001000", start, "start"));
        assert!(function_matches("0x140001000", start, "start"));
        assert!(!function_matches("memcpy", start, "start"));
    }

    #[test]
    fn function_labels_and_reverse_jump_navigation() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("nav.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_functions();
        assert_eq!(app.functions.len(), app.function_labels.len());
        assert!(
            app.function_labels.iter().all(|label| !label.is_empty()),
            "every function row needs a display name"
        );
        let target = app.functions[0].start;
        let ctx = egui::Context::default();
        app.tab = Tab::Pseudo;
        app.apply_action(&ctx, Action::NavigateToDisasm(target));
        assert!(matches!(app.tab, Tab::Disasm));
        assert_eq!(app.disasm_start, target);
    }

    #[test]
    fn query_ui_answers_and_links_addresses() {
        assert_eq!(
            parse_query_addr("0x140001000").map(|addr| addr.get()),
            Some(0x140001000)
        );
        assert_eq!(
            parse_query_addr("4198400").map(|addr| addr.get()),
            Some(4198400)
        );
        assert_eq!(parse_query_addr("nope"), None);

        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("query.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.ai_settings.endpoint = "https://example.test".to_owned();
        app.ai_settings.api_key = Some("test-key".to_owned());
        app.ai_settings.refresh_enabled();
        app.ai_query = "urls".to_owned();
        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::AiQuery);
        assert!(
            app.ai_answer.is_some(),
            "offline query should answer, status: {}",
            app.ai_status
        );
        app.apply_action(&ctx, Action::AiClear);
        assert!(app.ai_answer.is_none());
    }

    #[test]
    fn back_and_forward_history_roundtrip() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("history.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        let entry = app.disasm_start;
        let second = Address::new(entry.get() + 0x10);
        let third = Address::new(entry.get() + 0x20);
        app.navigate(second);
        app.navigate(third);
        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::Back);
        assert_eq!(app.disasm_start, second);
        app.apply_action(&ctx, Action::Forward);
        assert_eq!(app.disasm_start, third);
        app.apply_action(&ctx, Action::Forward);
        assert_eq!(app.disasm_start, third, "history end is a no-op");
    }

    #[test]
    fn sync_disasm_keeps_the_current_tab() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("sync.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        let target = Address::new(app.disasm_start.get() + 0x10);
        app.tab = Tab::Pseudo;
        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::SyncDisasm(target));
        assert!(matches!(app.tab, Tab::Pseudo), "sync must not switch tabs");
        assert_eq!(app.disasm_start, target);
        assert_eq!(app.selected, 0);
        assert_eq!(app.pseudo_start, target);
    }

    #[test]
    fn pseudo_line_click_selects_one_row_and_keeps_the_listing() {
        let start = Address::new(0x140001000);
        let mut app = ReCraftApp {
            pseudo_start: start,
            pseudo: Some(vec![
                PseudoLine {
                    addr: start,
                    text: "// header".to_owned(),
                },
                PseudoLine {
                    addr: start,
                    text: "first();".to_owned(),
                },
                PseudoLine {
                    addr: Address::new(start.get() + 0x10),
                    text: "second();".to_owned(),
                },
            ]),
            disasm_rows: vec![DisasmLine {
                addr: start,
                len: 1,
                raw: [0x90; 15],
                mnemonic: "nop".to_owned(),
                operands: String::new(),
                name: None,
                comment: None,
                flags: Flags::code(),
            }],
            ..ReCraftApp::default()
        };
        assert_eq!(
            app.active_pseudo_line(app.pseudo.as_deref().unwrap()),
            Some(0),
            "without a click, the first line at the address is current"
        );

        let ctx = egui::Context::default();
        app.apply_action(
            &ctx,
            Action::SyncPseudoLine {
                addr: start,
                line: 1,
            },
        );
        assert_eq!(app.pseudo_selected_line, Some(1));
        assert_eq!(
            app.pseudo_start, start,
            "the listing must not be re-anchored"
        );
        assert_eq!(app.disasm_start, start);
        assert_eq!(app.selected, 0);
        assert_eq!(
            app.active_pseudo_line(app.pseudo.as_deref().unwrap()),
            Some(1),
            "the clicked row wins over the first line of the address group"
        );

        let next = Address::new(start.get() + 0x10);
        app.apply_action(&ctx, Action::SyncDisasm(next));
        assert_eq!(app.pseudo_selected_line, None);
        assert_eq!(app.pseudo_start, next);
    }

    #[test]
    fn pseudo_symbol_at_expands_to_the_whole_token() {
        let text = "  result = sub_140001100(a1, arg_2);";
        let start = text.find("sub_140001100").unwrap();
        assert_eq!(pseudo_symbol_at(text, start), Some("sub_140001100"));
        assert_eq!(pseudo_symbol_at(text, start + 5), Some("sub_140001100"));
        // The caret from the layout reports the closest boundary, so the right half of the
        // last letter points at `(`; the previous character still names the token.
        assert_eq!(
            pseudo_symbol_at(text, start + "sub_140001100".len()),
            Some("sub_140001100")
        );
        assert_eq!(pseudo_symbol_at("  a1 = b2;", 2), Some("a1"));
        assert_eq!(
            pseudo_symbol_at("  a1 = b2;", 5),
            None,
            "`=` is not a symbol"
        );
        assert_eq!(
            pseudo_symbol_at("  return 0;", 20),
            None,
            "index past the end"
        );
        assert_eq!(pseudo_symbol_at("", 0), None);
        // Character (not byte) indexing must hold for non-ASCII user renames.
        assert_eq!(pseudo_symbol_at("  fn名(x);", 2), Some("fn名"));
    }

    #[test]
    fn pseudo_string_literal_at_finds_the_quoted_text() {
        let text = "  return \"ReCraft analysis corpus\";";
        let start = text.find("ReCraft").unwrap();
        assert_eq!(
            pseudo_string_literal_at(text, start),
            Some("ReCraft analysis corpus")
        );
        assert_eq!(
            pseudo_string_literal_at(text, start + 10),
            Some("ReCraft analysis corpus")
        );
        // The opening quote and the code around the literal stay outside it.
        assert_eq!(pseudo_string_literal_at(text, 0), None);
        assert_eq!(
            pseudo_string_literal_at(text, text.find('"').unwrap()),
            None
        );
        assert_eq!(pseudo_string_literal_at(text, text.len() - 1), None);
        // Clicking the right half of the last character reports the closing quote; the
        // previous-character fallback still resolves the whole literal.
        assert_eq!(
            pseudo_string_literal_at(text, text.rfind('"').unwrap()),
            Some("ReCraft analysis corpus")
        );
        // Escaped quotes are part of the literal, not its end.
        let escaped = "  puts(\"a\\\"b\");";
        assert_eq!(
            pseudo_string_literal_at(escaped, escaped.find('a').unwrap()),
            Some("a\\\"b")
        );
    }

    #[test]
    fn pseudo_double_click_follows_the_string_a_literal_names() {
        let caller = Address::new(0x140001000);
        let data = Address::new(0x140002000);
        let app = ReCraftApp {
            pseudo_start: caller,
            pseudo_literals: vec![("\"ReCraft analysis corpus\"".to_owned(), data)],
            ..ReCraftApp::default()
        };
        let text = "  return \"ReCraft analysis corpus\";";
        let index = text.find("analysis").unwrap();
        assert_eq!(
            app.pseudo_double_click(text, index, caller),
            Action::NavigateToDisasm(data),
            "a string literal must follow its data, not the loading instruction"
        );

        // Without a recorded literal (an `__asm("…")` operand, for instance) the old
        // fallback to this row's disassembly stays.
        let plain = ReCraftApp::default();
        assert_eq!(
            plain.pseudo_double_click(text, index, caller),
            Action::NavigateToDisasm(caller)
        );
    }

    #[test]
    fn sub_symbol_address_parses_auto_names() {
        assert_eq!(
            sub_symbol_address("sub_140001100").map(|addr| addr.get()),
            Some(0x140001100)
        );
        assert_eq!(
            sub_symbol_address("sub_401000").map(|addr| addr.get()),
            Some(0x401000)
        );
        assert_eq!(sub_symbol_address("sub_"), None);
        assert_eq!(sub_symbol_address("sub_xyz"), None);
        assert_eq!(sub_symbol_address("main"), None);
    }

    #[test]
    fn auto_names_resolve_to_their_own_address() {
        let real = Address::new(0x140001100);
        let impostor = Address::new(0x140001500);
        let app = ReCraftApp {
            functions: vec![
                FunctionInfo {
                    start: real,
                    end: Address::new(0x140001200),
                },
                FunctionInfo {
                    start: impostor,
                    end: Address::new(0x140001600),
                },
            ],
            // Two functions share the text: the auto name still means the address it spells.
            function_labels: vec!["sub_140001100".to_owned(), "sub_140001100".to_owned()],
            ..ReCraftApp::default()
        };
        assert_eq!(app.pseudo_function_named("sub_140001100"), Some(real));
    }

    #[test]
    fn pseudo_double_click_enters_a_callee_function() {
        let caller = Address::new(0x140001000);
        let callee = Address::new(0x140001100);
        let renamed = Address::new(0x140001300);
        let app = ReCraftApp {
            pseudo_start: caller,
            functions: vec![
                FunctionInfo {
                    start: caller,
                    end: callee,
                },
                FunctionInfo {
                    start: callee,
                    end: renamed,
                },
                FunctionInfo {
                    start: renamed,
                    end: Address::new(0x140001400),
                },
            ],
            function_labels: vec![
                "main".to_owned(),
                "sub_140001100".to_owned(),
                "decrypt".to_owned(),
            ],
            ..ReCraftApp::default()
        };

        // An auto-named callee opens its body from anywhere inside the token.
        let call = "  sub_140001100(arg_1);";
        let start = call.find("sub_140001100").unwrap();
        assert_eq!(
            app.pseudo_double_click(call, start, caller),
            Action::EnterFunction(callee)
        );
        assert_eq!(
            app.pseudo_double_click(call, start + 11, caller),
            Action::EnterFunction(callee)
        );
        // A renamed callee resolves through the stored name.
        let named = "  decrypt(buf);";
        assert_eq!(
            app.pseudo_double_click(named, 2, caller),
            Action::EnterFunction(renamed)
        );
        // The function already on screen (header or recursive call) is not re-entered.
        let own = "  main();";
        assert_eq!(
            app.pseudo_double_click(own, 2, caller),
            Action::NavigateToDisasm(caller)
        );
        // Non-symbol text keeps the old behaviour: jump to this row's disassembly.
        let plain = "  return 0;";
        assert_eq!(
            app.pseudo_double_click(plain, 4, caller),
            Action::NavigateToDisasm(caller)
        );
        // A `sub_<hex>` call target that is not a defined function stays a plain jump.
        let stray = "  sub_140009000();";
        assert_eq!(
            app.pseudo_double_click(stray, 2, caller),
            Action::NavigateToDisasm(caller)
        );
    }

    #[test]
    fn enter_function_opens_the_callee_listing_and_back_returns() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("enter.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        if app.functions.len() < 2 {
            return;
        }
        let caller = app.functions[0].start;
        let callee = app.functions[1].start;
        app.navigate(caller);
        app.tick();
        let history_before = app.history.len();

        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::EnterFunction(callee));
        assert_eq!(
            app.pseudo_start, callee,
            "the listing must follow the callee"
        );
        assert_eq!(app.disasm_start, callee);
        assert!(
            matches!(app.tab, Tab::Pseudo),
            "entering a function keeps the decompiler in front"
        );
        assert_eq!(app.history.len(), history_before + 1);
        assert_eq!(app.history.last(), Some(&callee));
        app.tick();
        assert!(app.pseudo.is_some(), "the callee listing must render");

        // Entering the function already on screen must not push another history entry.
        app.apply_action(&ctx, Action::EnterFunction(callee));
        assert_eq!(app.history.len(), history_before + 1);

        // Back returns to the caller, like every other navigation.
        app.apply_action(&ctx, Action::Back);
        assert_eq!(app.pseudo_start, caller);
    }

    fn collect_text_runs(shape: &egui::Shape, out: &mut Vec<(String, egui::Pos2)>) {
        match shape {
            egui::Shape::Text(text) => out.push((text.galley.job.text.clone(), text.pos)),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect_text_runs(shape, out);
                }
            }
            _ => {}
        }
    }

    /// The first laid-out text run whose whole text is `needle`, so a test can inspect
    /// the colour sections of one rendered row.
    fn find_text_galley<'a>(shape: &'a egui::Shape, needle: &str) -> Option<&'a egui::Galley> {
        match shape {
            egui::Shape::Text(text) if text.galley.job.text == needle => Some(&text.galley),
            egui::Shape::Vec(shapes) => shapes
                .iter()
                .find_map(|shape| find_text_galley(shape, needle)),
            _ => None,
        }
    }

    /// Screen position of the middle of `char_index` in the first text run equal to `needle`.
    fn char_click_pos(shape: &egui::Shape, needle: &str, char_index: usize) -> Option<egui::Pos2> {
        match shape {
            egui::Shape::Text(text) if text.galley.job.text == needle => {
                let start = text
                    .galley
                    .pos_from_cursor(egui::text::CCursor::new(char_index));
                let end = text
                    .galley
                    .pos_from_cursor(egui::text::CCursor::new(char_index + 1));
                Some(egui::pos2(
                    text.pos.x + (start.center().x + end.center().x) / 2.0,
                    text.pos.y + start.center().y,
                ))
            }
            egui::Shape::Vec(shapes) => shapes
                .iter()
                .find_map(|shape| char_click_pos(shape, needle, char_index)),
            _ => None,
        }
    }

    #[test]
    fn double_clicking_a_callee_name_in_the_view_enters_it() {
        let caller = Address::new(0x140001000);
        let callee = Address::new(0x140001100);
        let line_text = "sub_140001100();";
        let mut app = ReCraftApp {
            pseudo_start: caller,
            functions: vec![
                FunctionInfo {
                    start: caller,
                    end: callee,
                },
                FunctionInfo {
                    start: callee,
                    end: Address::new(0x140001200),
                },
            ],
            function_labels: vec!["main".to_owned(), "sub_140001100".to_owned()],
            pseudo: Some(vec![PseudoLine {
                addr: caller,
                text: line_text.to_owned(),
            }]),
            ..ReCraftApp::default()
        };

        let ctx = egui::Context::default();
        let base = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 500.0),
            )),
            ..egui::RawInput::default()
        };
        let mut action = Action::None;
        let mut output = ctx.run_ui(base.clone(), |ui| app.pseudo_view(ui, &mut action));
        output.textures_delta.clear();
        let pos = output
            .shapes
            .iter()
            .find_map(|clipped| char_click_pos(&clipped.shape, line_text, 2))
            .expect("the pseudo-C line must be on screen");

        let click = |time: f64| {
            let mut input = base.clone();
            input.time = Some(time);
            input.events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ];
            input
        };

        // First click: selection only, the listing stays put.
        let mut action = Action::None;
        let mut output = ctx.run_ui(click(0.1), |ui| app.pseudo_view(ui, &mut action));
        output.textures_delta.clear();
        assert!(
            matches!(action, Action::SyncPseudoLine { .. }),
            "a single click must only select the row, got {action:?}"
        );

        // Second click inside the double-click window: enter the callee.
        let mut action = Action::None;
        let mut output = ctx.run_ui(click(0.2), |ui| app.pseudo_view(ui, &mut action));
        output.textures_delta.clear();
        assert_eq!(
            action,
            Action::EnterFunction(callee),
            "double-clicking the callee name must enter that function"
        );
    }

    #[test]
    fn pseudo_view_draws_line_numbers_instead_of_addresses() {
        let start = Address::new(0x140001000);
        let mut app = ReCraftApp {
            pseudo_start: start,
            pseudo: Some(vec![
                PseudoLine {
                    addr: start,
                    text: "// header".to_owned(),
                },
                PseudoLine {
                    addr: start,
                    text: "return 0;".to_owned(),
                },
            ]),
            ..ReCraftApp::default()
        };

        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            ..egui::RawInput::default()
        };
        let mut action = Action::None;
        let mut output = ctx.run_ui(input, |ui| app.pseudo_view(ui, &mut action));
        output.textures_delta.clear();
        let mut texts = Vec::new();
        for clipped in &output.shapes {
            collect_text_runs(&clipped.shape, &mut texts);
        }

        let number = texts
            .iter()
            .find(|(text, _)| text == "1")
            .expect("expected a line-number gutter");
        let code = texts
            .iter()
            .find(|(text, _)| text == "// header")
            .expect("expected the first pseudo-C row");
        assert!(
            number.1.x < code.1.x,
            "the line number must sit left of the code, got {number:?} and {code:?}"
        );
        assert!(
            texts.iter().any(|(text, _)| text == "2"),
            "expected the second pseudo-code row number, got {texts:?}"
        );
        assert!(
            texts.iter().any(|(text, _)| text == "return 0;"),
            "expected the second pseudo-code row, got {texts:?}"
        );
        assert!(
            !texts.iter().any(|(text, _)| text.contains("0x140001000")),
            "the pseudo-C view must not show an address prefix"
        );
    }

    #[test]
    fn pseudo_view_colours_decompiled_code() {
        let start = Address::new(0x140001000);
        let line = "if (rax != 0) return 0;";
        let mut app = ReCraftApp {
            pseudo_start: start,
            pseudo: Some(vec![PseudoLine {
                addr: start,
                text: line.to_owned(),
            }]),
            ..ReCraftApp::default()
        };
        let pal = app.palette();
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 500.0),
            )),
            ..egui::RawInput::default()
        };
        let mut action = Action::None;
        let mut output = ctx.run_ui(input, |ui| app.pseudo_view(ui, &mut action));
        output.textures_delta.clear();
        let galley = output
            .shapes
            .iter()
            .find_map(|clipped| find_text_galley(&clipped.shape, line))
            .expect("the pseudo-C row must be on screen");
        let colours: Vec<Color32> = galley
            .job
            .sections
            .iter()
            .map(|section| section.format.color)
            .collect();
        assert!(
            colours.contains(&pal.mnemonic),
            "keywords must be coloured, got {colours:?}"
        );
        assert!(
            colours.contains(&pal.register),
            "registers must be coloured, got {colours:?}"
        );
        assert!(
            colours.contains(&pal.number),
            "numbers must be coloured, got {colours:?}"
        );
        assert!(
            colours.contains(&pal.text),
            "punctuation must stay plain, got {colours:?}"
        );
    }

    /// The Decompile view appends a stored comment to the end of the row that carries its
    /// address (`code  // note`), like the disassembly's comment column; extra comment lines
    /// follow aligned under the marker. A comment whose address is no longer emitted keeps
    /// its rows at the end as comment-only rows (IDA's orphan comments).
    #[test]
    fn pseudo_comments_trail_their_row() {
        let start = Address::new(0x140001000);
        let body = Address::new(0x140001010);
        let gone = Address::new(0x140001020);
        let lines = vec![
            PseudoLine {
                addr: start,
                text: "void sub_140001000(void)".to_owned(),
            },
            PseudoLine {
                addr: start,
                text: "{".to_owned(),
            },
            PseudoLine {
                addr: body,
                text: "    v1 = 1;".to_owned(),
            },
        ];
        let comments = BTreeMap::from([
            (body, "setup".to_owned()),
            (gone, "orphan\nnote".to_owned()),
        ]);
        let (rows, cols) = ReCraftApp::with_pseudo_comments(lines, &comments);
        let texts: Vec<&str> = rows.iter().map(|row| row.text.as_str()).collect();
        assert_eq!(
            texts,
            vec![
                "void sub_140001000(void)",
                "{",
                "    v1 = 1;  // setup",
                "// orphan",
                "// note",
            ]
        );
        // The comment starts after the code; the view uses that boundary to keep names
        // inside it from becoming jump targets.
        let column = "    v1 = 1;".chars().count() + 2;
        assert_eq!(cols, vec![None, None, Some(column), Some(0), Some(0)]);
        // The annotated row keeps its own address, so clicking it still syncs the listing
        // and re-editing targets the same comment.
        assert_eq!(rows[2].addr, body);
        assert_eq!(rows[3].addr, gone);
        // Only one comment exists per address: the two rows at the function start do not
        // both carry it.
        assert_eq!(
            texts
                .iter()
                .filter(|text| text.contains("// setup"))
                .count(),
            1
        );
    }

    /// `/` (or `;`) in the Decompile view edits the trailing comment of the row under the
    /// cursor: the text lands in the `Pseudo` slot, trails that row, and an empty text
    /// deletes it again. The disassembly's own comment slot is left alone.
    #[test]
    fn decompile_view_edits_and_renders_trailing_comments() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("pseudo-comment.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_functions();
        app.tick();
        if app.functions.is_empty() {
            return;
        }
        app.pseudo_start = app.functions[0].start;
        app.pseudo_dirty = true;
        app.tick();
        let lines = app.pseudo.as_ref().expect("the entry function decompiles");
        let mut per_address = BTreeMap::new();
        for line in lines {
            *per_address.entry(line.addr).or_insert(0usize) += 1;
        }
        let index = lines
            .iter()
            .position(|line| {
                per_address[&line.addr] == 1
                    && !line.text.trim().is_empty()
                    && !line.text.trim_start().starts_with("//")
            })
            .expect("a row with its own address");
        let target = lines[index].addr;
        let code_text = lines[index].text.clone();
        let column = code_text.chars().count() + 2;

        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::EditPseudoComment { addr: Some(target) });
        let mut edit = app.edit.take().expect("the comment dialog must open");
        assert!(matches!(edit.kind, EditKind::PseudoComment));
        assert_eq!(edit.addr, target);
        edit.text = "first note\nsecond line".to_owned();
        app.run_edit(edit);
        app.tick();

        let rows = app.pseudo.as_ref().expect("the listing is re-rendered");
        let position = rows
            .iter()
            .position(|row| row.text.ends_with("// first note"))
            .expect("the comment is rendered");
        assert_eq!(
            rows[position].text,
            format!("{code_text}  // first note"),
            "the comment trails the annotated row"
        );
        assert_eq!(rows[position].addr, target);
        assert_eq!(
            rows[position + 1].text,
            format!("{}// second line", " ".repeat(column)),
            "continuation lines align under the comment marker"
        );
        assert_eq!(
            app.pseudo_comment_cols[position],
            Some(column),
            "the view knows where the trailing comment starts"
        );
        assert_eq!(
            app.session
                .as_ref()
                .unwrap()
                .comment_at(target, CommentKind::Pseudo)
                .unwrap()
                .as_deref(),
            Some("first note\nsecond line")
        );
        assert_eq!(
            app.session
                .as_ref()
                .unwrap()
                .comment_at(target, CommentKind::Regular)
                .unwrap(),
            None,
            "the Decompile comment must not leak into the disassembly slot"
        );

        // The extra row lays out and paints with the rest of the listing, coloured as a
        // comment (the view's syntax job treats `//` as one comment section).
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 600.0),
            )),
            ..egui::RawInput::default()
        };
        let mut action = Action::None;
        let mut output = ctx.run_ui(input, |ui| app.pseudo_view(ui, &mut action));
        output.textures_delta.clear();
        let comment_text = format!("{code_text}  // first note");
        assert!(
            output
                .shapes
                .iter()
                .any(|clipped| find_text_galley(&clipped.shape, &comment_text).is_some()),
            "the commented row must be painted with the listing: {comment_text:?}"
        );

        // Re-opening the dialog pre-fills the stored text; an empty one deletes it.
        app.apply_action(&ctx, Action::EditPseudoComment { addr: Some(target) });
        let mut edit = app.edit.take().expect("the second dialog must open");
        assert_eq!(
            edit.text, "first note\nsecond line",
            "the dialog pre-fills the comment"
        );
        edit.text = String::new();
        app.run_edit(edit);
        app.tick();
        let rows = app.pseudo.as_ref().unwrap();
        assert!(
            !rows.iter().any(|row| {
                row.text.contains("// first note") || row.text.contains("// second line")
            }),
            "an empty comment must remove every row of it"
        );
        assert!(
            rows.iter()
                .any(|row| row.addr == target && row.text == code_text),
            "deleting the comment restores the plain code row"
        );
        assert_eq!(
            app.session
                .as_ref()
                .unwrap()
                .comment_at(target, CommentKind::Pseudo)
                .unwrap(),
            None
        );

        // The keyboard path (`addr: None`) follows the cursor: with the listing synced to
        // the row, `/` targets that address (the comment is gone, so the dialog is empty).
        app.navigate(target);
        app.tick();
        app.apply_action(&ctx, Action::EditPseudoComment { addr: None });
        let edit = app.edit.take().expect("the cursor path opens the dialog");
        assert_eq!(edit.addr, target);
        assert_eq!(edit.text, "");
    }

    #[test]
    fn pseudo_gutter_stays_fixed_while_code_scrolls() {
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let row = egui::Rect::from_min_size(egui::pos2(0.0, 10.0), egui::vec2(1200.0, 15.0));
        let scrolled =
            egui::Rect::from_min_size(egui::pos2(-200.0, 10.0), egui::vec2(1200.0, 15.0));
        let (number, text, clip) = ReCraftApp::pseudo_row_geometry(viewport, row, 40.0);
        let (number_scrolled, text_scrolled, clip_scrolled) =
            ReCraftApp::pseudo_row_geometry(viewport, scrolled, 40.0);

        assert_eq!(
            number, number_scrolled,
            "line numbers must not scroll horizontally"
        );
        assert_eq!(
            clip.left(),
            clip_scrolled.left(),
            "the code clip must stay fixed"
        );
        assert!(
            (text.x - text_scrolled.x - 200.0).abs() < 0.01,
            "code text should follow the horizontal scroll offset"
        );
    }

    #[test]
    fn external_edit_marks_panels_dirty() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("external.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project.clone()),
            ..ReCraftApp::default()
        };
        app.after_open();
        assert!(app.data_version > 0, "after_open must record the baseline");

        // Another connection (an MCP server, a second GUI) renames the entry point.
        let entry = app.session.as_ref().unwrap().entry_point();
        let mut other = Session::open(&project).unwrap();
        other
            .execute(Command::Rename {
                addr: entry,
                name: Some("from_outside".to_owned()),
            })
            .unwrap();

        app.functions_dirty = false;
        app.disasm_dirty = false;
        app.pseudo_dirty = false;
        app.poll_external_changes();
        assert!(app.functions_dirty, "function list must reload");
        assert!(app.disasm_dirty, "disassembly must reload");
        assert!(app.pseudo_dirty, "decompile view must reload");

        // Polling again without a new commit does not keep invalidating the panes.
        app.functions_dirty = false;
        app.poll_external_changes();
        assert!(!app.functions_dirty);
    }

    #[test]
    fn data_ops_mark_flags_and_undo() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("dataops.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_disasm();
        app.tick();
        let line = app.disasm_rows[app.selected].clone();
        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::ApplyData(DataOp::Data));
        let flags = app.session.as_mut().unwrap().flags_at(line.addr).unwrap();
        assert_eq!(flags.kind, CodeData::Data, "D should mark data");
        app.apply_action(&ctx, Action::ApplyData(DataOp::Code));
        let flags = app.session.as_mut().unwrap().flags_at(line.addr).unwrap();
        assert_eq!(flags.kind, CodeData::Code, "C should mark code");
        app.apply_action(&ctx, Action::ApplyData(DataOp::Unknown));
        let flags = app.session.as_mut().unwrap().flags_at(line.addr).unwrap();
        assert_eq!(flags.kind, CodeData::Unknown, "U should undefine");
    }

    #[test]
    fn struct_form_defines_and_binds_argument_types() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("structs.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_functions();
        app.tick();
        if app.functions.is_empty() {
            return;
        }
        app.struct_name = "Probe".to_owned();
        app.struct_size = 24;
        app.struct_fields = "0:tag:u32, 16:value:i32".to_owned();
        app.define_struct_from_form();
        let structs = app.session.as_ref().unwrap().structs().unwrap();
        assert!(
            structs
                .iter()
                .any(|def| def.name == "Probe" && def.fields.len() == 2),
            "struct form did not persist: {structs:?}"
        );
        let func = app.functions[0].start;
        app.disasm_start = func;
        app.selected = 0;
        app.refresh_disasm();
        let target = app
            .current_function()
            .map(|(start, _)| start)
            .expect("cursor inside the first function");
        app.struct_name = "Probe".to_owned();
        app.struct_slot = 1;
        app.apply_struct_to_slot();
        assert!(
            app.status.contains("bound"),
            "apply failed with status: {}",
            app.status
        );
        let bound = app.session.as_ref().unwrap().arg_type(target, 1).unwrap();
        assert_eq!(bound.as_deref(), Some("Probe"));
    }

    #[test]
    fn prototype_edit_sets_and_clears() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("proto.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_functions();
        app.tick();
        if app.functions.is_empty() {
            return;
        }
        let start = app.functions[0].start;
        app.run_edit(EditState {
            kind: EditKind::Prototype,
            addr: start,
            text: "int probe(int a, int b);".to_owned(),
            just_opened: false,
        });
        assert_eq!(
            app.session
                .as_ref()
                .unwrap()
                .prototype_at(start)
                .unwrap()
                .as_deref(),
            Some("int probe(int a, int b);")
        );
        app.run_edit(EditState {
            kind: EditKind::Prototype,
            addr: start,
            text: String::new(),
            just_opened: false,
        });
        assert!(
            app.session
                .as_ref()
                .unwrap()
                .prototype_at(start)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn struct_field_spec_parser() {
        let field = parse_struct_field_spec("0x10:value:i32").unwrap();
        assert_eq!(field.offset, 16);
        assert_eq!(field.name, "value");
        assert_eq!(field.kind, "i32");
        assert!(parse_struct_field_spec("nope").is_err());
        assert!(parse_struct_field_spec("0:a:b:c").is_err());
    }

    #[test]
    fn collapsing_ai_sidebar_keeps_the_active_document_tab() {
        let mut app = ReCraftApp::default();
        app.ensure_dock();
        app.focus_pane(Pane::Pseudo);
        app.ai_sidebar = false;
        app.remove_ai_tab_from_dock();
        let state = app.dock.as_mut().expect("dock");
        let active = state.find_active_focused().map(|(_, tab)| *tab);
        assert_eq!(
            active,
            Some(Pane::Pseudo),
            "collapsing the sidebar must not switch the document tab"
        );
        assert!(state.find_tab(&Pane::Ai).is_none());
    }

    #[test]
    fn dock_default_layout_has_the_requested_tabs() {
        let mut app = ReCraftApp::default();
        app.ensure_dock();
        let state = app.dock.as_ref().expect("dock state");
        for pane in [
            Pane::View,
            Pane::Pseudo,
            Pane::Hex,
            Pane::Strings,
            Pane::Structs,
            Pane::Imports,
            Pane::Exports,
            Pane::Functions,
            Pane::GraphOverview,
            Pane::Output,
        ] {
            assert!(
                state.find_tab(&pane).is_some(),
                "default layout is missing {pane:?}"
            );
        }
        assert!(
            state.find_tab(&Pane::Ai).is_none(),
            "the AI tab is never part of the dock"
        );
        for hidden in [Pane::Names, Pane::Segments] {
            assert!(
                state.find_tab(&hidden).is_none(),
                "{hidden:?} should be hidden by default"
            );
        }
    }

    #[test]
    fn graph_overview_only_lives_in_graph_mode() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("overview-mode.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.ensure_dock();
        assert!(
            app.dock
                .as_ref()
                .unwrap()
                .find_tab(&Pane::GraphOverview)
                .is_some()
        );

        app.view_mode = ViewMode::Text;
        app.sync_graph_overview();
        let state = app.dock.as_ref().unwrap();
        assert!(
            state.find_tab(&Pane::GraphOverview).is_none(),
            "text mode must hide the overview, like IDA's graph-only window"
        );
        assert!(
            state.find_tab(&Pane::Functions).is_some(),
            "Functions takes the overview's space back"
        );

        app.view_mode = ViewMode::Graph;
        app.sync_graph_overview();
        assert!(
            app.dock
                .as_ref()
                .unwrap()
                .find_tab(&Pane::GraphOverview)
                .is_some(),
            "the overview comes back with the graph"
        );

        // Closing the project removes it too: there is no graph to overview.
        app.session = None;
        app.sync_graph_overview();
        assert!(
            app.dock
                .as_ref()
                .unwrap()
                .find_tab(&Pane::GraphOverview)
                .is_none()
        );
    }

    #[test]
    fn navigation_focuses_the_disassembly_pane() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("dock-nav.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.ensure_dock();
        let target = Address::new(app.disasm_start.get() + 0x20);
        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::NavigateToDisasm(target));
        let active = app
            .dock
            .as_mut()
            .and_then(|state| state.find_active_focused())
            .map(|(_, tab)| *tab);
        assert_eq!(active, Some(Pane::View));
        assert_eq!(app.view_mode, ViewMode::Text);
        assert_eq!(app.disasm_start, target);
    }

    #[test]
    fn views_menu_reopens_a_closed_pane() {
        let mut app = ReCraftApp::default();
        app.ensure_dock();
        let path = app
            .dock
            .as_ref()
            .and_then(|state| state.find_tab(&Pane::Functions))
            .expect("Functions pane");
        app.dock.as_mut().unwrap().remove_tab(path);
        assert!(
            app.dock
                .as_ref()
                .unwrap()
                .find_tab(&Pane::Functions)
                .is_none()
        );
        app.focus_pane(Pane::Functions);
        assert!(
            app.dock
                .as_ref()
                .unwrap()
                .find_tab(&Pane::Functions)
                .is_some(),
            "Views menu focus must re-dock a closed pane"
        );
    }

    #[test]
    fn dropped_pe_opens_a_project() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("dropped.exe");
        std::fs::write(&source, &bytes).unwrap();

        #[derive(Debug)]
        struct Dropped(std::path::PathBuf);
        impl egui::DroppedFile for Dropped {
            fn path(&self) -> &std::path::Path {
                &self.0
            }
            fn bytes(&self) -> Result<Vec<u8>, String> {
                std::fs::read(&self.0).map_err(|error| error.to_string())
            }
        }

        let ctx = egui::Context::default();
        let mut app = ReCraftApp::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            dropped_files: vec![Arc::new(Dropped(source.clone()))],
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
        output.textures_delta.clear();
        assert!(app.session.is_some(), "dropped PE must be imported");
        assert_eq!(app.project, Some(source.with_extension("rcraft")));
    }

    #[test]
    fn floating_ai_button_can_be_dragged() {
        let ctx = egui::Context::default();
        let mut app = ReCraftApp {
            ai_sidebar: false,
            ..ReCraftApp::default()
        };
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0));
        let mut run = |events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..egui::RawInput::default()
            };
            let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
            output.textures_delta.clear();
        };
        run(Vec::new());
        run(Vec::new());
        let start = egui::pos2(1360.0, 450.0);
        let end = egui::pos2(1100.0, 600.0);
        run(vec![
            egui::Event::PointerMoved(start),
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
        ]);
        run(vec![egui::Event::PointerMoved(end)]);
        run(vec![egui::Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        }]);
        let pos = app.ai_fab_pos.expect("button should be draggable");
        assert!(
            (pos.x - 1360.0).abs() > 100.0,
            "floating button did not move: {pos:?}"
        );
    }

    #[test]
    fn ai_sidebar_toggles_without_creating_a_dock_tab() {
        let ctx = egui::Context::default();
        let mut app = ReCraftApp::default();
        assert!(app.ai_sidebar);
        app.ensure_dock();
        app.apply_action(&ctx, Action::ToggleAiSidebar);
        assert!(!app.ai_sidebar);
        assert!(app.dock.as_ref().unwrap().find_tab(&Pane::Ai).is_none());
        app.apply_action(&ctx, Action::ToggleAiSidebar);
        assert!(app.ai_sidebar);
        app.apply_action(&ctx, Action::FocusPane(Pane::Ai));
        assert!(app.ai_sidebar);
        assert!(app.dock.as_ref().unwrap().find_tab(&Pane::Ai).is_none());
    }

    #[test]
    fn dock_layout_roundtrips_through_json() {
        let mut app = ReCraftApp::default();
        app.ensure_dock();
        let path = app
            .dock
            .as_ref()
            .and_then(|state| state.find_tab(&Pane::Functions))
            .expect("Functions pane");
        app.dock.as_mut().unwrap().remove_tab(path);
        // Lay the dock out once so pane rectangles carry finite coordinates.
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
        output.textures_delta.clear();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("layout.json");
        save_layout_to(
            &file,
            &SavedLayout {
                version: LAYOUT_VERSION,
                dock: app.dock.as_ref().unwrap().clone(),
                theme: ThemeMode::Light,
                zoom: 1.2,
                view_mode: ViewMode::Text,
                lang: Lang::Zh,
                ai_sidebar: true,
                ai_sidebar_width: 360.0,
                ai_fab_pos: Some([100.0, 200.0]),
            },
        )
        .unwrap();
        let loaded = load_layout_from(&file).expect("layout reload");
        assert_eq!(loaded.theme, ThemeMode::Light, "theme must persist");
        assert_eq!(loaded.lang, Lang::Zh, "language must persist");
        assert!(loaded.ai_sidebar, "AI sidebar visibility must persist");
        assert!(
            (loaded.ai_sidebar_width - 360.0).abs() < f32::EPSILON,
            "AI sidebar width must persist"
        );
        assert_eq!(
            loaded.ai_fab_pos,
            Some([100.0, 200.0]),
            "floating AI button position must persist"
        );
        assert!(
            (loaded.zoom - 1.2).abs() < f32::EPSILON,
            "zoom must persist"
        );
        assert_eq!(loaded.view_mode, ViewMode::Text, "view mode must persist");
        assert!(
            loaded.dock.find_tab(&Pane::View).is_some(),
            "primary view must survive a layout reload"
        );
        assert!(
            loaded.dock.find_tab(&Pane::Functions).is_none(),
            "closed panes must stay closed after reload"
        );
        // Stale layout versions and malformed files fall back to the default layout.
        let stale = dir.path().join("stale.json");
        std::fs::write(&stale, r#"{"version":1,"dock":{}}"#).unwrap();
        assert!(load_layout_from(&stale).is_none());
        assert!(
            load_layout_from(&dir.path().join("missing.json")).is_none(),
            "missing layout falls back to defaults"
        );
    }

    #[test]
    fn strings_decode_encodings_and_hide_only_header_rows() {
        assert_eq!(decode_string(b"hello\0", 1), "hello");
        assert_eq!(
            decode_string(&[b'h', 0, b'i', 0, 0, 0], 2),
            "hi",
            "UTF-16LE strings must decode to readable text"
        );
        // The list hides the DOS stub because IDA never maps the PE header as a segment,
        // so its list cannot contain it.
        assert!(is_header_string("HEADER"));
        assert!(is_header_string("header"));
        // Section names and compiler `$` names live in `.rdata` and IDA lists them, so they
        // must stay visible here too.
        assert!(!is_header_string(".rdata"));
        assert!(!is_header_string(""));
        assert!(!is_header_string(".text"));
    }

    /// A literal keeps one line in the Strings window: control characters are escaped instead
    /// of breaking the galley, and the escape keeps a `\t` out of the tab-separated row that
    /// Ctrl+C copies.
    #[test]
    fn string_cells_escape_control_characters() {
        // Printable text is passed through without a copy.
        assert!(matches!(
            string_cell_text("flag{strings_are_not_enough_you_know}"),
            Cow::Borrowed(_)
        ));
        // The PE DOS stub is the real-world case: it ends in a CR/LF pair and a NUL.
        assert_eq!(
            string_cell_text("This program cannot be run in DOS mode.\r\r\n$"),
            "This program cannot be run in DOS mode.\\r\\r\\n$"
        );
        assert_eq!(string_cell_text("a\tb\u{1}"), "a\\tb\\x01");
        // The string column is sized in monospace cells: CJK fallback glyphs are double wide.
        assert_eq!(string_cell_units("abcd"), 4);
        assert_eq!(string_cell_units("已加载"), 6);
    }

    /// Build a Strings row for the headless rendering tests below.
    fn ui_string_row(segment: &str, text: &str) -> StringRow {
        StringRow {
            addr: Address::new(0x140001000),
            len: text.len() as u32,
            enc: recraft_analysis::STRING_ENC_ASCII,
            kind: 0,
            segment: segment.to_owned(),
            text: text.to_owned(),
            raw: text.as_bytes().to_vec(),
            name: None,
            overridden: false,
        }
    }

    /// Render the Strings pane headlessly and return every painted text galley.
    fn strings_frame(
        app: &mut ReCraftApp,
        ctx: &egui::Context,
        input: &egui::RawInput,
    ) -> Vec<(String, egui::Pos2, egui::Vec2, usize)> {
        let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
        output.textures_delta.clear();
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some((
                    text.galley.text().to_owned(),
                    text.pos,
                    text.galley.size(),
                    text.galley.rows.len(),
                )),
                _ => None,
            })
            .collect()
    }

    /// Regression for the reported screenshot: the DOS stub row rendered as two lines because
    /// egui breaks a galley at `\n`, so its first line hung over the column header and the
    /// second over the next row, both clipped by the row rect. Escaped rows must paint as one
    /// line inside the row's height.
    #[test]
    fn headless_strings_row_with_a_newline_stays_on_one_line() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ui-strings-lines.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        let stub = "This program cannot be run in DOS mode.\r\r\n$";
        app.strings_dirty = false; // keep the hand-built rows: no database refresh this frame
        app.strings = vec![ui_string_row("HEADER", stub)];
        app.strings_hide_header = false; // the stub lives in the PE header
        app.focus_pane(Pane::Strings);
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let painted = strings_frame(&mut app, &ctx, &input);
        let (text, _, size, rows) = painted
            .iter()
            .find(|(text, ..)| text.contains("DOS mode"))
            .expect("the DOS stub row must be painted");
        assert_eq!(
            text, "This program cannot be run in DOS mode.\\r\\r\\n$",
            "the row must show the escapes instead of raw control characters"
        );
        assert_eq!(*rows, 1, "the row must stay on a single line");
        assert!(
            size.y <= app.row_height() + 4.0,
            "a single-line row must fit its height, got {} for {}",
            size.y,
            app.row_height()
        );
    }

    /// A literal wider than the pane must stay reachable: the list sizes its string column to
    /// the widest cell and scrolls sideways (as IDA's Strings window does) instead of cutting
    /// the text at the pane edge.
    #[test]
    fn headless_strings_list_scrolls_sideways_to_the_end_of_a_long_literal() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ui-strings-scroll.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        let long = format!("BEGIN-{}-END", "x".repeat(160));
        app.strings_dirty = false;
        app.strings = vec![ui_string_row(".rdata", &long)];
        app.focus_pane(Pane::Strings);
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let mut input = egui::RawInput {
            // The default window's minimum width: the literal cannot fit in the string column.
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            ..egui::RawInput::default()
        };
        let x_of = |painted: &[(String, egui::Pos2, egui::Vec2, usize)]| {
            painted
                .iter()
                .find(|(text, ..)| text == &long)
                .map(|(_, pos, ..)| pos.x)
                .expect("the long literal is painted")
        };
        let painted = strings_frame(&mut app, &ctx, &input);
        let before = x_of(&painted);
        let row_y = painted
            .iter()
            .find(|(text, ..)| text.starts_with(".rdata:"))
            .map(|(_, pos, ..)| pos.y)
            .expect("the row's address cell is painted");
        let row_x = painted
            .iter()
            .find(|(text, ..)| text.starts_with(".rdata:"))
            .map(|(_, pos, ..)| pos.x)
            .expect("the row's address cell is painted");

        // Hover the row's address cell (the string column starts past the pane edge here),
        // then wheel with Shift: egui maps that to the horizontal axis.
        let pointer = egui::pos2(row_x + 10.0, row_y + 4.0);
        let mut frame = 1.0f64;
        let mut step = |input: &mut egui::RawInput, app: &mut ReCraftApp| {
            frame += 1.0;
            input.time = Some(frame / 60.0);
            let painted = strings_frame(app, &ctx, input);
            input.events.clear();
            painted
        };
        input.events.push(egui::Event::PointerMoved(pointer));
        let _ = step(&mut input, &mut app);
        input.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -600.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::SHIFT,
        });
        let mut painted = step(&mut input, &mut app);
        // egui smooths a wheel notch over several frames.
        for _ in 0..40 {
            painted = step(&mut input, &mut app);
        }
        let after = x_of(&painted);
        assert!(
            after < before - 100.0,
            "the list must scroll sideways to reach the tail of a long literal: {before} -> {after}"
        );
    }

    /// The Output window must paint every log line whole and keep the rows apart. The rows
    /// used to be pinned at 11 px with 3 px spacing, so one line's descenders sat on the next
    /// line's ascenders and the thin top rows of a Chinese line looked cut off (the reported
    /// "已加载 那行顶部被挡住了").
    #[test]
    fn headless_output_pane_paints_whole_lines_apart() {
        let mut app = ReCraftApp {
            log: vec![
                "已加载 D:\\files\\projects\\ctf-simple-crackme\\challenge\\crackme_win_x64.rcraft"
                    .to_owned(),
                "把 .rcraft 项目或 PE 文件拖进窗口。".to_owned(),
            ],
            ..ReCraftApp::default()
        };
        app.focus_pane(Pane::Output);
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
        output.textures_delta.clear();
        let lines: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text)
                    if text.galley.text().starts_with("已加载")
                        || text.galley.text().starts_with("把 .rcraft") =>
                {
                    Some((text, clipped.clip_rect))
                }
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 2, "both log lines must be painted");
        for (text, clip) in &lines {
            assert_eq!(
                text.galley.job.sections[0].format.font_id.family,
                egui::FontFamily::Monospace,
                "the log stays on the listing's monospace font"
            );
            assert_eq!(text.galley.rows.len(), 1, "a log line stays on one line");
            let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
            assert!(
                clip.contains_rect(rect),
                "the log line must not be clipped: {rect:?} outside {clip:?}"
            );
        }
        let (first, second) = (&lines[0].0, &lines[1].0);
        assert!(
            second.pos.y >= first.pos.y + first.galley.size().y,
            "log lines must not overlap: {} then {}",
            first.pos.y,
            second.pos.y
        );
    }

    /// The Output window's log is selectable text, like IDA's: drag across two rows, then
    /// Ctrl+C copies exactly those rows onto the clipboard. The rows used to be painted as
    /// plain galleys, so the mouse could not select anything in the window at all.
    #[test]
    fn headless_output_log_copies_a_dragged_selection() {
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let mut app = ReCraftApp {
            log: vec![
                "analysis: 第一条日志行".to_owned(),
                "analysis: second line".to_owned(),
                "analysis: not selected".to_owned(),
            ],
            ..ReCraftApp::default()
        };
        let mut action = Action::None;
        let mut frame = 0.0f64;
        let mut step = |events: Vec<egui::Event>| {
            frame += 1.0;
            let input = egui::RawInput {
                time: Some(frame / 60.0),
                events,
                ..test_input()
            };
            let mut output = ctx.run_ui(input, |ui| app.output_pane(ui, &mut action));
            output.textures_delta.clear();
            output
        };

        let output = step(vec![]);
        let first_line = "analysis: 第一条日志行";
        let second_line = "analysis: second line";
        let find = |needle: &str, index: usize| {
            output
                .shapes
                .iter()
                .find_map(|clipped| char_click_pos(&clipped.shape, needle, index))
        };
        let start = find(first_line, 0).expect("the first row is on screen");
        let end = find(second_line, 9).expect("the second row is on screen");
        step(vec![egui::Event::PointerMoved(start)]);
        step(vec![press(start)]);
        step(vec![egui::Event::PointerMoved(end)]);
        step(vec![release(end)]);

        let selection = app.log_sel.expect("the drag must select log text");
        let (from, to) = selection.ordered();
        assert_eq!((from.row, to.row), (0, 1), "the drag spans both rows");
        assert_eq!(
            app.copy_focus,
            CopyFocus::Output,
            "dragging in the log gives it the clipboard"
        );

        // Ctrl+C now goes through the app's copy path, like the other panes.
        app.apply_action(&ctx, Action::Copy);
        let copied = copied_text(&ctx).expect("Ctrl+C must copy the dragged log rows");
        assert_eq!(
            copied.lines().count(),
            2,
            "the drag covers the first two rows only, got {copied:?}"
        );
        assert!(
            copied.contains("第一条日志行") && copied.contains("analysis:"),
            "both dragged rows are on the clipboard, got {copied:?}"
        );
        assert!(
            !copied.contains("second line"),
            "rows below the drag stay out of the copy, got {copied:?}"
        );
        // A drag that stops in the middle of a row copies a fragment: reporting characters
        // keeps "已复制 1 行" from claiming a whole line was copied.
        let chars = copied
            .chars()
            .filter(|character| *character != '\n')
            .count();
        assert_eq!(
            app.status,
            app.t_args("ui_copied_chars", &[("count", &chars.to_string())]),
            "a partial selection is reported as characters"
        );
    }

    /// End to end through the dock: select three whole log rows in the real Output pane,
    /// press Ctrl+C, and the clipboard must hold them with the status reporting three lines.
    #[test]
    fn headless_output_copy_through_the_dock_reports_every_row() {
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let mut app = ReCraftApp {
            log: (1..=4)
                .map(|line| format!("analysis: line {line}"))
                .collect(),
            ..ReCraftApp::default()
        };
        let mut frame = 0.0f64;
        let mut run = |app: &mut ReCraftApp, events: Vec<egui::Event>| {
            frame += 1.0;
            let input = egui::RawInput {
                time: Some(frame / 60.0),
                events,
                ..test_input()
            };
            let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
            output.textures_delta.clear();
            output
        };
        let mut output = run(&mut app, vec![]);
        // `stick_to_bottom` animates the scroll offset for the first few frames; press only
        // once the rows have stopped moving.
        for _ in 0..8 {
            output = run(&mut app, vec![]);
        }
        let find = |needle: &str, index: usize| {
            output
                .shapes
                .iter()
                .find_map(|clipped| char_click_pos(&clipped.shape, needle, index))
        };
        let start = find("analysis: line 1", 0).expect("the first log row is in the dock");
        // Past the end of line 3's text but still inside its row: the caret goes to its end.
        let end = find("analysis: line 3", 0).expect("the third log row is in the dock")
            + egui::vec2(400.0, 4.0);
        run(&mut app, vec![egui::Event::PointerMoved(start)]);
        run(&mut app, vec![press(start)]);
        run(&mut app, vec![egui::Event::PointerMoved(end)]);
        run(&mut app, vec![release(end)]);

        let output = run(&mut app, vec![egui::Event::Copy]);
        let selection = app.log_sel.expect("the drag must select log text");
        let (from, to) = selection.ordered();
        assert_eq!((from.row, to.row), (0, 2), "the drag spans three rows");
        let copied = output
            .platform_output
            .commands
            .iter()
            .find_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
            .expect("Ctrl+C must reach the app's copy command");
        assert_eq!(copied.lines().count(), 3, "got {copied:?}");
        assert!(
            copied.contains("line 1") && copied.contains("line 2") && copied.contains("line 3"),
            "got {copied:?}"
        );
        assert!(
            !copied.contains("line 4"),
            "rows below the drag stay out of the copy, got {copied:?}"
        );
        assert_eq!(
            app.status,
            app.t_args("ui_copied_lines", &[("count", "3")]),
            "the status must report the three copied rows"
        );
    }

    /// IDA selects the whole Output window with Ctrl+A, not just the line under the cursor.
    #[test]
    fn ctrl_a_selects_the_whole_output_log() {
        let ctx = egui::Context::default();
        let mut app = ReCraftApp {
            log: vec![
                "analysis: one".to_owned(),
                "error: two".to_owned(),
                "analysis: three".to_owned(),
            ],
            copy_focus: CopyFocus::Output,
            ..ReCraftApp::default()
        };
        app.apply_action(&ctx, Action::SelectAll);
        let selection = app.log_sel.expect("Ctrl+A must select every log row");
        assert_eq!(
            selection.ordered(),
            (
                TextPoint { row: 0, char: 0 },
                TextPoint {
                    row: 2,
                    char: "analysis: three".chars().count(),
                }
            )
        );
        app.apply_action(&ctx, Action::Copy);
        assert_eq!(
            copied_text(&ctx).as_deref(),
            Some("analysis: one\nerror: two\nanalysis: three"),
            "the clipboard holds the whole log, one line per row"
        );
        assert_eq!(
            app.status,
            app.t_args("ui_copied_lines", &[("count", "3")]),
            "the whole log is reported as three rows"
        );
    }

    /// A log line longer than the pane wraps; selecting all of it must highlight every
    /// layout row it occupies, not just the first screen row.
    #[test]
    fn headless_output_selection_highlights_wrapped_rows() {
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let long = format!("analysis: {}", "0123456789".repeat(12));
        let mut app = ReCraftApp {
            log: vec![long.clone()],
            ..ReCraftApp::default()
        };
        let mut action = Action::None;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(260.0, 240.0),
            )),
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input.clone(), |ui| app.output_pane(ui, &mut action));
        output.textures_delta.clear();
        let rows = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text().starts_with("analysis:") => {
                    Some(text.galley.rows.len())
                }
                _ => None,
            })
            .expect("the log row is painted");
        assert!(rows > 1, "the narrow pane must wrap the long line");

        app.log_sel = Some(TextSelection {
            anchor: TextPoint { row: 0, char: 0 },
            focus: TextPoint {
                row: 0,
                char: long.chars().count(),
            },
        });
        let mut output = ctx.run_ui(input, |ui| app.output_pane(ui, &mut action));
        output.textures_delta.clear();
        let highlights: Vec<egui::Rect> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect) if rect.fill == app.palette().text_selection => {
                    Some(rect.rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            highlights.len(),
            rows,
            "every wrapped row needs its own selection rect, got {highlights:?}"
        );
        for pair in highlights.windows(2) {
            assert!(
                pair[0].bottom() <= pair[1].top() + 0.5,
                "the highlights must stack down the wrapped rows, got {highlights:?}"
            );
        }
    }

    #[test]
    fn word_ranges_cover_identifier_and_punctuation_runs() {
        let text = "call sub_1400010a0();";
        assert_eq!(word_range_at(text, 6), (5, 18), "the whole symbol");
        assert_eq!(word_range_at(text, 18), (18, 21), "the punctuation run");
        assert_eq!(word_range_at(text, 4), (4, 4), "whitespace selects nothing");
        assert_eq!(word_range_at("", 0), (0, 0));
    }

    /// IDA's double click selects the word under the cursor in the Output window too.
    #[test]
    fn headless_output_double_click_selects_a_word() {
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let line = "analysis: entered sub_1400010a0";
        let mut app = ReCraftApp {
            log: vec![line.to_owned()],
            ..ReCraftApp::default()
        };
        let mut action = Action::None;
        let mut frame = 0.0f64;
        let mut step = |events: Vec<egui::Event>| {
            frame += 1.0;
            let input = egui::RawInput {
                time: Some(frame / 60.0),
                events,
                ..test_input()
            };
            let mut output = ctx.run_ui(input, |ui| app.output_pane(ui, &mut action));
            output.textures_delta.clear();
            output
        };
        let output = step(vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|clipped| char_click_pos(&clipped.shape, line, 20))
            .expect("the log row is on screen");
        step(vec![egui::Event::PointerMoved(pos)]);
        for _ in 0..2 {
            step(vec![press(pos)]);
            step(vec![release(pos)]);
        }
        let selection = app.log_sel.expect("the double click must select a word");
        let (from, to) = selection.ordered();
        assert_eq!(
            (from.row, from.char, to.char),
            (0, 18, 31),
            "only the symbol is selected"
        );
        app.apply_action(&ctx, Action::Copy);
        assert_eq!(copied_text(&ctx).as_deref(), Some("sub_1400010a0"));
        assert_eq!(
            app.status,
            app.t_args("ui_copied_chars", &[("count", "13")]),
            "a selected word is reported as characters, not as a whole line"
        );
    }

    #[test]
    fn string_row_at_finds_interior_addresses() {
        fn row(addr: u64, len: u32, text: &str) -> StringRow {
            StringRow {
                addr: Address::new(addr),
                len,
                enc: recraft_analysis::STRING_ENC_ASCII,
                kind: 0,
                segment: ".rdata".to_owned(),
                text: text.to_owned(),
                raw: text.as_bytes().to_vec(),
                name: None,
                overridden: false,
            }
        }
        let strings = vec![row(0x1000, 10, "O=========")];
        assert_eq!(
            string_row_at(&strings, Address::new(0x1000)),
            Some((0, 0)),
            "an exact start wins"
        );
        assert_eq!(
            string_row_at(&strings, Address::new(0x1005)),
            Some((0, 5)),
            "a literal can point inside a longer run"
        );
        assert_eq!(string_row_at(&strings, Address::new(0x100a)), None);
        assert_eq!(string_row_at(&strings, Address::new(0x0fff)), None);

        // Overlapping extracted ranges: the row that starts there is the right one.
        let strings = vec![row(0x1000, 12, "=====abcdefg"), row(0x1005, 3, "abc")];
        assert_eq!(string_row_at(&strings, Address::new(0x1005)), Some((1, 0)));
        assert_eq!(
            string_row_at(&strings, Address::new(0x1006)),
            Some((1, 1)),
            "the nearest preceding start describes an interior address best"
        );
    }

    #[test]
    fn c_literal_bytes_reverse_the_decompiler_escapes() {
        assert_eq!(
            c_literal_bytes(r#"a\"b\\c\n\t"#).unwrap(),
            b"a\"b\\c\n\t".to_vec()
        );
        assert_eq!(c_literal_bytes(r"\x01\x7f").unwrap(), vec![1, 0x7f]);
        assert_eq!(c_literal_bytes("plain").unwrap(), b"plain".to_vec());
    }

    #[test]
    fn rebuilding_strings_applies_the_minimum_length_setting() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/x64/corpus.exe");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("strings-setup.rcraft");
        let _ = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let _ = run_analysis_job(&project, &cancel);

        let session = Session::open(&project).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project.clone()),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        // The default is IDA 8.x's five characters, so the corpus's four-character `.bss`
        // is not listed until the minimum is lowered.
        assert_eq!(
            app.strings_min_len,
            recraft_analysis::DEFAULT_MIN_STRING_LEN
        );
        assert!(
            !app.strings.iter().any(|row| row.text == ".bss"),
            "four-character strings are below the default minimum"
        );

        app.start_strings_job(4);
        for _ in 0..500 {
            app.tick();
            if app.strings_job.is_none() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(app.strings_job.is_none(), "the rebuild did not finish");
        assert_eq!(app.strings_min_len, 4);
        assert!(
            app.strings.iter().any(|row| row.text == ".bss"),
            "the rebuild at 4 characters must list `.bss` again"
        );

        // The setting is project state, not a transient UI toggle.
        let session = Session::open(&project).unwrap();
        assert_eq!(recraft_analysis::string_min_len(&session), 4);
    }

    /// A user edit wins over the decoded bytes, deletion drops the row, and both survive a
    /// reopen: the `string` table is derived, the override is user data.
    #[test]
    fn string_overrides_edit_and_delete_rows() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/x64/corpus.exe");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("string-override.rcraft");
        let _ = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let _ = run_analysis_job(&project, &cancel);

        let session = Session::open(&project).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project.clone()),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        let (addr, decoded) = {
            let row = app.strings.first().expect("corpus has strings");
            (row.addr, row.text.clone())
        };

        app.apply_string_edit(addr, Some("====".to_owned()), false);
        let row = app
            .strings
            .iter()
            .find(|row| row.addr == addr)
            .expect("the edited row stays in the list");
        assert_eq!(row.text, "====");
        assert!(row.overridden);

        app.apply_string_edit(addr, None, true);
        assert!(
            app.strings.iter().all(|row| row.addr != addr),
            "a deleted row leaves the list"
        );

        // Deletion is a journaled command: undo puts the row (and its edit) back.
        app.apply_action(&egui::Context::default(), Action::Undo);
        app.tick();
        let row = app
            .strings
            .iter()
            .find(|row| row.addr == addr)
            .expect("undo restores the deleted row");
        assert_eq!(row.text, "====", "undo keeps the earlier edit");
        app.apply_string_edit(addr, None, true);

        // Both edits are user data: they survive closing and reopening the project.
        let session = Session::open(&project).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            project: Some(project.clone()),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        assert!(app.strings.iter().all(|row| row.addr != addr));

        // Restoring the row brings the decoded bytes back.
        app.apply_string_edit(addr, None, false);
        let row = app
            .strings
            .iter()
            .find(|row| row.addr == addr)
            .expect("the row is back after restoring it");
        assert_eq!(row.text, decoded);
        assert!(!row.overridden);
    }

    #[test]
    fn short_literals_without_a_string_row_still_open_as_data() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/x64/corpus.exe");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("short-literal.rcraft");
        let _ = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let _ = run_analysis_job(&project, &cancel);
        let session = Session::open(&project).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.tick();
        let banner = app
            .strings
            .iter()
            .find(|row| row.text == "ReCraft analysis corpus")
            .cloned()
            .expect("the corpus banner is a stored string");
        // An address no stored string covers, like a short literal below the extraction
        // minimum (the corpus has no such literal, so pick the first free address).
        let free = (1..0x2000u64)
            .map(|offset| Address::new(banner.addr.get() + offset))
            .find(|addr| string_row_at(&app.strings, *addr).is_none())
            .unwrap();
        let text = "  sub_1(\"\\r\\n\");";
        let caret = text.find('\\').unwrap();
        app.pseudo_literals = vec![("\"\\r\\n\"".to_owned(), free)];
        let ctx = egui::Context::default();
        let action = app.pseudo_double_click(text, caret, app.disasm_start);
        assert_eq!(action, Action::NavigateToDisasm(free));
        app.apply_action(&ctx, action);
        app.tick();
        let line = app.disasm_rows.first().expect("the short literal is data");
        assert_eq!(line.addr, free);
        assert_eq!(line.mnemonic, "db");
        assert!(
            line.operands.contains("\\r\\n"),
            "the quoted escapes stay visible, got {:?}",
            line.operands
        );
        assert!(line.flags.is_data());

        // The literal is cached, so a later disassembly refresh keeps the data row even
        // though `refresh_pseudo` has already dropped the listing's literal table.
        app.disasm_dirty = true;
        app.tick();
        assert_eq!(
            app.disasm_rows.first().map(|line| line.mnemonic.as_str()),
            Some("db")
        );
    }

    #[test]
    fn ai_job_result_updates_status_and_explanation() {
        let (sender, receiver) = mpsc::channel();
        sender
            .send(AiMessage::Done(Ok("int main(void) {}".to_owned())))
            .unwrap();
        let mut app = ReCraftApp {
            ai_job: Some(AiJob {
                receiver,
                started: Instant::now(),
                kind: AiJobKind::Polish,
                prompt: None,
                target: None,
                suggestions: Vec::new(),
            }),
            ..ReCraftApp::default()
        };
        app.tick();
        assert_eq!(app.ai_explanation.as_deref(), Some("int main(void) {}"));
        assert_eq!(
            app.ai_result_kind,
            AiResultKind::Code,
            "a polish answer is coloured as code"
        );
        assert!(
            app.ai_status.starts_with("polish done"),
            "{}",
            app.ai_status
        );
        assert!(app.ai_job.is_none());
    }

    #[test]
    fn ai_result_colours_polished_code_but_not_prose() {
        let mono = FontId::monospace(11.0);
        let code = "if (rcx != 0) sub_140001000(\"x\", 0x10); // note";
        let mut job = LayoutJob::default();
        append_ai_result(&mut job, code, AiResultKind::Code, Palette::DARK, &mono);
        assert_eq!(job.text, code, "colour sections must not change the text");
        let color = |needle: &str| {
            job.sections
                .iter()
                .find(|section| {
                    let range = &section.byte_range;
                    &job.text[range.start.0..range.end.0] == needle
                })
                .map(|section| section.format.color)
                .unwrap_or_else(|| panic!("no section for {needle:?}"))
        };
        assert_eq!(color("if"), Palette::DARK.mnemonic);
        assert_eq!(color("rcx"), Palette::DARK.register);
        assert_eq!(color("sub_140001000"), Palette::DARK.name);
        assert_eq!(color("0x10"), Palette::DARK.number);
        assert_eq!(color("// note"), Palette::DARK.comment);

        // An explanation report is prose: colouring it as C would tint random words.
        let prose = "This function opens a file.";
        let mut job = LayoutJob::default();
        append_ai_result(&mut job, prose, AiResultKind::Prose, Palette::DARK, &mono);
        assert_eq!(job.text, prose);
        assert_eq!(job.sections.len(), 1, "prose stays one plain section");
        assert_eq!(job.sections[0].format.color, Palette::DARK.text);
    }

    #[test]
    fn ai_stream_chunks_update_explanation() {
        let (sender, receiver) = mpsc::channel();
        let mut app = ReCraftApp {
            ai_job: Some(AiJob {
                receiver,
                started: Instant::now(),
                kind: AiJobKind::Polish,
                prompt: None,
                target: None,
                suggestions: Vec::new(),
            }),
            ..ReCraftApp::default()
        };
        sender.send(AiMessage::Chunk("int ".to_owned())).unwrap();
        app.poll_ai_job();
        assert_eq!(app.ai_explanation.as_deref(), Some("int "));
        assert!(app.ai_job.is_some());
        sender
            .send(AiMessage::Chunk("main(void)".to_owned()))
            .unwrap();
        sender
            .send(AiMessage::Done(Ok("int main(void)".to_owned())))
            .unwrap();
        app.poll_ai_job();
        assert_eq!(app.ai_explanation.as_deref(), Some("int main(void)"));
        assert!(app.ai_job.is_none());
    }

    #[test]
    fn ai_reasoning_streams_expands_and_folds_when_done() {
        let (sender, receiver) = mpsc::channel();
        let mut app = ReCraftApp {
            ai_job: Some(AiJob {
                receiver,
                started: Instant::now(),
                kind: AiJobKind::Polish,
                prompt: None,
                target: None,
                suggestions: Vec::new(),
            }),
            ai_thinking: true,
            ..ReCraftApp::default()
        };
        sender
            .send(AiMessage::Reasoning("plan ".to_owned()))
            .unwrap();
        app.poll_ai_job();
        assert_eq!(app.ai_reasoning, "plan ");
        assert!(
            app.ai_reasoning_open,
            "thinking stays expanded while the model reasons"
        );
        assert!(app.ai_explanation.is_none(), "no answer yet");

        sender
            .send(AiMessage::Reasoning("first".to_owned()))
            .unwrap();
        sender
            .send(AiMessage::Chunk("int main".to_owned()))
            .unwrap();
        sender
            .send(AiMessage::Done(Ok("int main".to_owned())))
            .unwrap();
        app.poll_ai_job();
        assert_eq!(app.ai_reasoning, "plan first");
        assert!(
            !app.ai_reasoning_open,
            "thinking folds once the answer completes"
        );
        assert_eq!(app.ai_explanation.as_deref(), Some("int main"));
        assert!(app.ai_job.is_none());
    }

    #[test]
    fn direct_answer_budget_is_capped_for_opencode() {
        assert_eq!(
            direct_answer_token_limit("https://opencode.ai/inference/go/openai", 65536),
            8192
        );
        assert_eq!(
            direct_answer_token_limit("https://api.deepseek.com", 65536),
            65536
        );
        assert_eq!(direct_answer_token_limit("https://opencode.ai", 2048), 2048);
    }

    #[test]
    fn switching_provider_clears_the_session_key_and_endpoint() {
        let mut app = ReCraftApp::default();
        recraft_ai::ProviderPreset::DeepSeek.apply(&mut app.ai_settings);
        app.ai_mode = AiMode::DeepSeek;
        app.ai_settings.api_key = Some("deepseek-secret".to_owned());
        app.ai_settings.api_key_env = "DEEPSEEK_KEY".to_owned();
        app.ai_api_key = "deepseek-secret".to_owned();
        app.ai_settings.enabled = true;

        // Selecting the already-active provider must not wipe a valid key.
        app.switch_ai_mode(AiMode::DeepSeek);
        assert_eq!(app.ai_api_key, "deepseek-secret");

        app.switch_ai_mode(AiMode::OpenCodeGo);
        assert_eq!(app.ai_settings.endpoint, recraft_ai::OPENCODE_GO_ENDPOINT);
        assert!(app.ai_settings.api_key.is_none());
        assert!(app.ai_api_key.is_empty());
        assert_eq!(
            app.ai_settings.api_key_env,
            recraft_ai::ProviderSettings::default().api_key_env
        );
        assert!(!app.ai_settings.enabled);
        assert!(app.ai_status.contains("API key"), "{}", app.ai_status);

        app.switch_ai_mode(AiMode::Custom);
        assert_eq!(app.ai_settings.endpoint, "https://api.openai.com");
        assert_eq!(
            app.ai_settings.model,
            recraft_ai::ProviderSettings::default().model
        );

        app.switch_ai_mode(AiMode::DeepSeek);
        assert_eq!(app.ai_api_key, "deepseek-secret");
        assert_eq!(app.ai_settings.api_key.as_deref(), Some("deepseek-secret"));
        assert!(app.ai_settings.enabled);
        assert!(
            app.ai_status.contains("API key"),
            "restored status: {}",
            app.ai_status
        );

        // Raw endpoint edits in Custom mode use the same per-provider key memory.
        app.ai_settings.endpoint = "https://custom.example/v1".to_owned();
        app.sync_provider_key_identity();
        assert!(app.ai_api_key.is_empty());
        app.ai_settings.endpoint = recraft_ai::DEEPSEEK_ENDPOINT.to_owned();
        app.sync_provider_key_identity();
        assert_eq!(app.ai_api_key, "deepseek-secret");
    }

    #[test]
    fn prompt_editors_follow_the_selected_language() {
        let mut app = ReCraftApp::default();
        app.ai_settings.prompt_language = recraft_ai::PromptLanguage::En;
        app.reload_prompt_drafts();
        assert_eq!(app.prompt_drafts.len(), 5);
        assert_eq!(
            app.prompt_drafts[2].1,
            recraft_ai::POLISH_SYSTEM,
            "the editor starts from the built-in English prompt"
        );

        app.ai_settings.prompt_language = recraft_ai::PromptLanguage::Zh;
        app.reload_prompt_drafts();
        assert_eq!(app.prompt_drafts[2].1, recraft_ai::POLISH_SYSTEM_ZH);

        app.ai_settings
            .set_prompt_override(recraft_ai::PromptKind::Polish, Some("自定义".to_owned()));
        app.reload_prompt_drafts();
        assert_eq!(app.prompt_drafts[2].1, "自定义");
    }

    #[test]
    fn ai_modes_map_to_builtin_presets() {
        use recraft_ai::ProviderPreset;
        let mut settings = recraft_ai::ProviderSettings::default();
        ProviderPreset::DeepSeek.apply(&mut settings);
        assert_eq!(AiMode::from_settings(&settings), AiMode::DeepSeek);
        ProviderPreset::OpenCodeGo.apply(&mut settings);
        assert_eq!(AiMode::from_settings(&settings), AiMode::OpenCodeGo);
        assert_eq!(settings.endpoint, recraft_ai::OPENCODE_GO_ENDPOINT);
        assert!(settings.model.starts_with("deepseek-v4"));

        settings.endpoint = "https://example.test".to_owned();
        assert_eq!(AiMode::from_settings(&settings), AiMode::Custom);
    }

    #[test]
    fn view_mode_toggles_between_graph_and_text() {
        let mut app = ReCraftApp::default();
        let ctx = egui::Context::default();
        assert_eq!(app.view_mode, ViewMode::Graph);
        app.apply_action(&ctx, Action::ToggleViewMode);
        assert_eq!(app.view_mode, ViewMode::Text);
        app.apply_action(&ctx, Action::ToggleViewMode);
        assert_eq!(app.view_mode, ViewMode::Graph);
        app.apply_action(&ctx, Action::ToggleViewMode);
        assert_eq!(app.view_mode, ViewMode::Text);
    }

    #[test]
    fn zoom_actions_clamp_and_reset() {
        let mut app = ReCraftApp::default();
        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::ZoomIn);
        assert!(app.mono_zoom > 1.0);
        for _ in 0..50 {
            app.apply_action(&ctx, Action::ZoomIn);
        }
        assert_eq!(app.mono_zoom, ZOOM_MAX);
        app.apply_action(&ctx, Action::ZoomReset);
        assert_eq!(app.mono_zoom, 1.0);
        for _ in 0..50 {
            app.apply_action(&ctx, Action::ZoomOut);
        }
        assert_eq!(app.mono_zoom, ZOOM_MIN);
    }

    #[test]
    fn graph_layout_uses_real_cfg_geometry() {
        use recraft_analysis::{BasicBlock, Cfg, CfgInsn, EdgeKind};
        let block =
            |index: usize, start: u64, end: u64, edges: Vec<(usize, EdgeKind)>| BasicBlock {
                index,
                start: Address::new(start),
                end: Address::new(end),
                insns: vec![CfgInsn {
                    addr: Address::new(start),
                    text: "ret".to_owned(),
                }],
                edges,
            };
        let cfg = Cfg {
            function: Address::new(0x1000),
            entry: 0,
            blocks: vec![
                block(0, 0x1000, 0x1001, vec![(1, EdgeKind::Unconditional)]),
                block(1, 0x1001, 0x1002, Vec::new()),
            ],
            external: Vec::new(),
            jump_tables: Default::default(),
        };
        let (rects, canvas) = graph_layout(&cfg, 1.0);
        assert_eq!(rects.len(), 2);
        assert!(
            rects[1].min.x > rects[0].min.x,
            "the target block must sit in a later layer"
        );
        assert!(canvas.x > rects[1].max.x, "canvas must contain the nodes");
        assert!(canvas.y >= rects[0].max.y);
    }

    #[test]
    fn operand_tokens_are_classified_like_ida() {
        assert_eq!(classify_operand("rsp"), OperandKind::Register);
        assert_eq!(classify_operand("xmm3"), OperandKind::Register);
        assert_eq!(classify_operand("r8d"), OperandKind::Register);
        assert_eq!(classify_operand("r9w"), OperandKind::Register);
        assert_eq!(classify_operand("r15b"), OperandKind::Register);
        assert_eq!(classify_operand("ymm2"), OperandKind::Register);
        assert_eq!(classify_operand("gs"), OperandKind::Register);
        assert_eq!(classify_operand("0x140001f98"), OperandKind::Number);
        assert_eq!(classify_operand("0x28"), OperandKind::Number);
        assert_eq!(classify_operand("sub_140001f98"), OperandKind::Name);
        assert_eq!(classify_operand("loc_140001234"), OperandKind::Name);
        assert_eq!(classify_operand("\"hello\""), OperandKind::String);
        assert_eq!(classify_operand("foo"), OperandKind::Other);
        let mut job = LayoutJob::default();
        let mono = FontId::monospace(12.0);
        append_operands(&mut job, "rsp, 0x28 ; note", Palette::DARK, &mono);
        assert!(
            job.sections.len() >= 3,
            "operands must use several coloured sections"
        );
    }

    #[test]
    fn pseudo_c_words_are_classified_like_hex_rays() {
        assert_eq!(pseudo_word_kind("if"), Some(PseudoKind::Keyword));
        assert_eq!(pseudo_word_kind("else"), Some(PseudoKind::Keyword));
        assert_eq!(pseudo_word_kind("return"), Some(PseudoKind::Keyword));
        assert_eq!(pseudo_word_kind("__asm"), Some(PseudoKind::Keyword));
        assert_eq!(pseudo_word_kind("int64"), Some(PseudoKind::Type));
        assert_eq!(pseudo_word_kind("uint32"), Some(PseudoKind::Type));
        assert_eq!(pseudo_word_kind("__m128"), Some(PseudoKind::Type));
        assert_eq!(pseudo_word_kind("r8d"), Some(PseudoKind::Register));
        assert_eq!(pseudo_word_kind("xmm0"), Some(PseudoKind::Register));
        assert_eq!(pseudo_word_kind("SendMessageW"), Some(PseudoKind::Name));
        assert_eq!(pseudo_word_kind("loc_140001234"), Some(PseudoKind::Name));
        assert_eq!(pseudo_word_kind("qword_140001000"), Some(PseudoKind::Name));
        assert_eq!(pseudo_word_kind("_initterm"), Some(PseudoKind::Name));
        assert_eq!(pseudo_word_kind("var_40"), None);
        assert_eq!(pseudo_word_kind("arg1"), None);
        assert_eq!(pseudo_word_kind("field_1a"), None);
        assert_eq!(pseudo_word_kind("_"), None);
    }

    #[test]
    fn pseudo_c_line_is_split_into_coloured_tokens() {
        let line = r#"if (r8d != 0) sub_140001000("a\"b", 0x10); // note"#;
        let mut job = LayoutJob::default();
        let mono = FontId::monospace(12.0);
        append_pseudo_line(&mut job, line, Palette::DARK, &mono);
        assert_eq!(job.text, line, "colour sections must not change the text");
        let color = |needle: &str| {
            job.sections
                .iter()
                .find(|section| {
                    let range = &section.byte_range;
                    &job.text[range.start.0..range.end.0] == needle
                })
                .map(|section| section.format.color)
                .unwrap_or_else(|| panic!("no section for {needle:?}"))
        };
        assert_eq!(color("if"), Palette::DARK.mnemonic);
        assert_eq!(color("r8d"), Palette::DARK.register);
        assert_eq!(color("0x10"), Palette::DARK.number);
        assert_eq!(color(r#""a\"b""#), Palette::DARK.comment);
        assert_eq!(color("// note"), Palette::DARK.comment);
        assert_eq!(color("sub_140001000"), Palette::DARK.name);
        assert_eq!(color("("), Palette::DARK.text);

        // A lower-case CRT call has no upper-case letter or auto prefix; the call
        // parentheses are what mark it as a symbol.
        let mut job = LayoutJob::default();
        append_pseudo_line(&mut job, "wcsnlen(arg4, 0x20);", Palette::DARK, &mono);
        let color = |needle: &str| {
            job.sections
                .iter()
                .find(|section| {
                    let range = &section.byte_range;
                    &job.text[range.start.0..range.end.0] == needle
                })
                .map(|section| section.format.color)
                .unwrap_or_else(|| panic!("no section for {needle:?}"))
        };
        assert_eq!(color("wcsnlen"), Palette::DARK.name);
        assert_eq!(color("(arg4, "), Palette::DARK.text);
    }

    #[test]
    fn pseudo_c_comments_and_strings_swallow_their_contents() {
        let mut job = LayoutJob::default();
        let mono = FontId::monospace(12.0);
        append_pseudo_line(&mut job, "// if (rax) {", Palette::DARK, &mono);
        assert_eq!(job.text, "// if (rax) {");
        assert_eq!(job.sections.len(), 1, "a comment is one colour section");
        assert_eq!(job.sections[0].format.color, Palette::DARK.comment);

        let mut job = LayoutJob::default();
        append_pseudo_line(
            &mut job,
            r#"if (edi < 2 /* signed */) { "#,
            Palette::DARK,
            &mono,
        );
        let comment = job
            .sections
            .iter()
            .find(|section| {
                let range = &section.byte_range;
                &job.text[range.start.0..range.end.0] == "/* signed */"
            })
            .expect("the inline comment must be a section of its own");
        assert_eq!(comment.format.color, Palette::DARK.comment);
        assert!(
            job.sections
                .iter()
                .any(|section| section.format.color == Palette::DARK.mnemonic),
            "the if keyword must still be coloured"
        );

        // Unterminated literals and comments must not panic or drop text.
        for line in ["return \"unterminated;", "/* open", "char c = 'x;"] {
            let mut job = LayoutJob::default();
            append_pseudo_line(&mut job, line, Palette::DARK, &mono);
            assert_eq!(job.text, line);
        }
    }

    #[test]
    fn theme_toggle_switches_palette() {
        let mut app = ReCraftApp::default();
        assert_eq!(app.theme, ThemeMode::Light, "light is the default theme");
        assert_eq!(app.palette().panel, Palette::LIGHT.panel);
        assert_eq!(
            app.palette().view,
            Color32::WHITE,
            "full light theme uses white code views"
        );
        let ctx = egui::Context::default();
        app.apply_action(&ctx, Action::ToggleTheme);
        assert_eq!(app.theme, ThemeMode::Dark);
        assert_eq!(app.palette().view, Palette::DARK.view);
        app.apply_action(&ctx, Action::SetTheme(ThemeMode::Light));
        assert_eq!(app.theme, ThemeMode::Light);
    }

    #[test]
    fn function_shortcut_focuses_the_functions_pane() {
        let mut app = ReCraftApp::default();
        let ctx = egui::Context::default();
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        input.events.push(egui::Event::Key {
            key: egui::Key::F2,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
        output.textures_delta.clear();
        let active = app
            .dock
            .as_mut()
            .and_then(|state| state.find_active_focused())
            .map(|(_, tab)| *tab);
        assert_eq!(active, Some(Pane::Functions));
    }

    #[test]
    fn strings_shortcut_opens_the_strings_pane_like_ida() {
        assert_eq!(
            Pane::Strings.shortcut(),
            Some("Shift+F12"),
            "the Views menu must advertise IDA's Strings-window chord"
        );
        let mut app = ReCraftApp::default();
        app.ensure_dock();
        let path = app
            .dock
            .as_ref()
            .and_then(|state| state.find_tab(&Pane::Strings))
            .expect("Strings pane");
        app.dock.as_mut().unwrap().remove_tab(path);

        let ctx = egui::Context::default();
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        input
            .events
            .push(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
        input.events.push(egui::Event::Key {
            key: egui::Key::F12,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::SHIFT,
        });
        let mut output = ctx.run_ui(input, |ui| app.update_inner(ui));
        output.textures_delta.clear();

        let state = app.dock.as_mut().expect("dock");
        assert!(
            state.find_tab(&Pane::Strings).is_some(),
            "Shift+F12 must re-dock a closed Strings pane, like IDA's window shortcut"
        );
        assert_eq!(
            state.find_active_focused().map(|(_, tab)| *tab),
            Some(Pane::Strings),
            "Shift+F12 must bring the Strings pane to the front"
        );
    }

    #[test]
    fn command_palette_opens_and_executes_a_command() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ui-palette.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        let ctx = egui::Context::default();
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        let run = |app: &mut ReCraftApp, input: &egui::RawInput| {
            let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            output.textures_delta.clear();
        };
        run(&mut app, &input);
        input
            .events
            .push(egui::Event::ModifiersChanged(egui::Modifiers {
                command: true,
                ..egui::Modifiers::NONE
            }));
        input.events.push(egui::Event::Key {
            key: egui::Key::P,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                command: true,
                ..egui::Modifiers::NONE
            },
        });
        run(&mut app, &input);
        assert!(app.palette.is_some(), "Ctrl+P should open the palette");
        input
            .events
            .push(egui::Event::ModifiersChanged(egui::Modifiers::NONE));

        // Filter to one command and press Enter.
        app.palette = Some(PaletteState {
            query: "rename".to_owned(),
            selected: 0,
        });
        input.events.clear();
        run(&mut app, &input);
        input.events.push(egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        run(&mut app, &input);
        assert!(app.palette.is_none(), "Enter should close the palette");
        assert!(
            app.edit.is_some(),
            "Rename command should open the edit dialog"
        );
    }

    /// Address/y of every disassembly row painted in a frame (`0x...` galleys carry the row).
    fn painted_rows(output: &egui::FullOutput) -> Vec<(u64, egui::Pos2)> {
        let mut rows = Vec::new();
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                let s = text.galley.text();
                if let Some(rest) = s.strip_prefix("0x") {
                    let digits: String =
                        rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
                    if digits.len() >= 8
                        && let Ok(addr) = u64::from_str_radix(&digits, 16)
                    {
                        rows.push((addr, text.pos));
                    }
                }
            }
        }
        rows
    }

    /// Headless app with notepad.exe analysed and shown in text view.
    fn headless_disassembly() -> Option<(ReCraftApp, egui::Context, egui::RawInput)> {
        let bytes = std::fs::read("C:/Windows/System32/notepad.exe").ok()?;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ui-scroll.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        // Analysis gives the session function starts, which is what the paging walk anchors on.
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.view_mode = ViewMode::Text;
        app.focus_pane(Pane::View);
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..egui::RawInput::default()
        };
        Some((app, ctx, input))
    }

    /// Wheel scrolling at the top edge used to stop dead because the view only holds a fixed
    /// window of instructions; it must now load code above the window instead.
    #[test]
    fn headless_disassembly_wheel_reaches_code_above_the_window() {
        let Some((mut app, ctx, mut input)) = headless_disassembly() else {
            return;
        };
        let mut frame = 0.0f64;
        let mut step = |input: &mut egui::RawInput, app: &mut ReCraftApp| -> egui::FullOutput {
            frame += 1.0;
            input.time = Some(frame / 60.0);
            let mut out = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            input.events.clear();
            out.textures_delta.clear();
            out
        };
        let (target, pos) = {
            let output = step(&mut input, &mut app);
            let rows = painted_rows(&output);
            rows.iter()
                .find(|(_, pos)| pos.y > 200.0 && pos.y < 700.0 && pos.x > 100.0)
                .copied()
                .expect("a visible disassembly row")
        };
        let pointer = pos + egui::vec2(200.0, 4.0);
        input.events.push(egui::Event::PointerMoved(pointer));
        let output = step(&mut input, &mut app);
        let y_of = |output: &egui::FullOutput| {
            painted_rows(output)
                .iter()
                .find(|(addr, _)| *addr == target)
                .map(|(_, pos)| pos.y)
        };
        let base_y = y_of(&output).expect("row painted before scrolling");
        let anchor = app.disasm_start;
        let anchor_rows: Vec<Address> = app
            .disasm_rows
            .iter()
            .take(6)
            .map(|line| line.addr)
            .collect();

        input.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, 8.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        let mut output = output;
        for _ in 0..40 {
            output = step(&mut input, &mut app);
        }

        assert!(
            app.disasm_start < anchor,
            "wheel up at the top edge did not load earlier code: start stayed {:#x}",
            app.disasm_start.get()
        );
        // The rows must stay on the real instruction stream: the previous anchor is still
        // decoded at the same place, with no byte drift.
        let index = app
            .disasm_rows
            .iter()
            .position(|line| line.addr == anchor)
            .expect("previous anchor still in the window");
        assert!(
            index >= 1,
            "window did not grow above the previous anchor ({index})"
        );
        let followed: Vec<Address> = app
            .disasm_rows
            .iter()
            .skip(index)
            .take(6)
            .map(|line| line.addr)
            .collect();
        assert_eq!(
            followed, anchor_rows,
            "re-anchored window is not instruction aligned"
        );
        // Seamless: only the wheel itself moved the text, not the page that was loaded.
        let after_y = y_of(&output).expect("row still painted after re-anchoring");
        assert!(
            (after_y - base_y).abs() < 2.0 * app.row_height(),
            "re-anchoring moved the text by {} px",
            after_y - base_y
        );
    }

    /// The column header stays pinned while the listing scrolls. It used to be the first item
    /// *inside* the scroll area, so the wheel carried it off the top with the instructions and
    /// the columns were only visible at offset 0.
    #[test]
    fn headless_disassembly_header_stays_pinned_when_the_listing_scrolls() {
        let Some((mut app, ctx, mut input)) = headless_disassembly() else {
            return;
        };
        let mut frame = 0.0f64;
        let mut step = |input: &mut egui::RawInput, app: &mut ReCraftApp| -> egui::FullOutput {
            frame += 1.0;
            input.time = Some(frame / 60.0);
            let mut out = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            input.events.clear();
            out.textures_delta.clear();
            out
        };
        let address_head = app.t("ui_address");
        let header_pos = |output: &egui::FullOutput| {
            output
                .shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) if text.galley.text().starts_with(&address_head) => {
                        Some(text.pos)
                    }
                    _ => None,
                })
        };
        let pos_of = |output: &egui::FullOutput, addr: u64| {
            painted_rows(output)
                .iter()
                .find(|(painted, _)| *painted == addr)
                .map(|(_, pos)| *pos)
        };
        let output = step(&mut input, &mut app);
        let before = header_pos(&output).expect("the column header is painted");
        let first_row = app.disasm_rows[0].addr.get();
        let row_pos = pos_of(&output, first_row).expect("first row is painted");
        let row_before_y = row_pos.y;
        // Every header column must start on the x of its row glyph. A CJK label is wider than
        // one monospace cell, so painting the header as one `{:<14}` padded string pushed
        // 助记符/操作数 to the right of their columns.
        let row_galley = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text)
                    if text
                        .galley
                        .text()
                        .starts_with(&format!("{first_row:#012x}")) =>
                {
                    Some(text.galley.clone())
                }
                _ => None,
            })
            .expect("first row galley");
        let glyph_x = |cell: usize| row_galley.rows[0].glyphs[cell].pos.x;
        for (cell, label) in [
            (0usize, app.t("ui_address")),
            (14, app.t("ui_bytes")),
            (60, app.t("ui_mnemonic")),
            (69, app.t("ui_operands")),
        ] {
            let x = output
                .shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) if text.galley.text() == label => Some(text.pos.x),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("the {label} column header is not painted"));
            let expected = row_pos.x + glyph_x(cell);
            assert!(
                (x - expected).abs() < 1.0,
                "the {label} header starts at {x} but cell {cell} of the row starts at {expected}"
            );
        }
        // The header line is taller than its 15 px strip (CJK fallback line height), which is why
        // it is painted without a vertical clip; it must still stop before the first row's text.
        let (head_pos, head_height) = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text() == address_head.as_str() => {
                    Some((text.pos, text.galley.size().y))
                }
                _ => None,
            })
            .expect("the address header galley");
        assert!(
            head_pos.y + head_height <= row_pos.y,
            "the header text ({}..{}) runs into the first row at {}",
            head_pos.y,
            head_pos.y + head_height,
            row_pos.y
        );
        // Turn the wheel down over the listing: the rows move up, the header must not.
        input
            .events
            .push(egui::Event::PointerMoved(before + egui::vec2(200.0, 120.0)));
        let _ = step(&mut input, &mut app);
        input.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -40.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        let mut output = output;
        for _ in 0..20 {
            output = step(&mut input, &mut app);
        }
        assert!(
            app.disasm_offset > 0.0,
            "the wheel did not scroll the listing (offset {})",
            app.disasm_offset
        );
        let after = header_pos(&output).expect("the header is painted after scrolling");
        assert!(
            (after.y - before.y).abs() < 1.0,
            "the header moved from {} to {} while the listing scrolled",
            before.y,
            after.y
        );
        let row_after = pos_of(&output, first_row).expect("first row is still painted");
        assert!(
            row_after.y < row_before_y - 10.0,
            "the first row did not scroll up ({row_before_y} -> {})",
            row_after.y
        );
    }

    /// The report's gesture, end to end: double-click the first string, then turn the wheel
    /// up over the disassembly. The window must load the bytes above the literal (as data),
    /// not stop dead at the top of the literal list.
    #[test]
    fn headless_wheel_scrolls_a_string_window_up() {
        let Some((mut app, ctx, mut input)) = headless_disassembly() else {
            return;
        };
        {
            let session = app.session.as_mut().unwrap();
            let strings = recraft_analysis::extract_strings(session, 4, 200_000).unwrap();
            recraft_analysis::persist_strings(session, &strings).unwrap();
        }
        app.refresh_strings();
        let row = app
            .strings
            .iter()
            .find(|row| !is_header_string(&row.segment))
            .cloned()
            .expect("notepad has a visible literal");
        app.apply_action(&ctx, Action::NavigateToDisasm(row.addr));
        let mut frame = 0.0f64;
        let mut step = |input: &mut egui::RawInput, app: &mut ReCraftApp| -> egui::FullOutput {
            frame += 1.0;
            input.time = Some(frame / 60.0);
            let mut out = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            input.events.clear();
            out.textures_delta.clear();
            out
        };
        let output = step(&mut input, &mut app);
        assert!(
            app.disasm_rows
                .first()
                .is_some_and(|line| line.flags.is_data()),
            "the literal opens a data row"
        );
        let (_, pos) = painted_rows(&output)
            .into_iter()
            .find(|(_, pos)| pos.y > 200.0 && pos.y < 700.0 && pos.x > 100.0)
            .expect("a visible data row");
        input
            .events
            .push(egui::Event::PointerMoved(pos + egui::vec2(200.0, 4.0)));
        let output = step(&mut input, &mut app);
        let anchor = app.disasm_start;
        let base_y = painted_rows(&output)
            .iter()
            .find(|(addr, _)| *addr == anchor.get())
            .map(|(_, pos)| pos.y)
            .expect("the literal row is painted");

        input.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, 8.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        let mut output = output;
        for _ in 0..40 {
            output = step(&mut input, &mut app);
        }
        assert!(
            app.disasm_start < anchor,
            "the wheel did not load the bytes above the first literal: start stayed {:#x}",
            app.disasm_start.get()
        );
        assert!(
            app.disasm_rows.iter().all(|line| line.flags.is_data()),
            "the wheel decoded data as code"
        );
        assert!(
            app.disasm_rows.iter().any(|line| line.addr == anchor),
            "the literal must stay in the window"
        );
        // The text on screen does not jump: only the wheel moved it.
        let after_y = painted_rows(&output)
            .iter()
            .find(|(addr, _)| *addr == anchor.get())
            .map(|(_, pos)| pos.y)
            .expect("the literal row is still painted");
        assert!(
            (after_y - base_y).abs() < 2.0 * app.row_height(),
            "re-anchoring moved the text by {} px",
            after_y - base_y
        );
    }

    /// PageUp must move the window by whole instructions, not by a byte guess made from the
    /// first row's length (which used to land mid-instruction and show garbage).
    #[test]
    fn headless_page_up_moves_by_instructions() {
        let Some((mut app, ctx, mut input)) = headless_disassembly() else {
            return;
        };
        let mut frame = 0.0f64;
        let mut step = |input: &mut egui::RawInput, app: &mut ReCraftApp| -> egui::FullOutput {
            frame += 1.0;
            input.time = Some(frame / 60.0);
            let mut out = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            input.events.clear();
            out.textures_delta.clear();
            out
        };
        step(&mut input, &mut app);
        let anchor = app.disasm_start;
        let old_rows: Vec<Address> = app.disasm_rows.iter().map(|line| line.addr).collect();
        assert!(old_rows.len() > 100);

        // Reference stream decoded from the first executable byte of the image (a boundary the
        // loader guarantees), independent of the paging code under test.
        let segment_start = app
            .session
            .as_ref()
            .unwrap()
            .segments()
            .iter()
            .filter(|segment| segment.perm.is_executable())
            .map(|segment| segment.range.start)
            .min()
            .unwrap();
        let reference: Vec<Address> = app
            .session
            .as_mut()
            .unwrap()
            .disasm(segment_start, 2000)
            .unwrap()
            .iter()
            .map(|line| line.addr)
            .collect();
        let position = reference
            .iter()
            .position(|addr| *addr == anchor)
            .expect("entry in the reference stream");
        assert!(
            position >= 20,
            "reference stream too short before the entry"
        );

        input.events.push(egui::Event::Key {
            key: egui::Key::PageUp,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        step(&mut input, &mut app);
        step(&mut input, &mut app);

        assert!(
            app.disasm_start < anchor,
            "PageUp did not move the window up"
        );
        assert_eq!(
            app.disasm_start,
            reference[position - 20],
            "PageUp must land 20 real instructions back, not 20 byte-guesses"
        );
        assert_eq!(
            app.disasm_rows[20].addr, anchor,
            "PageUp did not move by 20 instructions"
        );
        assert_eq!(app.disasm_rows[21].addr, old_rows[1]);
    }

    /// Runs the background worker function directly (no UI thread): it must persist
    /// functions/strings through a second SQLite connection and honour cancellation.
    #[test]
    fn background_analysis_job_persists_and_cancels() {
        let Ok(bytes) = std::fs::read("C:/Windows/System32/notepad.exe") else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("bg.rcraft");
        {
            let _session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        }

        let cancel = Arc::new(AtomicBool::new(false));
        match run_analysis_job(&project, &cancel) {
            AnalysisMessage::Done {
                functions, strings, ..
            } => {
                assert!(functions > 100, "only {functions} functions");
                assert!(strings > 100, "only {strings} strings");
            }
            AnalysisMessage::Cancelled => panic!("job was cancelled unexpectedly"),
            AnalysisMessage::Failed(error) => panic!("job failed: {error}"),
        }

        // A fresh connection sees the persisted derived data.
        let session = Session::open(&project).unwrap();
        assert!(session.functions().unwrap().len() > 100);
        assert!(session.strings().unwrap().len() > 100);

        let cancel = Arc::new(AtomicBool::new(true));
        assert!(matches!(
            run_analysis_job(&project, &cancel),
            AnalysisMessage::Cancelled
        ));
    }

    /// A minimal instruction row for the selection tests (no session needed).
    fn synthetic_line(addr: u64, mnemonic: &str, operands: &str) -> DisasmLine {
        DisasmLine {
            addr: Address::new(addr),
            len: 2,
            raw: [0x90; 15],
            mnemonic: mnemonic.to_owned(),
            operands: operands.to_owned(),
            name: None,
            comment: None,
            flags: Flags::code(),
        }
    }

    fn synthetic_disassembly() -> ReCraftApp {
        ReCraftApp {
            disasm_start: Address::new(0x140001000),
            disasm_rows: vec![
                synthetic_line(0x140001000, "mov", "rax, rbx"),
                synthetic_line(0x140001003, "test", "rax, rax"),
                synthetic_line(0x140001006, "ret", ""),
            ],
            ..ReCraftApp::default()
        }
    }

    const TEST_SCREEN: egui::Vec2 = egui::vec2(1200.0, 700.0);

    fn test_input() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, TEST_SCREEN)),
            ..egui::RawInput::default()
        }
    }

    fn press(pos: egui::Pos2) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn release(pos: egui::Pos2) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// Text the last `copy_text` produced, read back from the context's output commands.
    fn copied_text(ctx: &egui::Context) -> Option<String> {
        ctx.output(|output| {
            output.commands.iter().find_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
        })
    }

    #[test]
    fn a_text_selection_copies_exactly_the_dragged_characters() {
        let rows = vec!["alpha  ; note".to_owned(), "beta".to_owned()];
        let selection = TextSelection {
            anchor: TextPoint { row: 0, char: 3 },
            focus: TextPoint { row: 1, char: 2 },
        };
        assert_eq!(
            selected_text(selection, &rows).as_deref(),
            Some("ha  ; note\nbe"),
            "one line per row, interior padding kept, trailing padding trimmed"
        );
        let reversed = TextSelection {
            anchor: selection.focus,
            focus: selection.anchor,
        };
        assert_eq!(
            selected_text(reversed, &rows),
            selected_text(selection, &rows)
        );

        // A caret selects nothing; the view then copies its current row instead.
        let caret = TextSelection::collapsed(TextPoint { row: 1, char: 4 });
        assert!(caret.is_empty());
        assert_eq!(selected_text(caret, &rows).as_deref(), Some(""));
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn ctrl_c_and_ctrl_a_reach_the_focused_pane() {
        let ctx = egui::Context::default();
        let mut app = ReCraftApp::default();
        let run = |app: &mut ReCraftApp, events: Vec<egui::Event>| -> Action {
            let mut action = Action::None;
            let input = egui::RawInput {
                events,
                ..egui::RawInput::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                let _ = ui;
                action = app.handle_keys(&ctx);
            });
            output.textures_delta.clear();
            action
        };
        assert_eq!(run(&mut app, vec![egui::Event::Copy]), Action::Copy);
        assert_eq!(
            run(
                &mut app,
                vec![egui::Event::Key {
                    key: Key::A,
                    physical_key: Some(Key::A),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                }]
            ),
            Action::SelectAll
        );
        // A plain `A` still marks the bytes as a string in the listing.
        app.tab = Tab::Disasm;
        assert_eq!(
            run(
                &mut app,
                vec![egui::Event::Key {
                    key: Key::A,
                    physical_key: Some(Key::A),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }]
            ),
            Action::ApplyData(DataOp::Ascii)
        );
    }

    #[test]
    fn a_text_selection_follows_the_rows_when_the_window_reloads() {
        let rows: Vec<DisasmLine> = (0..5)
            .map(|index| synthetic_line(0x140001000 + index * 3, "nop", ""))
            .collect();
        // A window holding the last three rows, selected from its first to its last line.
        let window = rows[2..].to_vec();
        let selection = TextSelection {
            anchor: TextPoint { row: 0, char: 4 },
            focus: TextPoint { row: 2, char: 7 },
        };
        // Paging up loads the two rows above: the same addresses are now at 2 and 4.
        let remapped = remap_selection(selection, &window, &rows).expect("both ends are visible");
        assert_eq!(remapped.anchor, TextPoint { row: 2, char: 4 });
        assert_eq!(remapped.focus, TextPoint { row: 4, char: 7 });

        // An end that scrolled out of the window drops the whole selection.
        let scrolled_away = rows[..2].to_vec();
        assert_eq!(remap_selection(selection, &window, &scrolled_away), None);
    }

    /// Dragging across the listing selects the characters under the pointer, and Ctrl+C
    /// copies exactly that text (the report: "selection and copy need work").
    #[test]
    fn dragging_across_the_listing_selects_and_copies_text() {
        let mut app = synthetic_disassembly();
        let ctx = egui::Context::default();
        let mut action = Action::None;
        let mut output = ctx.run_ui(test_input(), |ui| app.disasm_view(ui, &mut action));
        output.textures_delta.clear();
        let first = app.disasm_line_text(&app.disasm_rows[0]);
        let last = app.disasm_line_text(&app.disasm_rows[2]);
        // Start just before the "mov" mnemonic (column 60: address 14 + bytes 46) and end
        // after the "ret" of the third row.
        let start = output
            .shapes
            .iter()
            .find_map(|clipped| char_click_pos(&clipped.shape, &first, 59))
            .expect("the first row is on screen");
        let end = output
            .shapes
            .iter()
            .find_map(|clipped| char_click_pos(&clipped.shape, &last, 63))
            .expect("the last row is on screen");

        let mut frame = 0.0f64;
        let mut step = |events: Vec<egui::Event>, app: &mut ReCraftApp| {
            frame += 1.0;
            let input = egui::RawInput {
                time: Some(frame / 60.0),
                events,
                ..test_input()
            };
            let mut action = Action::None;
            let mut output = ctx.run_ui(input, |ui| app.disasm_view(ui, &mut action));
            output.textures_delta.clear();
            output
        };
        step(vec![egui::Event::PointerMoved(start)], &mut app);
        step(vec![press(start)], &mut app);
        step(vec![egui::Event::PointerMoved(end)], &mut app);
        let painted_output = step(vec![release(end)], &mut app);

        let selection = app.disasm_sel.expect("a drag must build a text selection");
        let (from, to) = selection.ordered();
        assert_eq!((from.row, to.row), (0, 2), "the selection spans both rows");
        assert!(
            painted_output.shapes.iter().any(|clipped| {
                matches!(
                    &clipped.shape,
                    egui::Shape::Rect(rect) if rect.fill == app.palette().text_selection
                )
            }),
            "the dragged selection must be painted"
        );

        app.apply_action(&ctx, Action::Copy);
        let copied = copied_text(&ctx).expect("Ctrl+C must fill the clipboard");
        let rows: Vec<String> = app
            .disasm_rows
            .iter()
            .map(|line| app.disasm_line_text(line))
            .collect();
        assert_eq!(
            copied,
            selected_text(selection, &rows).expect("the selection has text"),
            "the clipboard must hold exactly the dragged characters"
        );
        assert!(
            copied.contains("mov") && copied.contains("test") && copied.ends_with("ret"),
            "the copy must cover every row the drag crossed, got {copied:?}"
        );
    }

    #[test]
    fn dragging_in_the_decompile_view_selects_code() {
        let start = Address::new(0x140001000);
        let mut app = ReCraftApp {
            pseudo_start: start,
            pseudo: Some(vec![
                PseudoLine {
                    addr: start,
                    text: "int main()".to_owned(),
                },
                PseudoLine {
                    addr: start,
                    text: "{".to_owned(),
                },
                PseudoLine {
                    addr: start,
                    text: "return 0;".to_owned(),
                },
                PseudoLine {
                    addr: start,
                    text: "}".to_owned(),
                },
            ]),
            ..ReCraftApp::default()
        };
        let ctx = egui::Context::default();
        let mut action = Action::None;
        let mut output = ctx.run_ui(test_input(), |ui| app.pseudo_view(ui, &mut action));
        output.textures_delta.clear();
        let anchor = output
            .shapes
            .iter()
            .find_map(|clipped| char_click_pos(&clipped.shape, "int main()", 4))
            .expect("the first pseudo line is on screen");
        let end = output
            .shapes
            .iter()
            .find_map(|clipped| char_click_pos(&clipped.shape, "return 0;", 8))
            .expect("the last pseudo line is on screen");

        let mut frame = 0.0f64;
        let mut step = |events: Vec<egui::Event>, app: &mut ReCraftApp| {
            frame += 1.0;
            let input = egui::RawInput {
                time: Some(frame / 60.0),
                events,
                ..test_input()
            };
            let mut action = Action::None;
            let mut output = ctx.run_ui(input, |ui| app.pseudo_view(ui, &mut action));
            output.textures_delta.clear();
            output
        };
        step(vec![egui::Event::PointerMoved(anchor)], &mut app);
        step(vec![press(anchor)], &mut app);
        step(vec![egui::Event::PointerMoved(end)], &mut app);
        step(vec![release(end)], &mut app);

        let selection = app.pseudo_sel.expect("a drag must build a text selection");
        assert_eq!(selection.ordered().0.row, 0);
        assert_eq!(selection.ordered().1.row, 2);
        app.apply_action(&ctx, Action::Copy);
        assert_eq!(
            copied_text(&ctx).as_deref(),
            Some("main()\n{\nreturn 0;"),
            "the copy must be the dragged pseudo-C, without line numbers"
        );
    }

    /// Hex bytes are picked as a range and copied as a dump, not one byte at a time.
    #[test]
    fn dragging_in_the_hex_view_copies_a_byte_range() {
        let Some((mut app, ctx, _input)) = headless_disassembly() else {
            return;
        };
        let base = app.session.as_ref().unwrap().image().image_base;
        app.hex_start = Address::new(base.get());
        app.hex_cursor = Some(app.hex_start);
        app.hex_synced = Some(app.hex_start);
        let mut action = Action::None;
        let mut output = ctx.run_ui(test_input(), |ui| app.hex_view(ui, &mut action));
        output.textures_delta.clear();
        // The MZ header: the first two byte cells are "4D" and "5A".
        let first = output
            .shapes
            .iter()
            .find_map(|clipped| char_click_pos(&clipped.shape, "4D", 0))
            .map(|pos| pos + egui::vec2(2.0, 2.0))
            .expect("the first byte cell is on screen");
        let second = output
            .shapes
            .iter()
            .find_map(|clipped| char_click_pos(&clipped.shape, "5A", 0))
            .map(|pos| pos + egui::vec2(2.0, 2.0))
            .expect("the second byte cell is on screen");
        let pitch = second.x - first.x;
        let end = egui::pos2(first.x + pitch * 3.0, first.y);

        let mut frame = 0.0f64;
        let mut step = |events: Vec<egui::Event>, app: &mut ReCraftApp| {
            frame += 1.0;
            let input = egui::RawInput {
                time: Some(frame / 60.0),
                events,
                ..test_input()
            };
            let mut action = Action::None;
            let mut output = ctx.run_ui(input, |ui| app.hex_view(ui, &mut action));
            output.textures_delta.clear();
            output
        };
        step(vec![egui::Event::PointerMoved(first)], &mut app);
        step(vec![press(first)], &mut app);
        step(vec![egui::Event::PointerMoved(end)], &mut app);
        step(vec![release(end)], &mut app);

        let selection = app.hex_sel.expect("a drag must pick a byte range");
        assert_eq!(
            selection.len(),
            4,
            "four bytes from the anchor to the pointer"
        );
        app.apply_action(&ctx, Action::Copy);
        assert_eq!(
            copied_text(&ctx).as_deref(),
            Some("4D 5A 90 00"),
            "the clipboard must hold the selected bytes"
        );
    }

    #[test]
    fn a_pseudo_selection_becomes_agent_context() {
        let lines = vec![
            PseudoLine {
                addr: Address::new(0x1000),
                text: "int main(void)".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1004),
                text: "{".to_owned(),
            },
            PseudoLine {
                addr: Address::new(0x1008),
                text: "    return 0;".to_owned(),
            },
        ];
        let selection = TextSelection {
            anchor: TextPoint { row: 0, char: 4 },
            focus: TextPoint { row: 2, char: 12 },
        };
        assert_eq!(
            selection_text(&lines, selection).as_deref(),
            Some("main(void)\n{\n    return 0")
        );
        // A caret selects nothing, so it cannot become context.
        let caret = TextSelection::collapsed(TextPoint { row: 1, char: 0 });
        assert!(selection_text(&lines, caret).is_none());
    }

    #[test]
    fn the_about_icon_texture_is_created_once() {
        let ctx = egui::Context::default();
        let mut app = ReCraftApp::default();
        assert!(!app.about_open, "the About window starts closed");
        let first = app.icon_texture(&ctx).id();
        let second = app.icon_texture(&ctx).id();
        assert_eq!(first, second, "the texture is uploaded once and reused");
    }

    #[test]
    fn stack_slots_are_found_in_pseudo_lines() {
        assert_eq!(stack_slot_token("    var_20 = 0;"), Some((0x20, "var_20")));
        assert_eq!(
            stack_slot_token("    *(u8*)(&var_n8 + i) = 0;"),
            Some((-8, "var_n8"))
        );
        assert_eq!(
            stack_slot_token("    var_20.field = 1;"),
            Some((0x20, "var_20"))
        );
        assert_eq!(stack_slot_token("    return 0;"), None);
        assert_eq!(stack_slot_token("    var_zz = 1;"), None);
    }

    #[test]
    fn list_rows_copy_their_visible_columns() {
        let mut app = ReCraftApp {
            strings: vec![StringRow {
                addr: Address::new(0x140010000),
                len: 12,
                enc: 0,
                kind: 0,
                segment: ".rdata".to_owned(),
                text: "hello".to_owned(),
                raw: vec![b'h', b'e', b'l', b'l', b'o'],
                name: None,
                overridden: false,
            }],
            names: vec![(Address::new(0x140001000), "main".to_owned())],
            imports: vec![ImportRow {
                module: "KERNEL32.dll".to_owned(),
                api: "CreateFileW".to_owned(),
                category: "file",
                iat: Address::new(0x140020000),
                refs: 2,
                first_from: None,
            }],
            ..ReCraftApp::default()
        };
        assert_eq!(
            app.list_row_text(Pane::Strings, 0x140010000).as_deref(),
            Some(".rdata:0000000140010000\t0000000c\tC\thello")
        );
        assert_eq!(
            app.list_row_text(Pane::Names, 0x140001000).as_deref(),
            Some("0x0140001000\tmain")
        );
        assert_eq!(
            app.list_row_text(Pane::Imports, 0x140020000).as_deref(),
            Some("KERNEL32.dll!CreateFileW\tfile\t×2")
        );

        // Ctrl+C over the list emits the picked row.
        app.list_sel = Some((Pane::Names, 0x140001000));
        app.copy_focus = CopyFocus::List(Pane::Names);
        let ctx = egui::Context::default();
        app.copy_focused(&ctx);
        assert_eq!(copied_text(&ctx).as_deref(), Some("0x0140001000\tmain"));

        // Nothing picked: no stale row from another list is copied.
        app.copy_focus = CopyFocus::List(Pane::Strings);
        app.copy_focused(&ctx);
        assert_eq!(
            copied_text(&ctx).as_deref(),
            Some("0x0140001000\tmain"),
            "the older clipboard text must survive an empty copy"
        );
        assert_eq!(app.status, app.t("ui_nothing_to_copy"));
    }

    /// End to end through the dock: drag over two rows in the real Code View, then press
    /// Ctrl+C, and the selection must land on the clipboard.
    #[test]
    fn headless_ctrl_c_copies_the_dragged_disassembly_text() {
        let Some((mut app, ctx, mut input)) = headless_disassembly() else {
            return;
        };
        let mut frame = 0.0f64;
        let mut step = |input: &mut egui::RawInput, app: &mut ReCraftApp| -> egui::FullOutput {
            frame += 1.0;
            input.time = Some(frame / 60.0);
            let mut out = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            input.events.clear();
            out.textures_delta.clear();
            out
        };
        let output = step(&mut input, &mut app);
        let visible: Vec<(u64, egui::Pos2)> = painted_rows(&output)
            .into_iter()
            .filter(|(_, pos)| pos.y > 200.0 && pos.y < 700.0 && pos.x > 100.0)
            .collect();
        assert!(visible.len() > 3, "expected several visible rows");
        let start = visible[0].1 + egui::vec2(200.0, 4.0);
        let end = visible[2].1 + egui::vec2(200.0, 4.0);

        input.events.push(egui::Event::PointerMoved(start));
        step(&mut input, &mut app);
        input.events.push(press(start));
        step(&mut input, &mut app);
        input.events.push(egui::Event::PointerMoved(end));
        step(&mut input, &mut app);
        input.events.push(release(end));
        step(&mut input, &mut app);

        let selection = app
            .disasm_sel
            .expect("the drag must select disassembly text");
        let (from, to) = selection.ordered();
        assert_eq!(to.row - from.row, 2, "three rows are selected");

        input.events.push(egui::Event::Copy);
        let output = step(&mut input, &mut app);
        let copied = output
            .platform_output
            .commands
            .iter()
            .find_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
            .expect("Ctrl+C must fill the clipboard");
        assert_eq!(
            copied.lines().count(),
            3,
            "one line per selected row, got {copied:?}"
        );
        assert!(
            copied.contains("0x"),
            "the copied rows are the displayed text, got {copied:?}"
        );
    }

    /// A text selection inside a plain label (the AI panel and the status/hint lines) keeps
    /// Ctrl+C: the code views and the Output log must not hijack it with their own, older
    /// selection.
    #[test]
    fn ctrl_c_leaves_a_selected_label_to_egui() {
        let ctx = egui::Context::default();
        let mut app = ReCraftApp::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(480.0, 240.0));
        let mut frame = 0.0f64;
        let mut step = |events: Vec<egui::Event>| {
            frame += 1.0;
            let input = egui::RawInput {
                screen_rect: Some(screen),
                time: Some(frame / 60.0),
                events,
                ..egui::RawInput::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                ui.label("hello world");
            });
            output.textures_delta.clear();
            output
        };
        let output = step(vec![]);
        let label = output
            .shapes
            .iter()
            .find_map(|clipped| char_click_pos(&clipped.shape, "hello world", 1))
            .expect("the label is on screen");
        step(vec![egui::Event::PointerMoved(label)]);
        step(vec![press(label)]);
        step(vec![egui::Event::PointerMoved(
            label + egui::vec2(60.0, 0.0),
        )]);
        step(vec![release(label + egui::vec2(60.0, 0.0))]);

        let mut action = Action::Copy;
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events: vec![egui::Event::Copy],
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            action = app.handle_keys(&ctx);
            ui.label("hello world");
        });
        output.textures_delta.clear();
        assert_eq!(
            action,
            Action::None,
            "a selected label must keep the clipboard"
        );
        let copied = output
            .platform_output
            .commands
            .iter()
            .find_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
            .expect("egui's label selection must copy its text");
        assert!(copied.contains("hello"), "got {copied:?}");
    }
    #[test]
    fn answer_addresses_are_extracted_and_deduplicated() {
        let links = address_links(
            "入口 0x1400019c0 调用 0x140001840，再到 0x1400019c0（重复）和 0x20（太短）",
        );
        let values: Vec<u64> = links.iter().map(|addr| addr.get()).collect();
        assert_eq!(values, vec![0x1400019c0, 0x140001840]);
        assert!(address_links("没有地址").is_empty());
    }
    /// "Dig deeper" hands the function under the cursor to the agent by name and address.
    #[test]
    fn dig_question_names_the_function_under_the_cursor() {
        let Some(bytes) = sample_pe() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("dig.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let mut app = ReCraftApp {
            session: Some(session),
            ..ReCraftApp::default()
        };
        app.after_open();
        app.refresh_functions();
        app.refresh_disasm();
        if app.disasm_rows.is_empty() {
            return;
        }
        app.selected = 0;
        let question = app.agent_dig_question().expect("question");
        let entry = app.disasm_rows[0].addr;
        assert!(question.contains("0x"), "{question}");
        assert!(
            question.contains(&format!("{:#x}", entry.get())),
            "{question}"
        );
        assert!(question.contains("start"), "{question}");
    }
}
