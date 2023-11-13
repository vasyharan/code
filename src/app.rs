use std::io::Write;

use bstr::ByteSlice;
use clap::Parser;
use crossterm::cursor;
use crossterm::event::{
    Event, EventStream, KeyCode, KeyModifiers, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal;
use crossterm::QueueableCommand;
use futures::{future::FutureExt, StreamExt};
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::{Buffer, Rect};
use ratatui::widgets::Widget;
use ratatui::Terminal;
use tokio::fs::File;
use tokio::sync::mpsc;

use crate::error;
use crate::rope;

#[derive(Parser)]
pub struct Args {
    /// Paths to files to open
    paths: Option<Vec<std::path::PathBuf>>,
}

#[derive(Debug)]
enum Command {
    FileOpen(std::path::PathBuf),
    Quit,
}

pub fn main(args: Args) -> error::Result<()> {
    let supports_keyboard_enhancement =
        matches!(terminal::supports_keyboard_enhancement(), Ok(true));
    setup_panic_handler(supports_keyboard_enhancement);
    terminal_enter(supports_keyboard_enhancement)?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let (command_tx, command_rx) = mpsc::channel(1);
        let app = tokio::spawn(app_main(command_rx));
        if let Some(paths) = args.paths {
            for p in paths.iter() {
                _ = command_tx.send(Command::FileOpen(p.clone())).await?;
            }
        }

        _ = app.await?;
        Ok::<(), error::Error>(())
    })?;

    terminal_exit(supports_keyboard_enhancement)?;
    Ok(())
}

fn terminal_enter(supports_keyboard_enhancement: bool) -> error::Result<()> {
    let mut stdout = std::io::stdout();
    terminal::enable_raw_mode()?;
    let command_queue = stdout.queue(terminal::EnterAlternateScreen)?;
    if supports_keyboard_enhancement {
        command_queue.queue(PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
        ))?;
    }
    command_queue.flush()?;
    Ok(())
}

fn terminal_exit(supports_keyboard_enhancement: bool) -> error::Result<()> {
    let mut stdout = std::io::stdout();
    let command_queue = stdout
        .queue(terminal::Clear(terminal::ClearType::All))?
        .queue(terminal::LeaveAlternateScreen)?
        .queue(cursor::Show)?;
    if supports_keyboard_enhancement {
        command_queue.queue(PopKeyboardEnhancementFlags)?;
    }
    command_queue.flush()?;
    terminal::disable_raw_mode()?;
    Ok(())
}

fn setup_panic_handler(supports_keyboard_enhancement: bool) {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        _ = terminal_exit(supports_keyboard_enhancement);
        default_panic(info);
    }));
}

#[derive(Debug)]
struct EditorPane {
    buffer: rope::Rope,
}

impl EditorPane {
    fn new() -> Self {
        let buffer = rope::Rope::empty();
        Self { buffer }
    }
}

impl Widget for &EditorPane {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut lines = self.buffer.lines();
        let x = area.left();
        for y in area.top()..area.bottom() {
            if let Some(line) = lines.next() {
                let chars = line
                    .chunks()
                    // .flat_map(|chunk| chunk.chars())
                    .flat_map(ByteSlice::chars)
                    .take(area.width.into());
                for (offset, c) in chars.enumerate() {
                    buf.get_mut(x + (offset as u16), y).set_char(c);
                }
            } else {
                buf.get_mut(x, y).set_char('~');
            }
        }
    }
}

async fn app_main(mut command_rx: mpsc::Receiver<Command>) -> error::Result<()> {
    let mut event_stream = EventStream::new();
    let mut tui = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    let mut state = EditorPane::new();

    'main: loop {
        tui.draw(|f| {
            f.render_widget(&state, f.size());
            f.set_cursor(0, 0);
        })?;

        let maybe_command = tokio::select! {
            maybe_event = event_stream.next().fuse() => {
                match maybe_event {
                    Some(event) => process_event(event?),
                    None => break 'main,
                }
            }
            maybe_command = command_rx.recv() => { maybe_command }
        };

        if let Some(command) = maybe_command {
            match command {
                Command::Quit => break 'main,
                Command::FileOpen(p) => {
                    let mut blocks = rope::BlockBuffer::new();
                    let mut file = File::open(p).await?;
                    loop {
                        let (block, read) = blocks.read(&mut file).await?;
                        if read == 0 {
                            break;
                        }
                        state.buffer = state.buffer.insert(state.buffer.len(), block)?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn process_event(event: Event) -> Option<Command> {
    match event {
        Event::FocusGained => todo!(),
        Event::FocusLost => todo!(),
        Event::Paste(_) => todo!(),
        Event::Mouse(_) => todo!(),
        Event::Resize(_, _) => None,
        Event::Key(key) => match key.code {
            KeyCode::Char(c) => {
                if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) {
                    Some(Command::Quit)
                } else {
                    None
                }
            }
            _ => None,
        },
    }
}
