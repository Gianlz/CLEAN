use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub panel: Color,
    pub text: Color,
    pub muted: Color,
    pub cyan: Color,
    pub violet: Color,
    pub lime: Color,
    pub warning: Color,
    pub danger: Color,
}

impl Theme {
    pub fn for_mode(no_color: bool) -> Self {
        if no_color {
            Self {
                bg: Color::Reset,
                panel: Color::Reset,
                text: Color::White,
                muted: Color::Gray,
                cyan: Color::White,
                violet: Color::White,
                lime: Color::White,
                warning: Color::White,
                danger: Color::White,
            }
        } else {
            Self {
                bg: Color::Rgb(5, 8, 18),
                panel: Color::Rgb(12, 18, 34),
                text: Color::Rgb(226, 232, 255),
                muted: Color::Rgb(112, 126, 154),
                cyan: Color::Rgb(49, 231, 255),
                violet: Color::Rgb(170, 92, 255),
                lime: Color::Rgb(166, 255, 96),
                warning: Color::Rgb(255, 201, 71),
                danger: Color::Rgb(255, 92, 130),
            }
        }
    }

    pub fn title(self) -> Style {
        Style::default().fg(self.cyan).add_modifier(Modifier::BOLD)
    }

    pub fn selected(self) -> Style {
        Style::default()
            .fg(self.bg)
            .bg(self.cyan)
            .add_modifier(Modifier::BOLD)
    }
}

pub fn gradient(tick: u64, index: usize, no_color: bool) -> Color {
    if no_color {
        return Color::White;
    }
    let phase = ((tick / 2 + index as u64 * 3) % 24) as i16;
    if phase < 12 {
        Color::Rgb((49 + phase * 10) as u8, (231 - phase * 10) as u8, 255)
    } else {
        let phase = phase - 12;
        Color::Rgb(
            (169 - phase * 9) as u8,
            (111 + phase * 10) as u8,
            (255 - phase * 5) as u8,
        )
    }
}
