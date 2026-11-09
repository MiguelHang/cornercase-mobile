use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;

#[cfg(test)]
pub(crate) mod fixtures;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Colour {
    Index(u8),
    Rgb([u8; 3]),
}

#[expect(clippy::struct_excessive_bools, reason = "mirrors the attributes of cornercase's style table")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Style {
    pub fg: Option<Colour>,
    pub bg: Option<Colour>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    #[serde(rename = "underline_style")]
    pub underline_kind: Option<UnderlineKind>,
    pub underline_colour: Option<Colour>,
    pub overline: bool,
    pub blink: bool,
    pub inverse: bool,
    pub invisible: bool,
    pub strikethrough: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnderlineKind {
    Double,
    Curly,
    Dotted,
    Dashed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Hyperlink {
    pub uri: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Area {
    pub x: u16,
    pub y: u16,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Node {
    Leaf(u64),
    Split(Box<Branch>),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Branch {
    pub dir: String,
    pub first: Node,
    pub second: Node,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorShape {
    #[default]
    Block,
    Bar,
    Underline,
    BlockHollow,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Cursor {
    pub x: u16,
    pub y: u16,
    pub shape: CursorShape,
    pub blinking: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseMode {
    #[default]
    None,
    Press,
    PressRelease,
    ButtonMotion,
    AnyMotion,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseEncoding {
    #[default]
    Default,
    Utf8,
    Sgr,
}

#[expect(clippy::struct_excessive_bools, reason = "mirrors the terminal modes cornercase reports")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Modes {
    pub mouse: MouseMode,
    pub mouse_encoding: MouseEncoding,
    pub alternate_screen: bool,
    pub alternate_scroll: bool,
    pub bracketed_paste: bool,
    pub application_cursor: bool,
    pub focus_events: bool,
    pub kitty_keyboard: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Scroll {
    pub total: u64,
    pub offset: u64,
    pub rows: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Colours {
    pub fg: Option<[u8; 3]>,
    pub bg: Option<[u8; 3]>,
    pub cursor: Option<[u8; 3]>,
    pub palette: BTreeMap<String, [u8; 3]>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct WireRun {
    pub style: usize,
    pub hyperlink: Option<usize>,
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum WireLines {
    All(Vec<Vec<WireRun>>),
    Changed(HashMap<String, Vec<WireRun>>),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Copy {
    pub from: u16,
    pub to: u16,
    pub count: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct WirePane {
    pub pane: u64,
    pub active: Option<bool>,
    pub rect: Option<Area>,
    pub size: Option<Size>,
    pub cursor: Option<Cursor>,
    pub modes: Option<Modes>,
    pub scroll: Option<Scroll>,
    pub colours: Option<Colours>,
    pub copy: Option<Copy>,
    pub lines: Option<WireLines>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Frame {
    pub boot: String,
    pub session: u64,
    pub revision: u64,
    pub surface: u64,
    pub tab: u64,
    pub view: Option<u64>,
    pub cols: u16,
    pub rows: u16,
    pub base: Option<u64>,
    pub projection: Option<u64>,
    pub layout: Option<Node>,
    pub styles: Vec<Style>,
    pub hyperlinks: Vec<Hyperlink>,
    pub panes: Vec<WirePane>,
}

#[cfg(test)]
mod tests;
