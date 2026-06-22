use crate::format::{
    Color, FormatError, MAX_AREA_PER_FRAME, MAX_HEIGHT, MAX_WIDTH, PaintedCell, Project,
    TerminalStyle, TextAttr, export_plain_text, is_valid_v1_character, load_project_from_path,
    save_project_to_path,
};
use anyhow::{Context, Result, anyhow};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color as TuiColor, Modifier, Style as TuiStyle};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_NEW_WIDTH: u16 = 48;
const DEFAULT_NEW_HEIGHT: u16 = 16;

pub fn run_interactive(initial: Startup) -> Result<()> {
    let mut terminal = TerminalGuard::enter()?;
    let mut app = AppState::from_startup(initial);

    loop {
        terminal.terminal.draw(|frame| draw(frame, &mut app))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(40))? {
            match event::read()? {
                Event::Key(key) => app.handle_key(key),
                Event::Mouse(mouse) => app.handle_mouse(mouse),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    Ok(())
}

pub enum Startup {
    Welcome,
    New {
        width: u16,
        height: u16,
        path: PathBuf,
    },
    Open(PathBuf),
    CreateAt(PathBuf),
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Welcome,
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Pencil,
    Eraser,
    Eyedropper,
}

#[derive(Debug, Clone)]
enum Modal {
    NewImage {
        input: String,
        target_path: Option<PathBuf>,
    },
    OpenFile {
        input: String,
    },
    SaveAs {
        input: String,
    },
    BrushChar {
        input: String,
    },
    NewStyle {
        input: String,
    },
    RenameStyle {
        input: String,
    },
    SetFg {
        input: String,
    },
    SetBg {
        input: String,
    },
    ExportText {
        input: String,
    },
    QuitConfirm,
}

#[derive(Debug, Clone, Default)]
struct Stroke {
    changes: BTreeMap<(u16, u16), CellChange>,
}

#[derive(Debug, Clone)]
struct CellChange {
    before: Option<PaintedCell>,
    after: Option<PaintedCell>,
}

struct AppState {
    screen: Screen,
    project: Project,
    file_path: Option<PathBuf>,
    dirty: bool,
    tool: Tool,
    brush_char: char,
    current_style: usize,
    canvas_area: Rect,
    undo_stack: Vec<Stroke>,
    redo_stack: Vec<Stroke>,
    active_stroke: Option<Stroke>,
    modal: Option<Modal>,
    message: String,
    should_quit: bool,
}

impl AppState {
    fn from_startup(startup: Startup) -> Self {
        match startup {
            Startup::Welcome => Self::welcome("N new image, O open file, Q quit"),
            Startup::New {
                width,
                height,
                path,
            } => {
                let name = asset_name_from_path(&path);
                let project = Project::new_image(name, width, height);
                Self::editor(
                    project,
                    Some(path),
                    true,
                    "New image ready. Ctrl-S saves it.",
                )
            }
            Startup::Open(path) => match load_project_from_path(&path) {
                Ok(loaded) => {
                    let warning_count = loaded.warnings.len();
                    let message = if warning_count == 0 {
                        format!("Opened {}", path.display())
                    } else {
                        format!("Opened {} with {warning_count} warning(s)", path.display())
                    };
                    Self::editor(loaded.project, Some(path), false, message)
                }
                Err(error) => Self::welcome(format!("Could not open {}: {error}", path.display())),
            },
            Startup::CreateAt(path) => {
                let mut app = Self::welcome(format!(
                    "{} does not exist. Enter dimensions like 48x16 to create it.",
                    path.display()
                ));
                app.modal = Some(Modal::NewImage {
                    input: format!("{DEFAULT_NEW_WIDTH}x{DEFAULT_NEW_HEIGHT}"),
                    target_path: Some(path),
                });
                app
            }
        }
    }

    fn welcome(message: impl Into<String>) -> Self {
        Self {
            screen: Screen::Welcome,
            project: Project::new_image("untitled", DEFAULT_NEW_WIDTH, DEFAULT_NEW_HEIGHT),
            file_path: None,
            dirty: false,
            tool: Tool::Pencil,
            brush_char: '#',
            current_style: 0,
            canvas_area: Rect::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            active_stroke: None,
            modal: None,
            message: message.into(),
            should_quit: false,
        }
    }

    fn editor(
        project: Project,
        file_path: Option<PathBuf>,
        dirty: bool,
        message: impl Into<String>,
    ) -> Self {
        Self {
            screen: Screen::Editor,
            project,
            file_path,
            dirty,
            tool: Tool::Pencil,
            brush_char: '#',
            current_style: 0,
            canvas_area: Rect::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            active_stroke: None,
            modal: None,
            message: message.into(),
            should_quit: false,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }

        if self.modal.is_some() {
            self.handle_modal_key(key);
            return;
        }

        match self.screen {
            Screen::Welcome => self.handle_welcome_key(key),
            Screen::Editor => self.handle_editor_key(key),
        }
    }

    fn handle_welcome_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.modal = Some(Modal::NewImage {
                    input: format!("{DEFAULT_NEW_WIDTH}x{DEFAULT_NEW_HEIGHT}"),
                    target_path: None,
                });
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                self.modal = Some(Modal::OpenFile {
                    input: String::new(),
                });
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn handle_editor_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        if ctrl {
            match key.code {
                KeyCode::Char('s') => {
                    self.save();
                    return;
                }
                KeyCode::Char('S') => {
                    self.open_save_as();
                    return;
                }
                KeyCode::Char('z') => {
                    self.undo();
                    return;
                }
                KeyCode::Char('y') => {
                    self.redo();
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.tool = Tool::Pencil;
                self.message = "Pencil selected".to_string();
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.tool = Tool::Eraser;
                self.message = "Eraser selected".to_string();
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                self.tool = Tool::Eyedropper;
                self.message = "Eyedropper selected".to_string();
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.modal = Some(Modal::BrushChar {
                    input: self.brush_char.to_string(),
                });
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.modal = Some(Modal::NewStyle {
                    input: next_style_id(&self.project),
                });
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                let id = self.project.styles[self.current_style].id.clone();
                self.modal = Some(Modal::RenameStyle { input: id });
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                let fg = self.project.styles[self.current_style].fg.to_hex();
                self.modal = Some(Modal::SetFg { input: fg });
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                let bg = self.project.styles[self.current_style]
                    .bg
                    .map(Color::to_hex)
                    .unwrap_or_else(|| "transparent".to_string());
                self.modal = Some(Modal::SetBg { input: bg });
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                let default = self
                    .file_path
                    .as_ref()
                    .map(|path| default_text_export_path(path.as_path()))
                    .unwrap_or_else(|| PathBuf::from("terminal-art.txt"));
                self.modal = Some(Modal::ExportText {
                    input: default.display().to_string(),
                });
            }
            KeyCode::Char('[') => self.previous_style(),
            KeyCode::Char(']') | KeyCode::Tab => self.next_style(),
            KeyCode::Char('q') | KeyCode::Char('Q') => self.request_quit(),
            KeyCode::Esc => self.request_quit(),
            _ => {}
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent) {
        if matches!(self.modal, Some(Modal::QuitConfirm)) {
            match key.code {
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.save();
                    if !self.dirty {
                        self.should_quit = true;
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.should_quit = true;
                }
                KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Esc => {
                    self.modal = None;
                    self.message = "Quit canceled".to_string();
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.modal = None;
                self.message = "Canceled".to_string();
            }
            KeyCode::Enter => {
                if let Some(modal) = self.modal.take() {
                    self.commit_modal(modal);
                }
            }
            KeyCode::Backspace => {
                if let Some(input) = modal_input_mut(&mut self.modal) {
                    input.pop();
                }
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(input) = modal_input_mut(&mut self.modal) {
                    input.push(ch);
                }
            }
            KeyCode::Char(_) => {}
            _ => {}
        }
    }

    fn commit_modal(&mut self, modal: Modal) {
        match modal {
            Modal::NewImage { input, target_path } => match parse_dimensions(&input) {
                Some((width, height)) => {
                    let name = target_path
                        .as_deref()
                        .map(asset_name_from_path)
                        .unwrap_or_else(|| "untitled".to_string());
                    let project = Project::new_image(name, width, height);
                    *self = Self::editor(
                        project,
                        target_path,
                        true,
                        format!("Created {width}x{height} image"),
                    );
                }
                None => {
                    self.message = "Enter dimensions as WIDTHxHEIGHT, like 48x16".to_string();
                    self.modal = Some(Modal::NewImage { input, target_path });
                }
            },
            Modal::OpenFile { input } => {
                let path = PathBuf::from(input.trim());
                if path.as_os_str().is_empty() {
                    self.message = "Path cannot be empty".to_string();
                    self.modal = Some(Modal::OpenFile {
                        input: String::new(),
                    });
                    return;
                }

                if path.exists() {
                    match load_project_from_path(&path) {
                        Ok(loaded) => {
                            let warning_count = loaded.warnings.len();
                            *self = Self::editor(
                                loaded.project,
                                Some(path),
                                false,
                                format!("Opened file with {warning_count} warning(s)"),
                            );
                        }
                        Err(error) => {
                            self.message = format!("Open failed: {error}");
                        }
                    }
                } else {
                    self.modal = Some(Modal::NewImage {
                        input: format!("{DEFAULT_NEW_WIDTH}x{DEFAULT_NEW_HEIGHT}"),
                        target_path: Some(path),
                    });
                    self.message =
                        "File does not exist. Enter dimensions to create it.".to_string();
                }
            }
            Modal::SaveAs { input } => {
                let path = PathBuf::from(input.trim());
                if path.as_os_str().is_empty() {
                    self.message = "Save path cannot be empty".to_string();
                    self.open_save_as();
                    return;
                }

                if path.exists() {
                    self.message = format!("Overwriting {}", path.display());
                }

                self.file_path = Some(path);
                self.save();
            }
            Modal::BrushChar { input } => {
                let mut chars = input.chars();
                match (chars.next(), chars.next()) {
                    (Some(ch), None) if is_valid_v1_character(ch) => {
                        self.brush_char = ch;
                        self.message = format!("Brush character set to {ch:?}");
                    }
                    _ => {
                        self.message = "Brush character must be one valid V1 character".to_string();
                        self.modal = Some(Modal::BrushChar { input });
                    }
                }
            }
            Modal::NewStyle { input } => {
                let id = input.trim();
                if id.is_empty() || self.project.styles.iter().any(|style| style.id == id) {
                    self.message = "Style ID must be non-empty and unique".to_string();
                    self.modal = Some(Modal::NewStyle { input });
                    return;
                }

                let mut style = self.project.styles[self.current_style].clone();
                style.id = id.to_string();
                style.role = None;
                self.project.styles.push(style);
                self.current_style = self.project.styles.len() - 1;
                self.dirty = true;
                self.message = format!("Created style {id:?}");
            }
            Modal::RenameStyle { input } => {
                let id = input.trim();
                let duplicate = self
                    .project
                    .styles
                    .iter()
                    .enumerate()
                    .any(|(index, style)| index != self.current_style && style.id == id);

                if id.is_empty() || duplicate {
                    self.message = "Style ID must be non-empty and unique".to_string();
                    self.modal = Some(Modal::RenameStyle { input });
                    return;
                }

                self.project.styles[self.current_style].id = id.to_string();
                self.dirty = true;
                self.message = format!("Renamed style to {id:?}");
            }
            Modal::SetFg { input } => match Color::parse_hex(input.trim()) {
                Some(color) => {
                    self.project.styles[self.current_style].fg = color;
                    self.dirty = true;
                    self.message = format!("Foreground set to {}", color.to_hex());
                }
                None => {
                    self.message = "Foreground must be #RRGGBB".to_string();
                    self.modal = Some(Modal::SetFg { input });
                }
            },
            Modal::SetBg { input } => {
                let value = input.trim();
                if value == "transparent" {
                    self.project.styles[self.current_style].bg = None;
                    self.dirty = true;
                    self.message = "Background set to transparent".to_string();
                } else if let Some(color) = Color::parse_hex(value) {
                    self.project.styles[self.current_style].bg = Some(color);
                    self.dirty = true;
                    self.message = format!("Background set to {}", color.to_hex());
                } else {
                    self.message = "Background must be #RRGGBB or transparent".to_string();
                    self.modal = Some(Modal::SetBg { input });
                }
            }
            Modal::ExportText { input } => {
                let path = PathBuf::from(input.trim());
                if path.as_os_str().is_empty() {
                    self.message = "Export path cannot be empty".to_string();
                    self.modal = Some(Modal::ExportText { input });
                    return;
                }

                match fs::write(&path, export_plain_text(&self.project, 0)) {
                    Ok(()) => self.message = format!("Exported {}", path.display()),
                    Err(error) => self.message = format!("Export failed: {error}"),
                }
            }
            Modal::QuitConfirm => {}
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.screen != Screen::Editor || self.modal.is_some() {
            return;
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.active_stroke = Some(Stroke::default());
                self.apply_tool_at_screen(mouse.column, mouse.row);
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.active_stroke.is_none() {
                    self.active_stroke = Some(Stroke::default());
                }
                self.apply_tool_at_screen(mouse.column, mouse.row);
            }
            MouseEventKind::Up(MouseButton::Left) => self.finish_stroke(),
            _ => {}
        }
    }

    fn apply_tool_at_screen(&mut self, column: u16, row: u16) {
        let Some((x, y)) = self.canvas_cell_at(column, row) else {
            return;
        };

        match self.tool {
            Tool::Pencil => {
                let after = Some(PaintedCell {
                    ch: self.brush_char,
                    style_index: self.current_style,
                });
                self.set_cell_for_stroke(x, y, after);
            }
            Tool::Eraser => self.set_cell_for_stroke(x, y, None),
            Tool::Eyedropper => {
                if let Some(cell) = self.project.first_frame().cells.get(&(y, x)) {
                    self.brush_char = cell.ch;
                    self.current_style = cell.style_index;
                    self.message = format!("Picked {ch:?}", ch = cell.ch);
                } else {
                    self.message = "Cell is transparent".to_string();
                }
            }
        }
    }

    fn set_cell_for_stroke(&mut self, x: u16, y: u16, after: Option<PaintedCell>) {
        let before = self.project.first_frame().cells.get(&(y, x)).cloned();
        if before == after {
            return;
        }

        if let Some(stroke) = self.active_stroke.as_mut() {
            stroke
                .changes
                .entry((y, x))
                .and_modify(|change| change.after = after.clone())
                .or_insert(CellChange {
                    before,
                    after: after.clone(),
                });
        }

        match after {
            Some(cell) => {
                self.project.first_frame_mut().cells.insert((y, x), cell);
            }
            None => {
                self.project.first_frame_mut().cells.remove(&(y, x));
            }
        }

        self.dirty = true;
        self.redo_stack.clear();
    }

    fn finish_stroke(&mut self) {
        let Some(mut stroke) = self.active_stroke.take() else {
            return;
        };

        stroke
            .changes
            .retain(|_, change| change.before != change.after);

        if !stroke.changes.is_empty() {
            self.undo_stack.push(stroke);
            self.message = "Stroke applied".to_string();
        }
    }

    fn undo(&mut self) {
        let Some(stroke) = self.undo_stack.pop() else {
            self.message = "Nothing to undo".to_string();
            return;
        };

        for (&(y, x), change) in &stroke.changes {
            match &change.before {
                Some(cell) => {
                    self.project
                        .first_frame_mut()
                        .cells
                        .insert((y, x), cell.clone());
                }
                None => {
                    self.project.first_frame_mut().cells.remove(&(y, x));
                }
            }
        }

        self.redo_stack.push(stroke);
        self.dirty = true;
        self.message = "Undo".to_string();
    }

    fn redo(&mut self) {
        let Some(stroke) = self.redo_stack.pop() else {
            self.message = "Nothing to redo".to_string();
            return;
        };

        for (&(y, x), change) in &stroke.changes {
            match &change.after {
                Some(cell) => {
                    self.project
                        .first_frame_mut()
                        .cells
                        .insert((y, x), cell.clone());
                }
                None => {
                    self.project.first_frame_mut().cells.remove(&(y, x));
                }
            }
        }

        self.undo_stack.push(stroke);
        self.dirty = true;
        self.message = "Redo".to_string();
    }

    fn save(&mut self) {
        let Some(path) = self.file_path.clone() else {
            self.open_save_as();
            return;
        };

        match save_project_to_path(&self.project, &path) {
            Ok(warnings) => {
                self.dirty = false;
                self.message = if warnings.is_empty() {
                    format!("Saved {}", path.display())
                } else {
                    format!(
                        "Saved {} with {} warning(s)",
                        path.display(),
                        warnings.len()
                    )
                };
            }
            Err(error) => {
                self.message = format_save_error(error);
            }
        }
    }

    fn open_save_as(&mut self) {
        let input = self
            .file_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "untitled.tanim.toml".to_string());
        self.modal = Some(Modal::SaveAs { input });
    }

    fn request_quit(&mut self) {
        if self.dirty {
            self.modal = Some(Modal::QuitConfirm);
        } else {
            self.should_quit = true;
        }
    }

    fn previous_style(&mut self) {
        if self.current_style == 0 {
            self.current_style = self.project.styles.len().saturating_sub(1);
        } else {
            self.current_style -= 1;
        }
    }

    fn next_style(&mut self) {
        self.current_style = (self.current_style + 1) % self.project.styles.len();
    }

    fn canvas_cell_at(&self, column: u16, row: u16) -> Option<(u16, u16)> {
        if column < self.canvas_area.x
            || row < self.canvas_area.y
            || column >= self.canvas_area.x + self.canvas_area.width
            || row >= self.canvas_area.y + self.canvas_area.height
        {
            return None;
        }

        let x = column - self.canvas_area.x;
        let y = row - self.canvas_area.y;

        if x < self.project.asset.width && y < self.project.asset.height {
            Some((x, y))
        } else {
            None
        }
    }
}

fn draw(frame: &mut Frame<'_>, app: &mut AppState) {
    match app.screen {
        Screen::Welcome => draw_welcome(frame, app),
        Screen::Editor => draw_editor(frame, app),
    }

    if app.modal.is_some() {
        draw_modal(frame, app);
    }
}

fn draw_welcome(frame: &mut Frame<'_>, app: &AppState) {
    let area = frame.area();
    let block = Block::default()
        .title("Terminal Animator")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from("Terminal Animator Phase 1"),
        Line::from(""),
        Line::from("N  New image"),
        Line::from("O  Open file"),
        Line::from("Q  Quit"),
        Line::from(""),
        Line::from(app.message.as_str()),
    ];
    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn draw_editor(frame: &mut Frame<'_>, app: &mut AppState) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(5),
        ])
        .split(frame.area());

    draw_top_bar(frame, app, root[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(10)])
        .split(root[1]);

    draw_sidebar(frame, app, body[0]);
    draw_canvas(frame, app, body[1]);
    draw_footer(frame, app, root[2]);
}

fn draw_top_bar(frame: &mut Frame<'_>, app: &AppState, area: Rect) {
    let dirty = if app.dirty { " *" } else { "" };
    let path = app
        .file_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "untitled".to_string());
    let text = format!(
        " {}{} | {}x{} | {} cells",
        path,
        dirty,
        app.project.asset.width,
        app.project.asset.height,
        app.project.first_frame().cells.len()
    );
    frame.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn draw_sidebar(frame: &mut Frame<'_>, app: &AppState, area: Rect) {
    let style = &app.project.styles[app.current_style];
    let tool = match app.tool {
        Tool::Pencil => "Pencil",
        Tool::Eraser => "Eraser",
        Tool::Eyedropper => "Eyedropper",
    };

    let bg = style
        .bg
        .map(Color::to_hex)
        .unwrap_or_else(|| "transparent".to_string());
    let attrs = if style.attrs.is_empty() {
        "none".to_string()
    } else {
        style
            .attrs
            .iter()
            .map(|attr| format!("{attr:?}").to_lowercase())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let lines = vec![
        Line::from(vec![
            Span::raw("Tool: "),
            Span::styled(tool, TuiStyle::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(format!("Brush: {:?}", app.brush_char)),
        Line::from(""),
        Line::from(format!(
            "Style {}/{}",
            app.current_style + 1,
            app.project.styles.len()
        )),
        Line::from(format!("ID: {}", style.id)),
        Line::from(format!("FG: {}", style.fg.to_hex())),
        Line::from(format!("BG: {bg}")),
        Line::from(format!("Attrs: {attrs}")),
        Line::from(""),
        Line::from("P/E/I tool"),
        Line::from("C char"),
        Line::from("[ ] style"),
        Line::from("A new style"),
        Line::from("R rename"),
        Line::from("F/G colors"),
    ];

    let paragraph = Paragraph::new(lines)
        .block(Block::default().title("Tools").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_canvas(frame: &mut Frame<'_>, app: &mut AppState, area: Rect) {
    let block = Block::default().title("Canvas").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.canvas_area = inner;

    let visible_width = app.project.asset.width.min(inner.width);
    let visible_height = app.project.asset.height.min(inner.height);

    for y in 0..visible_height {
        for x in 0..visible_width {
            let screen_x = inner.x + x;
            let screen_y = inner.y + y;
            let buffer_cell = &mut frame.buffer_mut()[(screen_x, screen_y)];

            if let Some(cell) = app.project.first_frame().cells.get(&(y, x)) {
                let style = style_to_tui(&app.project.styles[cell.style_index]);
                let symbol = cell.ch.to_string();
                buffer_cell.set_symbol(&symbol);
                buffer_cell.set_style(style);
            } else {
                buffer_cell.set_symbol(".");
                buffer_cell.set_style(TuiStyle::default().fg(TuiColor::DarkGray));
            }
        }
    }

    if app.project.asset.width > inner.width || app.project.asset.height > inner.height {
        let note = "canvas clipped by terminal size";
        let y = area.y + area.height.saturating_sub(1);
        for (offset, ch) in note.chars().enumerate() {
            let x = area.x + 2 + u16::try_from(offset).unwrap_or(u16::MAX);
            if x < area.x + area.width.saturating_sub(1) {
                frame.buffer_mut()[(x, y)]
                    .set_symbol(&ch.to_string())
                    .set_style(TuiStyle::default().fg(TuiColor::Yellow));
            }
        }
    }
}

fn draw_footer(frame: &mut Frame<'_>, app: &AppState, area: Rect) {
    let help = "Ctrl-S save | Ctrl-Shift-S save as | Ctrl-Z/Y undo/redo | T export text | Q quit";
    let lines = vec![Line::from(help), Line::from(app.message.as_str())];
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().title("Status").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_modal(frame: &mut Frame<'_>, app: &AppState) {
    let Some(modal) = app.modal.as_ref() else {
        return;
    };

    let area = centered_rect(72, 7, frame.area());
    frame.render_widget(Clear, area);

    let (title, body) = match modal {
        Modal::NewImage { input, .. } => ("New Image", format!("Dimensions: {input}")),
        Modal::OpenFile { input } => ("Open File", format!("Path: {input}")),
        Modal::SaveAs { input } => ("Save As", format!("Path: {input}")),
        Modal::BrushChar { input } => ("Brush Character", format!("Character: {input}")),
        Modal::NewStyle { input } => ("New Style", format!("Style ID: {input}")),
        Modal::RenameStyle { input } => ("Rename Style", format!("Style ID: {input}")),
        Modal::SetFg { input } => ("Foreground", format!("Color: {input}")),
        Modal::SetBg { input } => ("Background", format!("Color: {input}")),
        Modal::ExportText { input } => ("Export Text", format!("Path: {input}")),
        Modal::QuitConfirm => (
            "Unsaved Changes",
            "S save and quit, D discard, C cancel".to_string(),
        ),
    };

    let paragraph = Paragraph::new(vec![
        Line::from(body),
        Line::from(""),
        Line::from("Enter confirms, Esc cancels"),
    ])
    .block(Block::default().title(title).borders(Borders::ALL))
    .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn style_to_tui(style: &TerminalStyle) -> TuiStyle {
    let mut tui_style = TuiStyle::default().fg(TuiColor::Rgb(style.fg.r, style.fg.g, style.fg.b));

    if let Some(bg) = style.bg {
        tui_style = tui_style.bg(TuiColor::Rgb(bg.r, bg.g, bg.b));
    }

    for attr in &style.attrs {
        tui_style = match attr {
            TextAttr::Bold => tui_style.add_modifier(Modifier::BOLD),
            TextAttr::Dim => tui_style.add_modifier(Modifier::DIM),
            TextAttr::Italic => tui_style.add_modifier(Modifier::ITALIC),
            TextAttr::Underline => tui_style.add_modifier(Modifier::UNDERLINED),
            TextAttr::Reverse => tui_style.add_modifier(Modifier::REVERSED),
        };
    }

    tui_style
}

fn modal_input_mut(modal: &mut Option<Modal>) -> Option<&mut String> {
    match modal.as_mut()? {
        Modal::NewImage { input, .. }
        | Modal::OpenFile { input }
        | Modal::SaveAs { input }
        | Modal::BrushChar { input }
        | Modal::NewStyle { input }
        | Modal::RenameStyle { input }
        | Modal::SetFg { input }
        | Modal::SetBg { input }
        | Modal::ExportText { input } => Some(input),
        Modal::QuitConfirm => None,
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn parse_dimensions(input: &str) -> Option<(u16, u16)> {
    let (width, height) = input.trim().split_once('x')?;
    let width = width.parse::<u16>().ok()?;
    let height = height.parse::<u16>().ok()?;
    if width == 0
        || height == 0
        || width > MAX_WIDTH
        || height > MAX_HEIGHT
        || u64::from(width) * u64::from(height) > MAX_AREA_PER_FRAME
    {
        return None;
    }
    Some((width, height))
}

pub fn parse_new_dimensions(input: &str) -> Result<(u16, u16)> {
    parse_dimensions(input).ok_or_else(|| {
        anyhow!(
            "expected dimensions as WIDTHxHEIGHT within {}x{} and {} total cells",
            MAX_WIDTH,
            MAX_HEIGHT,
            MAX_AREA_PER_FRAME
        )
    })
}

pub fn startup_from_path(path: PathBuf) -> Startup {
    if path.exists() {
        Startup::Open(path)
    } else {
        Startup::CreateAt(path)
    }
}

pub fn export_text_file(input: &Path, output: &Path) -> Result<()> {
    let loaded = load_project_from_path(input)
        .with_context(|| format!("failed to load {}", input.display()))?;
    fs::write(output, export_plain_text(&loaded.project, 0))
        .with_context(|| format!("failed to write {}", output.display()))?;
    Ok(())
}

fn asset_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("untitled")
        .to_string()
}

fn default_text_export_path(path: &Path) -> PathBuf {
    let mut export_path = path.to_path_buf();
    export_path.set_extension("txt");
    export_path
}

fn next_style_id(project: &Project) -> String {
    for index in 1.. {
        let candidate = format!("style-{index}");
        if !project.styles.iter().any(|style| style.id == candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded style ID search")
}

fn format_save_error(error: FormatError) -> String {
    match error {
        FormatError::Validation(report) => {
            if let Some(first) = report.errors.first() {
                format!("Save blocked: {}: {}", first.location, first.message)
            } else {
                "Save blocked by validation".to_string()
            }
        }
        other => format!("Save failed: {other}"),
    }
}
