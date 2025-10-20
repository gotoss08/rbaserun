use clap::Parser;

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::Text,
    widgets::{Block, List, ListState, Paragraph},
};

use tui_input::{Input, backend::crossterm::EventHandler};

use regex::Regex;

use std::error::Error;
use std::path::Path;
use std::process::Command;

use std::fs::File;
use std::io::{self, BufRead, Write};

#[derive(Parser)]
struct Cli {
    path: Option<String>,

    /// Launch in designer mode
    #[arg(short, long)]
    designer: bool,
}

#[derive(Debug)]
enum PathKind {
    Server { host: String, ref_name: String },
    File { path: String },
    Web { url: String },
}

fn parse_base_path(input_path: &str) -> Result<PathKind, Box<dyn Error>> {
    let s = input_path.trim();

    if s.contains("ws=") {
        return parse_base_web_form(&s);
    }

    let s = s.to_lowercase();

    if s.contains("file=") {
        parse_base_file_form(&s)
    } else if s.contains("srvr=") && s.contains("ref=") {
        parse_base_server_form(&s)
    } else if s.contains(";") {
        parse_base_simple_form(&s)
    } else {
        Err(format!("Could not parse provided path: {input_path}").into())
    }
}

fn parse_base_simple_form(input: &str) -> Result<PathKind, Box<dyn Error>> {
    let captures = Regex::new(r"(.+);(.+)")
        .unwrap()
        .captures(input)
        .ok_or("expected pattern: host;ref")?;
    Ok(PathKind::Server {
        host: captures[1].to_string(),
        ref_name: captures[2].to_string(),
    })
}

fn parse_base_server_form(input: &str) -> Result<PathKind, Box<dyn Error>> {
    let captures = Regex::new(r#""(.+)".+"(.+)""#)
        .unwrap()
        .captures(input)
        .ok_or("expected pattern: Srvr=\"host\";Ref=\"ref\";")?;
    Ok(PathKind::Server {
        host: captures[1].to_string(),
        ref_name: captures[2].to_string(),
    })
}

fn parse_base_file_form(input: &str) -> Result<PathKind, Box<dyn Error>> {
    let captures = Regex::new(r#""(.+)""#)
        .unwrap()
        .captures(&input)
        .ok_or("expected pattern: File=\"<path>\";")?;
    Ok(PathKind::File {
        path: captures[1].to_string(),
    })
}

fn parse_base_web_form(input: &str) -> Result<PathKind, Box<dyn Error>> {
    let captures = Regex::new(r#""(.+)""#)
        .unwrap()
        .captures(input)
        .ok_or("expected pattern: ws=\"<url>\";")?;
    Ok(PathKind::Web {
        url: captures[1].to_string(),
    })
}

fn launch_base(path: PathKind, designer: bool) -> Result<(), Box<dyn Error>> {
    // TODO: add option to get 1cestart.exe path from cmd args or config file
    let starter = Path::new(r#"c:\Program Files\1cv8\common\1cestart.exe"#);

    if !starter.exists() {
        return Err(format!("Could not locate 1C starter app: '{}'", starter.display()).into());
    }

    let launch_mode = if designer { "DESIGNER" } else { "ENTERPRISE" };

    match path {
        PathKind::Server { host, ref_name } => {
            Command::new(starter)
                .args([launch_mode, "/S", &format!("{host}\\{ref_name}")])
                .spawn()?;
        }

        PathKind::File { path } => {
            Command::new(starter)
                .args([launch_mode, "/F", &path])
                .spawn()?;
        }

        PathKind::Web { url } => {
            Command::new(starter)
                .args([launch_mode, "/WS", &url])
                .spawn()?;
        }
    }

    Ok(())
}

#[derive(Debug, Default)]
pub struct App {
    designer: bool,
    input: Input,
    error: bool,
    error_text: String,
    history: Vec<String>,
    history_state: ListState,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<(), Box<dyn Error>> {
        self.load_history();
        loop {
            let event = event::read()?;
            match event {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
                    match key_event.code {
                        KeyCode::Esc => break,
                        KeyCode::Char('d') if ctrl => self.designer = !self.designer,
                        KeyCode::Enter => {
                            if let Some(selected_index) = self.history_state.selected() {
                                self.input = self.history[selected_index].clone().into();
                                self.history_state.select(None);
                            } else if !self.input.value().is_empty() {
                                let result =
                                    try_parse_and_launch(self.input.value().to_string(), self.designer);
                                match result {
                                    Ok(()) => {
                                        self.add_to_history(self.input.value().to_string())?;
                                        break;
                                    }
                                    Err(e) => {
                                        self.error = true;
                                        self.error_text = e.to_string();
                                    }
                                };
                            }
                        },
                        KeyCode::Up => self.history_state.select_previous(),
                        KeyCode::Down => self.history_state.select_next(),
                        _ => {
                            self.history_state.select(None);
                        }
                    };
                    self.input.handle_event(&event);
                }
                _ => {}
            }
            terminal.draw(|frame| {
                let [input_area, config_area, history_area] = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Length(2),
                    Constraint::Min(1),
                ])
                .areas(frame.area());

                self.render_input(frame, input_area);
                self.render_config(frame, config_area);
                self.render_history(frame, history_area);
            })?;
        }
        Ok(())
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let width = area.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);
        let input_widget = Paragraph::new(self.input.value())
            .scroll((0, scroll as u16))
            .block(Block::bordered().title("Base path:"));

        frame.render_widget(input_widget, area);

        let x = self.input.visual_cursor().max(scroll) - scroll + 1;
        frame.set_cursor_position((area.x + x as u16, area.y + 1));
    }

    fn render_config(&self, frame: &mut Frame, area: Rect) {
        let mut lines = Vec::new();

        if self.error {
            lines.push(self.error_text.to_string().red().into());
        }

        if self.designer {
            lines.push("Ctrl+D: Designer (on)".green().into());
        } else {
            lines.push("Ctrl+D: Designer (off)".into());
        };

        let config_widget = Paragraph::new(lines);
        frame.render_widget(config_widget, area);
    }

    fn render_history(&mut self, frame: &mut Frame, area: Rect) {
        let list = List::new(self.history.clone())
            .block(Block::bordered().title("History"))
            .highlight_style(Style::new().reversed());
            // .highlight_symbol(">>");
        frame.render_stateful_widget(list, area, &mut self.history_state);
    }

    fn add_to_history(&mut self, path: String) -> Result<(), std::io::Error> {
        if !self.history.contains(&path) {
            self.history.push(path);
        } else if let Some(index) = self.history.iter().position(|x| x.to_string() == path.to_string()) {
            let removed_value = self.history.remove(index);
            self.history.insert(0, removed_value);
        }
        self.dump_history()
    }

    fn load_history(&mut self) {
        if let Ok(lines) = read_lines("./rbaserun_history.txt") {
            for line in lines.map_while(Result::ok) {
                self.history.push(line);
            }
        }
    }

    fn dump_history(&self) -> Result<(), std::io::Error> {
        if let Ok(mut file) = File::create("./rbaserun_history.txt") {
            for line in &self.history {
                writeln!(file, "{}", line)?;
            }
        }
        Ok(())
    }
}

fn try_parse_and_launch(path: String, designer: bool) -> Result<(), Box<dyn Error>> {
    let parsed_path = match parse_base_path(&path) {
        Ok(path) => path,
        Err(e) => return Err(format!("Parsing error: {}", e).into()),
    };

    match launch_base(parsed_path, designer) {
        Ok(()) => {}
        Err(e) => return Err(format!("Launcher error: {}", e).into()),
    };

    Ok(())
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if let Some(path) = cli.path {
        try_parse_and_launch(path, cli.designer)
    } else {
        let mut terminal = ratatui::init();
        let app_result = App::default().run(&mut terminal);
        ratatui::restore();
        app_result
    }
}
