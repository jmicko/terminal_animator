use crate::format::{
    AssetKind, Color, FormatError, Frame as ProjectFrame, MAX_AREA_PER_FRAME, MAX_FRAMES,
    MAX_HEIGHT, MAX_STYLES, MAX_WIDTH, PaintedCell, Project, TerminalStyle, TextAttr,
    export_plain_text, is_valid_v1_character, load_project_from_path, save_project_to_path,
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
const EXPANDED_COLOR_COLUMNS: u16 = 12;
const EXPANDED_HUE_COUNT: usize = 12;
const EXPANDED_TONE_COUNT: usize = 16;
const EXPANDED_NEUTRAL_COUNT: usize = 36;
const MORE_COLOR_COUNT: usize = EXPANDED_HUE_COUNT * EXPANDED_TONE_COUNT + EXPANDED_NEUTRAL_COUNT;
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
    PreviousFrame,
    NextFrame,
    DuplicateFrame,
    BlankFrame,
    ToggleOnionSkin,
    Save,
    SaveAs,
    ExportText,
    Quit,
}

impl TopAction {
    fn label(self) -> &'static str {
        match self {
            Self::PreviousFrame => "Prev",
            Self::NextFrame => "Next",
            Self::DuplicateFrame => "Dup",
            Self::BlankFrame => "Blank",
            Self::ToggleOnionSkin => "Onion",
            Self::Save => "Save",
            Self::SaveAs => "Save As",
            Self::ExportText => "Export",
            Self::Quit => "Quit",
        }
    }
}

const TOP_ACTIONS: &[TopAction] = &[
    TopAction::PreviousFrame,
    TopAction::NextFrame,
    TopAction::DuplicateFrame,
    TopAction::BlankFrame,
    TopAction::ToggleOnionSkin,
    TopAction::Save,
    TopAction::SaveAs,
    TopAction::ExportText,
    TopAction::Quit,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WelcomeAction {
    NewImage,
    NewAnimation,
    OpenFile,
    Quit,
}

impl WelcomeAction {
    fn label(self) -> &'static str {
        match self {
            Self::NewImage => "New Image",
            Self::NewAnimation => "New Animation",
            Self::OpenFile => "Open File",
            Self::Quit => "Quit",
        }
    }
}

const WELCOME_ACTIONS: &[WelcomeAction] = &[
    WelcomeAction::NewImage,
    WelcomeAction::NewAnimation,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RgbChannel {
    Red,
    Green,
    Blue,
}

impl RgbChannel {
    fn short_label(self) -> &'static str {
        match self {
            Self::Red => "R",
            Self::Green => "G",
            Self::Blue => "B",
        }
    }

    fn value(self, color: Color) -> u8 {
        match self {
            Self::Red => color.r,
            Self::Green => color.g,
            Self::Blue => color.b,
        }
    }

    fn set_value(self, color: &mut Color, value: u8) {
        match self {
            Self::Red => color.r = value,
            Self::Green => color.g = value,
            Self::Blue => color.b = value,
        }
    }
}

const RGB_CHANNELS: &[RgbChannel] = &[RgbChannel::Red, RgbChannel::Green, RgbChannel::Blue];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RgbControl {
    Decrement(RgbChannel),
    Slider(RgbChannel),
    Increment(RgbChannel),
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
    NewAnimation {
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
        color: Color,
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
    frame_index: usize,
    changes: BTreeMap<(u16, u16), CellChange>,
}

impl Stroke {
    fn new(frame_index: usize) -> Self {
        Self {
            frame_index,
            changes: BTreeMap::new(),
        }
    }
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
    current_frame_index: usize,
    onion_skin: bool,
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
    modal_rgb_areas: Vec<ButtonHit<RgbControl>>,
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
    hovered_rgb_control: Option<RgbControl>,
    dragging_rgb_channel: Option<RgbChannel>,
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
            current_frame_index: 0,
            onion_skin: false,
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
            modal_rgb_areas: Vec::new(),
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
            hovered_rgb_control: None,
            dragging_rgb_channel: None,
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
            current_frame_index: 0,
            onion_skin: false,
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
            modal_rgb_areas: Vec::new(),
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
            hovered_rgb_control: None,
            dragging_rgb_channel: None,
            hovered_canvas_cell: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            active_stroke: None,
            modal: None,
            message: message.into(),
            should_quit: false,
        }
    }

    fn current_frame(&self) -> &ProjectFrame {
        &self.project.frames[self.current_frame_index.min(self.project.frames.len() - 1)]
    }

    fn current_frame_mut(&mut self) -> &mut ProjectFrame {
        let frame_index = self.current_frame_index.min(self.project.frames.len() - 1);
        &mut self.project.frames[frame_index]
    }

    fn previous_frame(&self) -> Option<&ProjectFrame> {
        self.current_frame_index
            .checked_sub(1)
            .and_then(|index| self.project.frames.get(index))
    }

    fn normalize_frame_kind(&mut self) {
        if self.project.frames.len() > 1 {
            self.project.asset.kind = AssetKind::Animation;
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
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.modal = Some(Modal::NewAnimation {
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
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Right => {
                self.next_frame_or_create();
            }
            KeyCode::Left => {
                self.previous_frame_or_message();
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.duplicate_current_frame();
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                self.insert_blank_frame();
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                self.toggle_onion_skin();
            }
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

        if matches!(self.modal, Some(Modal::RgbInput { .. })) {
            match key.code {
                KeyCode::Esc => self.close_modal("RGB canceled"),
                KeyCode::Enter => {
                    if let Some(modal) = self.modal.take() {
                        self.commit_modal(modal);
                    }
                }
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
            Modal::NewAnimation { input, target_path } => match parse_dimensions(&input) {
                Some((width, height)) => {
                    let name = target_path
                        .as_deref()
                        .map(asset_name_from_path)
                        .unwrap_or_else(|| "untitled".to_string());
                    let mut project = Project::new_image(name, width, height);
                    project.asset.kind = AssetKind::Animation;
                    *self = Self::editor(
                        project,
                        target_path,
                        true,
                        format!("Created {width}x{height} animation"),
                    );
                }
                None => {
                    self.message = "Enter dimensions as WIDTHxHEIGHT, like 48x16".to_string();
                    self.modal = Some(Modal::NewAnimation { input, target_path });
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
            Modal::RgbInput { color } => {
                self.select_custom_color(color, false);
                self.message = format!("Selected RGB {}", color.to_hex());
            }
            Modal::ExportText { input } => {
                let path = PathBuf::from(input.trim());
                if path.as_os_str().is_empty() {
                    self.message = "Export path cannot be empty".to_string();
                    self.modal = Some(Modal::ExportText { input });
                    return;
                }

                match fs::write(
                    &path,
                    export_plain_text(&self.project, self.current_frame_index),
                ) {
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
                self.active_stroke = Some(Stroke::new(self.current_frame_index));
                self.apply_tool_at_screen(mouse.column, mouse.row);
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.active_stroke.is_none() {
                    self.active_stroke = Some(Stroke::new(self.current_frame_index));
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
                        let color = expanded_palette_color(index);
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

        if matches!(self.modal, Some(Modal::RgbInput { .. })) {
            self.handle_rgb_mouse(mouse);
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

    fn handle_rgb_mouse(&mut self, mouse: MouseEvent) {
        self.hovered_rgb_control = self.modal_rgb_control_at(mouse.column, mouse.row);
        self.hovered_modal_action = self.modal_action_at(mouse.column, mouse.row);

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(control) = self.hovered_rgb_control {
                    self.apply_rgb_control(control, mouse.column);
                } else if let Some(action) = self.hovered_modal_action {
                    self.run_modal_action(action);
                } else if !rect_contains(self.modal_area, mouse.column, mouse.row) {
                    self.close_modal("RGB canceled");
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(channel) = self.dragging_rgb_channel {
                    self.set_rgb_channel_from_slider(channel, mouse.column);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.dragging_rgb_channel = None;
            }
            _ => {}
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

    fn modal_rgb_control_at(&self, column: u16, row: u16) -> Option<RgbControl> {
        self.modal_rgb_areas
            .iter()
            .find(|hit| rect_contains(hit.area, column, row))
            .map(|hit| hit.action)
    }

    fn apply_rgb_control(&mut self, control: RgbControl, column: u16) {
        match control {
            RgbControl::Decrement(channel) => self.adjust_rgb_channel(channel, -1),
            RgbControl::Increment(channel) => self.adjust_rgb_channel(channel, 1),
            RgbControl::Slider(channel) => {
                self.dragging_rgb_channel = Some(channel);
                self.set_rgb_channel_from_slider(channel, column);
            }
        }
    }

    fn adjust_rgb_channel(&mut self, channel: RgbChannel, delta: i16) {
        let hex = if let Some(Modal::RgbInput { color }) = self.modal.as_mut() {
            let value = i16::from(channel.value(*color))
                .saturating_add(delta)
                .clamp(0, 255) as u8;
            channel.set_value(color, value);
            Some(color.to_hex())
        } else {
            None
        };

        if let Some(hex) = hex {
            self.message = format!("RGB preview {hex}");
        }
    }

    fn set_rgb_channel_from_slider(&mut self, channel: RgbChannel, column: u16) {
        let Some(area) = self
            .modal_rgb_areas
            .iter()
            .find(|hit| hit.action == RgbControl::Slider(channel))
            .map(|hit| hit.area)
        else {
            return;
        };
        let value = rgb_slider_value_at(area, column);
        let hex = if let Some(Modal::RgbInput { color }) = self.modal.as_mut() {
            channel.set_value(color, value);
            Some(color.to_hex())
        } else {
            None
        };

        if let Some(hex) = hex {
            self.message = format!("RGB preview {hex}");
        }
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
                if let Some(cell) = self.current_frame().cells.get(&(y, x)).cloned() {
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
        let frame_index = self.current_frame_index;
        let should_restart_stroke = self
            .active_stroke
            .as_ref()
            .is_some_and(|stroke| stroke.frame_index != frame_index);
        if should_restart_stroke {
            self.finish_stroke();
        }
        if self.active_stroke.is_none() {
            self.active_stroke = Some(Stroke::new(frame_index));
        }

        let before = self.current_frame().cells.get(&(y, x)).cloned();
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
                self.current_frame_mut().cells.insert((y, x), cell);
            }
            None => {
                self.current_frame_mut().cells.remove(&(y, x));
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

        let frame_index = stroke.frame_index.min(self.project.frames.len() - 1);
        self.current_frame_index = frame_index;
        let frame = &mut self.project.frames[frame_index];
        for (&(y, x), change) in &stroke.changes {
            match &change.before {
                Some(cell) => {
                    frame.cells.insert((y, x), cell.clone());
                }
                None => {
                    frame.cells.remove(&(y, x));
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

        let frame_index = stroke.frame_index.min(self.project.frames.len() - 1);
        self.current_frame_index = frame_index;
        let frame = &mut self.project.frames[frame_index];
        for (&(y, x), change) in &stroke.changes {
            match &change.after {
                Some(cell) => {
                    frame.cells.insert((y, x), cell.clone());
                }
                None => {
                    frame.cells.remove(&(y, x));
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

    fn previous_frame_or_message(&mut self) {
        self.finish_stroke();
        if self.current_frame_index == 0 {
            self.message = "Already on first frame".to_string();
            return;
        }

        self.current_frame_index -= 1;
        self.message = self.frame_position_message("Frame");
    }

    fn next_frame_or_create(&mut self) {
        self.finish_stroke();
        if self.current_frame_index + 1 < self.project.frames.len() {
            self.current_frame_index += 1;
            self.message = self.frame_position_message("Frame");
            return;
        }

        self.insert_frame_after_current(self.current_frame().cells.clone(), "Added frame");
    }

    fn duplicate_current_frame(&mut self) {
        self.finish_stroke();
        self.insert_frame_after_current(self.current_frame().cells.clone(), "Duplicated frame");
    }

    fn insert_blank_frame(&mut self) {
        self.finish_stroke();
        self.insert_frame_after_current(BTreeMap::new(), "Added blank frame");
    }

    fn insert_frame_after_current(
        &mut self,
        cells: BTreeMap<(u16, u16), PaintedCell>,
        action: &'static str,
    ) {
        if self.project.frames.len() >= MAX_FRAMES {
            self.message = format!("Cannot add frame: limit is {MAX_FRAMES}");
            return;
        }

        let insert_index = self.current_frame_index + 1;
        let frame = ProjectFrame {
            id: Some(next_frame_id(&self.project)),
            duration_ms: self.project.asset.default_frame_duration_ms,
            cells,
        };
        self.project.frames.insert(insert_index, frame);
        self.current_frame_index = insert_index;
        self.normalize_frame_kind();
        self.dirty = true;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.message = self.frame_position_message(action);
    }

    fn toggle_onion_skin(&mut self) {
        self.onion_skin = !self.onion_skin;
        self.message = if self.onion_skin {
            "Onion skin on".to_string()
        } else {
            "Onion skin off".to_string()
        };
    }

    fn frame_position_message(&self, prefix: &str) -> String {
        format!(
            "{prefix} {}/{}",
            self.current_frame_index + 1,
            self.project.frames.len()
        )
    }

    fn run_top_action(&mut self, action: TopAction) {
        match action {
            TopAction::PreviousFrame => self.previous_frame_or_message(),
            TopAction::NextFrame => self.next_frame_or_create(),
            TopAction::DuplicateFrame => self.duplicate_current_frame(),
            TopAction::BlankFrame => self.insert_blank_frame(),
            TopAction::ToggleOnionSkin => self.toggle_onion_skin(),
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
            WelcomeAction::NewAnimation => {
                self.modal = Some(Modal::NewAnimation {
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
        self.hovered_rgb_control = None;
        self.dragging_rgb_channel = None;
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
        self.modal = Some(Modal::RgbInput { color });
        self.hovered_rgb_control = None;
        self.dragging_rgb_channel = None;
        self.message = "Adjust RGB with sliders".to_string();
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
            .clamp(1, EXPANDED_COLOR_COLUMNS)
    }

    fn color_picker_visible_rows(&self) -> u16 {
        self.color_picker_grid_area().height
    }

    fn color_picker_grid_area(&self) -> Rect {
        if self.modal_area.width < 4 || self.modal_area.height < 7 {
            return Rect::default();
        }

        let width = self
            .modal_area
            .width
            .saturating_sub(4)
            .min(EXPANDED_COLOR_COLUMNS.saturating_mul(COLOR_SLOT_WIDTH));

        Rect {
            x: self
                .modal_area
                .x
                .saturating_add(self.modal_area.width.saturating_sub(width) / 2),
            y: self.modal_area.y + 3,
            width,
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
    let kind = match app.project.asset.kind {
        AssetKind::Image => "image",
        AssetKind::Animation => "animation",
    };
    let onion = if app.onion_skin {
        "onion on"
    } else {
        "onion off"
    };
    let text = format!(
        " {}{} | {}x{} | {} | frame {}/{} | {} cells | {}",
        path,
        dirty,
        app.project.asset.width,
        app.project.asset.height,
        kind,
        app.current_frame_index + 1,
        app.project.frames.len(),
        app.current_frame().cells.len(),
        onion
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

fn expanded_palette_color(index: usize) -> Color {
    let index = index.min(MORE_COLOR_COUNT - 1);

    if index < EXPANDED_HUE_COUNT * EXPANDED_TONE_COUNT {
        const HUES: [f32; EXPANDED_HUE_COUNT] = [
            0.0, 24.0, 42.0, 58.0, 86.0, 126.0, 156.0, 184.0, 204.0, 226.0, 268.0, 314.0,
        ];
        const TONES: [(f32, f32); EXPANDED_TONE_COUNT] = [
            (0.68, 0.18),
            (0.72, 0.25),
            (0.76, 0.33),
            (0.80, 0.42),
            (0.84, 0.51),
            (0.82, 0.60),
            (0.74, 0.69),
            (0.64, 0.78),
            (0.54, 0.86),
            (0.44, 0.92),
            (0.34, 0.30),
            (0.42, 0.40),
            (0.50, 0.50),
            (0.46, 0.62),
            (0.38, 0.74),
            (0.30, 0.84),
        ];

        let hue = HUES[index % EXPANDED_HUE_COUNT];
        let (saturation, lightness) = TONES[index / EXPANDED_HUE_COUNT];
        return hsl_to_rgb(hue, saturation, lightness);
    }

    neutral_palette_color(index - (EXPANDED_HUE_COUNT * EXPANDED_TONE_COUNT))
}

fn neutral_palette_color(index: usize) -> Color {
    let row = index / usize::from(EXPANDED_COLOR_COLUMNS);
    let column = index % usize::from(EXPANDED_COLOR_COLUMNS);

    match row {
        0 => {
            let value = lerp_u8(18, 242, column, usize::from(EXPANDED_COLOR_COLUMNS) - 1);
            Color {
                r: value,
                g: value,
                b: value,
            }
        }
        1 => lerp_color(
            Color {
                r: 55,
                g: 38,
                b: 27,
            },
            Color {
                r: 244,
                g: 222,
                b: 186,
            },
            column,
            usize::from(EXPANDED_COLOR_COLUMNS) - 1,
        ),
        _ => lerp_color(
            Color {
                r: 30,
                g: 43,
                b: 54,
            },
            Color {
                r: 222,
                g: 238,
                b: 245,
            },
            column,
            usize::from(EXPANDED_COLOR_COLUMNS) - 1,
        ),
    }
}

fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> Color {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue_section = hue / 60.0;
    let x = chroma * (1.0 - (hue_section % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hue_section as u8 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;

    Color {
        r: float_channel_to_u8(r1 + m),
        g: float_channel_to_u8(g1 + m),
        b: float_channel_to_u8(b1 + m),
    }
}

fn float_channel_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn lerp_color(start: Color, end: Color, step: usize, max_step: usize) -> Color {
    Color {
        r: lerp_u8(start.r, end.r, step, max_step),
        g: lerp_u8(start.g, end.g, step, max_step),
        b: lerp_u8(start.b, end.b, step, max_step),
    }
}

fn lerp_u8(start: u8, end: u8, step: usize, max_step: usize) -> u8 {
    if max_step == 0 {
        return start;
    }

    let start = i32::from(start);
    let end = i32::from(end);
    let delta = end - start;
    (start + delta * i32::try_from(step).unwrap_or(0) / i32::try_from(max_step).unwrap_or(1))
        .clamp(0, 255) as u8
}

fn is_visible_palette_color(color: Color) -> bool {
    COLOR_PALETTE
        .iter()
        .any(|palette_color| palette_color.color == color)
}

fn style_id_base_for_color(color: Color) -> String {
    format!("color-{:02x}{:02x}{:02x}", color.r, color.g, color.b)
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

            if let Some(cell) = app.current_frame().cells.get(&(y, x)) {
                let mut style = style_to_tui(&app.project.styles[cell.style_index]);
                if hovered {
                    style = style.bg(TuiColor::Rgb(33, 77, 85));
                }
                let symbol = cell.ch.to_string();
                buffer_cell.set_symbol(&symbol);
                buffer_cell.set_style(style);
            } else if app.onion_skin
                && let Some(cell) = app
                    .previous_frame()
                    .and_then(|previous| previous.cells.get(&(y, x)))
            {
                let mut style = style_to_tui(&app.project.styles[cell.style_index])
                    .fg(TuiColor::Rgb(98, 123, 132))
                    .add_modifier(Modifier::DIM);
                if hovered {
                    style = style.bg(TuiColor::Rgb(24, 52, 58));
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
    let help = "Click Tool/top actions/palettes | N/Right next | D duplicate | B blank | O onion";
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

    if let Some(cell) = app.current_frame().cells.get(&(y, x)) {
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
    app.modal_rgb_areas.clear();

    if matches!(modal, Modal::ToolMenu) {
        draw_tool_menu(frame, app);
        return;
    }

    if matches!(modal, Modal::ColorPicker) {
        draw_color_picker(frame, app);
        return;
    }

    if let Modal::RgbInput { color } = modal {
        draw_rgb_picker(frame, app, color);
        return;
    }

    let area = centered_rect(72, 9, frame.area());
    app.modal_area = area;
    frame.render_widget(Clear, area);
    let is_quit_confirm = matches!(modal, Modal::QuitConfirm);

    let (title, body) = match modal {
        Modal::NewImage { input, .. } => ("New Image", format!("Dimensions: {input}")),
        Modal::NewAnimation { input, .. } => ("New Animation", format!("Dimensions: {input}")),
        Modal::OpenFile { input } => ("Open File", format!("Path: {input}")),
        Modal::SaveAs { input } => ("Save As", format!("Path: {input}")),
        Modal::BrushChar { input } => ("Brush Character", format!("Character: {input}")),
        Modal::NewStyle { input } => ("New Style", format!("Style ID: {input}")),
        Modal::RenameStyle { input } => ("Rename Style", format!("Style ID: {input}")),
        Modal::SetFg { input } => ("Foreground", format!("Color: {input}")),
        Modal::SetBg { input } => ("Background", format!("Color: {input}")),
        Modal::ExportText { input } => ("Export Text", format!("Path: {input}")),
        Modal::ColorPicker => unreachable!("color picker is drawn separately"),
        Modal::RgbInput { .. } => unreachable!("RGB picker is drawn separately"),
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

fn draw_rgb_picker(frame: &mut Frame<'_>, app: &mut AppState, color: Color) {
    let area = centered_rect(76, 18, frame.area());
    app.modal_area = area;
    frame.render_widget(Clear, area);

    let block = Block::default().title("RGB Mixer").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    draw_text(
        frame,
        inner.x,
        inner.y,
        inner.width,
        "Click or drag a slider. Use +/- for fine changes.",
        TuiStyle::default().fg(TuiColor::Rgb(170, 184, 194)),
    );

    let compact = inner.width < 54;
    let preview = if compact {
        Rect {
            x: inner.x,
            y: inner.y.saturating_add(2),
            width: inner.width.min(20),
            height: 4,
        }
    } else {
        Rect {
            x: inner.x,
            y: inner.y.saturating_add(2),
            width: 16,
            height: 6,
        }
    };
    draw_rgb_preview(frame, preview, color);

    let controls = if compact {
        Rect {
            x: inner.x,
            y: preview.y.saturating_add(preview.height).saturating_add(1),
            width: inner.width,
            height: inner.height.saturating_sub(8),
        }
    } else {
        Rect {
            x: inner.x.saturating_add(19),
            y: inner.y.saturating_add(2),
            width: inner.width.saturating_sub(19),
            height: 9,
        }
    };

    for (offset, channel) in RGB_CHANNELS.iter().enumerate() {
        let row = Rect {
            x: controls.x,
            y: controls
                .y
                .saturating_add(u16::try_from(offset).unwrap_or(0).saturating_mul(3)),
            width: controls.width,
            height: 1,
        };
        draw_rgb_channel_row(frame, app, row, *channel, color);
    }

    let hex_line = format!(
        "{}   RGB {}, {}, {}",
        color.to_hex(),
        color.r,
        color.g,
        color.b
    );
    draw_text(
        frame,
        inner.x,
        inner.y + inner.height.saturating_sub(3),
        inner.width,
        &hex_line,
        TuiStyle::default().fg(TuiColor::Rgb(211, 220, 228)),
    );

    draw_modal_buttons(
        frame,
        app,
        area,
        &[ModalAction::Confirm, ModalAction::Cancel],
    );
}

fn draw_rgb_preview(frame: &mut Frame<'_>, area: Rect, color: Color) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let style = TuiStyle::default()
        .fg(readable_tui_text_color(color))
        .bg(tui_color(color))
        .add_modifier(Modifier::BOLD);
    fill_rect(frame, area, style);

    let middle = area.y.saturating_add(area.height / 2);
    draw_text_centered(
        frame,
        area,
        middle.saturating_sub(1),
        &color.to_hex(),
        style,
    );
    if area.height > 2 {
        let rgb = format!("{},{},{}", color.r, color.g, color.b);
        draw_text_centered(frame, area, middle.saturating_add(1), &rgb, style);
    }
}

fn draw_rgb_channel_row(
    frame: &mut Frame<'_>,
    app: &mut AppState,
    area: Rect,
    channel: RgbChannel,
    color: Color,
) {
    if area.width < 26 || area.height == 0 {
        return;
    }

    let label = format!("{} {:>3}", channel.short_label(), channel.value(color));
    draw_text(
        frame,
        area.x,
        area.y,
        6,
        &label,
        TuiStyle::default()
            .fg(channel_tui_color(channel))
            .add_modifier(Modifier::BOLD),
    );

    let button_width = 3;
    let gap = 1;
    let value_width = 3;
    let dec_area = Rect {
        x: area.x.saturating_add(7),
        y: area.y,
        width: button_width,
        height: 1,
    };
    draw_rgb_control_button(frame, app, dec_area, "-", RgbControl::Decrement(channel));

    let track_x = dec_area.x.saturating_add(button_width).saturating_add(gap);
    let trailing_width = gap
        .saturating_add(button_width)
        .saturating_add(gap)
        .saturating_add(value_width);
    let track_width = area
        .x
        .saturating_add(area.width)
        .saturating_sub(track_x)
        .saturating_sub(trailing_width);
    if track_width < 8 {
        return;
    }

    let track_area = Rect {
        x: track_x,
        y: area.y,
        width: track_width,
        height: 1,
    };
    draw_rgb_slider(frame, app, track_area, channel, color);

    let inc_area = Rect {
        x: track_area
            .x
            .saturating_add(track_area.width)
            .saturating_add(gap),
        y: area.y,
        width: button_width,
        height: 1,
    };
    draw_rgb_control_button(frame, app, inc_area, "+", RgbControl::Increment(channel));

    draw_text(
        frame,
        inc_area.x.saturating_add(button_width).saturating_add(gap),
        area.y,
        value_width,
        &format!("{:>3}", channel.value(color)),
        TuiStyle::default().fg(TuiColor::Rgb(211, 220, 228)),
    );
}

fn draw_rgb_control_button(
    frame: &mut Frame<'_>,
    app: &mut AppState,
    area: Rect,
    label: &str,
    control: RgbControl,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let style = choice_style(false, app.hovered_rgb_control == Some(control));
    fill_rect(frame, area, style);
    draw_text_centered(frame, area, area.y, label, style);
    app.modal_rgb_areas.push(ButtonHit {
        action: control,
        area,
    });
}

fn draw_rgb_slider(
    frame: &mut Frame<'_>,
    app: &mut AppState,
    area: Rect,
    channel: RgbChannel,
    color: Color,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    app.modal_rgb_areas.push(ButtonHit {
        action: RgbControl::Slider(channel),
        area,
    });

    for offset in 0..area.width {
        let mut preview = color;
        channel.set_value(
            &mut preview,
            rgb_slider_value_for_offset(area.width, offset),
        );
        frame.buffer_mut()[(area.x + offset, area.y)]
            .set_symbol(" ")
            .set_style(TuiStyle::default().bg(tui_color(preview)));
    }

    let knob_offset = rgb_slider_offset_for_value(area.width, channel.value(color));
    let knob_x = area
        .x
        .saturating_add(knob_offset.min(area.width.saturating_sub(1)));
    let mut knob_color = color;
    channel.set_value(&mut knob_color, channel.value(color));
    let active = app.hovered_rgb_control == Some(RgbControl::Slider(channel))
        || app.dragging_rgb_channel == Some(channel);
    let symbol = if active { "◆" } else { "│" };
    frame.buffer_mut()[(knob_x, area.y)]
        .set_symbol(symbol)
        .set_style(
            TuiStyle::default()
                .fg(readable_tui_text_color(knob_color))
                .bg(tui_color(knob_color))
                .add_modifier(Modifier::BOLD),
        );
}

fn tui_color(color: Color) -> TuiColor {
    TuiColor::Rgb(color.r, color.g, color.b)
}

fn readable_tui_text_color(color: Color) -> TuiColor {
    let luminance =
        (u32::from(color.r) * 299 + u32::from(color.g) * 587 + u32::from(color.b) * 114) / 1000;
    if luminance > 150 {
        TuiColor::Rgb(24, 28, 32)
    } else {
        TuiColor::Rgb(245, 248, 250)
    }
}

fn channel_tui_color(channel: RgbChannel) -> TuiColor {
    match channel {
        RgbChannel::Red => TuiColor::Rgb(255, 113, 113),
        RgbChannel::Green => TuiColor::Rgb(92, 214, 132),
        RgbChannel::Blue => TuiColor::Rgb(105, 160, 255),
    }
}

fn rgb_slider_value_at(area: Rect, column: u16) -> u8 {
    if area.width <= 1 {
        return 0;
    }

    let max_x = area.x.saturating_add(area.width.saturating_sub(1));
    let clamped = column.clamp(area.x, max_x);
    rgb_slider_value_for_offset(area.width, clamped.saturating_sub(area.x))
}

fn rgb_slider_value_for_offset(width: u16, offset: u16) -> u8 {
    if width <= 1 {
        return 0;
    }

    let denominator = u32::from(width.saturating_sub(1));
    let value =
        (u32::from(offset.min(width.saturating_sub(1))) * 255 + denominator / 2) / denominator;
    value.min(255) as u8
}

fn rgb_slider_offset_for_value(width: u16, value: u8) -> u16 {
    if width <= 1 {
        return 0;
    }

    let max_offset = u32::from(width.saturating_sub(1));
    ((u32::from(value) * max_offset + 127) / 255) as u16
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
        "Hue columns, tone rows. Wheel/PageUp/PageDown scroll. U opens RGB mixer.",
        TuiStyle::default().fg(TuiColor::Rgb(170, 184, 194)),
    );

    let grid_area = app.color_picker_grid_area();
    let columns = app.color_picker_columns().max(1);
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
        |offset| expanded_palette_color(start_index + offset),
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
        | Modal::NewAnimation { input, .. }
        | Modal::OpenFile { input }
        | Modal::SaveAs { input }
        | Modal::BrushChar { input }
        | Modal::NewStyle { input }
        | Modal::RenameStyle { input }
        | Modal::SetFg { input }
        | Modal::SetBg { input }
        | Modal::ExportText { input } => Some(input),
        Modal::RgbInput { .. } | Modal::ColorPicker | Modal::ToolMenu | Modal::QuitConfirm => None,
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

fn next_frame_id(project: &Project) -> String {
    for index in 1.. {
        let candidate = format!("frame-{index}");
        if !project
            .frames
            .iter()
            .any(|frame| frame.id.as_deref() == Some(candidate.as_str()))
        {
            return candidate;
        }
    }
    unreachable!("unbounded frame ID search")
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
    fn next_frame_creates_animation_frame_by_copying_current_frame() {
        let mut app = AppState::editor(Project::new_image("anim", 4, 2), None, false, "");
        app.set_cell_for_stroke(
            1,
            0,
            Some(PaintedCell {
                ch: '#',
                style_index: 0,
            }),
        );
        app.finish_stroke();

        app.next_frame_or_create();

        assert_eq!(app.project.asset.kind, AssetKind::Animation);
        assert_eq!(app.project.frames.len(), 2);
        assert_eq!(app.current_frame_index, 1);
        assert_eq!(
            app.current_frame().cells[&(0, 1)],
            PaintedCell {
                ch: '#',
                style_index: 0
            }
        );
        assert!(app.dirty);
    }

    #[test]
    fn blank_frame_creates_empty_current_frame() {
        let mut app = AppState::editor(Project::new_image("anim", 4, 2), None, false, "");
        app.set_cell_for_stroke(
            1,
            0,
            Some(PaintedCell {
                ch: '#',
                style_index: 0,
            }),
        );
        app.finish_stroke();

        app.insert_blank_frame();

        assert_eq!(app.project.frames.len(), 2);
        assert!(app.current_frame().cells.is_empty());
        assert_eq!(app.project.asset.kind, AssetKind::Animation);
    }

    #[test]
    fn undo_applies_to_frame_that_was_edited() {
        let mut app = AppState::editor(Project::new_image("anim", 4, 2), None, false, "");
        app.insert_blank_frame();
        app.set_cell_for_stroke(
            2,
            0,
            Some(PaintedCell {
                ch: '*',
                style_index: 0,
            }),
        );
        app.finish_stroke();
        app.current_frame_index = 0;

        app.undo();

        assert_eq!(app.current_frame_index, 1);
        assert!(!app.project.frames[1].cells.contains_key(&(0, 2)));
    }

    #[test]
    fn onion_skin_toggle_is_local_view_state() {
        let mut app = AppState::editor(Project::new_image("anim", 4, 2), None, false, "");

        app.toggle_onion_skin();
        assert!(app.onion_skin);
        assert!(!app.dirty);

        app.toggle_onion_skin();
        assert!(!app.onion_skin);
    }

    #[test]
    fn expanded_palette_is_arranged_as_hue_columns() {
        let red_dark = expanded_palette_color(0);
        let red_lighter = expanded_palette_color(EXPANDED_HUE_COUNT);
        let blue_dark = expanded_palette_color(9);

        assert_ne!(red_dark, red_lighter);
        assert!(red_dark.r > red_dark.b);
        assert!(blue_dark.b > blue_dark.r);
    }

    #[test]
    fn rgb_slider_maps_mouse_columns_to_values() {
        let area = Rect {
            x: 10,
            y: 2,
            width: 11,
            height: 1,
        };

        assert_eq!(rgb_slider_value_at(area, 0), 0);
        assert_eq!(rgb_slider_value_at(area, 10), 0);
        assert_eq!(rgb_slider_value_at(area, 15), 128);
        assert_eq!(rgb_slider_value_at(area, 20), 255);
        assert_eq!(rgb_slider_value_at(area, 99), 255);
    }

    #[test]
    fn rgb_slider_control_updates_modal_color() {
        let mut app = AppState::editor(Project::new_image("rgb", 4, 2), None, false, "");
        app.modal = Some(Modal::RgbInput {
            color: Color { r: 0, g: 0, b: 0 },
        });
        app.modal_rgb_areas.push(ButtonHit {
            action: RgbControl::Slider(RgbChannel::Red),
            area: Rect {
                x: 10,
                y: 1,
                width: 11,
                height: 1,
            },
        });

        app.apply_rgb_control(RgbControl::Slider(RgbChannel::Red), 20);

        assert_eq!(app.dragging_rgb_channel, Some(RgbChannel::Red));
        assert!(matches!(
            app.modal,
            Some(Modal::RgbInput {
                color: Color { r: 255, g: 0, b: 0 }
            })
        ));
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
        app.run_welcome_action(WelcomeAction::NewAnimation);
        assert!(matches!(app.modal, Some(Modal::NewAnimation { .. })));

        app.close_modal("");
        app.run_welcome_action(WelcomeAction::OpenFile);
        assert!(matches!(app.modal, Some(Modal::OpenFile { .. })));
    }

    #[test]
    fn new_animation_modal_creates_animation_asset() {
        let mut app = AppState::welcome("");
        app.modal = Some(Modal::NewAnimation {
            input: "12x4".to_string(),
            target_path: None,
        });

        let modal = app.modal.take().expect("new animation modal");
        app.commit_modal(modal);

        assert_eq!(app.screen, Screen::Editor);
        assert_eq!(app.project.asset.kind, AssetKind::Animation);
        assert_eq!(app.project.asset.width, 12);
        assert_eq!(app.project.asset.height, 4);
        assert_eq!(app.project.frames.len(), 1);
    }
}
