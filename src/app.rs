use std::collections::VecDeque;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Sparkline, Table},
};
use serde::Deserialize;

const MAX_HISTORY: usize = 60;

// --- /varz response ---

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct Varz {
    #[serde(default)]
    pub server_id: String,
    #[serde(default)]
    pub server_name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub uptime: String,
    #[serde(default)]
    pub cpu: f64,
    #[serde(default)]
    pub mem: i64,
    #[serde(default)]
    pub connections: u64,
    #[serde(default)]
    pub total_connections: u64,
    #[serde(default)]
    pub subscriptions: u32,
    #[serde(default)]
    pub slow_consumers: u64,
    #[serde(default)]
    pub in_msgs: u64,
    #[serde(default)]
    pub out_msgs: u64,
    #[serde(default)]
    pub in_bytes: u64,
    #[serde(default)]
    pub out_bytes: u64,
    #[serde(default)]
    pub routes: u32,
    #[serde(default)]
    pub remotes: u32,
    #[serde(default)]
    pub leafnodes: u32,
}

// --- /jsz response ---

#[derive(Debug, Deserialize, Default)]
pub struct Jsz {
    #[serde(default)]
    pub streams: u64,
    #[serde(default)]
    pub consumers: u64,
    #[serde(default)]
    pub messages: u64,
    #[serde(default)]
    pub bytes: u64,
    #[serde(default)]
    pub api: JszApi,
    #[serde(default)]
    pub account_details: Vec<AccountDetail>,
}

#[derive(Debug, Deserialize, Default)]
pub struct JszApi {
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub errors: u64,
}

#[derive(Debug, Deserialize, Default)]
pub struct AccountDetail {
    #[serde(default)]
    pub stream_detail: Vec<StreamDetail>,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct StreamDetail {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub state: StreamState,
    #[serde(default)]
    pub consumer_detail: Vec<ConsumerDetail>,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct StreamState {
    #[serde(default)]
    pub messages: u64,
    #[serde(default)]
    pub bytes: u64,
    #[serde(default)]
    pub consumer_count: u64,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct ConsumerDetail {
    #[serde(default)]
    pub stream_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub delivered: ConsumerSeqInfo,
    #[serde(default)]
    pub ack_floor: ConsumerSeqInfo,
    #[serde(default)]
    pub num_ack_pending: u64,
    #[serde(default)]
    pub num_redelivered: u64,
    #[serde(default)]
    pub num_waiting: u64,
    #[serde(default)]
    pub num_pending: u64,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct ConsumerSeqInfo {
    #[serde(default)]
    pub consumer_seq: u64,
    #[serde(default)]
    pub stream_seq: u64,
}

// --- App state ---

pub struct App {
    pub varz: Varz,
    pub jsz: Jsz,
    pub connection_history: VecDeque<u64>,
    pub msgs_history: VecDeque<u64>,
    pub last_in_msgs: u64,
    pub last_error: Option<String>,
    pub url: String,
}

impl App {
    pub fn new(url: String) -> Self {
        Self {
            varz: Varz::default(),
            jsz: Jsz::default(),
            connection_history: VecDeque::with_capacity(MAX_HISTORY + 1),
            msgs_history: VecDeque::with_capacity(MAX_HISTORY + 1),
            last_in_msgs: 0,
            last_error: None,
            url,
        }
    }

    pub fn update_varz(&mut self, varz: Varz) {
        if self.connection_history.len() >= MAX_HISTORY {
            self.connection_history.pop_front();
        }
        self.connection_history.push_back(varz.connections);

        // Track message rate (msgs/sec)
        if self.msgs_history.len() >= MAX_HISTORY {
            self.msgs_history.pop_front();
        }
        let rate = varz.in_msgs.saturating_sub(self.last_in_msgs);
        // Skip the first tick where last_in_msgs is 0
        if self.last_in_msgs > 0 {
            self.msgs_history.push_back(rate);
        }
        self.last_in_msgs = varz.in_msgs;

        self.varz = varz;
        self.last_error = None;
    }

    pub fn update_jsz(&mut self, jsz: Jsz) {
        self.jsz = jsz;
    }

    pub fn set_error(&mut self, err: String) {
        self.last_error = Some(err);
    }
}

// --- Rendering ---

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Length(9),  // stats
            Constraint::Min(5),    // jetstream consumers table + charts
            Constraint::Length(1), // footer
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_stats(frame, app, chunks[1]);
    render_bottom(frame, app, chunks[2]);
    render_footer(frame, app, chunks[3]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let name = if app.varz.server_name.is_empty() {
        "N/A"
    } else {
        &app.varz.server_name
    };
    let version = if app.varz.version.is_empty() {
        "N/A"
    } else {
        &app.varz.version
    };
    let uptime = if app.varz.uptime.is_empty() {
        "N/A"
    } else {
        &app.varz.uptime
    };

    let header_text = Line::from(vec![
        Span::styled(
            " NATS Monitor ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("│ "),
        Span::styled(name, Style::default().fg(Color::Green)),
        Span::raw(" (v"),
        Span::styled(version, Style::default().fg(Color::Yellow)),
        Span::raw(") │ Uptime: "),
        Span::styled(uptime, Style::default().fg(Color::White)),
    ]);

    let header = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(header, area);
}

fn render_stats(frame: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    // Left column: message and byte stats
    let left_lines = vec![
        stat_line("Messages In", format_number(app.varz.in_msgs), Color::Green),
        stat_line(
            "Messages Out",
            format_number(app.varz.out_msgs),
            Color::Green,
        ),
        Line::raw(""),
        stat_line("Bytes In", format_bytes(app.varz.in_bytes), Color::Blue),
        stat_line("Bytes Out", format_bytes(app.varz.out_bytes), Color::Blue),
        Line::raw(""),
        stat_line("CPU", format!("{:.1}%", app.varz.cpu), Color::Magenta),
    ];

    let left = Paragraph::new(left_lines).block(
        Block::default()
            .title(" Throughput ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(left, columns[0]);

    // Middle column: connection stats
    let mid_lines = vec![
        stat_line(
            "Connections",
            format_number(app.varz.connections),
            Color::Cyan,
        ),
        stat_line(
            "Total Conns",
            format_number(app.varz.total_connections),
            Color::Cyan,
        ),
        Line::raw(""),
        stat_line(
            "Subscriptions",
            format_number(app.varz.subscriptions as u64),
            Color::Yellow,
        ),
        stat_line(
            "Slow Consumers",
            format_number(app.varz.slow_consumers),
            if app.varz.slow_consumers > 0 {
                Color::Red
            } else {
                Color::Green
            },
        ),
        Line::raw(""),
        stat_line(
            "Memory",
            format_bytes(app.varz.mem as u64),
            Color::Magenta,
        ),
    ];

    let mid = Paragraph::new(mid_lines).block(
        Block::default()
            .title(" Connections ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(mid, columns[1]);

    // Right column: JetStream overview
    let right_lines = vec![
        stat_line("Streams", format_number(app.jsz.streams), Color::Cyan),
        stat_line("Consumers", format_number(app.jsz.consumers), Color::Cyan),
        Line::raw(""),
        stat_line("Stored Msgs", format_number(app.jsz.messages), Color::Green),
        stat_line("Stored Bytes", format_bytes(app.jsz.bytes), Color::Blue),
        Line::raw(""),
        stat_line("API Total", format_number(app.jsz.api.total), Color::Yellow),
        stat_line(
            "API Errors",
            format_number(app.jsz.api.errors),
            if app.jsz.api.errors > 0 {
                Color::Red
            } else {
                Color::Green
            },
        ),
    ];

    let right = Paragraph::new(right_lines).block(
        Block::default()
            .title(" JetStream ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta)),
    );
    frame.render_widget(right, columns[2]);
}

fn render_bottom(frame: &mut Frame, app: &App, area: Rect) {
    // Split bottom into: consumers table (left) and charts (right)
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    render_consumers_table(frame, app, cols[0]);
    render_charts(frame, app, cols[1]);
}

fn render_consumers_table(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Consumer").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Stream").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Delivered").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Ack Pend").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Pending").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Redlvr").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]);

    let mut rows = Vec::new();
    for account in &app.jsz.account_details {
        for stream in &account.stream_detail {
            for consumer in &stream.consumer_detail {
                let ack_color = if consumer.num_ack_pending > 0 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                let pending_color = if consumer.num_pending > 0 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                let redlvr_color = if consumer.num_redelivered > 0 {
                    Color::Red
                } else {
                    Color::Green
                };

                rows.push(Row::new(vec![
                    Cell::from(consumer.name.clone()).style(Style::default().fg(Color::White)),
                    Cell::from(stream.name.clone()).style(Style::default().fg(Color::DarkGray)),
                    Cell::from(format_number(consumer.delivered.consumer_seq))
                        .style(Style::default().fg(Color::Green)),
                    Cell::from(format_number(consumer.num_ack_pending))
                        .style(Style::default().fg(ack_color)),
                    Cell::from(format_number(consumer.num_pending))
                        .style(Style::default().fg(pending_color)),
                    Cell::from(format_number(consumer.num_redelivered))
                        .style(Style::default().fg(redlvr_color)),
                ]));
            }
        }
    }

    if rows.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No consumers found",
            Style::default().fg(Color::DarkGray),
        )))
        .block(
            Block::default()
                .title(" Consumers ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );
        frame.render_widget(empty, area);
    } else {
        let table = Table::new(
            rows,
            [
                Constraint::Min(20),
                Constraint::Min(12),
                Constraint::Min(10),
                Constraint::Min(9),
                Constraint::Min(8),
                Constraint::Min(7),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title(" Consumers ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );
        frame.render_widget(table, area);
    }
}

fn render_charts(frame: &mut Frame, app: &App, area: Rect) {
    let charts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Connections sparkline
    let conn_data: Vec<u64> = app.connection_history.iter().copied().collect();
    let conn_sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(" Connections (60s) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .data(&conn_data)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(conn_sparkline, charts[0]);

    // Messages/sec sparkline
    let msgs_data: Vec<u64> = app.msgs_history.iter().copied().collect();
    let msgs_sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(" Messages/sec (60s) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .data(&msgs_data)
        .style(Style::default().fg(Color::Green));
    frame.render_widget(msgs_sparkline, charts[1]);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(ref err) = app.last_error {
        Line::from(vec![
            Span::styled(
                " (q) Quit ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw("│ "),
            Span::styled(
                format!("ERROR: {err}"),
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                " (q) Quit ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw("│ "),
            Span::styled(
                format!("Polling {}", app.url),
                Style::default().fg(Color::DarkGray),
            ),
        ])
    };

    let footer = Paragraph::new(content);
    frame.render_widget(footer, area);
}

fn stat_line(label: &str, value: String, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {label:<16}"),
            Style::default().fg(Color::Gray),
        ),
        Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;
    match bytes {
        b if b >= TB => format!("{:.1} TB", b as f64 / TB as f64),
        b if b >= GB => format!("{:.1} GB", b as f64 / GB as f64),
        b if b >= MB => format!("{:.1} MB", b as f64 / MB as f64),
        b if b >= KB => format!("{:.1} KB", b as f64 / KB as f64),
        b => format!("{b} B"),
    }
}

fn format_number(n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
