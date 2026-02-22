use crate::tui::{Screen, SearchMode};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use std::io;
use tui_input::backend::crossterm::EventHandler;

pub fn handle(tui: &mut super::Tui) -> io::Result<()> {
    let event = crossterm::event::read()?;
    handle_key_event(tui, event);
    Ok(())
}

fn handle_key_event(tui: &mut super::Tui, event: Event) {
    if let Event::Key(key_event) = event {
        if key_event.kind != KeyEventKind::Press {
            return;
        }

        match tui.current_screen {
            Screen::Main => match tui.search_mode {
                SearchMode::Normal => match key_event.code {
                    KeyCode::Char('q') => tui.current_screen = Screen::ConfirmExit,
                    KeyCode::Char('/') => {
                        tui.search_mode = SearchMode::Insert;
                        tui.search_input.reset();
                    }
                    KeyCode::Char('c') => {
                        tui.search = String::new();
                        tui.search_input.reset();
                    }
                    KeyCode::Char('s') => {
                        tui.current_screen = Screen::ConfirmSave;
                    }
                    KeyCode::Char('G') => tui.nav_last_line(),
                    KeyCode::Char('g') => tui.nav_first_line(),
                    KeyCode::Up | KeyCode::Char('k') => tui.nav_prev_line(),
                    KeyCode::Down | KeyCode::Char('j') => tui.nav_next_line(),
                    KeyCode::Left => tui.nav_prev_page(),
                    KeyCode::Right => tui.nav_next_page(),
                    KeyCode::Char('0') => tui.nav_first_page(),
                    KeyCode::Char('9') => tui.nav_last_page(),
                    KeyCode::Tab => tui.toggle_log_type(),
                    _ => {}
                },
                SearchMode::Insert => match key_event.code {
                    KeyCode::Enter => {
                        tui.search = String::from(tui.search_input.value());
                        tui.search_mode = SearchMode::Normal;
                    }
                    KeyCode::Esc => {
                        tui.search = String::new();
                        tui.search_input.reset();
                        tui.search_mode = SearchMode::Normal;
                    }
                    _ => {
                        tui.search_input.handle_event(&event);
                    }
                },
            },
            Screen::ConfirmExit => match key_event.code {
                KeyCode::Char('y') => tui.exit(),
                KeyCode::Char('n') => tui.current_screen = Screen::Main,
                _ => {}
            },
            Screen::ConfirmSave => match key_event.code {
                KeyCode::Char('y') => {
                    if let Err(e) = tui.save_to_file() {
                        println!("Error saving to file: {}", e);
                    }
                }
                KeyCode::Char('n') => tui.current_screen = Screen::Main,
                _ => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sbsearch, tui::*};
    use crossterm::event::{KeyEvent, KeyModifiers};

    #[test]
    fn handle_key_events_on_main_screen() {
        let mut cache = SearchCache {
            all: Vec::new(),
            system: Vec::new(),
            workload: Vec::new(),
        };
        let tui = &mut Tui::new("sb_path", "pvc_name", &mut cache);
        tui.result.workload_entries_offset = vec![
            sbsearch::Entry {
                level: String::from("level=info"),
                path: String::from("/path/to/log1"),
                content: String::from("This is an info log entry."),
                timestamp: Some(chrono::Utc::now()),
                log_type: sbsearch::LogType::Workload,
            },
            sbsearch::Entry {
                level: String::from("level=warning"),
                path: String::from("/path/to/log2"),
                content: String::from("This is an warning log entry."),
                timestamp: Some(chrono::Utc::now()),
                log_type: sbsearch::LogType::Workload,
            },
            sbsearch::Entry {
                level: String::from("level=error"),
                path: String::from("/path/to/log3"),
                content: String::from("This is an error log entry."),
                timestamp: Some(chrono::Utc::now()),
                log_type: sbsearch::LogType::Workload,
            },
        ];

        assert_eq!(tui.sbpath, "sb_path");
        assert_eq!(tui.keyword, "pvc_name");
        assert_eq!(tui.current_screen, Screen::Main);
        assert_eq!(tui.log_type, LogType::Workload);

        // navigation keys
        let key_event = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.nav_state.selected(), Some(1));

        let key_event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.nav_state.selected(), Some(2));

        let key_event = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.nav_state.selected(), Some(1));

        let key_event = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.nav_state.selected(), Some(0));

        let key_event = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.nav_state.selected(), Some(2));

        let key_event = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.nav_state.selected(), Some(0));

        let key_event = KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.page_goto, 1);

        let key_event = KeyEvent::new(KeyCode::Char('9'), KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.page_goto, tui.page_final);

        // toggle between workload and system logs
        let key_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.log_type, LogType::System);

        let key_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.log_type, LogType::Workload);

        // confirm exit
        let key_event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.current_screen, Screen::ConfirmExit);

        let key_event = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert!(tui.exit);
    }

    #[test]
    fn handle_key_events_on_search() {
        let mut cache = SearchCache {
            all: Vec::new(),
            system: Vec::new(),
            workload: Vec::new(),
        };
        let tui = &mut Tui::new("sb_path", "pvc_name", &mut cache);
        assert_eq!(tui.search_mode, SearchMode::Normal);

        // enable search mode
        let key_event = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.search_mode, SearchMode::Insert);

        tui.search_input = tui
            .search_input
            .clone()
            .with_value(String::from("test input value"));

        let key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.search, String::from("test input value"));
        assert_eq!(tui.search_mode, SearchMode::Normal);

        // clear search
        let key_event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.search, String::new());
    }

    #[test]
    fn handle_key_events_on_save() {
        let mut cache = SearchCache {
            all: Vec::new(),
            system: Vec::new(),
            workload: Vec::new(),
        };
        let tui = &mut Tui::new("sb_path", "pvc_name", &mut cache);
        tui.current_screen = Screen::Main;
        tui.last_saved_filename = String::new();

        // show confirm save search results
        let key_event = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.current_screen, Screen::ConfirmSave);

        // exit save popup
        let key_event = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        let event = Event::Key(key_event);
        handle_key_event(tui, event);
        assert_eq!(tui.current_screen, Screen::Main);
    }
}
