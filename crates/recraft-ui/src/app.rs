// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! ReCraft desktop application (M0 shell).
//!
//! Deliberately small: one session, three panes (navigation, disassembly/pseudo-C,
//! status), keyboard-first commands. All state changes go through `recraft-core`
//! commands, so undo/redo behave exactly like the CLI.

use std::path::{Path, PathBuf};

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Key, RichText, Sense};

use recraft_analysis::{analyze, persist};
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
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SideTab {
    Segments,
    Functions,
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

enum Action {
    None,
    Open,
    Import,
    Analyze,
    Undo,
    Redo,
    ToggleLang,
    ToggleTab,
    GotoDialog,
    EditName,
    EditComment,
    MoveSelection(i32),
    Page(i32),
    Back,
    Navigate(Address),
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
    functions: Vec<FunctionInfo>,
    functions_dirty: bool,
    edit: Option<EditState>,
    goto: Option<String>,
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
            functions: Vec::new(),
            functions_dirty: false,
            edit: None,
            goto: None,
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
        if self.disasm_dirty {
            self.refresh_disasm();
        }
        if self.pseudo_dirty {
            self.refresh_pseudo();
        }
        if self.functions_dirty {
            self.refresh_functions();
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
        match pseudocode_l0(session, self.pseudo_start, 400) {
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

    // ---------------------------------------------------------------- draw --

    fn top_bar(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let file_label = self.t("File", "文件");
        let open_label = self.t("Open project…", "打开项目…");
        let import_label = self.t("Import PE…", "导入 PE…");
        let undo_label = self.t("Undo", "撤销");
        let redo_label = self.t("Redo", "重做");
        let no_project = self.t("<no project>", "<未打开项目>");
        let analyze_label = self.t("Analyze functions", "分析函数");
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
                if ui.button(analyze_label).clicked() {
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
                    "G goto · N rename · ; comment · Space pseudo · Ctrl+Z undo · Esc back",
                    "G 跳转 · N 改名 · ; 注释 · 空格切换伪代码 · Ctrl+Z 撤销 · Esc 后退",
                ));
            });
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let segments_label = self.t("Segments", "段");
        let functions_label = self.t("Functions", "函数");
        egui::Panel::left("recraft_side")
            .default_size(250.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.side_tab, SideTab::Segments, segments_label);
                    ui.selectable_value(&mut self.side_tab, SideTab::Functions, functions_label);
                });
                ui.separator();
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
                                    "No functions yet — automatic analysis lands in M1.",
                                    "还没有函数——自动分析在 M1 落地。",
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
                    });
            });
    }

    fn central(&mut self, ui: &mut egui::Ui, action: &mut Action) {
        let disasm_label = self.t("Disassembly", "反汇编");
        let pseudo_label = self.t("Pseudo-C (L0)", "伪代码 (L0)");
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Disasm, disasm_label);
                ui.selectable_value(&mut self.tab, Tab::Pseudo, pseudo_label);
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
        if self.edit.is_some() || self.goto.is_some() || ctx.egui_wants_keyboard_input() {
            return Action::None;
        }
        let mut action = Action::None;
        ctx.input(|input| {
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
                let Some(session) = self.session.as_mut() else {
                    return;
                };
                let started = std::time::Instant::now();
                let outcome = analyze(session).and_then(|result| {
                    persist(session, &result)?;
                    let strings = recraft_analysis::extract_strings(session, 4, 200_000)?;
                    recraft_analysis::persist_strings(session, &strings)?;
                    Ok((result, strings.len()))
                });
                let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
                match outcome {
                    Ok((result, string_count)) => {
                        self.functions_dirty = true;
                        self.disasm_dirty = true;
                        self.pseudo_dirty = true;
                        self.set_status(format!(
                            "{} functions, {} strings in {elapsed_ms:.0} ms",
                            result.functions.len(),
                            string_count
                        ));
                    }
                    Err(error) => self.set_error(error),
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
                    Tab::Pseudo => Tab::Disasm,
                };
            }
            Action::GotoDialog => {
                if self.session.is_some() {
                    self.goto = Some(String::new());
                }
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
}
