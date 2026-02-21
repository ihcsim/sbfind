use super::sbsearch::{Entry, LogType};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};
use std::rc::Rc;
use textwrap::Options;

pub fn draw_popup(title: &str, text: &str, percent_x: u16, percent_y: u16, frame: &mut Frame) {
    let popup_area = split_popup_layout(percent_x, percent_y, frame.area());
    let popup_block = Block::default()
        .title(Line::from(title).centered())
        .borders(Borders::ALL)
        .style(Style::default());
    let popup_para = Paragraph::new(text)
        .block(popup_block)
        .alignment(Alignment::Center);
    frame.render_widget(popup_para, popup_area);
}

pub struct Renderer<'a> {
    filepath: String,
    keyword: String,
    page_final: usize,
    page_goto: usize,
    page_total_entries: usize,
    selected: usize,
    title: String,

    search_cursor_pos: u16,
    search_cursor_show: bool,
    search_scroll: u16,
    search_value: String,

    entries: &'a Vec<Entry>,
    log_type: LogType,
    nav_state: &'a mut ListState,
    vertical_scroll_state: ScrollbarState,
}

impl<'a> Renderer<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        filepath: String,
        keyword: String,
        page_final: usize,
        page_goto: usize,
        page_total_entries: usize,
        selected: usize,
        title: String,
        search_cursor_pos: u16,
        search_cursor_show: bool,
        search_scroll: u16,
        search_value: String,
        entries: &'a Vec<Entry>,
        log_type: LogType,
        nav_state: &'a mut ListState,
        vertical_scroll_state: ScrollbarState,
    ) -> Self {
        Renderer {
            filepath,
            keyword,
            page_final,
            page_goto,
            page_total_entries,
            selected,
            title,
            search_cursor_pos,
            search_cursor_show,
            search_scroll,
            search_value,
            entries,
            log_type,
            nav_state,
            vertical_scroll_state,
        }
    }

    pub fn render_title_section(&self, area: Rect, frame: &mut Frame) {
        let instructions = Line::from(vec![
            Span::styled(" | (Line)", Style::default().fg(Color::White)),
            Span::styled(" Up", Style::default()),
            Span::styled("<Up>", Style::default().fg(Color::Blue).bold()),
            Span::styled(" Down", Style::default()),
            Span::styled("<Down>", Style::default().fg(Color::Blue).bold()),
            Span::styled(" Start", Style::default()),
            Span::styled("<g>", Style::default().fg(Color::Blue).bold()),
            Span::styled(" End", Style::default()),
            Span::styled("<G>", Style::default().fg(Color::Blue).bold()),
            Span::styled(" | (Page)", Style::default().fg(Color::White)),
            Span::styled(" Previous", Style::default()),
            Span::styled("<Left>", Style::default().fg(Color::Blue).bold()),
            Span::styled(" Next", Style::default()),
            Span::styled("<Right>", Style::default().fg(Color::Blue).bold()),
            Span::styled(" First", Style::default()),
            Span::styled("<0>", Style::default().fg(Color::Blue).bold()),
            Span::styled(" Last", Style::default()),
            Span::styled("<9>", Style::default().fg(Color::Blue).bold()),
            Span::styled(" | (Search)", Style::default().fg(Color::White)),
            Span::styled(" Edit", Style::default()),
            Span::styled("</>", Style::default().fg(Color::Blue).bold()),
            Span::styled(" Clear", Style::default()),
            Span::styled("<c>", Style::default().fg(Color::Blue).bold()),
            Span::styled(" | ", Style::default().fg(Color::White)),
            Span::styled(" Save", Style::default()),
            Span::styled("<s>", Style::default().fg(Color::Blue).bold()),
            Span::styled(" Quit", Style::default()),
            Span::styled("<q>", Style::default().fg(Color::Blue).bold()),
            Span::styled(" | ", Style::default().fg(Color::White)),
        ]);
        let title_block = Block::default()
            .borders(Borders::ALL)
            .title_bottom(instructions)
            .title_alignment(Alignment::Center);
        let title_para = Paragraph::new(Text::styled(
            self.title.clone(),
            Style::default().fg(Color::Green).bold(),
        ))
        .alignment(Alignment::Center)
        .block(title_block);
        frame.render_widget(title_para, area);
    }

    pub fn render_meta_section(&self, area: Rect, frame: &mut Frame) {
        let meta_block = Block::default().borders(Borders::ALL);
        let meta_lines = vec![
            Line::from(vec![
                Span::styled("Keyword: ", Style::default().fg(Color::Green).bold()),
                Span::styled(&self.keyword, Style::default().fg(Color::Green).bold()),
                Span::styled(" | ", Style::default().fg(Color::White)),
                Span::styled("Line: ", Style::default().fg(Color::Green).bold()),
                Span::styled(
                    format!("{}/{}", self.selected, self.page_total_entries),
                    Style::default().fg(Color::Green).bold(),
                ),
                Span::styled(" | ", Style::default().fg(Color::White)),
                Span::styled("Page: ", Style::default().fg(Color::Green).bold()),
                Span::styled(
                    format!("{}/{}", self.page_goto, self.page_final),
                    Style::default().fg(Color::Green).bold(),
                ),
            ]),
            Line::from(vec![
                Span::styled("Filepath: ", Style::default().fg(Color::Green).bold()),
                Span::styled(
                    self.filepath.clone(),
                    Style::default().fg(Color::Green).bold(),
                ),
            ]),
        ];
        let meta_para = Paragraph::new(meta_lines)
            .block(meta_block)
            .alignment(Alignment::Center);
        frame.render_widget(meta_para, area);
    }

    pub fn render_search_section(&self, area: Rect, frame: &mut Frame) {
        let search_block = Block::default().borders(Borders::ALL);
        let search_lines = Line::from(vec![
            Span::styled("Search: ", Style::default().fg(Color::Green).bold()),
            Span::styled(self.search_value.clone(), Style::default()),
        ]);
        let input = Paragraph::new(search_lines)
            .style(Style::default())
            .scroll((0, self.search_scroll))
            .block(search_block);
        frame.render_widget(input, area);

        // show cursor only in insert mode
        if self.search_cursor_show {
            frame.set_cursor_position((area.x + self.search_cursor_pos, area.y + 1));
        }
    }

    pub fn render_logs_section(&mut self, area: Rect, frame: &mut Frame) {
        let mut lines: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let width = frame.area().as_size().width as usize;
                let options = Options::new(width);
                let text = format!("{}", entry);
                let wrapped = textwrap::fill(text.as_str(), options);
                let list_item = match entry.level.as_str() {
                    "error" => ListItem::new(wrapped).red(),
                    "warn" | "warning" => ListItem::new(wrapped).yellow(),
                    _ => ListItem::new(wrapped),
                };
                if !self.search_value.is_empty()
                    && text
                        .clone()
                        .to_lowercase()
                        .contains(self.search_value.clone().to_lowercase().as_str())
                {
                    list_item.on_blue()
                } else {
                    list_item
                }
            })
            .collect();
        if lines.is_empty() {
            lines = vec![ListItem::new("No log entries found.".to_string())];
        }

        let title = Line::from(vec![
            Span::styled(
                format!(" | {} logs", self.log_type),
                Style::default().fg(Color::White),
            ),
            Span::styled(" | (Tab)", Style::default().fg(Color::White)),
            Span::styled(" toggle | ", Style::default()),
        ]);

        let lines_count = lines.len();
        let list_block = Block::default().borders(Borders::ALL).title_top(title);
        let list = List::new(lines)
            .block(list_block)
            .style(Style::default())
            .highlight_symbol(">> ")
            .highlight_style(Style::default().bg(Color::LightMagenta));
        frame.render_stateful_widget(list, area, self.nav_state);

        // render scrollbar
        self.vertical_scroll_state = self.vertical_scroll_state.content_length(lines_count);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area,
            &mut self.vertical_scroll_state,
        );
    }
}

pub fn split_main_layout(r: Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(r)
}

fn split_popup_layout(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_area[1])[1]
}
