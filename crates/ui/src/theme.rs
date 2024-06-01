use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct Color(pub ratatui::style::Color);

impl Into<ratatui::style::Color> for Color {
    fn into(self) -> ratatui::style::Color {
        self.0
    }
}

impl From<&str> for Color {
    fn from(src: &str) -> Self {
        let r = u8::from_str_radix(&src[1..3], 16).expect("valid hex red value");
        let g = u8::from_str_radix(&src[3..5], 16).expect("valid hex green value");
        let b = u8::from_str_radix(&src[5..7], 16).expect("valid hex blue value");
        Color(ratatui::style::Color::Rgb(r, g, b))
    }
}

#[derive(Debug)]
pub struct Theme {
    palette: HashMap<String, Color>,
    scheme: HashMap<String, String>,
}

impl Theme {
    pub(crate) fn scheme(&self, name: &str) -> Option<Color> {
        self.scheme.get(name).map(|n| self.palette[n])
    }

    pub(crate) fn palette(&self, name: &str) -> Option<Color> {
        self.palette.get(name).copied()
    }
}

impl Default for Theme {
    fn default() -> Self {
        let palette = HashMap::from([
            ("bg0".into(), "#282828".into()),
            ("bg1".into(), "#32302f".into()),
            ("bg2".into(), "#32302f".into()),
            ("bg3".into(), "#45403d".into()),
            ("bg4".into(), "#45403d".into()),
            ("bg5".into(), "#5a524c".into()),
            ("bg_statusline1".into(), "#32302f".into()),
            ("bg_statusline2".into(), "#3a3735".into()),
            ("bg_statusline3".into(), "#504945".into()),
            ("bg_diff_green".into(), "#34381b".into()),
            ("bg_visual_green".into(), "#3b4439".into()),
            ("bg_diff_red".into(), "#402120".into()),
            ("bg_visual_red".into(), "#4c3432".into()),
            ("bg_diff_blue".into(), "#0e363e".into()),
            ("bg_visual_blue".into(), "#374141".into()),
            ("bg_visual_yellow".into(), "#4f422e".into()),
            ("bg_current_word".into(), "#3c3836".into()),
            ("fg0".into(), "#ebdbb2".into()),
            ("fg1".into(), "#ebdbb2".into()),
            ("red".into(), "#fb4934".into()),
            ("orange".into(), "#fe8019".into()),
            ("yellow".into(), "#fabd2f".into()),
            ("green".into(), "#b8bb26".into()),
            ("aqua".into(), "#8ec07c".into()),
            ("blue".into(), "#83a598".into()),
            ("purple".into(), "#d3869b".into()),
            ("bg_red".into(), "#cc241d".into()),
            ("bg_green".into(), "#b8bb26".into()),
            ("bg_yellow".into(), "#fabd2f".into()),
            ("grey0".into(), "#7c6f64".into()),
            ("grey1".into(), "#928374".into()),
            ("grey2".into(), "#a89984".into()),
        ]);
        let scheme = HashMap::from([
            ("type".into(), "yellow".into()),
            ("constant".into(), "purple".into()),
            ("constant.numeric".into(), "purple".into()),
            ("constant.character.escape".into(), "orange".into()),
            ("string".into(), "green".into()),
            ("string.regexp".into(), "blue".into()),
            ("comment".into(), "grey0".into()),
            ("variable".into(), "fg0".into()),
            ("variable.builtin".into(), "blue".into()),
            ("variable.parameter".into(), "fg0".into()),
            ("variable.other.member".into(), "fg0".into()),
            ("label".into(), "aqua".into()),
            ("punctuation".into(), "grey2".into()),
            ("punctuation.delimiter".into(), "grey2".into()),
            ("punctuation.bracket".into(), "fg0".into()),
            ("keyword".into(), "red".into()),
            ("keyword.directive".into(), "aqua".into()),
            ("operator".into(), "orange".into()),
            ("function".into(), "green".into()),
            ("function.builtin".into(), "blue".into()),
            ("function.macro".into(), "aqua".into()),
            ("tag".into(), "yellow".into()),
            ("namespace".into(), "aqua".into()),
            ("attribute".into(), "aqua".into()),
            ("constructor".into(), "yellow".into()),
            ("module".into(), "blue".into()),
            ("special".into(), "orange".into()),
        ]);

        Self { palette, scheme }
    }
}
