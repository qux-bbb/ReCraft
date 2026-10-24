// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! ReCraft desktop application (M0 shell).
//!
//! Deliberately small: one session, three panes (navigation, disassembly/pseudo-C,
//! status), keyboard-first commands. All state changes go through `recraft-core`
//! commands, so undo/redo behave exactly like the CLI.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Key, RichText, Sense};

use recraft_common::{Address, AddressRange};
use recraft_core::{
    CodeData, Command, CommentKind, DisasmLine, FunctionInfo, Session, StructDef, StructField,
};
use recraft_decompile::{PseudoLine, pseudocode_l0};

use crate::i18n::{Lang, tr};

const DISASM_ROWS: usize = 400;
const MONO_SIZE: f32 = 12.5;
const ROW_HEIGHT: f32 = 15.0;

/// Which colour scheme the workspace uses. Persisted with the dock layout.
/// `Light` is IDA's mixed look (light chrome around dark code views) and is the default.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum ThemeMode {
    Dark,
    #[default]
    Light,
}

/// Semantic palette shared by the chrome, code views and graph view.
#[derive(Clone, Copy)]
struct Palette {
    panel: Color32,
    view: Color32,
    text: Color32,
    /// Text drawn on the light/dark *chrome* (menus, toolbars, lists, status bar).
    chrome_text: Color32,
    /// Background for editable fields inside code views.
    field: Color32,
    /// Background for editable fields on the chrome.
    chrome_field: Color32,
    addr: Color32,
    bytes: Color32,
    mnemonic: Color32,
    name: Color32,
    comment: Color32,
    selected_row: Color32,
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
}

impl Palette {
    const DARK: Self = Self {
        panel: Color32::from_rgb(0x1e, 0x1e, 0x1e),
        view: Color32::from_rgb(0x14, 0x14, 0x14),
        text: Color32::from_rgb(0xd4, 0xd4, 0xd4),
        chrome_text: Color32::from_rgb(0xd4, 0xd4, 0xd4),
        field: Color32::from_rgb(0x14, 0x14, 0x14),
        chrome_field: Color32::from_rgb(0x14, 0x14, 0x14),
        addr: Color32::from_rgb(0xc9, 0xb4, 0x58),
        bytes: Color32::from_rgb(0x85, 0x85, 0x85),
        mnemonic: Color32::from_rgb(0x7e, 0x9e, 0xd0),
        name: Color32::from_rgb(0xd7, 0xd7, 0x87),
        comment: Color32::from_rgb(0x6a, 0x99, 0x55),
        selected_row: Color32::from_rgb(0x26, 0x4f, 0x78),
        hover_row: Color32::from_rgb(0x2a, 0x30, 0x38),
        tab_active_bg: Color32::from_rgb(0x2a, 0x2a, 0x2a),
        tab_focused_bg: Color32::from_rgb(0x33, 0x33, 0x33),
        tab_inactive_text: Color32::from_rgb(0x90, 0x98, 0xa6),
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
    };

    /// IDA's mixed scheme: light chrome (menu/toolbar/tabs/lists) around dark code
    /// views, like the reference screenshot.
    const LIGHT: Self = Self {
        panel: Color32::from_rgb(0xf0, 0xf0, 0xf0),
        view: Color32::from_rgb(0x1e, 0x1e, 0x1e),
        text: Color32::from_rgb(0xd4, 0xd4, 0xd4),
        chrome_text: Color32::from_rgb(0x1e, 0x1e, 0x1e),
        field: Color32::from_rgb(0x25, 0x25, 0x25),
        chrome_field: Color32::from_rgb(0xff, 0xff, 0xff),
        addr: Color32::from_rgb(0xc9, 0xb4, 0x58),
        bytes: Color32::from_rgb(0x9a, 0x9a, 0x9a),
        mnemonic: Color32::from_rgb(0x7e, 0x9e, 0xd0),
        name: Color32::from_rgb(0xd7, 0xd7, 0x87),
        comment: Color32::from_rgb(0x6a, 0x99, 0x55),
        selected_row: Color32::from_rgb(0x26, 0x4f, 0x78),
        hover_row: Color32::from_rgb(0x2a, 0x30, 0x38),
        tab_active_bg: Color32::from_rgb(0xff, 0xff, 0xff),
        tab_focused_bg: Color32::from_rgb(0xe4, 0xec, 0xf4),
        tab_inactive_text: Color32::from_rgb(0x50, 0x58, 0x60),
        grid_line: Color32::from_rgb(0xc0, 0xc6, 0xcc),
        graph_node: Color32::from_rgb(0x0d, 0x10, 0x12),
        graph_border: Color32::from_rgb(0x4a, 0x8f, 0x8f),
        graph_selected_border: Color32::from_rgb(0xc9, 0xb4, 0x58),
        graph_header: Color32::from_rgb(0x9c, 0xc8, 0xc8),
        graph_header_text: Color32::from_rgb(0x14, 0x2a, 0x2a),
        edge_jump: Color32::from_rgb(0x5a, 0x9a, 0x9a),
        edge_taken: Color32::from_rgb(0x6a, 0xb0, 0x6a),
        edge_not_taken: Color32::from_rgb(0xc0, 0x6a, 0x6a),
        edge_jump_table: Color32::from_rgb(0xa0, 0x8a, 0xc0),
    };
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Disasm,
    Pseudo,
    Graph,
}

/// Docked panes (IDA-style dockable tabs, backed by `egui_dock`).
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
enum Pane {
    Disasm,
    Pseudo,
    Graph,
    Hex,
    Functions,
    Strings,
    Imports,
    Exports,
    Xrefs,
    Names,
    Segments,
    Structs,
    Ai,
    GraphOverview,
    Output,
}

impl Pane {
    fn all() -> [Pane; 15] {
        [
            Pane::Disasm,
            Pane::Graph,
            Pane::Hex,
            Pane::Pseudo,
            Pane::Imports,
            Pane::Exports,
            Pane::Strings,
            Pane::Names,
            Pane::Xrefs,
            Pane::Segments,
            Pane::Structs,
            Pane::Ai,
            Pane::Functions,
            Pane::GraphOverview,
            Pane::Output,
        ]
    }

    fn title(self) -> (&'static str, &'static str) {
        match self {
            // IDA-style document/panel names from the reference screenshot.
            // Like IDA, the primary "IDA View-A" starts in graph mode; text is a tab.
            Pane::Graph => ("IDA View-A", "IDA View-A"),
            Pane::Disasm => ("Text View", "文本视图"),
            Pane::Pseudo => ("Pseudocode", "伪代码"),
            Pane::Hex => ("Hex View-1", "十六进制"),
            Pane::Functions => ("Functions", "函数"),
            Pane::Strings => ("Strings", "字符串"),
            Pane::Imports => ("Imports", "导入"),
            Pane::Exports => ("Exports", "导出"),
            Pane::Xrefs => ("Xrefs", "交叉引用"),
            Pane::Names => ("Names", "名字"),
            Pane::Segments => ("Segments", "段"),
            Pane::Structs => ("Local Types", "本地类型"),
            Pane::Ai => ("AI", "AI"),
            Pane::GraphOverview => ("Graph overview", "图形概览"),
            Pane::Output => ("Output", "输出"),
        }
    }

    /// Keyboard shortcut that focuses this pane (shown in the Views menu tooltip).
    fn shortcut(self) -> Option<&'static str> {
        match self {
            Pane::Functions => Some("F2"),
            Pane::Names => Some("F3"),
            Pane::Strings => Some("F4"),
            Pane::Imports => Some("F6"),
            Pane::Xrefs => Some("F7"),
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
        let (en, zh) = tab.title();
        self.app.t(en, zh).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Pane) {
        // Mixed IDA theme: code views are dark with light text, chrome-like panes
        // (Functions) are light with dark text.
        let pal = self.app.palette();
        let is_chrome = matches!(tab, Pane::Functions);
        if is_chrome {
            let visuals = ui.visuals_mut();
            visuals.override_text_color = Some(pal.chrome_text);
            visuals.widgets.noninteractive.fg_stroke.color = pal.chrome_text;
            visuals.widgets.inactive.fg_stroke.color = pal.chrome_text;
            visuals.extreme_bg_color = pal.chrome_field;
            ui.painter().rect_filled(ui.clip_rect(), 0.0, pal.panel);
        } else {
            // Code views stay dark in the mixed theme, so give their widgets dark
            // backgrounds (otherwise light-theme buttons render as blank boxes).
            let mut visuals = egui::Visuals::dark();
            visuals.override_text_color = Some(pal.text);
            visuals.panel_fill = pal.view;
            visuals.window_fill = pal.view;
            visuals.extreme_bg_color = pal.field;
            visuals.faint_bg_color = pal.hover_row;
            visuals.selection.bg_fill = pal.selected_row;
            visuals.widgets.noninteractive.fg_stroke.color = pal.text;
            visuals.widgets.inactive.fg_stroke.color = pal.text;
            visuals.widgets.hovered.fg_stroke.color = pal.text;
            visuals.widgets.active.fg_stroke.color = pal.text;
            *ui.visuals_mut() = visuals;
        }
        match tab {
            Pane::Disasm => {
                if self.app.session.is_none() {
                    self.app.welcome(ui, self.action);
                } else {
                    self.app.disasm_view(ui, self.action);
                }
            }
            Pane::Pseudo => self.app.pseudo_view(ui, self.action),
            Pane::Graph => self.app.graph_view(ui, self.action),
            Pane::Hex => self.app.hex_view(ui, self.action),
            Pane::Functions => self.app.functions_list(ui, self.action),
            Pane::Strings => self.app.strings_list(ui, self.action),
            Pane::Imports => self.app.imports_list(ui, self.action),
            Pane::Xrefs => self.app.xrefs_list(ui, self.action),
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
            Pane::Disasm | Pane::Graph | Pane::Functions | Pane::Output | Pane::GraphOverview
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

/// Background LLM request; HTTP calls never run on the UI thread.
struct AiJob {
    receiver: mpsc::Receiver<std::result::Result<String, String>>,
    started: Instant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditKind {
    Rename,
    Comment,
    Prototype,
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
const LAYOUT_VERSION: u32 = 5;

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedLayout {
    version: u32,
    dock: egui_dock::DockState<Pane>,
    #[serde(default)]
    theme: ThemeMode,
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

fn save_layout_to(
    path: &Path,
    state: &egui_dock::DockState<Pane>,
    theme: ThemeMode,
) -> std::result::Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let saved = SavedLayout {
        version: LAYOUT_VERSION,
        dock: state.clone(),
        theme,
    };
    let text = serde_json::to_string(&saved).map_err(|error| error.to_string())?;
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

/// Selected rename suggestion in the AI panel (index into `ai_proposals`).
type AiSelection = Option<usize>;

/// Fuzzy command palette state (DESIGN §8.1: menu / command palette).
#[derive(Debug, Clone, Default)]
struct PaletteState {
    query: String,
    selected: usize,
}

/// Commands offered by the palette, in display order.
const PALETTE_COMMANDS: &[(&str, &str, Action)] = &[
    ("Open project…", "打开项目…", Action::Open),
    ("Import PE file…", "导入 PE 文件…", Action::Import),
    ("Analyze (F5)", "分析 (F5)", Action::Analyze),
    ("Undo (Ctrl+Z)", "撤销 (Ctrl+Z)", Action::Undo),
    ("Redo (Ctrl+Y)", "重做 (Ctrl+Y)", Action::Redo),
    ("Switch view (Space)", "切换视图 (Space)", Action::ToggleTab),
    ("Go to address (G)", "跳转到地址 (G)", Action::GotoDialog),
    ("Rename (N)", "改名 (N)", Action::EditName),
    ("Comment (;)", "注释 (;)", Action::EditComment),
    ("Cross references (X)", "交叉引用 (X)", Action::ShowXrefs),
    ("AI name suggestions", "AI 命名建议", Action::AiSuggest),
    (
        "Toggle language (EN/中文)",
        "切换语言 (EN/中文)",
        Action::ToggleLang,
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    None,
    Open,
    Import,
    Analyze,
    CancelAnalysis,
    Undo,
    Redo,
    ToggleLang,
    ToggleTab,
    GotoDialog,
    EditName,
    EditComment,
    ShowXrefs,
    MoveSelection(i32),
    Page(i32),
    Back,
    Navigate(Address),
    AiSuggest,
    AiApply,
    AiClear,
    TogglePalette,
    PaletteCommand(usize),
    Follow,
    OpenRecent(usize),
    AiExplain,
    AiPolish,
    AiQuery,
    NavigateToDisasm(Address),
    SyncDisasm(Address),
    Forward,
    ApplyData(DataOp),
    EditPrototype,
    ShowStructs,
    FocusPane(Pane),
    ResetLayout,
    SetTheme(ThemeMode),
    ToggleTheme,
}

pub struct ReCraftApp {
    session: Option<Session>,
    project: Option<PathBuf>,
    lang: Lang,
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
    status: String,
    is_error: bool,
    disasm_start: Address,
    disasm_rows: Vec<DisasmLine>,
    selected: usize,
    disasm_dirty: bool,
    pseudo_start: Address,
    pseudo: Option<Vec<PseudoLine>>,
    pseudo_dirty: bool,
    pseudo_l1: bool,
    pseudo_l2: bool,
    functions: Vec<FunctionInfo>,
    /// Names parallel to `functions` (for the side list), refreshed with it.
    function_labels: Vec<String>,
    functions_dirty: bool,
    function_filter: String,
    strings: Vec<(Address, String)>,
    strings_dirty: bool,
    string_filter: String,
    names: Vec<(Address, String)>,
    names_filter: String,
    imports: Vec<ImportRow>,
    imports_dirty: bool,
    xrefs: Vec<(Address, &'static str)>,
    xrefs_for: Option<Address>,
    xrefs_dirty: bool,
    cfg: Option<recraft_analysis::Cfg>,
    cfg_for: Option<Address>,
    cfg_dirty: bool,
    /// Graph-view zoom factor (0.5..=2.0), adjusted by the on-pane +/- controls.
    graph_zoom: f32,
    analysis: Option<AnalysisJob>,
    /// AI rename suggestions (never applied implicitly).
    ai_proposals: Vec<recraft_ai::Proposal>,
    ai_selected: AiSelection,
    ai_status: String,
    ai_settings: recraft_ai::ProviderSettings,
    ai_explanation: Option<String>,
    ai_query: String,
    ai_answer: Option<recraft_ai::QueryAnswer>,
    ai_job: Option<AiJob>,
    struct_name: String,
    struct_size: u64,
    struct_fields: String,
    struct_slot: u8,
    edit: Option<EditState>,
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
            lang: Lang::En,
            tab: Tab::Disasm,
            dock: None,
            layout_saved: None,
            frame_count: 0,
            ui_ready: false,
            theme: ThemeMode::Light,
            applied_theme: None,
            log: Vec::new(),
            status: String::new(),
            is_error: false,
            disasm_start: Address::ZERO,
            disasm_rows: Vec::new(),
            selected: 0,
            disasm_dirty: false,
            pseudo_start: Address::ZERO,
            pseudo: None,
            pseudo_dirty: false,
            pseudo_l1: false,
            pseudo_l2: false,
            functions: Vec::new(),
            function_labels: Vec::new(),
            functions_dirty: false,
            function_filter: String::new(),
            strings: Vec::new(),
            strings_dirty: false,
            string_filter: String::new(),
            names: Vec::new(),
            names_filter: String::new(),
            imports: Vec::new(),
            imports_dirty: false,
            xrefs: Vec::new(),
            xrefs_for: None,
            xrefs_dirty: false,
            cfg: None,
            cfg_for: None,
            graph_zoom: 1.0,
            cfg_dirty: false,
            analysis: None,
            ai_proposals: Vec::new(),
            ai_selected: None,
            ai_status: String::new(),
            ai_settings: recraft_ai::ProviderSettings::default(),
            ai_explanation: None,
            ai_query: String::new(),
            ai_answer: None,
            ai_job: None,
            struct_name: String::new(),
            struct_size: 16,
            struct_fields: String::new(),
            struct_slot: 0,
            edit: None,
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
        install_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut app = Self {
            recent: load_recent(),
            ai_settings: recraft_ai::ProviderSettings::load(),
            ..Self::default()
        };
        app.restore_layout();
        if let Some(path) = initial {
            app.open_path(&path);
        } else {
            app.set_status(tr(
                Lang::En,
                "Drop a .rcraft project or a PE file anywhere in this window.",
                "把 .rcraft 项目或 PE 文件拖进窗口。",
            ));
        }
        app
    }

    fn t(&self, en: &'static str, zh: &'static str) -> &'static str {
        tr(self.lang, en, zh)
    }

    fn set_status(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.log(&message);
        self.status = message;
        self.is_error = false;
    }

    fn set_error(&mut self, error: impl std::fmt::Display) {
        let message = format!("error: {error}");
        self.log(&message);
        self.status = message;
        self.is_error = true;
    }

    fn log(&mut self, message: &str) {
        self.log.push(message.to_owned());
        if self.log.len() > 500 {
            self.log.drain(..100);
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
                    self.set_status(format!("loaded {}", path.display()));
                }
                Err(error) => self.set_error(error),
            }
        } else {
            let project = path.with_extension("rcraft");
            if project.exists() {
                self.open_path(&project);
            } else {
                self.import_pe(path, &project);
            }
        }
    }

    fn import_pe(&mut self, source: &Path, project: &Path) {
        let Ok(bytes) = std::fs::read(source) else {
            self.set_error(format!("cannot read {}", source.display()));
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

    fn after_open(&mut self) {
        let entry = self
            .session
            .as_ref()
            .map(Session::entry_point)
            .unwrap_or_default();
        self.disasm_start = entry;
        self.selected = 0;
        self.disasm_dirty = true;
        self.pseudo_start = entry;
        self.pseudo_dirty = true;
        self.functions_dirty = true;
        self.strings_dirty = true;
        self.imports_dirty = true;
        self.xrefs_for = None;
        self.cfg_dirty = true;
        self.cfg_for = None;
        self.history.clear();
        self.history.push(entry);
        self.history_pos = 0;
        self.edit = None;
        self.goto = None;
    }

    fn navigate(&mut self, addr: Address) {
        if self.session.is_none() {
            return;
        }
        self.disasm_start = addr;
        self.selected = 0;
        self.disasm_dirty = true;
        self.pseudo_start = addr;
        self.pseudo_dirty = true;
        self.history.truncate(self.history_pos + 1);
        self.history_pos = self.history.len();
        self.history.push(addr);
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
                self.xrefs_dirty = true;
                self.cfg_dirty = true;
                self.set_status(format!(
                    "{} at {:#x}",
                    match op {
                        DataOp::Code => "code",
                        DataOp::Data => "data",
                        DataOp::Ascii => "ascii data",
                        DataOp::Unknown => "undefined",
                        DataOp::DefineFunction => "function",
                        DataOp::UndefineFunction => "function removed",
                    },
                    addr.get()
                ));
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
        self.pseudo_dirty = true;
    }

    fn run_edit(&mut self, edit: EditState) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let text = edit.text.trim().to_owned();
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
            EditKind::Prototype => session.execute(Command::SetPrototype {
                addr: edit.addr,
                text: if text.is_empty() { None } else { Some(text) },
            }),
        };
        match result {
            Ok(_) => {
                self.disasm_dirty = true;
                self.pseudo_dirty = true;
                self.set_status(format!("changed {:#x}", edit.addr.get()));
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

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let pal = self.palette();
        let mut visuals = match self.theme {
            ThemeMode::Dark => egui::Visuals::dark(),
            ThemeMode::Light => egui::Visuals::light(),
        };
        visuals.panel_fill = pal.panel;
        visuals.window_fill = pal.panel;
        visuals.extreme_bg_color = pal.chrome_field;
        visuals.faint_bg_color = pal.tab_focused_bg;
        visuals.override_text_color = Some(pal.chrome_text);
        visuals.selection.bg_fill = pal.selected_row;
        ctx.set_theme(match self.theme {
            ThemeMode::Dark => egui::Theme::Dark,
            ThemeMode::Light => egui::Theme::Light,
        });
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
            style.spacing.item_spacing = egui::vec2(6.0, 3.0);
            style.spacing.button_padding = egui::vec2(6.0, 2.0);
        });
    }

    fn install_fonts(&mut self, ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        let candidates = [
            ("consolas", "C:\\Windows\\Fonts\\consola.ttf"),
            ("consolas_bold", "C:\\Windows\\Fonts\\consolab.ttf"),
            ("segoeui", "C:\\Windows\\Fonts\\segoeui.ttf"),
        ];
        let mut loaded = false;
        for (name, path) in candidates {
            if let Ok(bytes) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert(name.to_owned(), egui::FontData::from_owned(bytes).into());
                loaded = true;
            }
        }
        // CJK fallback (IDA's UI is English but ReCraft ships a Chinese UI too).
        for (name, path) in [
            ("cjk", "C:\\Windows\\Fonts\\msyh.ttc"),
            ("cjk", "C:\\Windows\\Fonts\\simhei.ttf"),
            ("cjk", "C:\\Windows\\Fonts\\simsun.ttc"),
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                let mut data = egui::FontData::from_owned(bytes);
                data.index = 0;
                fonts.font_data.insert(name.to_owned(), data.into());
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    fonts
                        .families
                        .entry(family)
                        .or_default()
                        .push(name.to_owned());
                }
                loaded = true;
                break;
            }
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
        if loaded {
            ctx.set_fonts(fonts);
        }
    }

    fn tick(&mut self) {
        self.poll_analysis();
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
        if self.tab == Tab::Graph {
            self.refresh_cfg();
        }
        self.sync_hex();
    }

    /// Anchor the hex pane on the current disassembly selection (unless the user picked
    /// an explicit hex position).
    fn sync_hex(&mut self) {
        let anchor = match self.tab {
            Tab::Disasm => self.disasm_rows.get(self.selected).map(|line| line.addr),
            Tab::Pseudo | Tab::Graph => Some(self.pseudo_start),
        };
        let Some(anchor) = anchor else {
            return;
        };
        if self.hex_synced == Some(anchor) {
            return;
        }
        self.hex_start = hex_row_start(anchor);
        self.hex_cursor = Some(anchor);
        self.hex_synced = Some(anchor);
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
                self.xrefs_for = None;
                self.cfg_dirty = true;
                self.cfg_for = None;
                self.set_status(format!(
                    "{functions} functions, {strings} strings in {elapsed_ms:.0} ms"
                ));
            }
            AnalysisMessage::Cancelled => {
                self.set_status(self.t("analysis cancelled", "分析已取消").to_owned());
            }
            AnalysisMessage::Failed(error) => {
                self.set_error(recraft_common::Error::internal(error));
            }
        }
    }

    /// Poll a background polish request (spawned by [`Action::AiPolish`]).
    fn poll_ai_job(&mut self) {
        let Some(job) = self.ai_job.as_ref() else {
            return;
        };
        match job.receiver.try_recv() {
            Ok(Ok(text)) => {
                let elapsed = job.started.elapsed().as_millis();
                self.ai_explanation = Some(text);
                self.ai_status = format!("polish done in {elapsed} ms");
                self.ai_job = None;
            }
            Ok(Err(error)) => {
                self.ai_job = None;
                self.ai_status = format!("polish failed: {error}");
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.ai_job = None;
                self.ai_status = "polish worker stopped unexpectedly".to_owned();
            }
        }
    }

    fn refresh_disasm(&mut self) {
        let Some(session) = self.session.as_mut() else {
            self.disasm_rows.clear();
            self.disasm_dirty = false;
            return;
        };
        match session.disasm(self.disasm_start, DISASM_ROWS) {
            Ok(rows) => {
                self.disasm_rows = rows;
                self.disasm_dirty = false;
                if self.selected >= self.disasm_rows.len() {
                    self.selected = self.disasm_rows.len().saturating_sub(1);
                }
            }
            Err(error) => {
                self.disasm_rows.clear();
                self.disasm_dirty = false;
                self.set_error(error);
            }
        }
    }

    fn refresh_pseudo(&mut self) {
        let Some(session) = self.session.as_mut() else {
            self.pseudo = None;
            self.pseudo_dirty = false;
            return;
        };
        let code = if self.pseudo_l1 || self.pseudo_l2 {
            let function_start = session
                .functions()
                .ok()
                .and_then(|functions| {
                    functions
                        .iter()
                        .find(|function| {
                            self.pseudo_start >= function.start && self.pseudo_start < function.end
                        })
                        .map(|function| function.start)
                })
                .unwrap_or(self.pseudo_start);
            if self.pseudo_l2 {
                recraft_decompile::pseudocode_l2(session, function_start)
            } else {
                recraft_decompile::pseudocode_l1(session, function_start)
            }
        } else {
            pseudocode_l0(session, self.pseudo_start, 400)
        };
        match code {
            Ok(code) => {
                self.pseudo = Some(code.lines);
                self.pseudo_dirty = false;
            }
            Err(error) => {
                self.pseudo = None;
                self.pseudo_dirty = false;
                self.set_status(format!("decompile: {error}"));
            }
        }
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
                let mut strings = Vec::with_capacity(rows.len());
                for row in rows {
                    let mut bytes = vec![0u8; row.len.min(64) as usize];
                    let read = session.read_at(row.addr, &mut bytes).unwrap_or(0);
                    bytes.truncate(read);
                    let text = String::from_utf8_lossy(&bytes)
                        .replace('\0', "")
                        .trim()
                        .to_owned();
                    strings.push((row.addr, text));
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

    fn refresh_xrefs(&mut self) {
        let Some(current) = self.disasm_rows.get(self.selected).map(|line| line.addr) else {
            self.xrefs.clear();
            self.xrefs_for = None;
            return;
        };
        if self.xrefs_for == Some(current) && !self.xrefs_dirty {
            return;
        }
        let Some(session) = self.session.as_ref() else {
            self.xrefs.clear();
            self.xrefs_for = None;
            return;
        };
        match session.xrefs_to(current) {
            Ok(rows) => {
                self.xrefs = rows
                    .iter()
                    .map(|xref| (xref.from, xref_kind_name(xref.kind)))
                    .collect();
                self.xrefs_for = Some(current);
                self.xrefs_dirty = false;
            }
            Err(error) => {
                self.xrefs.clear();
                self.xrefs_for = Some(current);
                self.xrefs_dirty = false;
                self.set_error(error);
            }
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

    // ---------------------------------------------------------------- draw --

    /// Focus/Show entries shared by the View and Windows menus.
    fn pane_menu_items(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        for pane in Pane::all() {
            let (en, zh) = pane.title();
            let button = ui.button(self.t(en, zh));
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
        let open_label = self.t("Open project…", "打开项目…");
        let import_label = self.t("Import PE…", "导入 PE…");
        let undo_label = self.t("Undo", "撤销");
        let redo_label = self.t("Redo", "重做");
        let goto_label = self.t("Go to address…", "跳转到地址…");
        let xrefs_label = self.t("Cross references", "交叉引用");
        let back_label = self.t("Back", "后退");
        let forward_label = self.t("Forward", "前进");
        let reset_label = self.t("Reset layout", "重置布局");
        let analyze_label = self.t("Analyze functions", "分析函数");
        let cancel_label = self.t("Cancel", "取消");
        let analyzing_label = self.t("analyzing", "分析中");
        let no_project = self.t("<no project>", "<未打开项目>");
        egui::Panel::top("recraft_top").show(ui, |ui| {
            // IDA-style menu bar.
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button(open_label).clicked() {
                        *action = Action::Open;
                        ui.close();
                    }
                    if ui.button(import_label).clicked() {
                        *action = Action::Import;
                        ui.close();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button(undo_label).clicked() {
                        *action = Action::Undo;
                        ui.close();
                    }
                    if ui.button(redo_label).clicked() {
                        *action = Action::Redo;
                        ui.close();
                    }
                });
                ui.menu_button("Jump", |ui| {
                    if ui.button(goto_label).clicked() {
                        *action = Action::GotoDialog;
                        ui.close();
                    }
                    if ui.button(xrefs_label).clicked() {
                        *action = Action::ShowXrefs;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(back_label).clicked() {
                        *action = Action::Back;
                        ui.close();
                    }
                    if ui.button(forward_label).clicked() {
                        *action = Action::Forward;
                        ui.close();
                    }
                });
                ui.menu_button("Search", |ui| {
                    for pane in [Pane::Strings, Pane::Names, Pane::Imports, Pane::Exports] {
                        let (en, zh) = pane.title();
                        if ui.button(self.t(en, zh)).clicked() {
                            *action = Action::FocusPane(pane);
                            ui.close();
                        }
                    }
                });
                ui.menu_button("View", |ui| {
                    self.pane_menu_items(ui, action);
                    ui.separator();
                    if ui.button(reset_label).clicked() {
                        *action = Action::ResetLayout;
                        ui.close();
                    }
                });
                ui.menu_button("Options", |ui| {
                    ui.menu_button("Theme", |ui| {
                        if ui
                            .selectable_label(self.theme == ThemeMode::Dark, "Dark")
                            .clicked()
                        {
                            *action = Action::SetTheme(ThemeMode::Dark);
                            ui.close();
                        }
                        if ui
                            .selectable_label(self.theme == ThemeMode::Light, "Light")
                            .clicked()
                        {
                            *action = Action::SetTheme(ThemeMode::Light);
                            ui.close();
                        }
                    });
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
                ui.menu_button("Windows", |ui| {
                    self.pane_menu_items(ui, action);
                    ui.separator();
                    if ui.button(reset_label).clicked() {
                        *action = Action::ResetLayout;
                        ui.close();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About ReCraft").clicked() {
                        self.set_status(format!("ReCraft {}", env!("CARGO_PKG_VERSION")));
                        ui.close();
                    }
                });
            });
            ui.separator();
            // Toolbar row.
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(self.history_pos > 0, egui::Button::new("◀"))
                    .on_hover_text(back_label)
                    .clicked()
                {
                    *action = Action::Back;
                }
                if ui
                    .add_enabled(
                        self.history_pos + 1 < self.history.len(),
                        egui::Button::new("▶"),
                    )
                    .on_hover_text(forward_label)
                    .clicked()
                {
                    *action = Action::Forward;
                }
                ui.separator();
                let graph_active = matches!(self.tab, Tab::Graph);
                if ui.selectable_label(!graph_active, "Text").clicked() {
                    *action = Action::FocusPane(Pane::Disasm);
                }
                if ui.selectable_label(graph_active, "Graph").clicked() {
                    *action = Action::FocusPane(Pane::Graph);
                }
                ui.separator();
                if let Some(job) = self.analysis.as_ref() {
                    let elapsed = job.started.elapsed().as_secs_f64();
                    ui.label(format!("{analyzing_label}… {elapsed:.1}s"));
                    if ui.button(cancel_label).clicked() {
                        *action = Action::CancelAnalysis;
                    }
                } else if ui.button(analyze_label).clicked() {
                    *action = Action::Analyze;
                }
                ui.separator();
                if ui.button("AI").clicked() {
                    *action = Action::FocusPane(Pane::Ai);
                }
                if ui
                    .button(match self.theme {
                        ThemeMode::Dark => "◐",
                        ThemeMode::Light => "◑",
                    })
                    .on_hover_text(self.t("Toggle theme (Ctrl+T)", "切换主题 (Ctrl+T)"))
                    .clicked()
                {
                    *action = Action::ToggleTheme;
                }
                ui.separator();
                if ui
                    .button(if self.lang == Lang::En {
                        "中文"
                    } else {
                        "English"
                    })
                    .clicked()
                {
                    *action = Action::ToggleLang;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let path_text = self
                        .project
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| no_project.to_owned());
                    ui.weak(path_text);
                });
            });
            ui.separator();
            // Navigation band + legend (IDA's address-space strip).
            self.navigation_band(ui, action);
            ui.horizontal_wrapped(|ui| {
                for (color, label) in [
                    (
                        Color32::from_rgb(0x3a, 0x5a, 0x7a),
                        self.t("Regular function", "常规函数"),
                    ),
                    (
                        Color32::from_rgb(0x2f, 0x6a, 0x6a),
                        self.t("Instruction", "指令"),
                    ),
                    (Color32::from_rgb(0x4a, 0x6a, 0x3a), self.t("Data", "数据")),
                    (
                        Color32::from_rgb(0x3a, 0x3a, 0x3a),
                        self.t("Unexplored", "未探索"),
                    ),
                    (
                        Color32::from_rgb(0x7a, 0x3a, 0x6a),
                        self.t("External symbol", "外部符号"),
                    ),
                ] {
                    ui.colored_label(color, "■");
                    ui.weak(label);
                }
            });
        });
    }

    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("recraft_bottom").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.weak("AU: idle");
                ui.separator();
                let color = if self.is_error {
                    Color32::from_rgb(0xe0, 0x6a, 0x6a)
                } else {
                    ui.visuals().weak_text_color()
                };
                if !self.status.is_empty() {
                    ui.colored_label(color, &self.status);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(self.t("(Synchronized with Hex View-1)", "（与 Hex View-1 同步）"));
                    ui.separator();
                    if let Some((start, _)) = self.current_function() {
                        let name = self
                            .session
                            .as_ref()
                            .and_then(|session| session.name_at(start).ok().flatten())
                            .unwrap_or_else(|| format!("sub_{:x}", start.get()));
                        ui.label(format!("{name}."));
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
        let byte_label = self.t("Bytes", "字节");
        let word_label = self.t("Words", "字");
        let follow_label = self.t("Follow selection", "跟随选中");
        let empty_label = self.t("No readable bytes here.", "该地址没有可读字节。");
        let address_label = self.t("Address", "地址");
        let mut unit = self.hex_unit;
        ui.horizontal(|ui| {
            ui.strong(self.t("Hex", "十六进制"));
            ui.selectable_value(&mut unit, HexUnit::Byte, byte_label);
            ui.selectable_value(&mut unit, HexUnit::Word, word_label);
            ui.separator();
            ui.weak(format!("{:#x}", self.hex_start.get()));
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
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let columns = match unit {
                    HexUnit::Byte => 18,
                    HexUnit::Word => 10,
                };
                egui::Grid::new("hex_grid")
                    .num_columns(columns)
                    .striped(true)
                    .spacing([8.0, 1.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new(address_label).monospace().strong());
                        match unit {
                            HexUnit::Byte => {
                                for offset in 0..16 {
                                    ui.label(
                                        RichText::new(format!("{offset:02x}")).monospace().weak(),
                                    );
                                }
                            }
                            HexUnit::Word => {
                                for word in 0..8 {
                                    ui.label(
                                        RichText::new(format!("{:04x}", word * 2))
                                            .monospace()
                                            .weak(),
                                    );
                                }
                            }
                        }
                        ui.label(RichText::new("ASCII").monospace().strong());
                        ui.end_row();
                        for row in 0..rows {
                            let row_addr = Address::new(start.get() + (row as u64) * 16);
                            ui.label(
                                RichText::new(format!("{:#012x}", row_addr.get()))
                                    .monospace()
                                    .color(pal.addr),
                            );
                            match unit {
                                HexUnit::Byte => {
                                    for column in 0..16 {
                                        let address = Address::new(row_addr.get() + column as u64);
                                        if let Some(byte) = buffer.get(row * 16 + column) {
                                            let response = ui.selectable_label(
                                                cursor == Some(address),
                                                RichText::new(format!("{byte:02x}")).monospace(),
                                            );
                                            if response.clicked() {
                                                clicked = Some(address);
                                            }
                                            if response.double_clicked() {
                                                navigated = Some(address);
                                            }
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
                                            let selected = cursor.is_some_and(|cursor| {
                                                cursor.get() >= address.get()
                                                    && cursor.get() < address.get() + 2
                                            });
                                            let response = ui.selectable_label(
                                                selected,
                                                RichText::new(format!("{value:04x}")).monospace(),
                                            );
                                            if response.clicked() {
                                                clicked = Some(address);
                                            }
                                            if response.double_clicked() {
                                                navigated = Some(address);
                                            }
                                        } else {
                                            ui.label(RichText::new("    ").monospace());
                                        }
                                    }
                                }
                            }
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
                            ui.label(RichText::new(ascii).monospace().color(pal.bytes));
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
    }

    /// Build the default IDA-like dock layout once: Functions + Graph overview on the
    /// left, document tabs (IDA View-A/Hex/Pseudocode/Imports/…) filling the right and an
    /// Output window across the bottom.
    fn ensure_dock(&mut self) {
        if self.dock.is_some() {
            return;
        }
        let mut state = egui_dock::DockState::new(vec![
            Pane::Graph,
            Pane::Disasm,
            Pane::Hex,
            Pane::Pseudo,
            Pane::Imports,
            Pane::Exports,
            Pane::Strings,
            Pane::Names,
            Pane::Xrefs,
            Pane::Segments,
            Pane::Structs,
            Pane::Ai,
        ]);
        {
            let surface = state.main_surface_mut();
            // Output window across the bottom (about 18% like the IDA reference).
            let [top, _output] =
                surface.split_below(egui_dock::NodeIndex::root(), 0.82, vec![Pane::Output]);
            // Functions window with a graph overview underneath it.
            let [left, _docs] = surface.split_left(top, 0.22, vec![Pane::Functions]);
            let [_functions, _overview] =
                surface.split_below(left, 0.74, vec![Pane::GraphOverview]);
        }
        // IDA shows the graph by default; Text remains one click away.
        if let Some(path) = state.find_tab(&Pane::Graph) {
            let _ = state.set_active_tab(path);
        }
        self.dock = Some(state);
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
        if let Some(path) = state.find_tab(&pane) {
            let node_path = path.node_path();
            if state.set_active_tab(path).is_ok() {
                state.set_focused_node_and_surface(node_path);
            }
        } else {
            state.push_to_focused_leaf(pane);
        }
    }

    /// Persist the dock layout and theme to `%LOCALAPPDATA%\ReCraft\layout.json` when they
    /// change.
    fn persist_layout_if_changed(&mut self) {
        self.frame_count = self.frame_count.wrapping_add(1);
        if self.frame_count % 30 != 0 || cfg!(test) {
            return;
        }
        let Some(state) = self.dock.as_ref() else {
            return;
        };
        let theme = self.theme;
        let saved = SavedLayout {
            version: LAYOUT_VERSION,
            dock: state.clone(),
            theme,
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
        if save_layout_to(&path, state, theme).is_ok() {
            self.layout_saved = Some(text);
        }
    }

    fn restore_layout(&mut self) {
        if let Some(path) = layout_store_path()
            && let Some(saved) = load_layout_from(&path)
        {
            self.theme = saved.theme;
            self.dock = Some(saved.dock);
        }
    }

    fn segments_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let Some(session) = self.session.as_ref() else {
            ui.weak(self.t("no project", "无项目"));
            return;
        };
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
                    let response = ui.selectable_label(false, label);
                    if response.clicked() || response.double_clicked() {
                        *action = Action::NavigateToDisasm(segment.range.start);
                    }
                }
            });
    }

    /// PE exports list (document tab, like IDA's Exports window).
    fn exports_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let exports: Vec<(String, Address, Option<String>)> = {
            let Some(session) = self.session.as_ref() else {
                ui.weak(self.t("no project", "无项目"));
                return;
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
        };
        if exports.is_empty() {
            ui.weak(self.t("No exports.", "没有导出。"));
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("exports_grid")
                    .num_columns(3)
                    .striped(true)
                    .spacing([12.0, 2.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new(self.t("Name", "名字")).strong());
                        ui.label(RichText::new(self.t("Address", "地址")).strong());
                        ui.label(RichText::new(self.t("Forwarder", "转发")).strong());
                        ui.end_row();
                        for (name, address, forwarder) in &exports {
                            let response = ui.selectable_label(false, name);
                            ui.label(RichText::new(format!("{:#x}", address.get())).monospace());
                            ui.label(RichText::new(forwarder.as_deref().unwrap_or("")).weak());
                            ui.end_row();
                            if response.clicked() || response.double_clicked() {
                                *action = Action::NavigateToDisasm(*address);
                            }
                        }
                    });
            });
    }

    /// Miniature CFG map under the Functions window (IDA's "Graph overview").
    fn graph_overview(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let pal = self.palette();
        let Some((start, end)) = self.current_function() else {
            ui.weak(self.t("no function selected", "没有选中函数"));
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
            ui.weak(self.t("no graph", "没有图形"));
            return;
        };
        if cfg.blocks.is_empty() {
            return;
        }
        let layer_of = recraft_analysis::layers(&cfg);
        let mut layers: std::collections::BTreeMap<usize, Vec<usize>> =
            std::collections::BTreeMap::new();
        for (index, layer) in layer_of.iter().enumerate() {
            layers.entry(*layer).or_default().push(index);
        }
        let width = ui.available_width().max(80.0);
        let height = ui.available_height().max(80.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 2.0, pal.view);
        let columns = layers.len().max(1) as f32;
        let rows = layers.values().map(Vec::len).max().unwrap_or(1).max(1) as f32;
        let cell_w = rect.width() / columns;
        let cell_h = rect.height() / rows;
        let selected = self
            .disasm_rows
            .get(self.selected)
            .map(|line| line.addr.get());
        for (column, (_, blocks)) in layers.iter().enumerate() {
            for (row, &index) in blocks.iter().enumerate() {
                let block = &cfg.blocks[index];
                let block_w = (cell_w * 0.7).clamp(6.0, 64.0);
                let block_h = (cell_h * 0.6).clamp(4.0, 14.0);
                let block_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.left() + column as f32 * cell_w + 3.0,
                        rect.top() + row as f32 * cell_h + 3.0,
                    ),
                    egui::vec2(block_w, block_h),
                );
                let current = selected
                    .is_some_and(|addr| addr >= block.start.get() && addr < block.end.get());
                painter.rect_filled(
                    block_rect,
                    1.0,
                    if current { pal.addr } else { pal.edge_jump },
                );
                let response = ui.interact(
                    block_rect,
                    ui.id().with(("overview", block.start.get())),
                    Sense::click(),
                );
                if response.clicked() {
                    *action = Action::NavigateToDisasm(block.start);
                }
            }
        }
    }

    /// Output window: analysis/status log lines (IDA's Output pane).
    fn output_pane(&mut self, ui: &mut egui::Ui, _action: &mut Action) {
        let pal = self.palette();
        if self.log.is_empty() {
            ui.weak(self.t("No messages yet.", "还没有消息。"));
            return;
        }
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for line in &self.log {
                    let color = if line.starts_with("error:") {
                        Color32::from_rgb(0xe0, 0x6a, 0x6a)
                    } else {
                        pal.text
                    };
                    ui.label(RichText::new(line).monospace().size(11.0).color(color));
                }
            });
    }

    fn functions_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let pal = self.palette();
        if self.functions.is_empty() {
            ui.weak(self.t(
                "No functions yet — press Analyze (F5).",
                "还没有函数——按 Analyze（F5）。",
            ));
        } else {
            let hint = self.t("name or address…", "名字或地址…");
            ui.horizontal(|ui| {
                ui.weak(self.t("filter", "过滤"));
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
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("functions_grid")
                    .num_columns(3)
                    .striped(true)
                    .spacing([10.0, 2.0])
                    .show(ui, |ui| {
                        ui.label("");
                        ui.label(RichText::new(self.t("Function name", "函数名")).strong());
                        ui.label(RichText::new(self.t("Segment", "段")).strong());
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
                            let response = ui.selectable_label(false, name).on_hover_text(format!(
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
                        }
                    });
                if shown == 0 && !self.functions.is_empty() {
                    ui.weak(self.t("(no match)", "（无匹配）"));
                }
            });
    }

    fn strings_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        if self.strings.is_empty() {
            ui.weak(self.t(
                "No strings yet — run Analyze (F5).",
                "还没有字符串——运行 Analyze（F5）。",
            ));
        }
        let hint = self.t("substring…", "子串…");
        ui.horizontal(|ui| {
            ui.weak(self.t("filter", "过滤"));
            ui.add(
                egui::TextEdit::singleline(&mut self.string_filter)
                    .desired_width(f32::INFINITY)
                    .hint_text(hint),
            );
        });
        let filter = self.string_filter.to_lowercase();
        let mut shown = 0usize;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (addr, text) in &self.strings {
                    if !filter.is_empty() && !text.to_lowercase().contains(&filter) {
                        continue;
                    }
                    if shown >= 2000 {
                        ui.weak(self.t("(first 2000 matches)", "(仅显示前 2000 条)"));
                        break;
                    }
                    shown += 1;
                    let label = format!("{:#012x}  {text}", addr.get());
                    let response = ui.selectable_label(false, label);
                    if response.clicked() {
                        *action = Action::Navigate(*addr);
                    }
                    if response.double_clicked() {
                        *action = Action::NavigateToDisasm(*addr);
                    }
                }
            });
    }

    fn imports_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        if self.imports.is_empty() {
            ui.weak(self.t(
                "No imports (or open a project first).",
                "没有导入（或先打开项目）。",
            ));
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for import in &self.imports {
                    let label = format!(
                        "{}!{}  [{}]  ×{}",
                        import.module, import.api, import.category, import.refs
                    );
                    let response = ui.selectable_label(false, label);
                    let target = import.first_from.unwrap_or(import.iat);
                    if response.clicked() {
                        *action = Action::Navigate(target);
                    }
                    if response.double_clicked() {
                        *action = Action::NavigateToDisasm(target);
                    }
                }
            });
    }

    fn xrefs_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let Some(addr) = self.disasm_rows.get(self.selected).map(|line| line.addr) else {
            ui.weak(self.t("no selection", "没有选中行"));
            return;
        };
        ui.weak(format!(
            "{} {:#x}",
            self.t("references to", "引用目标"),
            addr.get()
        ));
        if self.xrefs.is_empty() {
            ui.weak(self.t("no references", "没有引用"));
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (from, kind) in &self.xrefs {
                    let label = format!("<- {:#012x}  {kind}", from.get());
                    let response = ui.selectable_label(false, label).on_hover_text(self.t(
                        "double-click to follow into disassembly",
                        "双击跟随到反汇编",
                    ));
                    if response.clicked() {
                        *action = Action::Navigate(*from);
                    }
                    if response.double_clicked() {
                        *action = Action::NavigateToDisasm(*from);
                    }
                }
            });
    }

    fn names_list(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        if self.names.is_empty() {
            ui.weak(self.t(
                "No names yet — open a project and run Analyze.",
                "还没有名字——打开项目并运行 Analyze。",
            ));
        }
        let hint = self.t("name or address…", "名字或地址…");
        ui.horizontal(|ui| {
            ui.weak(self.t("filter", "过滤"));
            ui.add(
                egui::TextEdit::singleline(&mut self.names_filter)
                    .desired_width(f32::INFINITY)
                    .hint_text(hint),
            );
        });
        let filter = self.names_filter.to_lowercase();
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
                    let response = ui.selectable_label(false, label);
                    if response.clicked() || response.double_clicked() {
                        *action = Action::NavigateToDisasm(*addr);
                    }
                }
            });
    }
    /// Structures window: list user layouts, define one, bind it to a function argument.
    fn structs_panel(&mut self, ui: &mut egui::Ui, _action: &mut Action) {
        let structs: Vec<StructDef> = {
            let Some(session) = self.session.as_ref() else {
                ui.weak(self.t("no project", "无项目"));
                return;
            };
            session.structs().unwrap_or_default()
        };
        if structs.is_empty() {
            ui.weak(self.t(
                "No structures yet — define one below.",
                "还没有结构体——在下面定义一个。",
            ));
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
        ui.strong(self.t("Define structure", "定义结构体"));
        egui::Grid::new("struct_form")
            .num_columns(2)
            .spacing([8.0, 2.0])
            .show(ui, |ui| {
                ui.label(self.t("name", "名字"));
                ui.add(egui::TextEdit::singleline(&mut self.struct_name).desired_width(260.0));
                ui.end_row();
                ui.label(self.t("size", "大小"));
                ui.add(egui::DragValue::new(&mut self.struct_size).range(1..=65536));
                ui.end_row();
                ui.label(self.t("fields", "字段"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.struct_fields)
                        .desired_width(260.0)
                        .hint_text("0:tag:u32, 16:value:i32"),
                );
                ui.end_row();
            });
        if ui.button(self.t("Define / update", "定义/更新")).clicked() {
            self.define_struct_from_form();
        }
        ui.separator();
        ui.strong(self.t("Apply to selected function argument", "绑定到当前函数参数"));
        ui.horizontal(|ui| {
            ui.label(self.t("slot", "参数槽"));
            ui.add(egui::DragValue::new(&mut self.struct_slot).range(0..=7));
            ui.label(self.t("type", "类型"));
            ui.add(egui::TextEdit::singleline(&mut self.struct_name).desired_width(140.0));
            if ui.button(self.t("Apply", "应用")).clicked() {
                self.apply_struct_to_slot();
            }
        });
        ui.weak(self.t(
            "Both operations are journaled; Ctrl+Z undoes them.",
            "这两个操作都走 journal；Ctrl+Z 可撤销。",
        ));
    }

    fn define_struct_from_form(&mut self) {
        let name = self.struct_name.trim().to_owned();
        if name.is_empty() {
            self.set_status(
                self.t("structure name is required", "结构体名字不能为空")
                    .to_owned(),
            );
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
                self.set_status(format!("structure {name} defined/updated"));
            }
            Err(error) => self.set_error(error),
        }
    }

    fn apply_struct_to_slot(&mut self) {
        let Some((start, _)) = self.current_function() else {
            self.set_status(
                self.t(
                    "put the cursor inside a function first",
                    "先把光标放到函数内",
                )
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
                self.set_status(format!(
                    "argument slot {} of {:#x} bound",
                    self.struct_slot,
                    start.get()
                ));
            }
            Err(error) => self.set_error(error),
        }
    }

    fn ai_panel(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let suggest = self.t("Suggest names", "建议命名");
        let apply_label = self.t("Apply selected", "应用选中");
        let explain_label = self.t("Explain", "解释");
        let polish_label = self.t("Polish", "润色");
        let clear = self.t("Clear", "清空");
        let has_selection = self
            .ai_selected
            .and_then(|index| self.ai_proposals.get(index))
            .is_some();
        ui.horizontal_wrapped(|ui| {
            if ui.button(suggest).clicked() {
                *action = Action::AiSuggest;
            }
            if ui
                .add_enabled(has_selection, egui::Button::new(apply_label))
                .clicked()
            {
                *action = Action::AiApply;
            }
            if ui.button(explain_label).clicked() {
                *action = Action::AiExplain;
            }
            if ui.button(polish_label).clicked() {
                *action = Action::AiPolish;
            }
            if ui.button(clear).clicked() {
                *action = Action::AiClear;
            }
        });
        self.ai_settings_ui(ui);
        let query_hint = self.t(
            "ask: urls | callers of CreateFileW | text http",
            "问：urls｜谁调用 CreateFileW｜text http",
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.ai_query)
                    .desired_width(f32::INFINITY)
                    .hint_text(query_hint),
            );
            if ui.button(self.t("Query", "查询")).clicked() {
                *action = Action::AiQuery;
            }
        });
        if !self.ai_status.is_empty() {
            ui.weak(&self.ai_status);
        }
        if let Some(answer) = self.ai_answer.clone() {
            ui.separator();
            ui.weak(format!(
                "{} · {} match(es){}",
                answer.intent,
                answer.items.len(),
                if answer.truncated { " (truncated)" } else { "" }
            ));
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
        }
        if let Some(text) = self.ai_explanation.clone() {
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("ai_explanation")
                .max_height(220.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.label(RichText::new(text).monospace().size(11.0));
                });
        }
        if self.ai_proposals.is_empty() {
            ui.weak(self.t(
                "No suggestions yet — run Analyze, then press Suggest names.",
                "还没有建议——先运行 Analyze，再点“建议命名”。",
            ));
            return;
        }
        ui.separator();
        for (index, proposal) in self.ai_proposals.iter().enumerate() {
            let selected = self.ai_selected == Some(index);
            let label = format!(
                "{:.2}  {:#012x} -> {}",
                proposal.confidence,
                proposal.target.get(),
                proposal.value
            );
            if ui.selectable_label(selected, label).clicked() {
                self.ai_selected = if selected { None } else { Some(index) };
            }
            ui.weak(format!(
                "{} ({} evidence)",
                proposal.rationale,
                proposal.evidence.len()
            ));
            if let Some(evidence) = proposal.evidence.first()
                && ui
                    .link(format!("evidence {:#x}", evidence.addr.get()))
                    .clicked()
            {
                *action = Action::Navigate(evidence.addr);
            }
        }
    }

    /// Provider settings editor (persisted; the API key itself is never stored).
    fn ai_settings_ui(&mut self, ui: &mut egui::Ui) {
        let title = self.t("Provider settings", "Provider 设置");
        egui::CollapsingHeader::new(title)
            .default_open(self.ai_settings.kind != recraft_ai::ProviderKind::Heuristic)
            .show(ui, |ui| {
                egui::ComboBox::from_id_salt("ai_provider_kind")
                    .selected_text(self.ai_settings.kind.as_str())
                    .show_ui(ui, |ui| {
                        for kind in [
                            recraft_ai::ProviderKind::Heuristic,
                            recraft_ai::ProviderKind::OpenAiCompatible,
                            recraft_ai::ProviderKind::Ollama,
                        ] {
                            ui.selectable_value(&mut self.ai_settings.kind, kind, kind.as_str());
                        }
                    });
                ui.horizontal(|ui| {
                    ui.label(self.t("endpoint", "接口地址"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.ai_settings.endpoint)
                            .desired_width(f32::INFINITY)
                            .hint_text("https://api.openai.com/v1/chat/completions"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(self.t("model", "模型"));
                    ui.add(egui::TextEdit::singleline(&mut self.ai_settings.model));
                    ui.label(self.t("key env", "密钥环境变量"));
                    ui.add(egui::TextEdit::singleline(
                        &mut self.ai_settings.api_key_env,
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label(self.t("max tokens", "最大 token"));
                    ui.add(egui::DragValue::new(&mut self.ai_settings.max_tokens).range(64..=8192));
                });
                if ui.button(self.t("Save settings", "保存设置")).clicked() {
                    self.ai_settings.refresh_enabled();
                    match self.ai_settings.save() {
                        Ok(()) => {
                            self.ai_status = format!(
                                "{}{}",
                                self.t("settings saved · ", "设置已保存 · "),
                                self.ai_settings.describe()
                            );
                        }
                        Err(error) => self.set_error(error),
                    }
                }
                let key_state = if self.ai_settings.api_key_present() {
                    self.t("API key env is set", "API key 环境变量已设置")
                } else {
                    self.t("API key env is not set", "API key 环境变量未设置")
                };
                ui.weak(format!(
                    "{key_state} · {}",
                    if self.ai_settings.enabled {
                        self.t("enabled", "已启用")
                    } else {
                        self.t("offline", "离线")
                    }
                ));
            });
    }

    /// Apply the selected AI proposal through the command bus (journaled/undoable).
    fn apply_ai_selection(&mut self) -> bool {
        let Some(index) = self.ai_selected else {
            return false;
        };
        let Some(proposal) = self.ai_proposals.get(index).cloned() else {
            return false;
        };
        let result = self
            .session
            .as_mut()
            .map(|session| recraft_ai::apply_proposals(session, std::slice::from_ref(&proposal)));
        match result {
            Some(Ok(1)) => {
                self.functions_dirty = true;
                self.disasm_dirty = true;
                self.ai_proposals.remove(index);
                self.ai_selected = None;
                self.ai_status = format!("renamed {:#x}", proposal.target.get());
                self.set_status(format!(
                    "applied 1 proposal ({:#x} -> {}); undo with Ctrl+Z",
                    proposal.target.get(),
                    proposal.value
                ));
                true
            }
            Some(Err(error)) => {
                self.set_error(error);
                false
            }
            _ => false,
        }
    }

    /// IDA-style graph view: basic-block nodes with teal frames and headers, curved
    /// coloured edges with arrowheads, entry marker, zoom controls and a status line.
    fn graph_view(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        self.tab = Tab::Graph;
        let pal = self.palette();
        let Some(cfg) = self.cfg.clone() else {
            ui.weak(self.t(
                "No control-flow graph — run Analyze (F5) and put the cursor inside a function.",
                "没有控制流图——运行 Analyze（F5），把光标放到函数内。",
            ));
            return;
        };
        if cfg.blocks.is_empty() {
            ui.weak(self.t("function has no decodable blocks", "函数没有可解码的基本块"));
            return;
        }
        let edge_color = |kind: recraft_analysis::EdgeKind| match kind {
            recraft_analysis::EdgeKind::Fallthrough => pal.edge_jump,
            recraft_analysis::EdgeKind::Taken => pal.edge_taken,
            recraft_analysis::EdgeKind::NotTaken => pal.edge_not_taken,
            recraft_analysis::EdgeKind::Unconditional => pal.edge_jump,
            recraft_analysis::EdgeKind::JumpTable => pal.edge_jump_table,
        };
        ui.horizontal(|ui| {
            for (label, color) in [
                (
                    "fallthrough",
                    edge_color(recraft_analysis::EdgeKind::Fallthrough),
                ),
                ("taken", edge_color(recraft_analysis::EdgeKind::Taken)),
                (
                    "not taken",
                    edge_color(recraft_analysis::EdgeKind::NotTaken),
                ),
                (
                    "jump",
                    edge_color(recraft_analysis::EdgeKind::Unconditional),
                ),
                (
                    "jump table",
                    edge_color(recraft_analysis::EdgeKind::JumpTable),
                ),
            ] {
                ui.colored_label(color, label);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("+").on_hover_text("Zoom in").clicked() {
                    self.graph_zoom = (self.graph_zoom + 0.1).min(2.0);
                }
                if ui.small_button("-").on_hover_text("Zoom out").clicked() {
                    self.graph_zoom = (self.graph_zoom - 0.1).max(0.5);
                }
                if ui.small_button("Fit").on_hover_text("Reset zoom").clicked() {
                    self.graph_zoom = 1.0;
                }
                ui.monospace(format!("{:.2}%", self.graph_zoom * 100.0));
            });
        });
        let zoom = self.graph_zoom;
        let node_w = 300.0 * zoom;
        let header_h = 18.0 * zoom;
        let line_h = 14.0 * zoom;
        let text_size = 11.0 * zoom;
        let gap_x = 120.0 * zoom;
        let gap_y = 24.0 * zoom;
        let pad = 10.0 * zoom;
        let max_lines = 10usize;
        let layer_of = recraft_analysis::layers(&cfg);
        let column_count = layer_of.iter().copied().max().unwrap_or(0) + 1;
        let mut column_y = vec![24.0 * zoom; column_count];
        let mut local: Vec<egui::Rect> = Vec::with_capacity(cfg.blocks.len());
        for (index, block) in cfg.blocks.iter().enumerate() {
            let layer = layer_of[index];
            let shown = block.insns.len().min(max_lines);
            let height = header_h + shown as f32 * line_h + pad * 2.0 + 4.0 * zoom;
            let x = 24.0 * zoom + layer as f32 * (node_w + gap_x);
            let y = column_y[layer];
            column_y[layer] = y + height + gap_y;
            local.push(egui::Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2(node_w, height),
            ));
        }
        let canvas_w = 48.0 * zoom + column_count as f32 * (node_w + gap_x);
        let canvas_h = column_y.iter().copied().fold(24.0f32 * zoom, f32::max) + 24.0 * zoom;
        let selected = self
            .disasm_rows
            .get(self.selected)
            .map(|line| line.addr.get());
        let mono = FontId::monospace(text_size);
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
                            "entry",
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
                            append(&mut job, operands, pal.text, &mono);
                            append(&mut job, &format!("; {comment}"), pal.comment, &mono);
                        } else {
                            append(&mut job, operands, pal.text, &mono);
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
        ui.separator();
        ui.horizontal(|ui| {
            ui.weak(format!("{:.2}%", zoom * 100.0));
            ui.separator();
            ui.weak(format!("{} blocks", cfg.blocks.len()));
            ui.separator();
            ui.weak(self.t("(Synchronized with Hex View-1)", "（与 Hex View-1 同步）"));
        });
    }

    /// First-run experience: quick actions, recent projects and a shortcut cheat sheet.
    fn welcome(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        ui.add_space(24.0);
        ui.heading(self.t(
            "ReCraft — reassembly & decompilation workbench",
            "ReCraft —— 反汇编与反编译工作台",
        ));
        ui.add_space(4.0);
        ui.label(self.t(
            "Drop a .rcraft project or a PE file anywhere in this window, or:",
            "把 .rcraft 项目或 PE 文件拖进窗口，或：",
        ));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(self.t("Open project…", "打开项目…")).clicked() {
                *action = Action::Open;
            }
            if ui.button(self.t("Import PE…", "导入 PE…")).clicked() {
                *action = Action::Import;
            }
        });
        if !self.recent.is_empty() {
            ui.add_space(16.0);
            ui.separator();
            ui.label(self.t("Recent projects", "最近打开"));
            let recent = self.recent.clone();
            for (index, path) in recent.iter().enumerate() {
                let label = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                let response = ui
                    .selectable_label(false, label)
                    .on_hover_text(path.display().to_string());
                if response.clicked() {
                    *action = Action::OpenRecent(index);
                }
            }
        }
        ui.add_space(16.0);
        ui.separator();
        ui.weak(self.t(
            "Shortcuts: Ctrl+P palette · G goto · N rename · ; comment · X xrefs · Space pseudo-C · Esc back · Enter follow",
            "快捷键：Ctrl+P 命令面板 · G 跳转 · N 改名 · ; 注释 · X 交叉引用 · 空格伪代码 · Esc 后退 · Enter 跟随",
        ));
    }

    fn disasm_view(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        self.tab = Tab::Disasm;
        let pal = self.palette();
        if self.disasm_rows.is_empty() {
            ui.weak(self.t(
                "No disassembly. Import a PE file first.",
                "没有反汇编。先导入 PE 文件。",
            ));
            return;
        }
        let mono = FontId::monospace(MONO_SIZE);
        let follow_menu = self.t("Follow operand", "跟随操作数");
        let rename_menu = self.t("Rename…", "改名…");
        let comment_menu = self.t("Comment…", "注释…");
        let xrefs_menu = self.t("Cross references", "交叉引用");
        let copy_menu = self.t("Copy address", "复制地址");
        let undo_menu = self.t("Undo", "撤销");
        let redo_menu = self.t("Redo", "重做");
        let mut clicked_row = None;
        let mut follow = false;
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let header = format!(
                    "{:<14}{:<46}{:<9}{:<36}{}",
                    self.t("Address", "地址"),
                    self.t("Bytes", "字节"),
                    self.t("Mnemonic", "助记符"),
                    self.t("Operands", "操作数"),
                    self.t("Comment", "注释"),
                );
                ui.add(egui::Label::new(RichText::new(header).monospace().strong()).truncate());
                let width = ui.available_width().max(1000.0);
                for (index, line) in self.disasm_rows.iter().enumerate() {
                    // IDA renders names as inline labels above the instruction instead of a
                    // dedicated "Name" column.
                    let label_height = if line.name.is_some() {
                        MONO_SIZE + 1.0
                    } else {
                        0.0
                    };
                    let row_height = ROW_HEIGHT + label_height;
                    // One full-row hit area, like IDA: the whole line is selectable and
                    // double-click follows the operand instead of only the address cell.
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(width, row_height), Sense::click());
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
                    let bytes = line
                        .bytes()
                        .iter()
                        .map(|byte| format!("{byte:02x} "))
                        .collect::<String>();
                    let mut job = LayoutJob::default();
                    append(
                        &mut job,
                        &format!("{:#012x}  ", line.addr.get()),
                        pal.addr,
                        &mono,
                    );
                    append(&mut job, &format!("{bytes:<46}"), pal.bytes, &mono);
                    append(
                        &mut job,
                        &format!("{:<9}", line.mnemonic),
                        pal.mnemonic,
                        &mono,
                    );
                    append(&mut job, &format!("{:<36}", line.operands), pal.text, &mono);
                    append(
                        &mut job,
                        line.comment.as_deref().unwrap_or(""),
                        pal.comment,
                        &mono,
                    );
                    let galley = ui.painter().layout_job(job);
                    ui.painter().galley(
                        rect.left_top() + egui::vec2(4.0, label_height + 1.0),
                        galley,
                        Color32::WHITE,
                    );
                    if response.clicked() || response.secondary_clicked() {
                        clicked_row = Some(index);
                    }
                    if response.double_clicked() {
                        clicked_row = Some(index);
                        follow = true;
                    }
                    response.context_menu(|ui| {
                        if ui.button(follow_menu).clicked() {
                            *action = Action::Follow;
                            ui.close();
                        }
                        if ui.button(rename_menu).clicked() {
                            *action = Action::EditName;
                            ui.close();
                        }
                        if ui.button(comment_menu).clicked() {
                            *action = Action::EditComment;
                            ui.close();
                        }
                        if ui.button(xrefs_menu).clicked() {
                            *action = Action::ShowXrefs;
                            ui.close();
                        }
                        ui.separator();
                        if ui
                            .button(self.t("Mark as code (C)", "标记为代码 (C)"))
                            .clicked()
                        {
                            *action = Action::ApplyData(DataOp::Code);
                            ui.close();
                        }
                        if ui
                            .button(self.t("Mark as data (D)", "标记为数据 (D)"))
                            .clicked()
                        {
                            *action = Action::ApplyData(DataOp::Data);
                            ui.close();
                        }
                        if ui
                            .button(self.t("Mark as string (A)", "标记为字符串 (A)"))
                            .clicked()
                        {
                            *action = Action::ApplyData(DataOp::Ascii);
                            ui.close();
                        }
                        if ui.button(self.t("Undefine (U)", "取消定义 (U)")).clicked() {
                            *action = Action::ApplyData(DataOp::Unknown);
                            ui.close();
                        }
                        if ui
                            .button(self.t("Create function (P)", "创建函数 (P)"))
                            .clicked()
                        {
                            *action = Action::ApplyData(DataOp::DefineFunction);
                            ui.close();
                        }
                        if ui.button(self.t("Remove function", "删除函数")).clicked() {
                            *action = Action::ApplyData(DataOp::UndefineFunction);
                            ui.close();
                        }
                        if ui.button(self.t("Edit prototype…", "编辑原型…")).clicked() {
                            *action = Action::EditPrototype;
                            ui.close();
                        }
                        if ui.button(self.t("Structures…", "结构体…")).clicked() {
                            *action = Action::ShowStructs;
                            ui.close();
                        }
                        ui.separator();
                        if ui.button(copy_menu).clicked() {
                            ui.ctx().copy_text(format!("{:#x}", line.addr.get()));
                            ui.close();
                        }
                        if ui.button(undo_menu).clicked() {
                            *action = Action::Undo;
                            ui.close();
                        }
                        if ui.button(redo_menu).clicked() {
                            *action = Action::Redo;
                            ui.close();
                        }
                    });
                }
            });
        if let Some(index) = clicked_row {
            self.selected = index;
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
                ui.weak(self.t("(Synchronized with Hex View-1)", "（与 Hex View-1 同步）"));
            });
        }
    }

    /// IDA-style navigation band: segment map of the image with a marker for the current
    /// disassembly position; clicking jumps proportionally.
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
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 12.0), Sense::click());
        let painter = ui.painter();
        painter.rect_filled(rect, 2.0, pal.view);
        for segment in segments {
            let x0 = rect.left()
                + ((segment.range.start.get().saturating_sub(base) as f32 / span) * rect.width());
            let x1 = rect.left()
                + ((segment.range.end.get().saturating_sub(base) as f32 / span) * rect.width())
                    .max(x0 + 1.0);
            let color = if segment.perm.is_executable() {
                Color32::from_rgb(0x33, 0x55, 0x77)
            } else if segment.perm.is_readable() {
                Color32::from_rgb(0x33, 0x44, 0x33)
            } else {
                Color32::from_rgb(0x44, 0x33, 0x33)
            };
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(x0, rect.top()),
                    egui::pos2(x1.min(rect.right()), rect.bottom()),
                ),
                0.0,
                color,
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
            Color32::from_rgb(0xe0, 0xd0, 0x80),
        );
        if response.clicked()
            && let Some(position) = response.interact_pointer_pos()
        {
            let fraction = ((position.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
            *action = Action::SyncDisasm(Address::new(base + (fraction * span as f64) as u64));
        }
    }

    fn pseudo_view(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        self.tab = Tab::Pseudo;
        let pal = self.palette();
        let mono = FontId::monospace(MONO_SIZE);
        let l1_label = self.t("L1 (structured)", "L1（结构化）");
        let l2_label = self.t("L2 (vars)", "L2（变量）");
        let mut l1 = self.pseudo_l1;
        let mut l2 = self.pseudo_l2;
        ui.horizontal(|ui| {
            if ui.checkbox(&mut l1, l1_label).changed() {
                self.pseudo_l1 = l1;
                if l1 {
                    self.pseudo_l2 = false;
                }
                self.pseudo_dirty = true;
            }
            if ui.checkbox(&mut l2, l2_label).changed() {
                self.pseudo_l2 = l2;
                if l2 {
                    self.pseudo_l1 = true;
                }
                self.pseudo_dirty = true;
            }
            ui.separator();
            ui.weak(self.t(
                "click selects/syncs · double-click jumps to disassembly",
                "单击选中并同步 · 双击跳到反汇编",
            ));
        });
        if self.pseudo_dirty {
            return;
        }
        let Some(lines) = self.pseudo.as_ref() else {
            ui.weak(self.t(
                "No pseudo-C. Press Space after opening a project.",
                "没有伪代码。打开项目后按空格。",
            ));
            return;
        };
        let synced = self.disasm_rows.get(self.selected).map(|line| line.addr);
        let mut clicked = None;
        let mut jumped = None;
        let width = ui.available_width().max(1100.0);
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                for line in lines {
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(width, ROW_HEIGHT), Sense::click());
                    if synced == Some(line.addr) {
                        ui.painter().rect_filled(rect, 1.0, pal.selected_row);
                    } else if response.hovered() {
                        ui.painter().rect_filled(rect, 1.0, pal.hover_row);
                    }
                    let mut job = LayoutJob::default();
                    append(
                        &mut job,
                        &format!("{:#012x}  ", line.addr.get()),
                        pal.addr,
                        &mono,
                    );
                    append(&mut job, &line.text, Color32::LIGHT_GRAY, &mono);
                    let galley = ui.painter().layout_job(job);
                    ui.painter().galley(
                        rect.left_top() + egui::vec2(4.0, 1.0),
                        galley,
                        Color32::WHITE,
                    );
                    if response.clicked() {
                        clicked = Some(line.addr);
                    }
                    if response.double_clicked() {
                        jumped = Some(line.addr);
                    }
                }
            });
        // Single click syncs the disassembly cursor without leaving pseudo-C; double-click
        // jumps into the disassembly tab (IDA-like select vs. follow).
        if let Some(addr) = clicked {
            *action = Action::SyncDisasm(addr);
        }
        if let Some(addr) = jumped {
            *action = Action::NavigateToDisasm(addr);
        }
    }

    // ------------------------------------------------------------- windows --

    fn edit_window(&mut self, ctx: &egui::Context, _action: &mut Action) {
        let Some(mut edit) = self.edit.take() else {
            return;
        };
        let mut commit = false;
        let mut cancel = false;
        let title = match edit.kind {
            EditKind::Rename => self.t("Rename", "改名").to_owned(),
            EditKind::Comment => self.t("Comment", "注释").to_owned(),
            EditKind::Prototype => self.t("Prototype", "原型").to_owned(),
        };
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("{:#x}", edit.addr.get()));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut edit.text)
                        .desired_width(360.0)
                        .hint_text(self.t("empty deletes", "留空则删除")),
                );
                if edit.just_opened {
                    response.request_focus();
                    edit.just_opened = false;
                }
                ui.horizontal(|ui| {
                    if ui.button(self.t("OK", "确定")).clicked() {
                        commit = true;
                    }
                    if ui.button(self.t("Cancel", "取消")).clicked() {
                        cancel = true;
                    }
                });
                if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
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
            .filter(|(_, (en, zh, _))| {
                query.is_empty() || en.to_lowercase().contains(&query) || zh.contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
        let labels: Vec<(usize, &'static str)> = filtered
            .iter()
            .map(|&index| {
                (
                    index,
                    tr(lang, PALETTE_COMMANDS[index].0, PALETTE_COMMANDS[index].1),
                )
            })
            .collect();
        state.selected = state.selected.min(labels.len().saturating_sub(1));
        let mut chosen: Option<usize> = None;
        let mut open = true;
        egui::Window::new(tr(lang, "Command palette", "命令面板"))
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .open(&mut open)
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.query)
                        .desired_width(400.0)
                        .hint_text(tr(lang, "Type a command…", "输入命令…")),
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
                        for (position, &(index, label)) in labels.iter().enumerate() {
                            if ui
                                .selectable_label(position == state.selected, label)
                                .clicked()
                            {
                                chosen = Some(index);
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
        egui::Window::new(self.t("Go to address / symbol", "跳转到地址/符号"))
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
                    if ui.button(self.t("Go", "跳转")).clicked() {
                        submit = Some(buffer.clone());
                    }
                    if ui.button(self.t("Cancel", "取消")).clicked() {
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

    // ------------------------------------------------------------- actions --

    fn handle_keys(&mut self, ctx: &egui::Context) -> Action {
        if self.palette.is_some() {
            if ctx.input(|input| input.key_pressed(Key::Escape)) {
                return Action::TogglePalette;
            }
            // The palette's text field owns the keyboard while it is open.
            return Action::None;
        }
        if self.edit.is_some() || self.goto.is_some() || ctx.egui_wants_keyboard_input() {
            return Action::None;
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
            if input.modifiers.command && input.key_pressed(Key::T) {
                action = Action::ToggleTheme;
            }
            if input.key_pressed(Key::F5) {
                action = Action::Analyze;
            }
            // IDA-like pane focus shortcuts (F2-F11; F1/F5 are taken).
            if input.key_pressed(Key::F2) {
                action = Action::FocusPane(Pane::Functions);
            } else if input.key_pressed(Key::F3) {
                action = Action::FocusPane(Pane::Names);
            } else if input.key_pressed(Key::F4) {
                action = Action::FocusPane(Pane::Strings);
            } else if input.key_pressed(Key::F6) {
                action = Action::FocusPane(Pane::Imports);
            } else if input.key_pressed(Key::F7) {
                action = Action::FocusPane(Pane::Xrefs);
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
            } else if input.key_pressed(Key::Semicolon) {
                action = Action::EditComment;
            } else if input.key_pressed(Key::Space) {
                action = Action::ToggleTab;
            } else if input.key_pressed(Key::X) {
                action = Action::ShowXrefs;
            } else if input.modifiers.command {
                // leave Ctrl/Cmd chords (copy, paste, …) to the platform.
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
            Action::Open => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("ReCraft project", &["rcraft"])
                    .pick_file()
                {
                    self.open_path(&path);
                }
            }
            Action::Import => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Windows executable", &["exe", "dll", "sys"])
                    .pick_file()
                {
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
                    self.focus_pane(Pane::Disasm);
                } else {
                    self.set_status(
                        self.t("no followable target", "没有可跟随的目标")
                            .to_owned(),
                    );
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
                self.focus_pane(Pane::Disasm);
            }
            Action::SyncDisasm(addr) => {
                self.disasm_start = addr;
                self.selected = 0;
                self.disasm_dirty = true;
                self.pseudo_start = addr;
                self.pseudo_dirty = true;
            }
            Action::ShowStructs => {
                self.focus_pane(Pane::Structs);
            }
            Action::FocusPane(pane) => {
                self.focus_pane(pane);
            }
            Action::ResetLayout => {
                self.dock = None;
                self.ensure_dock();
                self.layout_saved = None;
                if let Some(path) = layout_store_path() {
                    let _ = std::fs::remove_file(path);
                }
                self.set_status(self.t("layout reset", "布局已重置").to_owned());
            }
            Action::SetTheme(theme) => {
                self.theme = theme;
                self.layout_saved = None;
                self.set_status(match theme {
                    ThemeMode::Dark => self.t("dark theme", "深色主题").to_owned(),
                    ThemeMode::Light => self.t("light theme", "浅色主题").to_owned(),
                });
            }
            Action::ToggleTheme => {
                self.theme = match self.theme {
                    ThemeMode::Dark => ThemeMode::Light,
                    ThemeMode::Light => ThemeMode::Dark,
                };
                self.layout_saved = None;
                self.set_status(match self.theme {
                    ThemeMode::Dark => self.t("dark theme", "深色主题").to_owned(),
                    ThemeMode::Light => self.t("light theme", "浅色主题").to_owned(),
                });
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
                        self.t(
                            "put the cursor inside a function first",
                            "先把光标放到函数内",
                        )
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
                if self.analysis.is_some() {
                    return;
                }
                let Some(project) = self.project.clone() else {
                    self.set_status(self.t("no project", "无项目").to_owned());
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
                self.set_status(self.t("analyzing…", "分析中…").to_owned());
            }
            Action::CancelAnalysis => {
                if let Some(job) = self.analysis.as_ref() {
                    job.cancel.store(true, Ordering::Relaxed);
                    self.set_status(self.t("cancelling…", "正在取消…").to_owned());
                }
            }
            Action::AiSuggest => {
                let result = self
                    .session
                    .as_mut()
                    .map(|session| recraft_ai::suggest_names(session, 50));
                match result {
                    Some(Ok(proposals)) => {
                        self.ai_status = format!("{} suggestion(s)", proposals.len());
                        self.ai_proposals = proposals;
                        self.ai_selected = None;
                        self.set_status(
                            self.t(
                                "AI suggestions ready — review before applying",
                                "AI 建议已生成——应用前请 review",
                            )
                            .to_owned(),
                        );
                    }
                    Some(Err(error)) => self.set_error(error),
                    None => self.set_status(self.t("no project", "无项目").to_owned()),
                }
            }
            Action::AiApply => {
                self.apply_ai_selection();
            }
            Action::AiClear => {
                self.ai_proposals.clear();
                self.ai_selected = None;
                self.ai_status.clear();
                self.ai_explanation = None;
                self.ai_answer = None;
            }
            Action::AiQuery => {
                let text = self.ai_query.trim().to_owned();
                if text.is_empty() {
                    self.ai_status = self.t("type a question first", "先输入问题").to_owned();
                    return;
                }
                let Some(session) = self.session.as_mut() else {
                    self.set_status(self.t("open a project first", "先打开项目").to_owned());
                    return;
                };
                match recraft_ai::answer_query(session, &text) {
                    Ok(answer) => {
                        self.ai_status =
                            format!("{} · {} match(es)", answer.intent, answer.items.len());
                        self.ai_answer = Some(answer);
                    }
                    Err(error) => self.set_error(error),
                }
            }
            Action::AiExplain => {
                let Some((start, _)) = self.current_function() else {
                    self.ai_status = self
                        .t("put the cursor inside a function", "把光标放到函数内")
                        .to_owned();
                    return;
                };
                let Some(session) = self.session.as_mut() else {
                    self.set_status(self.t("open a project first", "先打开项目").to_owned());
                    return;
                };
                match recraft_ai::explain(session, start) {
                    Ok(explanation) => {
                        self.ai_status = explanation.summary.clone();
                        self.ai_explanation = Some(format_explanation(&explanation));
                    }
                    Err(error) => self.set_error(error),
                }
            }
            Action::AiPolish => {
                if self.ai_job.is_some() {
                    return;
                }
                self.ai_settings.refresh_enabled();
                if !self.ai_settings.enabled {
                    self.ai_status = self
                        .t(
                            "enable a provider above (endpoint + API key env) to use polish",
                            "先在上方启用 provider（接口地址 + API key 环境变量）再润色",
                        )
                        .to_owned();
                    return;
                }
                let Some((start, _)) = self.current_function() else {
                    self.ai_status = self
                        .t("put the cursor inside a function", "把光标放到函数内")
                        .to_owned();
                    return;
                };
                let settings = self.ai_settings.clone();
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
                let (sender, receiver) = mpsc::channel();
                std::thread::spawn(move || {
                    let mut provider = recraft_ai::provider_from_settings(&settings);
                    let result = provider
                        .complete(&prompt)
                        .map_err(|error| error.to_string());
                    let _ = sender.send(result);
                });
                self.ai_job = Some(AiJob {
                    receiver,
                    started: Instant::now(),
                });
                self.ai_status = self.t("calling provider…", "正在请求模型…").to_owned();
            }
            Action::TogglePalette => {
                self.palette = if self.palette.is_some() {
                    None
                } else {
                    Some(PaletteState::default())
                };
            }
            Action::PaletteCommand(index) => {
                self.palette = None;
                if let Some((_, _, command)) = PALETTE_COMMANDS.get(index) {
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
                        self.set_status(self.t("undo", "已撤销").to_owned());
                    }
                    Some(Ok(None)) => {
                        self.set_status(self.t("nothing to undo", "没有可撤销的操作").to_owned())
                    }
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
                        self.set_status(self.t("redo", "已重做").to_owned());
                    }
                    Some(Ok(None)) => {
                        self.set_status(self.t("nothing to redo", "没有可重做的操作").to_owned())
                    }
                    Some(Err(error)) => self.set_error(error),
                    None => {}
                }
            }
            Action::ToggleLang => {
                self.lang = self.lang.toggle();
            }
            Action::ToggleTab => {
                let pane = match self.tab {
                    Tab::Disasm => Pane::Pseudo,
                    Tab::Pseudo => Pane::Graph,
                    Tab::Graph => Pane::Disasm,
                };
                self.focus_pane(pane);
                self.tab = match pane {
                    Pane::Pseudo => Tab::Pseudo,
                    Pane::Graph => Tab::Graph,
                    _ => Tab::Disasm,
                };
            }
            Action::GotoDialog => {
                if self.session.is_some() {
                    self.goto = Some(String::new());
                }
            }
            Action::ShowXrefs => {
                self.focus_pane(Pane::Xrefs);
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

    fn shift_window(&mut self, rows: i32) {
        if self.session.is_none() {
            return;
        }
        let row_size = self
            .disasm_rows
            .first()
            .map(|line| u64::from(line.len))
            .unwrap_or(1)
            .max(1);
        let delta = (rows.unsigned_abs() as u64).saturating_mul(row_size);
        let new_start = if rows >= 0 {
            Address::new(self.disasm_start.get().saturating_add(delta))
        } else {
            Address::new(self.disasm_start.get().saturating_sub(delta))
        };
        self.disasm_start = new_start;
        self.disasm_dirty = true;
        self.selected = 0;
    }

    fn update_inner(&mut self, root: &mut egui::Ui) {
        let ctx = root.ctx().clone();
        self.install_style(&ctx);
        self.tick();
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
        self.dock_area(root, &mut action);
        self.edit_window(&ctx, &mut action);
        self.goto_window(&ctx, &mut action);
        self.palette_window(&ctx, &mut action);
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

/// Human-readable offline explanation shown in the AI panel.
fn format_explanation(explanation: &recraft_ai::Explanation) -> String {
    let name = explanation
        .name
        .clone()
        .unwrap_or_else(|| format!("sub_{:x}", explanation.start.get()));
    let mut text = format!(
        "{name}  {:#x}..{:#x}  {} insn(s)\n\n{}\n",
        explanation.start.get(),
        explanation.end.get(),
        explanation.instructions,
        explanation.summary
    );
    if !explanation.apis.is_empty() {
        text.push_str("\nAPIs:\n");
        for (addr, api, category) in &explanation.apis {
            text.push_str(&format!("  {:#x}  {api}  [{category}]\n", addr.get()));
        }
    }
    if !explanation.strings.is_empty() {
        text.push_str("\nStrings:\n");
        for (addr, value) in explanation.strings.iter().take(20) {
            text.push_str(&format!("  {:#x}  {value}\n", addr.get()));
        }
    }
    if !explanation.calls.is_empty() {
        text.push_str("\nCalls:\n");
        for (addr, callee) in &explanation.calls {
            text.push_str(&format!("  {:#x}  {callee}\n", addr.get()));
        }
    }
    text
}

fn xref_kind_name(kind: i64) -> &'static str {
    match kind {
        recraft_analysis::XREF_CALL => "call",
        recraft_analysis::XREF_JUMP => "jump",
        recraft_analysis::XREF_DATA => "data",
        _ => "other",
    }
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
        let strings = recraft_analysis::extract_strings(&mut session, 4, 200_000)?;
        recraft_analysis::persist_strings(&mut session, &strings)?;
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

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for path in [
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/simsun.ttc",
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(bytes).into());
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .push("cjk".to_owned());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

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
        assert!(
            max_frame_ms < 250.0,
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
            Pane::Xrefs,
            Pane::Names,
            Pane::Segments,
            Pane::Ai,
            Pane::Structs,
            Pane::Hex,
            Pane::Pseudo,
            Pane::Graph,
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

        // Graph tab: build and render the CFG for the entry function.
        app.tab = Tab::Graph;
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

        // L1 structured pseudo-C for the selected function.
        app.tab = Tab::Pseudo;
        app.pseudo_l1 = true;
        app.pseudo_dirty = true;
        for frame in 0..2 {
            input.time = Some(frame as f64 / 60.0);
            let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            output.textures_delta.clear();
        }
        let pseudo = app.pseudo.as_ref().expect("L1 pseudo-C was not produced");
        assert!(
            pseudo
                .iter()
                .any(|line| line.text.contains("L1 structure recovery")),
            "L1 header missing"
        );

        // L2 variant renders a real (non `_l1`) signature and argument names.
        app.pseudo_l2 = true;
        app.pseudo_dirty = true;
        for frame in 0..2 {
            input.time = Some(frame as f64 / 60.0);
            let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
            output.textures_delta.clear();
        }
        let pseudo = app.pseudo.as_ref().expect("L2 pseudo-C was not produced");
        assert!(
            pseudo.iter().any(|line| !line.text.contains("_l1")
                && line.text.ends_with('{')
                && (line.text.starts_with("void ")
                    || line.text.starts_with("int32 ")
                    || line.text.starts_with("int64 "))),
            "L2 signature missing"
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
        }];
        app.ai_selected = Some(0);
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

    #[test]
    fn x_shortcut_opens_the_xrefs_panel() {
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
        input.events.push(egui::Event::Key {
            key: egui::Key::X,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let mut second = ctx.run_ui(input, |ui| app.update_inner(ui));
        second.textures_delta.clear();
        assert!(
            app.dock
                .as_ref()
                .and_then(|state| state.find_tab(&Pane::Xrefs))
                .is_some(),
            "X should focus the xrefs pane"
        );
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
    fn ai_explanation_formats_offline_report() {
        let explanation = recraft_ai::Explanation {
            name: Some("start".to_owned()),
            start: Address::new(0x140001000),
            end: Address::new(0x140001050),
            instructions: 12,
            apis: vec![(
                Address::new(0x140001010),
                "kernel32!VirtualAlloc".to_owned(),
                "memory".to_owned(),
            )],
            strings: vec![(Address::new(0x140002000), "http://example.test".to_owned())],
            calls: vec![(Address::new(0x140001020), "sub_140001100".to_owned())],
            summary: "allocates memory then calls a helper".to_owned(),
        };
        let text = format_explanation(&explanation);
        assert!(text.contains("0x140001000"));
        assert!(text.contains("VirtualAlloc"));
        assert!(text.contains("http://example.test"));
        assert!(text.contains("sub_140001100"));
        assert!(text.contains("allocates memory"));
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
    fn ai_explain_works_offline_and_polish_asks_for_provider() {
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
            app.ai_explanation
                .as_ref()
                .is_some_and(|text| text.contains("insn")),
            "offline explanation: {:?} / status: {}",
            app.ai_explanation,
            app.ai_status
        );
        app.apply_action(&ctx, Action::AiPolish);
        assert!(
            app.ai_status.to_lowercase().contains("provider")
                || app.ai_status.to_lowercase().contains("enable"),
            "polish should ask for a provider: {}",
            app.ai_status
        );
        assert!(app.ai_job.is_none());
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
    fn offline_query_ui_answers_and_links_addresses() {
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
    fn dock_default_layout_contains_every_pane() {
        let mut app = ReCraftApp::default();
        app.ensure_dock();
        let state = app.dock.as_ref().expect("dock state");
        for pane in Pane::all() {
            assert!(
                state.find_tab(&pane).is_some(),
                "default layout is missing {pane:?}"
            );
        }
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
        assert_eq!(active, Some(Pane::Disasm));
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
        save_layout_to(&file, app.dock.as_ref().unwrap(), ThemeMode::Light).unwrap();
        let loaded = load_layout_from(&file).expect("layout reload");
        assert_eq!(loaded.theme, ThemeMode::Light, "theme must persist");
        assert!(
            loaded.dock.find_tab(&Pane::Disasm).is_some(),
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
    fn theme_toggle_switches_palette() {
        let mut app = ReCraftApp::default();
        assert_eq!(app.theme, ThemeMode::Light, "light is the default theme");
        assert_eq!(app.palette().panel, Palette::LIGHT.panel);
        assert_ne!(
            app.palette().view,
            Palette::LIGHT.panel,
            "mixed theme keeps code views dark while the chrome stays light"
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
}
