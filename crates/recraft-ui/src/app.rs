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

use recraft_common::Address;
use recraft_core::{Command, CommentKind, DisasmLine, FunctionInfo, Session};
use recraft_decompile::{PseudoLine, pseudocode_l0};

use crate::i18n::{Lang, tr};

const DISASM_ROWS: usize = 400;
const MONO_SIZE: f32 = 13.0;

const C_ADDR: Color32 = Color32::from_rgb(0x6a, 0x8f, 0xb5);
const C_BYTES: Color32 = Color32::from_rgb(0x85, 0x85, 0x85);
const C_MNEMONIC: Color32 = Color32::from_rgb(0x62, 0xa8, 0xe0);
const C_NAME: Color32 = Color32::from_rgb(0xd0, 0xa0, 0x60);
const C_COMMENT: Color32 = Color32::from_rgb(0x6a, 0xb0, 0x6a);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Disasm,
    Pseudo,
    Graph,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SideTab {
    Segments,
    Functions,
    Strings,
    Imports,
    Xrefs,
    Ai,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditKind {
    Rename,
    Comment,
}

struct EditState {
    kind: EditKind,
    addr: Address,
    text: String,
    just_opened: bool,
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
}

pub struct ReCraftApp {
    session: Option<Session>,
    project: Option<PathBuf>,
    lang: Lang,
    tab: Tab,
    side_tab: SideTab,
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
    functions_dirty: bool,
    strings: Vec<(Address, String)>,
    strings_dirty: bool,
    string_filter: String,
    imports: Vec<ImportRow>,
    imports_dirty: bool,
    xrefs: Vec<(Address, &'static str)>,
    xrefs_for: Option<Address>,
    xrefs_dirty: bool,
    cfg: Option<recraft_analysis::Cfg>,
    cfg_for: Option<Address>,
    cfg_dirty: bool,
    analysis: Option<AnalysisJob>,
    /// AI rename suggestions (never applied implicitly).
    ai_proposals: Vec<recraft_ai::Proposal>,
    ai_selected: AiSelection,
    ai_status: String,
    edit: Option<EditState>,
    goto: Option<String>,
    palette: Option<PaletteState>,
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
            side_tab: SideTab::Segments,
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
            functions_dirty: false,
            strings: Vec::new(),
            strings_dirty: false,
            string_filter: String::new(),
            imports: Vec::new(),
            imports_dirty: false,
            xrefs: Vec::new(),
            xrefs_for: None,
            xrefs_dirty: false,
            cfg: None,
            cfg_for: None,
            cfg_dirty: false,
            analysis: None,
            ai_proposals: Vec::new(),
            ai_selected: None,
            ai_status: String::new(),
            edit: None,
            goto: None,
            palette: None,
            history: Vec::new(),
            history_pos: 0,
        }
    }
}

impl ReCraftApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Option<PathBuf>) -> Self {
        install_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut app = Self::default();
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
        self.status = message.into();
        self.is_error = false;
    }

    fn set_error(&mut self, error: impl std::fmt::Display) {
        self.status = format!("error: {error}");
        self.is_error = true;
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
                    self.after_open();
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
                let message = format!("imported {} bytes -> {}", bytes.len(), project.display());
                self.after_open();
                self.set_status(message);
            }
            Err(error) => self.set_error(error),
        }
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

    fn tick(&mut self) {
        self.poll_analysis();
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
            self.functions_dirty = false;
            return;
        };
        match session.functions() {
            Ok(functions) => {
                self.functions = functions;
                self.functions_dirty = false;
            }
            Err(error) => {
                self.functions.clear();
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

    fn top_bar(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let file_label = self.t("File", "文件");
        let open_label = self.t("Open project…", "打开项目…");
        let import_label = self.t("Import PE…", "导入 PE…");
        let undo_label = self.t("Undo", "撤销");
        let redo_label = self.t("Redo", "重做");
        let no_project = self.t("<no project>", "<未打开项目>");
        let analyze_label = self.t("Analyze functions", "分析函数");
        let cancel_label = self.t("Cancel", "取消");
        let analyzing_label = self.t("analyzing", "分析中");
        egui::Panel::top("recraft_top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button(file_label, |ui| {
                    if ui.button(open_label).clicked() {
                        *action = Action::Open;
                        ui.close();
                    }
                    if ui.button(import_label).clicked() {
                        *action = Action::Import;
                        ui.close();
                    }
                });
                if ui.button(undo_label).clicked() {
                    *action = Action::Undo;
                }
                if ui.button(redo_label).clicked() {
                    *action = Action::Redo;
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
                ui.separator();
                let path_text = self
                    .project
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| no_project.to_owned());
                ui.label(path_text);
            });
        });
    }

    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("recraft_bottom").show(ui, |ui| {
            ui.horizontal(|ui| {
                let color = if self.is_error {
                    Color32::from_rgb(0xe0, 0x6a, 0x6a)
                } else {
                    ui.visuals().weak_text_color()
                };
                ui.colored_label(color, &self.status);
                ui.separator();
                if let Some(line) = self.disasm_rows.get(self.selected) {
                    ui.label(format!("{:#x}", line.addr.get()));
                }
                ui.separator();
                ui.weak(self.t(
                    "Ctrl+P palette · G goto · N rename · ; comment · X xrefs · Space pseudo · Ctrl+Z undo · Esc back",
                    "Ctrl+P 命令面板 · G 跳转 · N 改名 · ; 注释 · X 交叉引用 · 空格伪代码 · Ctrl+Z 撤销 · Esc 后退",
                ));
            });
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let segments_label = self.t("Segments", "段");
        let functions_label = self.t("Functions", "函数");
        let strings_label = self.t("Strings", "字符串");
        let imports_label = self.t("Imports", "导入");
        let xrefs_label = self.t("Xrefs", "引用");
        let ai_label = self.t("AI", "AI");
        egui::Panel::left("recraft_side")
            .default_size(280.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.side_tab, SideTab::Segments, segments_label);
                    ui.selectable_value(&mut self.side_tab, SideTab::Functions, functions_label);
                    ui.selectable_value(&mut self.side_tab, SideTab::Strings, strings_label);
                    ui.selectable_value(&mut self.side_tab, SideTab::Imports, imports_label);
                    ui.selectable_value(&mut self.side_tab, SideTab::Xrefs, xrefs_label);
                    ui.selectable_value(&mut self.side_tab, SideTab::Ai, ai_label);
                });
                ui.separator();
                if self.side_tab == SideTab::Strings {
                    let hint = self.t("substring…", "子串…");
                    ui.horizontal(|ui| {
                        ui.weak(self.t("filter", "过滤"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.string_filter)
                                .desired_width(f32::INFINITY)
                                .hint_text(hint),
                        );
                    });
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.side_tab {
                        SideTab::Segments => {
                            let Some(session) = self.session.as_ref() else {
                                ui.weak(self.t("no project", "无项目"));
                                return;
                            };
                            for segment in session.segments() {
                                let label = format!(
                                    "{:<8} {:#012x} .. {:#012x}",
                                    segment.name,
                                    segment.range.start.get(),
                                    segment.range.end.get()
                                );
                                if ui.selectable_label(false, label).clicked() {
                                    *action = Action::Navigate(segment.range.start);
                                }
                            }
                        }
                        SideTab::Functions => {
                            if self.functions.is_empty() {
                                ui.weak(self.t(
                                    "No functions yet — press Analyze (F5).",
                                    "还没有函数——按 Analyze（F5）。",
                                ));
                            }
                            for function in &self.functions {
                                let label = format!(
                                    "{:#012x} .. {:#012x}",
                                    function.start.get(),
                                    function.end.get()
                                );
                                if ui.selectable_label(false, label).clicked() {
                                    *action = Action::Navigate(function.start);
                                }
                            }
                        }
                        SideTab::Strings => {
                            if self.strings.is_empty() {
                                ui.weak(self.t(
                                    "No strings yet — run Analyze (F5).",
                                    "还没有字符串——运行 Analyze（F5）。",
                                ));
                            }
                            let filter = self.string_filter.to_lowercase();
                            let mut shown = 0usize;
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
                                if ui.selectable_label(false, label).clicked() {
                                    *action = Action::Navigate(*addr);
                                }
                            }
                        }
                        SideTab::Imports => {
                            if self.imports.is_empty() {
                                ui.weak(self.t(
                                    "No imports (or open a project first).",
                                    "没有导入（或先打开项目）。",
                                ));
                            }
                            for import in &self.imports {
                                let label = format!(
                                    "{}!{}  [{}]  ×{}",
                                    import.module, import.api, import.category, import.refs
                                );
                                if ui.selectable_label(false, label).clicked() {
                                    *action =
                                        Action::Navigate(import.first_from.unwrap_or(import.iat));
                                }
                            }
                        }
                        SideTab::Xrefs => {
                            let Some(addr) = self.disasm_rows.get(self.selected).map(|l| l.addr)
                            else {
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
                            for (from, kind) in &self.xrefs {
                                let label = format!("<- {:#012x}  {kind}", from.get());
                                if ui.selectable_label(false, label).clicked() {
                                    *action = Action::Navigate(*from);
                                }
                            }
                        }
                        SideTab::Ai => self.ai_panel(ui, action),
                    });
            });
    }

    fn ai_panel(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let suggest = self.t("Suggest names", "建议命名");
        let apply_label = self.t("Apply selected", "应用选中");
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
            if ui.button(clear).clicked() {
                *action = Action::AiClear;
            }
        });
        if !self.ai_status.is_empty() {
            ui.weak(&self.ai_status);
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

    fn graph_view(&mut self, ui: &mut egui::Ui, action: &mut Action) {
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
        let layers = recraft_analysis::layers(&cfg);
        const NODE_W: f32 = 250.0;
        const GAP_X: f32 = 110.0;
        const GAP_Y: f32 = 22.0;
        let column_count = layers.iter().copied().max().unwrap_or(0) + 1;
        let mut column_y = vec![20.0f32; column_count];
        let mut local: Vec<egui::Rect> = Vec::with_capacity(cfg.blocks.len());
        for (index, block) in cfg.blocks.iter().enumerate() {
            let layer = layers[index];
            let height = 26.0 + block.insns.len().min(8) as f32 * 14.0;
            let x = 20.0 + layer as f32 * (NODE_W + GAP_X);
            let y = column_y[layer];
            column_y[layer] = y + height + GAP_Y;
            local.push(egui::Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2(NODE_W, height),
            ));
        }
        let canvas_w = 20.0 + column_count as f32 * (NODE_W + GAP_X);
        let canvas_h = column_y.iter().copied().fold(20.0f32, f32::max) + 20.0;

        ui.horizontal(|ui| {
            ui.weak(self.t("edges:", "边:"));
            for (label, color) in [
                ("fallthrough", Color32::GRAY),
                ("taken", Color32::from_rgb(0x6a, 0xb0, 0x6a)),
                ("not taken", Color32::from_rgb(0xd0, 0x7a, 0x7a)),
                ("jump", Color32::from_rgb(0x6a, 0x8f, 0xb5)),
                ("jump table", Color32::from_rgb(0xb0, 0x8a, 0xd0)),
            ] {
                ui.colored_label(color, label);
            }
        });
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
                for block in &cfg.blocks {
                    for &(target, kind) in &block.edges {
                        let Some(target_rect) = local.get(target) else {
                            continue;
                        };
                        let from = local[block.index].right_center() + origin;
                        let to = target_rect.left_center() + origin;
                        let color = match kind {
                            recraft_analysis::EdgeKind::Fallthrough => Color32::GRAY,
                            recraft_analysis::EdgeKind::Taken => {
                                Color32::from_rgb(0x6a, 0xb0, 0x6a)
                            }
                            recraft_analysis::EdgeKind::NotTaken => {
                                Color32::from_rgb(0xd0, 0x7a, 0x7a)
                            }
                            recraft_analysis::EdgeKind::Unconditional => {
                                Color32::from_rgb(0x6a, 0x8f, 0xb5)
                            }
                            recraft_analysis::EdgeKind::JumpTable => {
                                Color32::from_rgb(0xb0, 0x8a, 0xd0)
                            }
                        };
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
                            egui::Stroke::new(1.4, color),
                        ));
                        painter.circle_filled(to, 3.0, color);
                    }
                }
                for (index, block) in cfg.blocks.iter().enumerate() {
                    let rect = local[index].translate(origin);
                    let is_entry = index == cfg.entry;
                    painter.rect_filled(rect, 6.0, Color32::from_rgb(0x22, 0x27, 0x30));
                    painter.rect_stroke(
                        rect,
                        6.0,
                        egui::Stroke::new(
                            1.0,
                            if is_entry {
                                Color32::from_rgb(0xd0, 0xa0, 0x60)
                            } else {
                                Color32::from_rgb(0x3a, 0x42, 0x50)
                            },
                        ),
                        egui::StrokeKind::Inside,
                    );
                    painter.text(
                        rect.min + egui::vec2(8.0, 4.0),
                        egui::Align2::LEFT_TOP,
                        format!("{:#x}", block.start.get()),
                        FontId::monospace(11.0),
                        C_ADDR,
                    );
                    for (line_index, insn) in block.insns.iter().take(8).enumerate() {
                        painter.text(
                            rect.min + egui::vec2(8.0, 20.0 + line_index as f32 * 14.0),
                            egui::Align2::LEFT_TOP,
                            truncate_chars(&insn.text, 44),
                            FontId::monospace(11.0),
                            Color32::from_gray(0xcf),
                        );
                    }
                    let response = ui.interact(rect, ui.id().with(("cfg", index)), Sense::click());
                    if response.clicked() {
                        *action = Action::Navigate(block.start);
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
    }

    fn central(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let disasm_label = self.t("Disassembly", "反汇编");
        let pseudo_label = self.t("Pseudo-C (L0)", "伪代码 (L0)");
        let graph_label = self.t("Graph", "控制流图");
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Disasm, disasm_label);
                ui.selectable_value(&mut self.tab, Tab::Pseudo, pseudo_label);
                ui.selectable_value(&mut self.tab, Tab::Graph, graph_label);
                ui.separator();
                ui.weak(if let Some(session) = self.session.as_ref() {
                    format!(
                        "{} x{} · entry {:#x}",
                        if session.mode().is_64() {
                            "PE32+"
                        } else {
                            "PE32"
                        },
                        if session.mode().is_64() { 64 } else { 32 },
                        session.entry_point().get()
                    )
                } else {
                    String::new()
                });
            });
            ui.separator();
            match self.tab {
                Tab::Disasm => self.disasm_view(ui, action),
                Tab::Pseudo => self.pseudo_view(ui, action),
                Tab::Graph => self.graph_view(ui, action),
            }
        });
    }

    fn disasm_view(&mut self, ui: &mut egui::Ui, _action: &mut Action) {
        if self.disasm_rows.is_empty() {
            ui.weak(self.t(
                "No disassembly. Import a PE file first.",
                "没有反汇编。先导入 PE 文件。",
            ));
            return;
        }
        let mut clicked_row = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("disasm_grid")
                    .num_columns(6)
                    .striped(true)
                    .spacing([10.0, 2.0])
                    .show(ui, |ui| {
                        for header in [
                            self.t("Address", "地址"),
                            self.t("Bytes", "字节"),
                            self.t("Mnemonic", "助记符"),
                            self.t("Operands", "操作数"),
                            self.t("Name", "名字"),
                            self.t("Comment", "注释"),
                        ] {
                            ui.label(RichText::new(header).monospace().strong());
                        }
                        ui.end_row();

                        for (index, line) in self.disasm_rows.iter().enumerate() {
                            let address = RichText::new(format!("{:#012x}", line.addr.get()))
                                .monospace()
                                .color(C_ADDR);
                            let response = ui.selectable_label(index == self.selected, address);
                            if response.clicked() {
                                clicked_row = Some(index);
                            }
                            let bytes = line
                                .bytes()
                                .iter()
                                .map(|byte| format!("{byte:02x} "))
                                .collect::<String>();
                            ui.label(RichText::new(bytes).monospace().color(C_BYTES));
                            ui.label(RichText::new(&line.mnemonic).monospace().color(C_MNEMONIC));
                            ui.label(RichText::new(&line.operands).monospace());
                            ui.label(
                                RichText::new(line.name.as_deref().unwrap_or(""))
                                    .monospace()
                                    .color(C_NAME),
                            );
                            ui.label(
                                RichText::new(line.comment.as_deref().unwrap_or(""))
                                    .monospace()
                                    .color(C_COMMENT),
                            );
                            ui.end_row();
                        }
                    });
            });
        if let Some(index) = clicked_row {
            self.selected = index;
        }
    }

    fn pseudo_view(&mut self, ui: &mut egui::Ui, action: &mut Action) {
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
        let mut clicked = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                for line in lines {
                    let mut job = LayoutJob::default();
                    append(
                        &mut job,
                        &format!("{:#012x}  ", line.addr.get()),
                        C_ADDR,
                        &mono,
                    );
                    append(&mut job, &line.text, Color32::LIGHT_GRAY, &mono);
                    let response = ui.add(egui::Label::new(job).sense(Sense::click()));
                    if response.clicked() {
                        clicked = Some(line.addr);
                    }
                }
            });
        if let Some(addr) = clicked {
            *action = Action::Navigate(addr);
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
            if input.key_pressed(Key::F5) {
                action = Action::Analyze;
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
                self.tab = match self.tab {
                    Tab::Disasm => Tab::Pseudo,
                    Tab::Pseudo => Tab::Graph,
                    Tab::Graph => Tab::Disasm,
                };
            }
            Action::GotoDialog => {
                if self.session.is_some() {
                    self.goto = Some(String::new());
                }
            }
            Action::ShowXrefs => {
                self.side_tab = SideTab::Xrefs;
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
        self.tick();
        let mut action = self.handle_keys(&ctx);
        self.top_bar(root, &mut action);
        self.bottom_bar(root);
        self.side_panel(root, &mut action);
        self.central(root, &mut action);
        self.edit_window(&ctx, &mut action);
        self.goto_window(&ctx, &mut action);
        self.palette_window(&ctx, &mut action);
        self.apply_action(&ctx, action);
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

fn xref_kind_name(kind: i64) -> &'static str {
    match kind {
        recraft_analysis::XREF_CALL => "call",
        recraft_analysis::XREF_JUMP => "jump",
        recraft_analysis::XREF_DATA => "data",
        _ => "other",
    }
}

fn truncate_chars(text: &str, max: usize) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    if chars.len() > max {
        chars.truncate(max);
        chars.push('…');
    }
    chars.into_iter().collect()
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
            max_frame_ms < 100.0,
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

        for tab in [
            SideTab::Functions,
            SideTab::Strings,
            SideTab::Imports,
            SideTab::Xrefs,
            SideTab::Segments,
        ] {
            app.side_tab = tab;
            for frame in 0..2 {
                input.time = Some(frame as f64 / 60.0);
                let mut output = ctx.run_ui(input.clone(), |ui| app.update_inner(ui));
                output.textures_delta.clear();
            }
            assert!(
                !app.disasm_rows.is_empty(),
                "disassembly empty while rendering {tab:?}"
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
        app.side_tab = SideTab::Ai;
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
        assert_eq!(
            app.side_tab,
            SideTab::Xrefs,
            "X should open the xrefs panel"
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
