use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_width::UnicodeWidthChar;

pub const SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_FRAME_DURATION_MS: u64 = 250;
pub const MAX_WIDTH: u16 = 500;
pub const MAX_HEIGHT: u16 = 300;
pub const MAX_AREA_PER_FRAME: u64 = 150_000;
pub const MAX_FRAMES: usize = 1_000;
pub const MAX_STYLES: usize = 256;
const MAX_RUNS_PER_FRAME: usize = 10_000;
const MAX_EXPLICIT_CELLS_PER_FRAME: usize = 150_000;
const MAX_EXPANDED_CELLS_ALL_FRAMES: usize = 5_000_000;

pub type CellKey = (u16, u16);
pub type CellMap = BTreeMap<CellKey, PaintedCell>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub asset: Asset,
    pub layout: Layout,
    pub styles: Vec<TerminalStyle>,
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub kind: AssetKind,
    pub width: u16,
    pub height: u16,
    pub default_frame_duration_ms: u64,
    pub loop_animation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Image,
    Animation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub min_width: u16,
    pub min_height: u16,
    pub anchor: Anchor,
    pub overflow: Overflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    Clip,
    Hide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalStyle {
    pub id: String,
    pub fg: Color,
    pub bg: Option<Color>,
    pub attrs: Vec<TextAttr>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAttr {
    Bold,
    Dim,
    Italic,
    Underline,
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub id: Option<String>,
    pub duration_ms: u64,
    pub cells: CellMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaintedCell {
    pub ch: char,
    pub style_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProject {
    pub project: Project,
    pub warnings: Vec<ValidationMessage>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    pub errors: Vec<ValidationMessage>,
    pub warnings: Vec<ValidationMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationMessage {
    pub location: String,
    pub message: String,
}

#[derive(Debug)]
pub enum FormatError {
    Toml(toml::de::Error),
    Serialize(toml::ser::Error),
    Validation(ValidationReport),
    Io(io::Error),
}

impl Project {
    pub fn new_image(name: impl Into<String>, width: u16, height: u16) -> Self {
        Self {
            asset: Asset {
                name: name.into(),
                kind: AssetKind::Image,
                width,
                height,
                default_frame_duration_ms: DEFAULT_FRAME_DURATION_MS,
                loop_animation: true,
            },
            layout: Layout {
                min_width: width,
                min_height: height,
                anchor: Anchor::Center,
                overflow: Overflow::Clip,
            },
            styles: vec![TerminalStyle::default_style()],
            frames: vec![Frame {
                id: Some("frame-1".to_string()),
                duration_ms: DEFAULT_FRAME_DURATION_MS,
                cells: CellMap::new(),
            }],
        }
    }

    pub fn first_frame(&self) -> &Frame {
        &self.frames[0]
    }

    #[cfg(test)]
    pub fn first_frame_mut(&mut self) -> &mut Frame {
        &mut self.frames[0]
    }
}

impl TerminalStyle {
    pub fn default_style() -> Self {
        Self {
            id: "default".to_string(),
            fg: Color {
                r: 238,
                g: 238,
                b: 238,
            },
            bg: None,
            attrs: Vec::new(),
            role: Some("default".to_string()),
        }
    }
}

impl Color {
    pub fn parse_hex(value: &str) -> Option<Self> {
        let hex = value.strip_prefix('#')?;
        if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

        Some(Self { r, g, b })
    }

    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

impl Anchor {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "top_left" => Some(Self::TopLeft),
            "top_center" => Some(Self::TopCenter),
            "top_right" => Some(Self::TopRight),
            "center_left" => Some(Self::CenterLeft),
            "center" => Some(Self::Center),
            "center_right" => Some(Self::CenterRight),
            "bottom_left" => Some(Self::BottomLeft),
            "bottom_center" => Some(Self::BottomCenter),
            "bottom_right" => Some(Self::BottomRight),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::TopCenter => "top_center",
            Self::TopRight => "top_right",
            Self::CenterLeft => "center_left",
            Self::Center => "center",
            Self::CenterRight => "center_right",
            Self::BottomLeft => "bottom_left",
            Self::BottomCenter => "bottom_center",
            Self::BottomRight => "bottom_right",
        }
    }
}

impl Overflow {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "clip" => Some(Self::Clip),
            "hide" => Some(Self::Hide),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Clip => "clip",
            Self::Hide => "hide",
        }
    }
}

impl TextAttr {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "bold" => Some(Self::Bold),
            "dim" => Some(Self::Dim),
            "italic" => Some(Self::Italic),
            "underline" => Some(Self::Underline),
            "reverse" => Some(Self::Reverse),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Bold => "bold",
            Self::Dim => "dim",
            Self::Italic => "italic",
            Self::Underline => "underline",
            Self::Reverse => "reverse",
        }
    }
}

impl AssetKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "image" => Some(Self::Image),
            "animation" => Some(Self::Animation),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Animation => "animation",
        }
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Toml(error) => write!(f, "TOML parse error: {error}"),
            Self::Serialize(error) => write!(f, "TOML serialize error: {error}"),
            Self::Validation(report) => {
                writeln!(f, "validation failed with {} error(s)", report.errors.len())?;
                for error in &report.errors {
                    writeln!(f, "- {}: {}", error.location, error.message)?;
                }
                Ok(())
            }
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for FormatError {}

impl From<toml::de::Error> for FormatError {
    fn from(value: toml::de::Error) -> Self {
        Self::Toml(value)
    }
}

impl From<toml::ser::Error> for FormatError {
    fn from(value: toml::ser::Error) -> Self {
        Self::Serialize(value)
    }
}

impl From<io::Error> for FormatError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Deserialize)]
struct RawProject {
    schema_version: u32,
    asset: RawAsset,
    #[serde(default)]
    layout: Option<RawLayout>,
    #[serde(default)]
    styles: Vec<RawStyle>,
    #[serde(default)]
    frames: Vec<RawFrame>,
}

#[derive(Debug, Deserialize)]
struct RawAsset {
    name: String,
    kind: String,
    width: i64,
    height: i64,
    default_frame_duration_ms: i64,
    #[serde(rename = "loop")]
    loop_animation: bool,
}

#[derive(Debug, Deserialize)]
struct RawLayout {
    min_width: Option<i64>,
    min_height: Option<i64>,
    anchor: Option<String>,
    overflow: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawStyle {
    id: String,
    fg: String,
    #[serde(default)]
    bg: Option<String>,
    #[serde(default)]
    attrs: Vec<String>,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawFrame {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    duration_ms: Option<i64>,
    #[serde(default)]
    runs: Vec<RawRun>,
    #[serde(default)]
    cells: Vec<RawCell>,
}

#[derive(Debug, Deserialize)]
struct RawRun {
    x: i64,
    y: i64,
    text: String,
    style: String,
}

#[derive(Debug, Deserialize)]
struct RawCell {
    x: i64,
    y: i64,
    ch: String,
    style: String,
}

#[derive(Debug, Serialize)]
struct OutProject {
    schema_version: u32,
    asset: OutAsset,
    layout: OutLayout,
    styles: Vec<OutStyle>,
    frames: Vec<OutFrame>,
}

#[derive(Debug, Serialize)]
struct OutAsset {
    name: String,
    kind: String,
    width: u16,
    height: u16,
    default_frame_duration_ms: u64,
    #[serde(rename = "loop")]
    loop_animation: bool,
}

#[derive(Debug, Serialize)]
struct OutLayout {
    min_width: u16,
    min_height: u16,
    anchor: String,
    overflow: String,
}

#[derive(Debug, Serialize)]
struct OutStyle {
    id: String,
    fg: String,
    bg: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attrs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
}

#[derive(Debug, Serialize)]
struct OutFrame {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    runs: Vec<OutRun>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cells: Vec<OutCell>,
}

#[derive(Debug, Serialize)]
struct OutRun {
    x: u16,
    y: u16,
    text: String,
    style: String,
}

#[derive(Debug, Serialize)]
struct OutCell {
    x: u16,
    y: u16,
    ch: String,
    style: String,
}

pub fn load_project_from_path(path: &Path) -> Result<LoadedProject, FormatError> {
    let input = fs::read_to_string(path)?;
    parse_project_str(&input)
}

pub fn parse_project_str(input: &str) -> Result<LoadedProject, FormatError> {
    let raw: RawProject = toml::from_str(input)?;
    validate_raw_project(raw)
}

pub fn project_to_toml_string(project: &Project) -> Result<String, FormatError> {
    let out = project_to_out(project);
    let mut encoded = toml::to_string_pretty(&out)?;
    if !encoded.ends_with('\n') {
        encoded.push('\n');
    }
    Ok(encoded)
}

pub fn save_project_to_path(
    project: &Project,
    path: &Path,
) -> Result<Vec<ValidationMessage>, FormatError> {
    let encoded = project_to_toml_string(project)?;
    let loaded = parse_project_str(&encoded)?;

    let tmp_path = temporary_path_for(path);
    fs::write(&tmp_path, encoded)?;
    fs::rename(&tmp_path, path)?;

    Ok(loaded.warnings)
}

pub fn export_plain_text(project: &Project, frame_index: usize) -> String {
    let frame = project
        .frames
        .get(frame_index)
        .unwrap_or_else(|| project.first_frame());
    let mut output = String::new();

    for y in 0..project.asset.height {
        for x in 0..project.asset.width {
            let ch = frame.cells.get(&(y, x)).map(|cell| cell.ch).unwrap_or(' ');
            output.push(ch);
        }

        if y + 1 < project.asset.height {
            output.push('\n');
        }
    }

    output
}

pub fn is_valid_v1_character(ch: char) -> bool {
    if ch.is_control() || matches!(ch, '\t' | '\n' | '\r' | '\u{1b}' | '\u{7f}') {
        return false;
    }

    match get_general_category(ch) {
        GeneralCategory::Control
        | GeneralCategory::Format
        | GeneralCategory::NonspacingMark
        | GeneralCategory::SpacingMark
        | GeneralCategory::EnclosingMark
        | GeneralCategory::Surrogate
        | GeneralCategory::Unassigned => return false,
        _ => {}
    }

    UnicodeWidthChar::width(ch) == Some(1)
}

fn validate_raw_project(raw: RawProject) -> Result<LoadedProject, FormatError> {
    let mut report = ValidationReport::default();

    if raw.schema_version != SCHEMA_VERSION {
        report.errors.push(message(
            "schema_version",
            format!(
                "unsupported schema version {}; expected {SCHEMA_VERSION}",
                raw.schema_version
            ),
        ));
    }

    let kind = match AssetKind::parse(&raw.asset.kind) {
        Some(kind) => kind,
        None => {
            report
                .errors
                .push(message("asset.kind", "must be \"image\" or \"animation\""));
            AssetKind::Image
        }
    };

    let width = validate_positive_u16(&mut report, "asset.width", raw.asset.width, Some(MAX_WIDTH));
    let height = validate_positive_u16(
        &mut report,
        "asset.height",
        raw.asset.height,
        Some(MAX_HEIGHT),
    );
    let default_duration = validate_positive_u64(
        &mut report,
        "asset.default_frame_duration_ms",
        raw.asset.default_frame_duration_ms,
    );

    if let (Some(width), Some(height)) = (width, height) {
        let area = u64::from(width) * u64::from(height);
        if area > MAX_AREA_PER_FRAME {
            report.errors.push(message(
                "asset",
                format!("area {area} exceeds maximum area per frame {MAX_AREA_PER_FRAME}"),
            ));
        }
    }

    if raw.frames.len() > MAX_FRAMES {
        report.errors.push(message(
            "frames",
            format!(
                "frame count {} exceeds maximum {MAX_FRAMES}",
                raw.frames.len()
            ),
        ));
    }

    match kind {
        AssetKind::Image if raw.frames.len() != 1 => report.errors.push(message(
            "frames",
            "kind = \"image\" requires exactly one frame",
        )),
        AssetKind::Animation if raw.frames.is_empty() => report.errors.push(message(
            "frames",
            "kind = \"animation\" requires one or more frames",
        )),
        _ => {}
    }

    let Some(width) = width else {
        return Err(FormatError::Validation(report));
    };
    let Some(height) = height else {
        return Err(FormatError::Validation(report));
    };
    let Some(default_duration) = default_duration else {
        return Err(FormatError::Validation(report));
    };

    let layout = validate_layout(&mut report, raw.layout, width, height);
    let styles = validate_styles(&mut report, raw.styles);

    if !report.errors.is_empty() {
        return Err(FormatError::Validation(report));
    }

    let Some(layout) = layout else {
        return Err(FormatError::Validation(report));
    };
    let Some(styles) = styles else {
        return Err(FormatError::Validation(report));
    };

    let mut style_indices = HashMap::new();
    for (index, style) in styles.iter().enumerate() {
        style_indices.insert(style.id.as_str(), index);
    }

    let frames = validate_frames(
        &mut report,
        raw.frames,
        width,
        height,
        default_duration,
        &style_indices,
    );

    if layout.min_width > width {
        report.warnings.push(message(
            "layout.min_width",
            "layout minimum width is larger than asset width",
        ));
    }

    if layout.min_height > height {
        report.warnings.push(message(
            "layout.min_height",
            "layout minimum height is larger than asset height",
        ));
    }

    if !report.errors.is_empty() {
        return Err(FormatError::Validation(report));
    }

    let Some(frames) = frames else {
        return Err(FormatError::Validation(report));
    };

    add_usage_warnings(&mut report, kind, &styles, &frames);

    Ok(LoadedProject {
        project: Project {
            asset: Asset {
                name: raw.asset.name,
                kind,
                width,
                height,
                default_frame_duration_ms: default_duration,
                loop_animation: raw.asset.loop_animation,
            },
            layout,
            styles,
            frames,
        },
        warnings: report.warnings,
    })
}

fn validate_layout(
    report: &mut ValidationReport,
    raw: Option<RawLayout>,
    width: u16,
    height: u16,
) -> Option<Layout> {
    let raw = raw.unwrap_or(RawLayout {
        min_width: None,
        min_height: None,
        anchor: None,
        overflow: None,
    });

    let min_width = match raw.min_width {
        Some(value) => validate_positive_u16(report, "layout.min_width", value, None),
        None => Some(width),
    };
    let min_height = match raw.min_height {
        Some(value) => validate_positive_u16(report, "layout.min_height", value, None),
        None => Some(height),
    };
    let anchor = match raw.anchor {
        Some(value) => match Anchor::parse(&value) {
            Some(anchor) => Some(anchor),
            None => {
                report
                    .errors
                    .push(message("layout.anchor", "invalid anchor value"));
                None
            }
        },
        None => Some(Anchor::Center),
    };
    let overflow = match raw.overflow {
        Some(value) => match Overflow::parse(&value) {
            Some(overflow) => Some(overflow),
            None => {
                report
                    .errors
                    .push(message("layout.overflow", "invalid overflow value"));
                None
            }
        },
        None => Some(Overflow::Clip),
    };

    Some(Layout {
        min_width: min_width?,
        min_height: min_height?,
        anchor: anchor?,
        overflow: overflow?,
    })
}

fn validate_styles(
    report: &mut ValidationReport,
    raw_styles: Vec<RawStyle>,
) -> Option<Vec<TerminalStyle>> {
    if raw_styles.len() > MAX_STYLES {
        report.errors.push(message(
            "styles",
            format!(
                "style count {} exceeds maximum {MAX_STYLES}",
                raw_styles.len()
            ),
        ));
    }

    let mut ids = HashSet::new();
    let mut styles = Vec::with_capacity(raw_styles.len());

    for (index, raw) in raw_styles.into_iter().enumerate() {
        let location = format!("styles[{index}]");

        if raw.id.trim().is_empty() {
            report.errors.push(message(
                format!("{location}.id"),
                "style ID cannot be empty",
            ));
        }

        if !ids.insert(raw.id.clone()) {
            report
                .errors
                .push(message(format!("{location}.id"), "duplicate style ID"));
        }

        let fg = if raw.fg == "transparent" {
            report.errors.push(message(
                format!("{location}.fg"),
                "foreground transparency is not valid in V1",
            ));
            None
        } else {
            Color::parse_hex(&raw.fg).or_else(|| {
                report
                    .errors
                    .push(message(format!("{location}.fg"), "invalid #RRGGBB color"));
                None
            })
        };

        let bg = match raw.bg.as_deref().unwrap_or("transparent") {
            "transparent" => Some(None),
            value => Color::parse_hex(value).map(Some).or_else(|| {
                report.errors.push(message(
                    format!("{location}.bg"),
                    "invalid background color",
                ));
                None
            }),
        };

        let mut attrs = Vec::with_capacity(raw.attrs.len());
        for attr in raw.attrs {
            match TextAttr::parse(&attr) {
                Some(attr) => attrs.push(attr),
                None => report.errors.push(message(
                    format!("{location}.attrs"),
                    format!("invalid style attribute {attr:?}"),
                )),
            }
        }

        if let (Some(fg), Some(bg)) = (fg, bg) {
            styles.push(TerminalStyle {
                id: raw.id,
                fg,
                bg,
                attrs,
                role: raw.role,
            });
        }
    }

    if report.errors.is_empty() {
        Some(styles)
    } else {
        None
    }
}

fn validate_frames(
    report: &mut ValidationReport,
    raw_frames: Vec<RawFrame>,
    width: u16,
    height: u16,
    default_duration_ms: u64,
    style_indices: &HashMap<&str, usize>,
) -> Option<Vec<Frame>> {
    let mut frames = Vec::with_capacity(raw_frames.len());
    let mut frame_ids = HashSet::new();
    let mut total_expanded_cells = 0usize;

    for (frame_index, raw_frame) in raw_frames.into_iter().enumerate() {
        let frame_location = format!("frames[{frame_index}]");

        if raw_frame.runs.len() > MAX_RUNS_PER_FRAME {
            report.errors.push(message(
                format!("{frame_location}.runs"),
                format!(
                    "run count {} exceeds maximum {MAX_RUNS_PER_FRAME}",
                    raw_frame.runs.len()
                ),
            ));
        }

        if raw_frame.cells.len() > MAX_EXPLICIT_CELLS_PER_FRAME {
            report.errors.push(message(
                format!("{frame_location}.cells"),
                format!(
                    "cell count {} exceeds maximum {MAX_EXPLICIT_CELLS_PER_FRAME}",
                    raw_frame.cells.len()
                ),
            ));
        }

        if let Some(id) = raw_frame.id.as_ref() {
            if id.trim().is_empty() {
                report.errors.push(message(
                    format!("{frame_location}.id"),
                    "frame ID cannot be empty",
                ));
            } else if !frame_ids.insert(id.clone()) {
                report.errors.push(message(
                    format!("{frame_location}.id"),
                    "duplicate frame ID",
                ));
            }
        }

        let duration_ms = match raw_frame.duration_ms {
            Some(value) => {
                validate_positive_u64(report, format!("{frame_location}.duration_ms"), value)
            }
            None => Some(default_duration_ms),
        };

        if let Some(duration_ms) = duration_ms
            && duration_ms < 80
        {
            report.warnings.push(message(
                format!("{frame_location}.duration_ms"),
                "very fast frame duration below 80 ms",
            ));
        }

        let mut cells = CellMap::new();
        let mut expanded_writes = 0usize;
        let max_expanded_writes = max_expanded_writes_per_frame(width, height);

        for (run_index, run) in raw_frame.runs.into_iter().enumerate() {
            let run_location = format!("{frame_location}.runs[{run_index}]");
            let Some((x, y)) =
                validate_coordinate(report, &run_location, run.x, run.y, width, height)
            else {
                continue;
            };

            let Some(style_index) = resolve_style(
                report,
                format!("{run_location}.style"),
                &run.style,
                style_indices,
            ) else {
                continue;
            };

            let mut run_chars = Vec::new();
            for (char_index, ch) in run.text.chars().enumerate() {
                if !is_valid_v1_character(ch) {
                    report.errors.push(message(
                        format!("{run_location}.text[{char_index}]"),
                        format!("invalid V1 terminal character {ch:?}"),
                    ));
                }
                run_chars.push(ch);
            }

            let run_len = match u16::try_from(run_chars.len()) {
                Ok(value) => value,
                Err(_) => {
                    report.errors.push(message(
                        format!("{run_location}.text"),
                        "run text is too long",
                    ));
                    continue;
                }
            };

            if u32::from(x) + u32::from(run_len) > u32::from(width) {
                report.errors.push(message(
                    format!("{run_location}.text"),
                    "run text extends outside the canvas",
                ));
                continue;
            }

            expanded_writes = expanded_writes.saturating_add(run_chars.len());
            if expanded_writes > max_expanded_writes {
                report.errors.push(message(
                    frame_location.clone(),
                    format!("expanded writes per frame exceeds maximum {max_expanded_writes}"),
                ));
                continue;
            }

            for (offset, ch) in run_chars.into_iter().enumerate() {
                let cell_x = x + u16::try_from(offset).expect("run length already fits u16");
                cells.insert((y, cell_x), PaintedCell { ch, style_index });
            }
        }

        for (cell_index, raw_cell) in raw_frame.cells.into_iter().enumerate() {
            let cell_location = format!("{frame_location}.cells[{cell_index}]");
            let Some((x, y)) = validate_coordinate(
                report,
                &cell_location,
                raw_cell.x,
                raw_cell.y,
                width,
                height,
            ) else {
                continue;
            };

            let Some(style_index) = resolve_style(
                report,
                format!("{cell_location}.style"),
                &raw_cell.style,
                style_indices,
            ) else {
                continue;
            };

            let mut chars = raw_cell.ch.chars();
            let Some(ch) = chars.next() else {
                report.errors.push(message(
                    format!("{cell_location}.ch"),
                    "cell character cannot be empty",
                ));
                continue;
            };

            if chars.next().is_some() {
                report.errors.push(message(
                    format!("{cell_location}.ch"),
                    "cell character must contain exactly one Unicode scalar value",
                ));
                continue;
            }

            if !is_valid_v1_character(ch) {
                report.errors.push(message(
                    format!("{cell_location}.ch"),
                    format!("invalid V1 terminal character {ch:?}"),
                ));
                continue;
            }

            expanded_writes = expanded_writes.saturating_add(1);
            if expanded_writes > max_expanded_writes {
                report.errors.push(message(
                    frame_location.clone(),
                    format!("expanded writes per frame exceeds maximum {max_expanded_writes}"),
                ));
                continue;
            }

            cells.insert((y, x), PaintedCell { ch, style_index });
        }

        total_expanded_cells = total_expanded_cells.saturating_add(cells.len());
        if total_expanded_cells > MAX_EXPANDED_CELLS_ALL_FRAMES {
            report.errors.push(message(
                "frames",
                format!(
                    "expanded cells across all frames exceeds maximum {MAX_EXPANDED_CELLS_ALL_FRAMES}"
                ),
            ));
        }

        if let Some(duration_ms) = duration_ms {
            frames.push(Frame {
                id: raw_frame.id,
                duration_ms,
                cells,
            });
        }
    }

    if report.errors.is_empty() {
        Some(frames)
    } else {
        None
    }
}

fn validate_positive_u16(
    report: &mut ValidationReport,
    location: impl Into<String>,
    value: i64,
    max: Option<u16>,
) -> Option<u16> {
    let location = location.into();

    if value <= 0 {
        report
            .errors
            .push(message(location, "must be a positive integer"));
        return None;
    }

    let Ok(value) = u16::try_from(value) else {
        report.errors.push(message(location, "is too large"));
        return None;
    };

    if let Some(max) = max
        && value > max
    {
        report
            .errors
            .push(message(location, format!("exceeds maximum {max}")));
        return None;
    }

    Some(value)
}

fn validate_positive_u64(
    report: &mut ValidationReport,
    location: impl Into<String>,
    value: i64,
) -> Option<u64> {
    let location = location.into();

    if value <= 0 {
        report
            .errors
            .push(message(location, "must be a positive integer"));
        return None;
    }

    u64::try_from(value).ok()
}

fn validate_coordinate(
    report: &mut ValidationReport,
    location: &str,
    x: i64,
    y: i64,
    width: u16,
    height: u16,
) -> Option<(u16, u16)> {
    if x < 0 || y < 0 {
        report
            .errors
            .push(message(location, "coordinate cannot be negative"));
        return None;
    }

    let Ok(x) = u16::try_from(x) else {
        report
            .errors
            .push(message(location, "x coordinate is too large"));
        return None;
    };
    let Ok(y) = u16::try_from(y) else {
        report
            .errors
            .push(message(location, "y coordinate is too large"));
        return None;
    };

    if x >= width || y >= height {
        report
            .errors
            .push(message(location, "coordinate is outside the canvas"));
        return None;
    }

    Some((x, y))
}

fn resolve_style(
    report: &mut ValidationReport,
    location: impl Into<String>,
    style_id: &str,
    style_indices: &HashMap<&str, usize>,
) -> Option<usize> {
    style_indices.get(style_id).copied().or_else(|| {
        report.errors.push(message(
            location,
            format!("unknown style reference {style_id:?}"),
        ));
        None
    })
}

fn max_expanded_writes_per_frame(width: u16, height: u16) -> usize {
    let area = usize::from(width) * usize::from(height);
    150_000usize.max(area.saturating_mul(4)).min(600_000)
}

fn add_usage_warnings(
    report: &mut ValidationReport,
    kind: AssetKind,
    styles: &[TerminalStyle],
    frames: &[Frame],
) {
    let mut used_styles = BTreeSet::new();
    for frame in frames {
        if kind == AssetKind::Animation && frame.cells.is_empty() {
            report
                .warnings
                .push(message("frames", "empty animation frame"));
        }

        for cell in frame.cells.values() {
            used_styles.insert(cell.style_index);
        }
    }

    for (index, style) in styles.iter().enumerate() {
        if !used_styles.contains(&index) {
            report.warnings.push(message(
                format!("styles[{index}]"),
                format!("unused style {:?}", style.id),
            ));
        }
    }
}

fn project_to_out(project: &Project) -> OutProject {
    OutProject {
        schema_version: SCHEMA_VERSION,
        asset: OutAsset {
            name: project.asset.name.clone(),
            kind: project.asset.kind.as_str().to_string(),
            width: project.asset.width,
            height: project.asset.height,
            default_frame_duration_ms: project.asset.default_frame_duration_ms,
            loop_animation: project.asset.loop_animation,
        },
        layout: OutLayout {
            min_width: project.layout.min_width,
            min_height: project.layout.min_height,
            anchor: project.layout.anchor.as_str().to_string(),
            overflow: project.layout.overflow.as_str().to_string(),
        },
        styles: project
            .styles
            .iter()
            .map(|style| OutStyle {
                id: style.id.clone(),
                fg: style.fg.to_hex(),
                bg: style
                    .bg
                    .map(Color::to_hex)
                    .unwrap_or_else(|| "transparent".to_string()),
                attrs: style
                    .attrs
                    .iter()
                    .map(|attr| attr.as_str().to_string())
                    .collect(),
                role: style.role.clone(),
            })
            .collect(),
        frames: project
            .frames
            .iter()
            .map(|frame| OutFrame {
                id: frame.id.clone(),
                duration_ms: if frame.duration_ms == project.asset.default_frame_duration_ms {
                    None
                } else {
                    Some(frame.duration_ms)
                },
                runs: Vec::new(),
                cells: frame
                    .cells
                    .iter()
                    .map(|(&(y, x), cell)| OutCell {
                        x,
                        y,
                        ch: cell.ch.to_string(),
                        style: project.styles[cell.style_index].id.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn temporary_path_for(path: &Path) -> std::path::PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("terminal_animator");
    path.with_file_name(format!(".{file_name}.tmp"))
}

fn message(location: impl Into<String>, message: impl Into<String>) -> ValidationMessage {
    ValidationMessage {
        location: location.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r##"
schema_version = 1

[asset]
name = "tiny-star"
kind = "image"
width = 7
height = 3
default_frame_duration_ms = 250
loop = true

[[styles]]
id = "star"
fg = "#E0B952"
bg = "transparent"
attrs = ["bold"]

[[frames]]
id = "frame-1"

[[frames.cells]]
x = 3
y = 0
ch = "*"
style = "star"
"##;

    #[test]
    fn parses_minimal_image_and_normalizes_layout() {
        let loaded = parse_project_str(MINIMAL).expect("valid project");

        assert_eq!(loaded.project.asset.name, "tiny-star");
        assert_eq!(loaded.project.layout.min_width, 7);
        assert_eq!(loaded.project.layout.min_height, 3);
        assert_eq!(loaded.project.layout.anchor, Anchor::Center);
        assert_eq!(loaded.project.first_frame().cells[&(0, 3)].ch, '*');
    }

    #[test]
    fn run_cells_compose_before_explicit_cells() {
        let input = r##"
schema_version = 1

[asset]
name = "compose"
kind = "image"
width = 4
height = 1
default_frame_duration_ms = 250
loop = true

[[styles]]
id = "plain"
fg = "#FFFFFF"

[[frames]]

[[frames.runs]]
x = 0
y = 0
text = "abcd"
style = "plain"

[[frames.cells]]
x = 1
y = 0
ch = "Z"
style = "plain"
"##;

        let loaded = parse_project_str(input).expect("valid project");
        assert_eq!(export_plain_text(&loaded.project, 0), "aZcd");
    }

    #[test]
    fn image_requires_exactly_one_frame() {
        let input = r##"
schema_version = 1

[asset]
name = "bad"
kind = "image"
width = 2
height = 1
default_frame_duration_ms = 250
loop = true

[[styles]]
id = "plain"
fg = "#FFFFFF"

[[frames]]

[[frames]]
"##;

        let error = parse_project_str(input).expect_err("invalid frame count");
        assert!(format!("{error}").contains("requires exactly one frame"));
    }

    #[test]
    fn rejects_invalid_colors_and_foreground_transparency() {
        let input = r##"
schema_version = 1

[asset]
name = "bad-color"
kind = "image"
width = 2
height = 1
default_frame_duration_ms = 250
loop = true

[[styles]]
id = "plain"
fg = "transparent"

[[frames]]
"##;

        let error = parse_project_str(input).expect_err("invalid color");
        assert!(format!("{error}").contains("foreground transparency"));
    }

    #[test]
    fn rejects_non_v1_characters() {
        assert!(is_valid_v1_character('*'));
        assert!(is_valid_v1_character('█'));
        assert!(!is_valid_v1_character('\n'));
        assert!(!is_valid_v1_character('\u{0301}'));
        assert!(!is_valid_v1_character('😀'));
    }

    #[test]
    fn serializes_and_round_trips_new_image() {
        let mut project = Project::new_image("round-trip", 3, 2);
        project.first_frame_mut().cells.insert(
            (1, 2),
            PaintedCell {
                ch: '@',
                style_index: 0,
            },
        );

        let encoded = project_to_toml_string(&project).expect("serialize");
        let loaded = parse_project_str(&encoded).expect("parse serialized");

        assert_eq!(loaded.project, project);
    }
}
