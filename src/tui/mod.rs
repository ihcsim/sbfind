use log::*;
use ratatui::{
    DefaultTerminal, Frame,
    widgets::{ListState, ScrollbarState},
};
use std::error::Error;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use tui_input::Input;

use super::sbsearch::{self, Entry, LogType, SearchCache, SearchResult};

mod event;
mod render;

pub const DEFAULT_MAX_ENTRIES_PER_PAGE: usize = 100;

#[derive(Debug)]
pub struct Tui<'a> {
    cache: &'a mut SearchCache,
    result: SearchResult,
    log_type: LogType,

    current_screen: Screen,
    exit: bool,
    nav_state: ListState,
    keyword: String,
    search: String,
    search_input: Input,
    search_mode: SearchMode,
    sbpath: String,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,

    page_final: usize,
    page_goto: usize,
    page_max_entries: usize,
    page_reload: bool,

    last_saved_filename: String,
}

#[derive(Debug, Default, PartialEq)]
enum Screen {
    #[default]
    Main,
    ConfirmExit,
    ConfirmSave,
}

#[derive(Debug, Default, PartialEq, Clone)]
enum SearchMode {
    #[default]
    Normal,
    Insert,
}

impl<'a> Tui<'a> {
    pub fn new(support_bundle_path: &str, keyword: &str, cache: &'a mut SearchCache) -> Self {
        Self {
            cache,
            result: SearchResult {
                system_entries_offset: Vec::new(),
                workload_entries_offset: Vec::new(),
            },
            log_type: LogType::Workload,

            current_screen: Screen::Main,
            exit: false,
            nav_state: ListState::default().with_selected(Some(0)),
            keyword: String::from(keyword),
            search: String::new(),
            search_input: Input::default(),
            search_mode: SearchMode::default(),
            sbpath: String::from(support_bundle_path),
            vertical_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,

            page_final: 1,
            page_goto: 1,
            page_max_entries: DEFAULT_MAX_ENTRIES_PER_PAGE,
            page_reload: true,

            last_saved_filename: String::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<(), Box<dyn Error>> {
        info!(
            "searching for '{}' in support bundle at '{}'",
            self.keyword, self.sbpath
        );
        while !self.exit {
            if self.page_reload {
                self.read_entries_from_sb()?;
            }

            terminal.draw(|frame| match self.current_screen {
                Screen::ConfirmExit => self.draw_popup(
                    "Confirm Exit",
                    "are you sure you want to exit? (y/n)",
                    30,
                    15,
                    frame,
                ),
                Screen::ConfirmSave => {
                    let filename =
                        format!("sbsearch_{}.log", chrono::Utc::now().format("%Y%m%d%H%M%S"));
                    self.draw_popup(
                        "Confirm Save",
                        format!("save search result to ./{}? (y/n)", filename).as_str(),
                        40,
                        15,
                        frame,
                    );
                    self.last_saved_filename = filename;
                }
                _ => self.draw_main(frame),
            })?;
            event::handle(self)?;
        }
        Ok(())
    }

    fn read_entries_from_sb(&mut self) -> Result<(), Box<dyn Error>> {
        let root_path = Path::new(self.sbpath.as_str());
        let keyword = self.keyword.as_str();
        if self.cache.all.is_empty() {
            sbsearch::search(root_path, keyword, self.cache)?
        }
        info!(
            "found {} entries matching '{}'",
            self.cache.all.len(),
            keyword
        );

        // paginate the entries in cache based on page_goto and page_max_entries
        let offset = self.page_goto * self.page_max_entries - self.page_max_entries;
        let limit = self
            .page_max_entries
            .min(self.cache.all.len().saturating_sub(offset));

        self.result.system_entries_offset = self
            .cache
            .system
            .iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();
        info!(
            "found {} system entries on page {}",
            self.result.system_entries_offset.len(),
            offset / limit + 1
        );

        self.result.workload_entries_offset = self
            .cache
            .workload
            .iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();
        info!(
            "found {} workload entries on page {}",
            self.result.workload_entries_offset.len(),
            offset / limit + 1
        );

        self.page_final = self.cache.all.len().div_ceil(self.page_max_entries);
        self.page_reload = false;
        self.nav_state = ListState::default().with_selected(Some(0));
        Ok(())
    }

    fn save_to_file(&mut self) -> io::Result<()> {
        if let Ok(file) = std::fs::File::create(&self.last_saved_filename) {
            info!("saving to file '{}'", &self.last_saved_filename);
            let mut writer = BufWriter::new(&file);
            for entry in &self.cache.all {
                write!(writer, "{}", entry)?;
            }
        }
        self.current_screen = Screen::Main;
        Ok(())
    }

    fn exit(&mut self) {
        info!("exiting sbsearch TUI");
        self.exit = true
    }

    fn focus_entries(&self) -> &Vec<Entry> {
        if self.log_type == LogType::Workload {
            &self.result.workload_entries_offset
        } else {
            &self.result.system_entries_offset
        }
    }

    fn draw_main(&mut self, frame: &mut Frame) {
        let sections = render::split_main_layout(frame.area());
        let offset = self.page_goto * self.page_max_entries - self.page_max_entries;
        let (filepath, selected) = match self.nav_state.selected() {
            Some(pos) => {
                if self.focus_entries().is_empty() {
                    ("", 0)
                } else {
                    let path_str = self.focus_entries()[pos].path.as_str();
                    let name_str = self.sbpath.as_str();
                    if let Some(index) = path_str.find(name_str) {
                        (
                            &path_str[index + name_str.len()..path_str.len()],
                            offset + pos + 1,
                        )
                    } else {
                        ("", 0)
                    }
                }
            }
            None => ("", 0),
        };
        let scroll_width = sections[2].width.max(3) - 3;
        let search_scroll = self.search_input.visual_scroll(scroll_width as usize);
        let search_cursor_pos =
            self.search_input.visual_cursor().max(search_scroll) - search_scroll + 8;
        let search_cursor_show = self.search_mode == SearchMode::Insert;

        let mut r = render::Renderer::new(
            String::from(filepath),
            self.keyword.clone(),
            self.page_final,
            self.page_goto,
            self.cache.all.len(),
            selected,
            self.sbpath.clone(),
            search_cursor_pos as u16,
            search_cursor_show,
            search_scroll as u16,
            self.search_input.value().to_string(),
            &self.result.system_entries_offset,
            &self.result.workload_entries_offset,
            self.log_type.clone(),
            &mut self.nav_state,
            self.vertical_scroll_state,
        );
        r.render_title_section(sections[0], frame);
        r.render_meta_section(sections[1], frame);
        r.render_search_section(sections[2], frame);
        r.render_logs_section(sections[3], frame);
    }

    fn draw_popup(&self, title: &str, text: &str, width: u16, height: u16, frame: &mut Frame) {
        render::draw_popup(title, text, width, height, frame);
    }

    fn nav_next_line(&mut self) {
        if self.focus_entries().is_empty() {
            return;
        }

        self.vertical_scroll = self.vertical_scroll.saturating_add(1);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
        let i = match self.nav_state.selected() {
            Some(i) => {
                if i < self.focus_entries().len() - 1 {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.nav_state.select(Some(i));
    }

    fn nav_prev_line(&mut self) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
        let i = match self.nav_state.selected() {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.nav_state.select(Some(i));
    }

    fn nav_first_line(&mut self) {
        self.vertical_scroll_state = self.vertical_scroll_state.position(0);
        self.nav_state.select(Some(0));
    }

    fn nav_last_line(&mut self) {
        if !self.focus_entries().is_empty() {
            let end = self.focus_entries().len() - 1;
            self.vertical_scroll_state = self.vertical_scroll_state.position(end);
            self.nav_state.select(Some(end));
        }
    }

    fn nav_next_page(&mut self) {
        if self.page_goto < self.page_final {
            self.page_goto = self.page_goto.saturating_add(1);
            self.page_reload = true;
        }
    }

    fn nav_prev_page(&mut self) {
        if self.page_goto > 1 {
            self.page_goto = self.page_goto.saturating_sub(1);
            self.page_reload = true;
        }
    }

    fn nav_first_page(&mut self) {
        self.page_goto = 1;
        self.page_reload = true;
    }

    fn nav_last_page(&mut self) {
        if self.page_final > 0 {
            self.page_goto = self.page_final;
            self.page_reload = true;
        }
    }

    fn toggle_log_type(&mut self) {
        if self.log_type == LogType::Workload {
            self.log_type = LogType::System;
        } else {
            self.log_type = LogType::Workload;
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_entries_from_sb() {
        let path = "./testdata/support_bundle";

        // there are 218 entries containing "vm-00" in the testdata support bundle.
        // after paging, only 100 entries are loaded into entries_offset with a total
        // of 3 pages.
        let keyword = "vm-00";
        let mut cache = SearchCache {
            all: Vec::new(),
            system: Vec::new(),
            workload: Vec::new(),
        };
        let mut tui = Tui::new(path, keyword, &mut cache);
        assert!(tui.read_entries_from_sb().is_ok());
        assert_eq!(tui.page_final, 3);
        assert_eq!(tui.nav_state, ListState::default().with_selected(Some(0)));
        assert!(!tui.page_reload);

        // check the number of entries loaded into entries_offset for workload and system logs
        assert_eq!(tui.focus_entries().len(), DEFAULT_MAX_ENTRIES_PER_PAGE);
        tui.toggle_log_type();
        assert_eq!(tui.focus_entries().len(), 26);
        assert_eq!(cache.all.len(), 244);

        // use a different keyword
        let keyword = "vm-00-disk-0-";
        let mut cache = SearchCache {
            all: Vec::new(),
            system: Vec::new(),
            workload: Vec::new(),
        };
        let mut tui = Tui::new(path, keyword, &mut cache);
        assert!(tui.read_entries_from_sb().is_ok());
        assert_eq!(tui.page_final, 1);
        assert_eq!(tui.nav_state, ListState::default().with_selected(Some(0)));
        assert!(!tui.page_reload);

        // check the number of entries loaded into entries_offset for workload and system logs
        assert_eq!(tui.focus_entries().len(), 72);
        tui.toggle_log_type();
        assert!(tui.focus_entries().is_empty());
        assert_eq!(tui.cache.all.len(), 72);
    }

    #[test]
    fn test_save_to_file() {
        let path = "./testdata/support_bundle/logs";
        let keyword = "vm-00";
        let mut cache = SearchCache {
            all: Vec::new(),
            system: Vec::new(),
            workload: Vec::new(),
        };
        let mut tui = Tui::new(path, keyword, &mut cache);
        let file = NamedTempFile::new().unwrap();
        tui.last_saved_filename = file.path().to_str().unwrap().to_string();
        assert!(tui.read_entries_from_sb().is_ok());

        let result = tui.save_to_file();
        assert!(result.is_ok());

        let opened = File::open(file.path()).unwrap();
        let reader = BufReader::new(opened);
        let mut num_lines = 0;
        for _line in reader.lines() {
            num_lines += 1;
        }
        assert_eq!(num_lines, tui.cache.all.len());
    }
}
