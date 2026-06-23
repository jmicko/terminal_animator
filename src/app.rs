use crate::format::{
    Color, FormatError, MAX_AREA_PER_FRAME, MAX_HEIGHT, MAX_STYLES, MAX_WIDTH, PaintedCell,
    Project, TerminalStyle, TextAttr, export_plain_text, is_valid_v1_character,
    load_project_from_path, save_project_to_path,
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
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color as TuiColor, Modifier, Style as TuiStyle};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_NEW_WIDTH: u16 = 48;
const DEFAULT_NEW_HEIGHT: u16 = 16;
const COLOR_SLOT_WIDTH: u16 = 4;
const COLOR_SWATCH_WIDTH: u16 = 2;
const CHAR_SLOT_WIDTH: u16 = 3;
const TOP_BUTTON_GAP: u16 = 1;
const MORE_COLOR_COUNT: usize = 216;
const MAX_RECENT_COLORS: usize = 24;

#[derive(Debug, Clone, Copy)]
struct PaletteColor {
    name: &'static str,
    id_base: &'static str,
    color: Color,
}

const COLOR_PALETTE: &[PaletteColor] = &[
    PaletteColor {
        name: "black",
        id_base: "palette-black",
        color: Color { r: 0, g: 0, b: 0 },
    },
    PaletteColor {
        name: "white",
        id_base: "palette-white",
        color: Color {
            r: 238,
            g: 238,
            b: 238,
        },
    },
    PaletteColor {
        name: "gray",
        id_base: "palette-gray",
        color: Color {
            r: 128,
            g: 128,
            b: 128,
        },
    },
    PaletteColor {
        name: "red",
        id_base: "palette-red",
        color: Color {
            r: 224,
            g: 73,
            b: 73,
        },
    },
    PaletteColor {
        name: "orange",
        id_base: "palette-orange",
        color: Color {
            r: 255,
            g: 157,
            b: 46,
        },
    },
    PaletteColor {
        name: "yellow",
        id_base: "palette-yellow",
        color: Color {
            r: 224,
            g: 185,
            b: 82,
        },
    },
    PaletteColor {
        name: "green",
        id_base: "palette-green",
        color: Color {
            r: 87,
            g: 166,
            b: 74,
        },
    },
    PaletteColor {
        name: "mint",
        id_base: "palette-mint",
        color: Color {
            r: 91,
            g: 194,
            b: 150,
        },
    },
    PaletteColor {
        name: "cyan",
        id_base: "palette-cyan",
        color: Color {
            r: 63,
            g: 169,
            b: 191,
        },
    },
    PaletteColor {
        name: "blue",
        id_base: "palette-blue",
        color: Color {
            r: 82,
            g: 128,
            b: 214,
        },
    },
    PaletteColor {
        name: "purple",
        id_base: "palette-purple",
        color: Color {
            r: 150,
            g: 104,
            b: 204,
        },
    },
    PaletteColor {
        name: "pink",
        id_base: "palette-pink",
        color: Color {
            r: 218,
            g: 101,
            b: 157,
        },
    },
    PaletteColor {
        name: "brown",
        id_base: "palette-brown",
        color: Color {
            r: 139,
            g: 89,
            b: 55,
        },
    },
    PaletteColor {
        name: "tan",
        id_base: "palette-tan",
        color: Color {
            r: 194,
            g: 166,
            b: 122,
        },
    },
    PaletteColor {
        name: "coral",
        id_base: "palette-coral",
        color: Color {
            r: 238,
            g: 112,
            b: 92,
        },
    },
    PaletteColor {
        name: "rose",
        id_base: "palette-rose",
        color: Color {
            r: 224,
            g: 91,
            b: 122,
        },
    },
    PaletteColor {
        name: "magenta",
        id_base: "palette-magenta",
        color: Color {
            r: 202,
            g: 90,
            b: 214,
        },
    },
    PaletteColor {
        name: "violet",
        id_base: "palette-violet",
        color: Color {
            r: 113,
            g: 89,
            b: 193,
        },
    },
    PaletteColor {
        name: "navy",
        id_base: "palette-navy",
        color: Color {
            r: 49,
            g: 74,
            b: 112,
        },
    },
    PaletteColor {
        name: "sky",
        id_base: "palette-sky",
        color: Color {
            r: 116,
            g: 174,
            b: 220,
        },
    },
    PaletteColor {
        name: "teal",
        id_base: "palette-teal",
        color: Color {
            r: 53,
            g: 140,
            b: 130,
        },
    },
    PaletteColor {
        name: "lime",
        id_base: "palette-lime",
        color: Color {
            r: 151,
            g: 195,
            b: 67,
        },
    },
    PaletteColor {
        name: "olive",
        id_base: "palette-olive",
        color: Color {
            r: 117,
            g: 132,
            b: 58,
        },
    },
    PaletteColor {
        name: "gold",
        id_base: "palette-gold",
        color: Color {
            r: 221,
            g: 166,
            b: 45,
        },
    },
];

const CHARACTER_PALETTE: &[char] = &[
    '#', ' ', '*', '+', '-', '/', '\\', '|', '_', '.', '\'', '"', '`', '~', '^', 'o', 'O', '█',
    '▓', '▒', '░', '▀', '▄', '▌', '▐', '■', '□', '▪', '▫', '─', '│', '┌', '┐', '└', '┘', '├', '┤',
    '┬', '┴', '┼', '╭', '╮', '╰', '╯', '•', '◆', '◇', '▲', '▼', '◀', '▶', '★', '☆', '✦', '✧', '✶',
    '✷', '✹', '✺',
];

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

impl Tool {
    fn label(self) -> &'static str {
        match self {
            Self::Pencil => "Pencil",
            Self::Eraser => "Eraser",
            Self::Eyedropper => "Eyedropper",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Pencil => "Paint selected character and style",
            Self::Eraser => "Clear cells to transparent",
            Self::Eyedropper => "Pick character and style from a cell",
        }
    }
}

const TOOL_CHOICES: &[Tool] = &[Tool::Pencil, Tool::Eraser, Tool::Eyedropper];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopAction {
    Save,
    SaveAs,
    ExportText,
    Quit,
}

impl TopAction {
    fn label(self) -> &'static str {
        match self {
            Self::Save => "Save",
            Self::SaveAs => "Save As",
            Self::ExportText => "Export",
            Self::Quit => "Quit",
        }
    }
}

const TOP_ACTIONS: &[TopAction] = &[
    TopAction::Save,
    TopAction::SaveAs,
    TopAction::ExportText,
    TopAction::Quit,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WelcomeAction {
    NewImage,
    OpenFile,
    Quit,
}

impl WelcomeAction {
    fn label(self) -> &'static str {
        match self {
            Self::NewImage => "New Image",
            Self::OpenFile => "Open File",
            Self::Quit => "Quit",
        }
    }
}

const WELCOME_ACTIONS: &[WelcomeAction] = &[
    WelcomeAction::NewImage,
    WelcomeAction::OpenFile,
    WelcomeAction::Quit,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModalAction {
    Confirm,
    Cancel,
    RgbInput,
    SaveAndQuit,
    Discard,
}

impl ModalAction {
    fn label(self) -> &'static str {
        match self {
            Self::Confirm => "OK",
            Self::Cancel => "Cancel",
            Self::RgbInput => "RGB",
            Self::SaveAndQuit => "Save",
            Self::Discard => "Discard",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ButtonHit<T> {
    action: T,
    area: Rect,
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
    RgbInput {
        input: String,
    },
    ExportText {
        input: String,
    },
    ColorPicker,
    ToolMenu,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaletteHover {
    Color(usize),
    Character(usize),
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
    tool_button_area: Rect,
    brush_button_area: Rect,
    color_palette_area: Rect,
    recent_color_palette_area: Rect,
    character_palette_area: Rect,
    more_colors_button_area: Rect,
    rgb_button_area: Rect,
    welcome_action_areas: Vec<ButtonHit<WelcomeAction>>,
    top_action_areas: Vec<ButtonHit<TopAction>>,
    modal_tool_areas: Vec<ButtonHit<Tool>>,
    modal_action_areas: Vec<ButtonHit<ModalAction>>,
    modal_color_areas: Vec<ButtonHit<usize>>,
    modal_area: Rect,
    recent_extra_colors: Vec<Color>,
    color_picker_scroll_row: usize,
    hovered_palette: Option<PaletteHover>,
    hovered_recent_color: Option<usize>,
    hovered_welcome_action: Option<WelcomeAction>,
    hovered_top_action: Option<TopAction>,
    hovered_tool_button: bool,
    hovered_brush_button: bool,
    hovered_more_colors_button: bool,
    hovered_rgb_button: bool,
    hovered_modal_tool: Option<Tool>,
    hovered_modal_action: Option<ModalAction>,
    hovered_modal_color: Option<usize>,
    hovered_canvas_cell: Option<(u16, u16)>,
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
            tool_button_area: Rect::default(),
            brush_button_area: Rect::default(),
            color_palette_area: Rect::default(),
            recent_color_palette_area: Rect::default(),
            character_palette_area: Rect::default(),
            more_colors_button_area: Rect::default(),
            rgb_button_area: Rect::default(),
            welcome_action_areas: Vec::new(),
            top_action_areas: Vec::new(),
            modal_tool_areas: Vec::new(),
            modal_action_areas: Vec::new(),
            modal_color_areas: Vec::new(),
            modal_area: Rect::default(),
            recent_extra_colors: Vec::new(),
            color_picker_scroll_row: 0,
            hovered_palette: None,
            hovered_recent_color: None,
            hovered_welcome_action: None,
            hovered_top_action: None,
            hovered_tool_button: false,
            hovered_brush_button: false,
            hovered_more_colors_button: false,
            hovered_rgb_button: false,
            hovered_modal_tool: None,
            hovered_modal_action: None,
            hovered_modal_color: None,
            hovered_canvas_cell: None,
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
            tool_button_area: Rect::default(),
            brush_button_area: Rect::default(),
            color_palette_area: Rect::default(),
            recent_color_palette_area: Rect::default(),
            character_palette_area: Rect::default(),
            more_colors_button_area: Rect::default(),
            rgb_button_area: Rect::default(),
            welcome_action_areas: Vec::new(),
            top_action_areas: Vec::new(),
            modal_tool_areas: Vec::new(),
            modal_action_areas: Vec::new(),
            modal_color_areas: Vec::new(),
            modal_area: Rect::default(),
            recent_extra_colors: Vec::new(),
            color_picker_scroll_row: 0,
            hovered_palette: None,
            hovered_recent_color: None,
            hovered_welcome_action: None,
            hovered_top_action: None,
            hovered_tool_button: false,
            hovered_brush_button: false,
            hovered_more_colors_button: false,
            hovered_rgb_button: false,
            hovered_modal_tool: None,
            hovered_modal_action: None,
            hovered_modal_color: None,
            hovered_canvas_cell: None,
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
                self.select_tool(Tool::Pencil);
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.select_tool(Tool::Eraser);
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                self.select_tool(Tool::Eyedropper);
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
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.open_color_picker();
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                self.open_rgb_input();
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.open_export_text();
            }
            KeyCode::Char('[') => self.previous_style(),
            KeyCode::Char(']') | KeyCode::Tab => self.next_style(),
            KeyCode::Char('q') | KeyCode::Char('Q') => self.request_quit(),
            KeyCode::Esc => self.request_quit(),
            _ => {}
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent) {
        if matches!(self.modal, Some(Modal::ColorPicker)) {
            match key.code {
                KeyCode::Esc => {
                    self.close_modal("Color picker closed");
                }
                KeyCode::PageDown | KeyCode::Down => self.scroll_color_picker(1),
                KeyCode::PageUp | KeyCode::Up => self.scroll_color_picker(-1),
                KeyCode::Char('u') | KeyCode::Char('U') => self.open_rgb_input(),
                _ => {}
            }
            return;
        }

        if matches!(self.modal, Some(Modal::ToolMenu)) {
            match key.code {
                KeyCode::Esc => {
                    self.modal = None;
                    self.message = "Tool menu closed".to_string();
                }
                KeyCode::Char('1') | KeyCode::Char('p') | KeyCode::Char('P') => {
                    self.select_tool_from_menu(Tool::Pencil);
                }
                KeyCode::Char('2') | KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.select_tool_from_menu(Tool::Eraser);
                }
                KeyCode::Char('3') | KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.select_tool_from_menu(Tool::Eyedropper);
                }
                _ => {}
            }
            return;
        }

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
            Modal::RgbInput { input } => match parse_rgb_input(&input) {
                Some(color) => {
                    self.select_custom_color(color, false);
                    self.message = format!("Selected RGB {}", color.to_hex());
                }
                None => {
                    self.message = "Enter RGB as 255,128,0 or #FF8000".to_string();
                    self.modal = Some(Modal::RgbInput { input });
                }
            },
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
            Modal::ColorPicker => {}
            Modal::ToolMenu => {}
            Modal::QuitConfirm => {}
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.screen == Screen::Welcome {
            if self.modal.is_some() {
                self.handle_modal_mouse(mouse);
            } else {
                self.handle_welcome_mouse(mouse);
            }
            return;
        }

        if self.screen != Screen::Editor {
            return;
        }

        if self.modal.is_some() {
            self.handle_modal_mouse(mouse);
            return;
        }

        self.update_hover(mouse.column, mouse.row);

        match mouse.kind {
            MouseEventKind::Moved => {}
            MouseEventKind::Down(MouseButton::Left) => {
                if self.apply_top_action_click(mouse.column, mouse.row) {
                    return;
                }
                if rect_contains(self.tool_button_area, mouse.column, mouse.row) {
                    self.modal = Some(Modal::ToolMenu);
                    self.hovered_modal_tool = None;
                    self.message = "Choose a tool".to_string();
                    return;
                }
                if rect_contains(self.brush_button_area, mouse.column, mouse.row) {
                    self.modal = Some(Modal::BrushChar {
                        input: self.brush_char.to_string(),
                    });
                    return;
                }
                if rect_contains(self.more_colors_button_area, mouse.column, mouse.row) {
                    self.open_color_picker();
                    return;
                }
                if rect_contains(self.rgb_button_area, mouse.column, mouse.row) {
                    self.open_rgb_input();
                    return;
                }
                if self.apply_palette_click(mouse.column, mouse.row) {
                    return;
                }
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

    fn handle_welcome_mouse(&mut self, mouse: MouseEvent) {
        self.update_welcome_hover(mouse.column, mouse.row);

        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            && let Some(action) = self.hovered_welcome_action
        {
            self.run_welcome_action(action);
        }
    }

    fn handle_modal_mouse(&mut self, mouse: MouseEvent) {
        if matches!(self.modal, Some(Modal::ColorPicker)) {
            self.hovered_modal_color = self.modal_color_at(mouse.column, mouse.row);
            self.hovered_modal_action = self.modal_action_at(mouse.column, mouse.row);
            match mouse.kind {
                MouseEventKind::ScrollDown => self.scroll_color_picker(1),
                MouseEventKind::ScrollUp => self.scroll_color_picker(-1),
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some(index) = self.hovered_modal_color {
                        let color = color_cube_color(index);
                        self.select_custom_color(color, true);
                        self.close_modal(format!("Selected {}", color.to_hex()));
                    } else if let Some(action) = self.modal_action_at(mouse.column, mouse.row) {
                        self.run_modal_action(action);
                    } else if !rect_contains(self.modal_area, mouse.column, mouse.row) {
                        self.close_modal("Color picker closed");
                    }
                }
                _ => {}
            }
            return;
        }

        if matches!(self.modal, Some(Modal::ToolMenu)) {
            self.hovered_modal_tool = self.modal_tool_at(mouse.column, mouse.row);

            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                if let Some(tool) = self.hovered_modal_tool {
                    self.select_tool_from_menu(tool);
                } else if !rect_contains(self.modal_area, mouse.column, mouse.row) {
                    self.close_modal("Tool menu closed");
                }
            }
            return;
        }

        self.hovered_modal_action = self.modal_action_at(mouse.column, mouse.row);
        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            if let Some(action) = self.hovered_modal_action {
                self.run_modal_action(action);
            } else if !rect_contains(self.modal_area, mouse.column, mouse.row) {
                self.close_modal("Canceled");
            }
        }
    }

    fn update_hover(&mut self, column: u16, row: u16) {
        self.hovered_top_action = self.top_action_at(column, row);
        self.hovered_tool_button = rect_contains(self.tool_button_area, column, row);
        self.hovered_brush_button = rect_contains(self.brush_button_area, column, row);
        self.hovered_more_colors_button = rect_contains(self.more_colors_button_area, column, row);
        self.hovered_rgb_button = rect_contains(self.rgb_button_area, column, row);
        self.hovered_canvas_cell = if self.hovered_top_action.is_none()
            && !self.hovered_tool_button
            && !self.hovered_brush_button
            && !self.hovered_more_colors_button
            && !self.hovered_rgb_button
        {
            self.canvas_cell_at(column, row).map(|(x, y)| (y, x))
        } else {
            None
        };

        self.hovered_palette = if let Some(index) = hit_palette_item(
            self.color_palette_area,
            COLOR_SLOT_WIDTH,
            column,
            row,
            COLOR_PALETTE.len(),
        ) {
            Some(PaletteHover::Color(index))
        } else {
            hit_palette_item(
                self.character_palette_area,
                CHAR_SLOT_WIDTH,
                column,
                row,
                CHARACTER_PALETTE.len(),
            )
            .map(PaletteHover::Character)
        };

        self.hovered_recent_color = hit_palette_item(
            self.recent_color_palette_area,
            COLOR_SLOT_WIDTH,
            column,
            row,
            self.recent_extra_colors.len(),
        );
    }

    fn update_welcome_hover(&mut self, column: u16, row: u16) {
        self.hovered_welcome_action = self
            .welcome_action_areas
            .iter()
            .find(|hit| rect_contains(hit.area, column, row))
            .map(|hit| hit.action);
    }

    fn top_action_at(&self, column: u16, row: u16) -> Option<TopAction> {
        self.top_action_areas
            .iter()
            .find(|hit| rect_contains(hit.area, column, row))
            .map(|hit| hit.action)
    }

    fn modal_tool_at(&self, column: u16, row: u16) -> Option<Tool> {
        self.modal_tool_areas
            .iter()
            .find(|hit| rect_contains(hit.area, column, row))
            .map(|hit| hit.action)
    }

    fn modal_action_at(&self, column: u16, row: u16) -> Option<ModalAction> {
        self.modal_action_areas
            .iter()
            .find(|hit| rect_contains(hit.area, column, row))
            .map(|hit| hit.action)
    }

    fn modal_color_at(&self, column: u16, row: u16) -> Option<usize> {
        self.modal_color_areas
            .iter()
            .find(|hit| rect_contains(hit.area, column, row))
            .map(|hit| hit.action)
    }

    fn apply_top_action_click(&mut self, column: u16, row: u16) -> bool {
        if let Some(action) = self.top_action_at(column, row) {
            self.run_top_action(action);
            true
        } else {
            false
        }
    }

    fn apply_palette_click(&mut self, column: u16, row: u16) -> bool {
        if let Some(index) = hit_palette_item(
            self.color_palette_area,
            COLOR_SLOT_WIDTH,
            column,
            row,
            COLOR_PALETTE.len(),
        ) {
            let palette_color = COLOR_PALETTE[index];
            self.select_palette_color(palette_color);
            return true;
        }

        if let Some(index) = hit_palette_item(
            self.recent_color_palette_area,
            COLOR_SLOT_WIDTH,
            column,
            row,
            self.recent_extra_colors.len(),
        ) {
            let color = self.recent_extra_colors[index];
            self.select_custom_color(color, false);
            self.message = format!("Selected recent {}", color.to_hex());
            return true;
        }

        if let Some(index) = hit_palette_item(
            self.character_palette_area,
            CHAR_SLOT_WIDTH,
            column,
            row,
            CHARACTER_PALETTE.len(),
        ) {
            self.brush_char = CHARACTER_PALETTE[index];
            self.tool = Tool::Pencil;
            self.message = format!("Brush character set to {:?}", self.brush_char);
            return true;
        }

        false
    }

    fn select_palette_color(&mut self, palette_color: PaletteColor) {
        self.select_color(
            palette_color.color,
            palette_color.name,
            palette_color.id_base,
            false,
        );
    }

    fn select_custom_color(&mut self, color: Color, add_recent: bool) {
        let hex = color.to_hex();
        self.select_color(color, &hex, &style_id_base_for_color(color), add_recent);
    }

    fn select_color(&mut self, color: Color, label: &str, id_base: &str, add_recent: bool) {
        self.tool = Tool::Pencil;

        if let Some(index) = self
            .project
            .styles
            .iter()
            .position(|style| style.fg == color && style.bg.is_none() && style.attrs.is_empty())
        {
            self.current_style = index;
            if add_recent {
                self.add_recent_extra_color(color);
            }
            self.message = format!("Selected {label} ({})", color.to_hex());
            return;
        }

        if self.project.styles.len() >= MAX_STYLES {
            self.message = format!("Cannot add {label}: style limit reached");
            return;
        }

        let id = unique_style_id(&self.project, id_base);
        self.project.styles.push(TerminalStyle {
            id,
            fg: color,
            bg: None,
            attrs: Vec::new(),
            role: Some("palette".to_string()),
        });
        self.current_style = self.project.styles.len() - 1;
        self.dirty = true;
        if add_recent {
            self.add_recent_extra_color(color);
        }
        self.message = format!("Selected {label} ({})", color.to_hex());
    }

    fn add_recent_extra_color(&mut self, color: Color) {
        if is_visible_palette_color(color) {
            return;
        }

        self.recent_extra_colors
            .retain(|recent_color| *recent_color != color);
        self.recent_extra_colors.insert(0, color);
        self.recent_extra_colors.truncate(MAX_RECENT_COLORS);
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

    fn select_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.message = format!("{} selected", tool.label());
    }

    fn select_tool_from_menu(&mut self, tool: Tool) {
        self.modal = None;
        self.hovered_modal_tool = None;
        self.select_tool(tool);
    }

    fn run_top_action(&mut self, action: TopAction) {
        match action {
            TopAction::Save => self.save(),
            TopAction::SaveAs => self.open_save_as(),
            TopAction::ExportText => self.open_export_text(),
            TopAction::Quit => self.request_quit(),
        }
    }

    fn run_welcome_action(&mut self, action: WelcomeAction) {
        match action {
            WelcomeAction::NewImage => {
                self.modal = Some(Modal::NewImage {
                    input: format!("{DEFAULT_NEW_WIDTH}x{DEFAULT_NEW_HEIGHT}"),
                    target_path: None,
                });
            }
            WelcomeAction::OpenFile => {
                self.modal = Some(Modal::OpenFile {
                    input: String::new(),
                });
            }
            WelcomeAction::Quit => self.should_quit = true,
        }
    }

    fn run_modal_action(&mut self, action: ModalAction) {
        match action {
            ModalAction::Confirm => {
                if let Some(modal) = self.modal.take() {
                    self.commit_modal(modal);
                }
            }
            ModalAction::Cancel => self.close_modal("Canceled"),
            ModalAction::RgbInput => self.open_rgb_input(),
            ModalAction::SaveAndQuit => {
                self.save();
                if !self.dirty {
                    self.should_quit = true;
                }
            }
            ModalAction::Discard => {
                self.should_quit = true;
            }
        }
    }

    fn close_modal(&mut self, message: impl Into<String>) {
        self.modal = None;
        self.hovered_modal_action = None;
        self.hovered_modal_tool = None;
        self.hovered_modal_color = None;
        self.message = message.into();
    }

    fn open_color_picker(&mut self) {
        self.modal = Some(Modal::ColorPicker);
        self.color_picker_scroll_row = 0;
        self.hovered_modal_color = None;
        self.message = "Choose a color".to_string();
    }

    fn open_rgb_input(&mut self) {
        let color = self.project.styles[self.current_style].fg;
        self.modal = Some(Modal::RgbInput {
            input: format!("{},{},{}", color.r, color.g, color.b),
        });
        self.message = "Enter RGB as 255,128,0".to_string();
    }

    fn scroll_color_picker(&mut self, delta_rows: isize) {
        let columns = self.color_picker_columns().max(1);
        let visible_rows = self.color_picker_visible_rows().max(1);
        let total_rows = MORE_COLOR_COUNT.div_ceil(usize::from(columns));
        let max_scroll = total_rows.saturating_sub(usize::from(visible_rows));
        let current = self.color_picker_scroll_row;

        self.color_picker_scroll_row = if delta_rows.is_negative() {
            current.saturating_sub(delta_rows.unsigned_abs())
        } else {
            current.saturating_add(delta_rows as usize).min(max_scroll)
        };
    }

    fn color_picker_columns(&self) -> u16 {
        palette_columns(self.color_picker_grid_area().width, COLOR_SLOT_WIDTH)
    }

    fn color_picker_visible_rows(&self) -> u16 {
        self.color_picker_grid_area().height
    }

    fn color_picker_grid_area(&self) -> Rect {
        if self.modal_area.width < 4 || self.modal_area.height < 7 {
            return Rect::default();
        }

        Rect {
            x: self.modal_area.x + 2,
            y: self.modal_area.y + 3,
            width: self.modal_area.width.saturating_sub(4),
            height: self.modal_area.height.saturating_sub(6),
        }
    }

    fn open_export_text(&mut self) {
        let default = self
            .file_path
            .as_ref()
            .map(|path| default_text_export_path(path.as_path()))
            .unwrap_or_else(|| PathBuf::from("terminal-art.txt"));
        self.modal = Some(Modal::ExportText {
            input: default.display().to_string(),
        });
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

fn draw_welcome(frame: &mut Frame<'_>, app: &mut AppState) {
    let area = frame.area();
    let block = Block::default()
        .title("Terminal Animator")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.welcome_action_areas.clear();

    let start_y = inner.y + inner.height.saturating_sub(10) / 2;
    draw_text_centered(
        frame,
        inner,
        start_y,
        "Terminal Animator Phase 1",
        TuiStyle::default().add_modifier(Modifier::BOLD),
    );
    draw_text_centered(
        frame,
        inner,
        start_y.saturating_add(1),
        "Create or open a .tanim.toml file",
        TuiStyle::default().fg(TuiColor::Rgb(170, 184, 194)),
    );

    let mut y = start_y.saturating_add(3);
    for action in WELCOME_ACTIONS {
        if y >= inner.y + inner.height {
            break;
        }

        let width = 20.min(inner.width);
        let x = inner.x + inner.width.saturating_sub(width) / 2;
        let button_area = Rect {
            x,
            y,
            width,
            height: 1,
        };
        let hovered = app.hovered_welcome_action == Some(*action);
        let style = choice_style(false, hovered);
        fill_rect(frame, button_area, style);
        draw_text_centered(frame, button_area, y, action.label(), style);
        app.welcome_action_areas.push(ButtonHit {
            action: *action,
            area: button_area,
        });
        y = y.saturating_add(2);
    }

    draw_text_centered(
        frame,
        inner,
        y.saturating_add(1),
        app.message.as_str(),
        TuiStyle::default().fg(TuiColor::Rgb(211, 220, 228)),
    );
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

fn draw_top_bar(frame: &mut Frame<'_>, app: &mut AppState, area: Rect) {
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
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.top_action_areas.clear();

    let mut x = inner.x + inner.width;
    let mut buttons = Vec::new();
    for action in TOP_ACTIONS.iter().rev() {
        let label = format!("[{}]", action.label());
        let width = u16::try_from(label.chars().count()).unwrap_or(u16::MAX);
        if width + TOP_BUTTON_GAP > x.saturating_sub(inner.x) {
            continue;
        }
        x = x.saturating_sub(width);
        let area = Rect {
            x,
            y: inner.y,
            width,
            height: 1,
        };
        buttons.push((*action, label, area));
        x = x.saturating_sub(TOP_BUTTON_GAP);
    }

    let text_width = x.saturating_sub(inner.x).saturating_sub(TOP_BUTTON_GAP);
    draw_text(
        frame,
        inner.x,
        inner.y,
        text_width,
        &text,
        TuiStyle::default(),
    );

    for (action, label, area) in buttons {
        let style = if app.hovered_top_action == Some(action) {
            hover_style()
        } else {
            TuiStyle::default().fg(TuiColor::Rgb(170, 184, 194))
        };
        draw_text(frame, area.x, area.y, area.width, &label, style);
        app.top_action_areas.push(ButtonHit { action, area });
    }
}

fn draw_sidebar(frame: &mut Frame<'_>, app: &mut AppState, area: Rect) {
    let style = app.project.styles[app.current_style].clone();
    let tool = app.tool.label();

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

    let block = Block::default().title("Tools").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    app.tool_button_area = Rect::default();
    app.brush_button_area = Rect::default();
    app.color_palette_area = Rect::default();
    app.recent_color_palette_area = Rect::default();
    app.character_palette_area = Rect::default();
    app.more_colors_button_area = Rect::default();
    app.rgb_button_area = Rect::default();

    let mut y = inner.y;
    let max_y = inner.y.saturating_add(inner.height);
    let normal = TuiStyle::default();
    let strong = TuiStyle::default().add_modifier(Modifier::BOLD);

    app.tool_button_area = draw_sidebar_control_line(
        frame,
        inner,
        &mut y,
        max_y,
        format!("Tool: {tool} ▾"),
        app.hovered_tool_button,
    );
    app.brush_button_area = draw_sidebar_control_line(
        frame,
        inner,
        &mut y,
        max_y,
        format!("Brush: {}", brush_label(app.brush_char)),
        app.hovered_brush_button,
    );
    draw_sidebar_line(
        frame,
        inner,
        &mut y,
        max_y,
        format!(
            "Style {}/{}: {}",
            app.current_style + 1,
            app.project.styles.len(),
            style.id
        ),
        normal,
    );
    draw_sidebar_line(
        frame,
        inner,
        &mut y,
        max_y,
        format!("FG: {}", style.fg.to_hex()),
        TuiStyle::default().fg(TuiColor::Rgb(style.fg.r, style.fg.g, style.fg.b)),
    );
    draw_sidebar_line(frame, inner, &mut y, max_y, format!("BG: {bg}"), normal);
    draw_sidebar_line(
        frame,
        inner,
        &mut y,
        max_y,
        format!("Attrs: {attrs}"),
        normal,
    );
    draw_sidebar_spacer(&mut y, max_y);

    draw_sidebar_line(frame, inner, &mut y, max_y, "Colors".to_string(), strong);
    if y < max_y {
        let color_columns = palette_columns(inner.width, COLOR_SLOT_WIDTH);
        let color_rows = palette_rows(COLOR_PALETTE.len(), color_columns);
        let color_height = color_rows.min(max_y - y);
        app.color_palette_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: color_height,
        };
        let hovered_color = match app.hovered_palette {
            Some(PaletteHover::Color(index)) => Some(index),
            _ => None,
        };
        draw_color_palette(frame, app.color_palette_area, style.fg, hovered_color);
        y = y.saturating_add(color_height);
    }

    if !app.recent_extra_colors.is_empty() {
        draw_sidebar_spacer(&mut y, max_y);
        draw_sidebar_line(frame, inner, &mut y, max_y, "Recents".to_string(), strong);
        if y < max_y {
            let recent_columns = palette_columns(inner.width, COLOR_SLOT_WIDTH);
            let recent_rows = palette_rows(app.recent_extra_colors.len(), recent_columns);
            let recent_height = recent_rows.min(max_y - y).min(3);
            app.recent_color_palette_area = Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: recent_height,
            };
            draw_color_grid(
                frame,
                app.recent_color_palette_area,
                style.fg,
                app.hovered_recent_color,
                app.recent_extra_colors.len(),
                |index| app.recent_extra_colors[index],
            );
            y = y.saturating_add(recent_height);
        }
    }

    draw_sidebar_spacer(&mut y, max_y);
    app.more_colors_button_area = draw_sidebar_control_line(
        frame,
        inner,
        &mut y,
        max_y,
        "More colors...".to_string(),
        app.hovered_more_colors_button,
    );
    app.rgb_button_area = draw_sidebar_control_line(
        frame,
        inner,
        &mut y,
        max_y,
        "RGB input...".to_string(),
        app.hovered_rgb_button,
    );

    draw_sidebar_spacer(&mut y, max_y);
    draw_sidebar_line(
        frame,
        inner,
        &mut y,
        max_y,
        "Characters".to_string(),
        strong,
    );
    if y < max_y {
        let character_columns = palette_columns(inner.width, CHAR_SLOT_WIDTH);
        let character_rows = palette_rows(CHARACTER_PALETTE.len(), character_columns);
        let character_height = character_rows.min(max_y - y);
        app.character_palette_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: character_height,
        };
        let hovered_character = match app.hovered_palette {
            Some(PaletteHover::Character(index)) => Some(index),
            _ => None,
        };
        draw_character_palette(
            frame,
            app.character_palette_area,
            app.brush_char,
            hovered_character,
        );
        y = y.saturating_add(character_height);
    }

    draw_sidebar_spacer(&mut y, max_y);
    draw_sidebar_line(
        frame,
        inner,
        &mut y,
        max_y,
        "Click swatches/symbols".to_string(),
        normal,
    );
    draw_sidebar_line(
        frame,
        inner,
        &mut y,
        max_y,
        "M more | U RGB | F/G edit".to_string(),
        normal,
    );
}

fn draw_sidebar_line(
    frame: &mut Frame<'_>,
    area: Rect,
    y: &mut u16,
    max_y: u16,
    text: String,
    style: TuiStyle,
) {
    if *y >= max_y {
        return;
    }

    draw_text(frame, area.x, *y, area.width, &text, style);
    *y = y.saturating_add(1);
}

fn draw_sidebar_control_line(
    frame: &mut Frame<'_>,
    area: Rect,
    y: &mut u16,
    max_y: u16,
    text: String,
    hovered: bool,
) -> Rect {
    if *y >= max_y {
        return Rect::default();
    }

    let line_area = Rect {
        x: area.x,
        y: *y,
        width: area.width,
        height: 1,
    };
    let style = if hovered {
        hover_style()
    } else {
        TuiStyle::default().fg(TuiColor::Rgb(211, 220, 228))
    };
    fill_rect(frame, line_area, style);
    draw_text(
        frame,
        line_area.x,
        line_area.y,
        line_area.width,
        &text,
        style,
    );
    *y = y.saturating_add(1);

    line_area
}

fn draw_sidebar_spacer(y: &mut u16, max_y: u16) {
    if *y < max_y {
        *y = y.saturating_add(1);
    }
}

fn draw_color_palette(
    frame: &mut Frame<'_>,
    area: Rect,
    selected_color: Color,
    hovered_index: Option<usize>,
) {
    draw_color_grid(
        frame,
        area,
        selected_color,
        hovered_index,
        COLOR_PALETTE.len(),
        |index| COLOR_PALETTE[index].color,
    );
}

fn draw_color_grid(
    frame: &mut Frame<'_>,
    area: Rect,
    selected_color: Color,
    hovered_index: Option<usize>,
    color_count: usize,
    mut color_at: impl FnMut(usize) -> Color,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let columns = palette_columns(area.width, COLOR_SLOT_WIDTH);

    for index in 0..color_count {
        let Some((x, y)) = palette_position(area, columns, COLOR_SLOT_WIDTH, index) else {
            break;
        };
        let color = color_at(index);

        let style = TuiStyle::default().bg(TuiColor::Rgb(color.r, color.g, color.b));

        let selected = color == selected_color;
        let hovered = hovered_index == Some(index);
        let indicator_style = if selected && hovered {
            TuiStyle::default()
                .fg(TuiColor::Rgb(198, 160, 246))
                .add_modifier(Modifier::BOLD)
        } else if selected {
            TuiStyle::default()
                .fg(TuiColor::Rgb(245, 190, 82))
                .add_modifier(Modifier::BOLD)
        } else if hovered {
            TuiStyle::default()
                .fg(TuiColor::Rgb(79, 209, 197))
                .add_modifier(Modifier::BOLD)
        } else {
            TuiStyle::default()
        };
        let (left_indicator, right_indicator) = if selected {
            ("▶", "◀")
        } else if hovered {
            (">", "<")
        } else {
            (" ", " ")
        };

        frame.buffer_mut()[(x, y)]
            .set_symbol(left_indicator)
            .set_style(indicator_style);

        let swatch_start = x.saturating_add(1);
        for offset in 0..COLOR_SWATCH_WIDTH.min(area.width.saturating_sub(swatch_start - area.x)) {
            frame.buffer_mut()[(swatch_start + offset, y)]
                .set_symbol(" ")
                .set_style(style);
        }

        let right_indicator_x = swatch_start + COLOR_SWATCH_WIDTH;
        if right_indicator_x < area.x + area.width {
            frame.buffer_mut()[(right_indicator_x, y)]
                .set_symbol(right_indicator)
                .set_style(indicator_style);
        }
    }
}

fn draw_character_palette(
    frame: &mut Frame<'_>,
    area: Rect,
    selected_char: char,
    hovered_index: Option<usize>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let columns = palette_columns(area.width, CHAR_SLOT_WIDTH);

    for (index, ch) in CHARACTER_PALETTE.iter().enumerate() {
        let Some((x, y)) = palette_position(area, columns, CHAR_SLOT_WIDTH, index) else {
            break;
        };

        let selected = *ch == selected_char;
        let hovered = hovered_index == Some(index);
        let style = choice_style(selected, hovered);

        frame.buffer_mut()[(x, y)].set_symbol(" ").set_style(style);

        if x + 1 < area.x + area.width {
            frame.buffer_mut()[(x + 1, y)]
                .set_symbol(&palette_char_symbol(*ch))
                .set_style(style);
        }

        if x + 2 < area.x + area.width {
            frame.buffer_mut()[(x + 2, y)]
                .set_symbol(" ")
                .set_style(style);
        }
    }
}

fn palette_columns(width: u16, slot_width: u16) -> u16 {
    (width / slot_width).max(1)
}

fn palette_rows(item_count: usize, columns: u16) -> u16 {
    let columns = usize::from(columns.max(1));
    let rows = item_count.div_ceil(columns);
    u16::try_from(rows).unwrap_or(u16::MAX)
}

fn palette_position(area: Rect, columns: u16, slot_width: u16, index: usize) -> Option<(u16, u16)> {
    let columns = usize::from(columns.max(1));
    let row = index / columns;
    let column = index % columns;
    let x = area
        .x
        .checked_add(u16::try_from(column).ok()?.saturating_mul(slot_width))?;
    let y = area.y.checked_add(u16::try_from(row).ok()?)?;

    if x < area.x + area.width && y < area.y + area.height {
        Some((x, y))
    } else {
        None
    }
}

fn hit_palette_item(
    area: Rect,
    slot_width: u16,
    column: u16,
    row: u16,
    item_count: usize,
) -> Option<usize> {
    if area.width == 0
        || area.height == 0
        || column < area.x
        || row < area.y
        || column >= area.x + area.width
        || row >= area.y + area.height
    {
        return None;
    }

    let columns = palette_columns(area.width, slot_width);
    let local_x = column - area.x;
    let local_y = row - area.y;
    let item_column = local_x / slot_width;
    let index = usize::from(local_y) * usize::from(columns) + usize::from(item_column);

    (index < item_count).then_some(index)
}

fn rect_contains(area: Rect, column: u16, row: u16) -> bool {
    area.width > 0
        && area.height > 0
        && column >= area.x
        && row >= area.y
        && column < area.x + area.width
        && row < area.y + area.height
}

fn fill_rect(frame: &mut Frame<'_>, area: Rect, style: TuiStyle) {
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            frame.buffer_mut()[(x, y)].set_symbol(" ").set_style(style);
        }
    }
}

fn hover_style() -> TuiStyle {
    TuiStyle::default()
        .fg(TuiColor::Rgb(20, 45, 50))
        .bg(TuiColor::Rgb(79, 209, 197))
        .add_modifier(Modifier::BOLD)
}

fn choice_style(selected: bool, hovered: bool) -> TuiStyle {
    if selected && hovered {
        TuiStyle::default()
            .fg(TuiColor::Rgb(35, 32, 52))
            .bg(TuiColor::Rgb(198, 160, 246))
            .add_modifier(Modifier::BOLD)
    } else if selected {
        TuiStyle::default()
            .fg(TuiColor::Rgb(34, 39, 46))
            .bg(TuiColor::Rgb(245, 190, 82))
            .add_modifier(Modifier::BOLD)
    } else if hovered {
        hover_style()
    } else {
        TuiStyle::default().fg(TuiColor::Rgb(170, 184, 194))
    }
}

fn brush_label(ch: char) -> String {
    if ch == ' ' {
        "space".to_string()
    } else {
        format!("{ch:?}")
    }
}

fn palette_char_symbol(ch: char) -> String {
    if ch == ' ' {
        "·".to_string()
    } else {
        ch.to_string()
    }
}

fn draw_text(frame: &mut Frame<'_>, x: u16, y: u16, width: u16, text: &str, style: TuiStyle) {
    for (offset, ch) in text.chars().take(usize::from(width)).enumerate() {
        let cell_x = x + u16::try_from(offset).unwrap_or(u16::MAX);
        frame.buffer_mut()[(cell_x, y)]
            .set_symbol(&ch.to_string())
            .set_style(style);
    }
}

fn draw_text_centered(frame: &mut Frame<'_>, area: Rect, y: u16, text: &str, style: TuiStyle) {
    if y < area.y || y >= area.y + area.height {
        return;
    }

    let text_width = u16::try_from(text.chars().count()).unwrap_or(u16::MAX);
    let x = area.x + area.width.saturating_sub(text_width.min(area.width)) / 2;
    draw_text(
        frame,
        x,
        y,
        area.width.saturating_sub(x - area.x),
        text,
        style,
    );
}

fn color_cube_color(index: usize) -> Color {
    let steps = [0, 51, 102, 153, 204, 255];
    let index = index.min(MORE_COLOR_COUNT - 1);
    let r = steps[(index / 36) % 6];
    let g = steps[(index / 6) % 6];
    let b = steps[index % 6];
    Color { r, g, b }
}

fn is_visible_palette_color(color: Color) -> bool {
    COLOR_PALETTE
        .iter()
        .any(|palette_color| palette_color.color == color)
}

fn style_id_base_for_color(color: Color) -> String {
    format!("color-{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

fn parse_rgb_input(input: &str) -> Option<Color> {
    let trimmed = input.trim();
    if trimmed.starts_with('#') {
        return Color::parse_hex(trimmed);
    }

    let parts = trimmed
        .split(|ch: char| ch == ',' || ch == ';' || ch.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .map(str::parse::<u8>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;

    let [r, g, b]: [u8; 3] = parts.try_into().ok()?;
    Some(Color { r, g, b })
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
            let hovered = app.hovered_canvas_cell == Some((y, x));

            if let Some(cell) = app.project.first_frame().cells.get(&(y, x)) {
                let mut style = style_to_tui(&app.project.styles[cell.style_index]);
                if hovered {
                    style = style.bg(TuiColor::Rgb(33, 77, 85));
                }
                let symbol = cell.ch.to_string();
                buffer_cell.set_symbol(&symbol);
                buffer_cell.set_style(style);
            } else {
                let style = if hovered {
                    TuiStyle::default()
                        .fg(TuiColor::Rgb(79, 209, 197))
                        .bg(TuiColor::Rgb(24, 52, 58))
                } else {
                    TuiStyle::default().fg(TuiColor::DarkGray)
                };
                buffer_cell.set_symbol(".");
                buffer_cell.set_style(style);
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
    let help = "Click Tool for menu | Click top actions | Click palettes | Ctrl-Z/Y undo/redo";
    let lines = vec![
        Line::from(help),
        Line::from(app.message.as_str()),
        Line::from(canvas_status(app)),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().title("Status").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn canvas_status(app: &AppState) -> String {
    let Some((y, x)) = app.hovered_canvas_cell else {
        return format!(
            "Brush {} with {} | {}",
            brush_label(app.brush_char),
            app.project.styles[app.current_style].id,
            app.tool.description()
        );
    };

    if let Some(cell) = app.project.first_frame().cells.get(&(y, x)) {
        let style = &app.project.styles[cell.style_index];
        format!(
            "Cell {},{}: {} using {}",
            x,
            y,
            brush_label(cell.ch),
            style.id
        )
    } else {
        format!("Cell {x},{y}: transparent")
    }
}

fn draw_modal(frame: &mut Frame<'_>, app: &mut AppState) {
    let Some(modal) = app.modal.clone() else {
        return;
    };

    app.modal_tool_areas.clear();
    app.modal_action_areas.clear();
    app.modal_color_areas.clear();

    if matches!(modal, Modal::ToolMenu) {
        draw_tool_menu(frame, app);
        return;
    }

    if matches!(modal, Modal::ColorPicker) {
        draw_color_picker(frame, app);
        return;
    }

    let area = centered_rect(72, 9, frame.area());
    app.modal_area = area;
    frame.render_widget(Clear, area);
    let is_quit_confirm = matches!(modal, Modal::QuitConfirm);

    let (title, body) = match modal {
        Modal::NewImage { input, .. } => ("New Image", format!("Dimensions: {input}")),
        Modal::OpenFile { input } => ("Open File", format!("Path: {input}")),
        Modal::SaveAs { input } => ("Save As", format!("Path: {input}")),
        Modal::BrushChar { input } => ("Brush Character", format!("Character: {input}")),
        Modal::NewStyle { input } => ("New Style", format!("Style ID: {input}")),
        Modal::RenameStyle { input } => ("Rename Style", format!("Style ID: {input}")),
        Modal::SetFg { input } => ("Foreground", format!("Color: {input}")),
        Modal::SetBg { input } => ("Background", format!("Color: {input}")),
        Modal::RgbInput { input } => ("RGB Input", format!("RGB or hex: {input}")),
        Modal::ExportText { input } => ("Export Text", format!("Path: {input}")),
        Modal::ColorPicker => unreachable!("color picker is drawn separately"),
        Modal::ToolMenu => unreachable!("tool menu is drawn separately"),
        Modal::QuitConfirm => (
            "Unsaved Changes",
            "Save changes before closing?".to_string(),
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

    let actions: &[ModalAction] = if is_quit_confirm {
        &[
            ModalAction::SaveAndQuit,
            ModalAction::Discard,
            ModalAction::Cancel,
        ]
    } else {
        &[ModalAction::Confirm, ModalAction::Cancel]
    };
    draw_modal_buttons(frame, app, area, actions);
}

fn draw_color_picker(frame: &mut Frame<'_>, app: &mut AppState) {
    let area = centered_rect(76, 24, frame.area());
    app.modal_area = area;
    frame.render_widget(Clear, area);

    let block = Block::default().title("More Colors").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    draw_text(
        frame,
        inner.x,
        inner.y,
        inner.width,
        "Click a color. Wheel/PageUp/PageDown scroll. U opens RGB input.",
        TuiStyle::default().fg(TuiColor::Rgb(170, 184, 194)),
    );

    let grid_area = app.color_picker_grid_area();
    let columns = palette_columns(grid_area.width, COLOR_SLOT_WIDTH).max(1);
    let visible_rows = grid_area.height;
    let start_index = app.color_picker_scroll_row * usize::from(columns);
    let visible_count = usize::from(columns) * usize::from(visible_rows);
    let selected_color = app.project.styles[app.current_style].fg;
    let hovered_visible_index = app
        .hovered_modal_color
        .and_then(|index| index.checked_sub(start_index))
        .filter(|index| *index < visible_count);

    draw_color_grid(
        frame,
        grid_area,
        selected_color,
        hovered_visible_index,
        visible_count.min(MORE_COLOR_COUNT.saturating_sub(start_index)),
        |offset| color_cube_color(start_index + offset),
    );

    for offset in 0..visible_count.min(MORE_COLOR_COUNT.saturating_sub(start_index)) {
        let Some((x, y)) = palette_position(grid_area, columns, COLOR_SLOT_WIDTH, offset) else {
            break;
        };
        app.modal_color_areas.push(ButtonHit {
            action: start_index + offset,
            area: Rect {
                x,
                y,
                width: COLOR_SLOT_WIDTH.min(grid_area.x + grid_area.width - x),
                height: 1,
            },
        });
    }

    let total_rows = MORE_COLOR_COUNT.div_ceil(usize::from(columns));
    let scroll = format!(
        "Rows {}/{}",
        app.color_picker_scroll_row + 1,
        total_rows.max(1)
    );
    draw_text(
        frame,
        inner.x,
        inner.y + inner.height.saturating_sub(2),
        inner.width,
        &scroll,
        TuiStyle::default().fg(TuiColor::Rgb(211, 220, 228)),
    );

    draw_modal_buttons(
        frame,
        app,
        area,
        &[ModalAction::RgbInput, ModalAction::Cancel],
    );
}

fn draw_modal_buttons(
    frame: &mut Frame<'_>,
    app: &mut AppState,
    modal_area: Rect,
    actions: &[ModalAction],
) {
    if actions.is_empty() || modal_area.width < 4 || modal_area.height < 3 {
        return;
    }

    let total_width: u16 = actions
        .iter()
        .map(|action| u16::try_from(action.label().chars().count()).unwrap_or(0) + 4)
        .sum::<u16>()
        .saturating_add(u16::try_from(actions.len().saturating_sub(1)).unwrap_or(0));
    let mut x = modal_area.x + modal_area.width.saturating_sub(total_width) / 2;
    let y = modal_area.y + modal_area.height.saturating_sub(2);

    for action in actions {
        let width = u16::try_from(action.label().chars().count()).unwrap_or(0) + 4;
        let area = Rect {
            x,
            y,
            width,
            height: 1,
        };
        let style = choice_style(false, app.hovered_modal_action == Some(*action));
        fill_rect(frame, area, style);
        draw_text_centered(frame, area, y, action.label(), style);
        app.modal_action_areas.push(ButtonHit {
            action: *action,
            area,
        });
        x = x.saturating_add(width + 1);
    }
}

fn draw_tool_menu(frame: &mut Frame<'_>, app: &mut AppState) {
    let area = centered_rect(62, 10, frame.area());
    app.modal_area = area;
    frame.render_widget(Clear, area);

    let block = Block::default().title("Choose Tool").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    draw_text(
        frame,
        inner.x,
        inner.y,
        inner.width,
        "Click a row or press P, E, I",
        TuiStyle::default().fg(TuiColor::Rgb(170, 184, 194)),
    );

    let mut y = inner.y.saturating_add(2);
    for (index, tool) in TOOL_CHOICES.iter().enumerate() {
        if y >= inner.y + inner.height {
            break;
        }

        let row = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        let selected = app.tool == *tool;
        let hovered = app.hovered_modal_tool == Some(*tool);
        let style = choice_style(selected, hovered);
        fill_rect(frame, row, style);

        let shortcut = match tool {
            Tool::Pencil => "P",
            Tool::Eraser => "E",
            Tool::Eyedropper => "I",
        };
        let label = format!(
            "{}  {}  {:<10} {}",
            index + 1,
            shortcut,
            tool.label(),
            tool.description()
        );
        draw_text(frame, row.x, row.y, row.width, &label, style);
        app.modal_tool_areas.push(ButtonHit {
            action: *tool,
            area: row,
        });
        y = y.saturating_add(1);
    }
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
        | Modal::RgbInput { input }
        | Modal::ExportText { input } => Some(input),
        Modal::ColorPicker | Modal::ToolMenu | Modal::QuitConfirm => None,
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

fn unique_style_id(project: &Project, base: &str) -> String {
    if !project.styles.iter().any(|style| style.id == base) {
        return base.to_string();
    }

    for index in 2.. {
        let candidate = format!("{base}-{index}");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_palette_contains_only_valid_v1_characters() {
        for ch in CHARACTER_PALETTE {
            assert!(is_valid_v1_character(*ch), "{ch:?} should be V1-valid");
        }
    }

    #[test]
    fn visible_color_palette_has_been_expanded() {
        assert!(COLOR_PALETTE.len() >= 24);
    }

    #[test]
    fn selecting_palette_color_creates_new_tool_style_without_recoloring_cells() {
        let mut app = AppState::editor(Project::new_image("palette", 4, 2), None, false, "");
        app.project.first_frame_mut().cells.insert(
            (0, 0),
            PaintedCell {
                ch: '#',
                style_index: 0,
            },
        );
        let original_style = app.project.styles[0].clone();
        let red = COLOR_PALETTE
            .iter()
            .find(|palette_color| palette_color.name == "red")
            .copied()
            .expect("red palette entry");

        app.select_palette_color(red);

        assert_eq!(app.project.styles[0], original_style);
        assert_eq!(app.project.first_frame().cells[&(0, 0)].style_index, 0);
        assert_eq!(app.project.styles[app.current_style].fg, red.color);
        assert_eq!(app.tool, Tool::Pencil);
        assert!(app.dirty);
    }

    #[test]
    fn palette_hit_testing_maps_cells_to_items() {
        let area = Rect {
            x: 10,
            y: 5,
            width: 8,
            height: 2,
        };

        assert_eq!(hit_palette_item(area, COLOR_SLOT_WIDTH, 10, 5, 4), Some(0));
        assert_eq!(hit_palette_item(area, COLOR_SLOT_WIDTH, 14, 5, 4), Some(1));
        assert_eq!(hit_palette_item(area, COLOR_SLOT_WIDTH, 10, 6, 4), Some(2));
        assert_eq!(hit_palette_item(area, COLOR_SLOT_WIDTH, 14, 6, 4), Some(3));
        assert_eq!(hit_palette_item(area, COLOR_SLOT_WIDTH, 18, 6, 4), None);
    }

    #[test]
    fn palette_hover_tracks_color_and_character_items() {
        let mut app = AppState::editor(Project::new_image("hover", 4, 2), None, false, "");
        app.color_palette_area = Rect {
            x: 1,
            y: 1,
            width: 8,
            height: 2,
        };
        app.character_palette_area = Rect {
            x: 1,
            y: 5,
            width: 9,
            height: 2,
        };

        app.update_hover(5, 1);
        assert_eq!(app.hovered_palette, Some(PaletteHover::Color(1)));

        app.update_hover(7, 5);
        assert_eq!(app.hovered_palette, Some(PaletteHover::Character(2)));

        app.update_hover(30, 30);
        assert_eq!(app.hovered_palette, None);
    }

    #[test]
    fn hover_tracks_toolbar_controls_and_canvas_cells() {
        let mut app = AppState::editor(Project::new_image("hover", 4, 2), None, false, "");
        app.top_action_areas.push(ButtonHit {
            action: TopAction::Save,
            area: Rect {
                x: 1,
                y: 1,
                width: 6,
                height: 1,
            },
        });
        app.tool_button_area = Rect {
            x: 1,
            y: 3,
            width: 10,
            height: 1,
        };
        app.canvas_area = Rect {
            x: 20,
            y: 5,
            width: 4,
            height: 2,
        };

        app.update_hover(2, 1);
        assert_eq!(app.hovered_top_action, Some(TopAction::Save));

        app.update_hover(3, 3);
        assert!(app.hovered_tool_button);

        app.update_hover(22, 6);
        assert_eq!(app.hovered_canvas_cell, Some((1, 2)));
    }

    #[test]
    fn tool_menu_selection_updates_tool_and_closes_modal() {
        let mut app = AppState::editor(Project::new_image("tool", 4, 2), None, false, "");
        app.modal = Some(Modal::ToolMenu);

        app.select_tool_from_menu(Tool::Eraser);

        assert_eq!(app.tool, Tool::Eraser);
        assert!(app.modal.is_none());
        assert_eq!(app.message, "Eraser selected");
    }

    #[test]
    fn rgb_input_accepts_comma_space_and_hex_forms() {
        assert_eq!(
            parse_rgb_input("255,128,0"),
            Some(Color {
                r: 255,
                g: 128,
                b: 0
            })
        );
        assert_eq!(
            parse_rgb_input("12 34 56"),
            Some(Color {
                r: 12,
                g: 34,
                b: 56
            })
        );
        assert_eq!(
            parse_rgb_input("#0A0B0C"),
            Some(Color {
                r: 10,
                g: 11,
                b: 12
            })
        );
        assert_eq!(parse_rgb_input("255,0"), None);
    }

    #[test]
    fn selecting_hidden_picker_color_adds_recent() {
        let mut app = AppState::editor(Project::new_image("recent", 4, 2), None, false, "");
        let color = Color {
            r: 51,
            g: 102,
            b: 153,
        };
        assert!(!is_visible_palette_color(color));

        app.select_custom_color(color, true);
        app.select_custom_color(color, true);

        assert_eq!(app.recent_extra_colors, vec![color]);
        assert_eq!(app.project.styles[app.current_style].fg, color);
    }

    #[test]
    fn welcome_actions_open_expected_flows() {
        let mut app = AppState::welcome("");

        app.run_welcome_action(WelcomeAction::NewImage);
        assert!(matches!(app.modal, Some(Modal::NewImage { .. })));

        app.close_modal("");
        app.run_welcome_action(WelcomeAction::OpenFile);
        assert!(matches!(app.modal, Some(Modal::OpenFile { .. })));
    }
}
