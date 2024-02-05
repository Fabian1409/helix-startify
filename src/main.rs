use std::env;
use std::fs::File;
use std::os::unix::process::CommandExt;
use std::{
    error::Error,
    fs,
    io::{self, Write},
    process::Command,
    time::{Duration, Instant},
};

use clap::{arg, command};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Item(String);

impl Item {
    fn as_line(&self, c: char) -> Line {
        let (path, name) = self.0.rsplit_once('/').unwrap();
        Line::from(vec![
            Span::styled("[", Style::default().fg(Color::Gray)),
            Span::styled(c.to_string(), Style::default().fg(Color::Blue)),
            Span::styled("]  ", Style::default().fg(Color::Gray)),
            Span::styled(path.to_owned() + "/", Style::default().fg(Color::DarkGray)),
            Span::styled(name, Style::default()),
        ])
    }
}

#[derive(Default, Serialize, Deserialize)]
struct App {
    recents: Vec<Item>,
    bookmarks: Vec<Item>,
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> Result<Option<String>, Box<dyn Error>> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                        KeyCode::Char(c @ '0'..='f') => {
                            let idx = c.to_digit(16).unwrap() as usize;
                            if let Some(path) = app.recents.get(idx) {
                                return Ok(Some(path.0.clone()));
                            }
                            let idx = idx - app.recents.len();
                            if let Some(path) = app.bookmarks.get(idx) {
                                return Ok(Some(path.0.clone()));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(13 + 5), Constraint::Min(0)])
        .split(f.size());

    let logo = fs::read_to_string("./logo").unwrap();
    let logo_width = logo.lines().map(|x| x.len()).max().unwrap();
    let left_pad = (chunks[0].width - logo_width as u16) / 2;

    f.render_widget(
        Paragraph::new(Text::styled(logo, Style::default().fg(Color::Red)))
            .block(Block::default().padding(Padding::new(left_pad, 0, 5, 0))),
        chunks[0],
    );

    let mut lines = vec![
        Line::styled("Recents", Style::default().fg(Color::Red)),
        Line::default(),
    ];
    for (i, item) in app.recents.iter().enumerate() {
        lines.push(item.as_line(char::from_digit(i as u32, 16).unwrap()));
    }
    lines.append(&mut vec![
        Line::default(),
        Line::styled("Bookmarks", Style::default().fg(Color::Red)),
        Line::default(),
    ]);
    for (i, item) in app.bookmarks.iter().enumerate() {
        lines.push(item.as_line(char::from_digit((i + app.recents.len()) as u32, 16).unwrap()));
    }

    let lines_width = lines.iter().map(|x| x.width()).max().unwrap();
    let left_pad = (chunks[1].width - lines_width as u16) / 2;

    f.render_widget(
        Paragraph::new(lines).block(Block::default().padding(Padding::new(left_pad, 0, 5, 0))),
        chunks[1],
    );
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = command!()
        .arg(arg!(-b --bookmark <PATH> "Add path to bookmarks"))
        .arg(arg!(-c --clear "Clear recents and bookmarks"))
        .get_matches();

    let db_path = format!(
        "/home/{}/.local/share/helix-startify",
        env::var("USER").unwrap()
    );

    let _ = fs::create_dir(&db_path);
    let _ = File::options()
        .create_new(true)
        .open(format!("{db_path}/app.db"));

    let mut app: App =
        serde_json::from_str(&fs::read_to_string(format!("{db_path}/app.db"))?).unwrap_or_default();

    if let Some(path) = matches.get_one::<String>("bookmark") {
        app.bookmarks.push(Item(path.clone()));
        let data = serde_json::to_string(&app)?;
        let mut file = File::options()
            .write(true)
            .truncate(true)
            .open(format!("{db_path}/app.db"))?;
        write!(file, "{}", data)?;
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);
    let res = run_app(&mut terminal, app, tick_rate)?;

    if let Some(path) = res {
        Command::new("hx").arg(path).exec();
    } else {
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
    }

    Ok(())
}
