//! Pager state and key handling. Pure, so it is easy to test.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Down(usize),
    Up(usize),
    PageDown,
    PageUp,
    HalfDown,
    HalfUp,
    Top,
    Bottom,
    Quit,
}

pub fn key_action(key: KeyEvent) -> Option<Action> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    Some(match key.code {
        KeyCode::Char('c') if ctrl => Action::Quit,
        KeyCode::Char('f') if ctrl => Action::PageDown,
        KeyCode::Char('b') if ctrl => Action::PageUp,
        KeyCode::Char('d') if ctrl => Action::HalfDown,
        KeyCode::Char('u') if ctrl => Action::HalfUp,
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down | KeyCode::Enter => Action::Down(1),
        KeyCode::Char('k') | KeyCode::Up => Action::Up(1),
        KeyCode::Char(' ') | KeyCode::PageDown | KeyCode::Char('f') => Action::PageDown,
        KeyCode::Char('b') | KeyCode::PageUp => Action::PageUp,
        KeyCode::Char('d') => Action::HalfDown,
        KeyCode::Char('u') => Action::HalfUp,
        KeyCode::Char('g') | KeyCode::Home => Action::Top,
        KeyCode::Char('G') | KeyCode::End => Action::Bottom,
        _ => return None,
    })
}

#[derive(Debug, Default, Clone)]
pub struct Pager {
    pub scroll: usize,
    /// Rows available for content.
    pub height: usize,
    /// Total content lines.
    pub len: usize,
    pub quit: bool,
}

impl Pager {
    pub fn max_scroll(&self) -> usize {
        self.len.saturating_sub(self.height)
    }

    pub fn clamp(&mut self) {
        self.scroll = self.scroll.min(self.max_scroll());
    }

    pub fn apply(&mut self, a: Action) {
        let page = self.height.saturating_sub(2).max(1);
        let half = (self.height / 2).max(1);
        match a {
            Action::Down(n) => self.scroll += n,
            Action::Up(n) => self.scroll = self.scroll.saturating_sub(n),
            Action::PageDown => self.scroll += page,
            Action::PageUp => self.scroll = self.scroll.saturating_sub(page),
            Action::HalfDown => self.scroll += half,
            Action::HalfUp => self.scroll = self.scroll.saturating_sub(half),
            Action::Top => self.scroll = 0,
            Action::Bottom => self.scroll = self.max_scroll(),
            Action::Quit => self.quit = true,
        }
        self.clamp();
    }

    /// Percentage through the document, for the status bar.
    pub fn percent(&self) -> usize {
        if self.max_scroll() == 0 {
            100
        } else {
            self.scroll * 100 / self.max_scroll()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_clamps() {
        let mut p = Pager {
            scroll: 0,
            height: 10,
            len: 25,
            quit: false,
        };
        p.apply(Action::PageDown);
        assert_eq!(p.scroll, 8);
        p.apply(Action::PageDown);
        assert_eq!(p.scroll, 15);
        p.apply(Action::Up(100));
        assert_eq!(p.scroll, 0);
        p.apply(Action::Bottom);
        assert_eq!(p.percent(), 100);
    }

    #[test]
    fn short_doc_does_not_scroll() {
        let mut p = Pager {
            scroll: 0,
            height: 10,
            len: 3,
            quit: false,
        };
        p.apply(Action::Down(1));
        assert_eq!(p.scroll, 0);
    }
}
