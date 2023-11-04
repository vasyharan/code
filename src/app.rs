use clap::Parser;
use tokio::fs::File;
use tokio::sync::mpsc;

pub use crate::error::Result;
use crate::term;
use crate::{
    error,
    rope::{self},
};

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

pub fn main(args: Args) -> Result<()> {
    let mut raw_mode = term::RawMode::new()?;
    raw_mode.enable()?;

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
        Ok(())
    })
}

#[derive(Debug)]
struct State {
    display: term::Display,
    buffer: rope::Rope,
}

impl State {
    fn new() -> Self {
        let buffer = rope::Rope::empty();
        let display = term::Display::new();
        Self { buffer, display }
    }
}

async fn app_main(mut command_rx: mpsc::Receiver<Command>) -> error::Result<()> {
    let mut keyboard = term::KeyboardInput::new();
    let mut state = State::new();
    state.display.enable_alternate_screen().await?;

    'main: loop {
        redraw_screen(&mut state).await?;

        let maybe_command = tokio::select! {
            key = keyboard.read_key() => { process_key(key?) }
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

async fn redraw_screen(state: &mut State) -> error::Result<()> {
    state.display.hide_cursor().await?;
    state.display.flush().await?;

    state.display.cursor_position(0, 0).await?;
    let mut lines = state.buffer.lines();
    for linenum in 1..state.display.dimensions.rows {
        if let Some(line) = lines.next() {
            for chunk in line.chunks() {
                state.display.write_all(chunk).await?;
            }
        } else {
            state.display.write_all(b"~").await?;
        }
        // state.display.write_all(b"~").await?;

        state
            .display
            .erase_in_line(term::display::EraseInMode::FromPos)
            .await?;
        if linenum < state.display.dimensions.rows - 1 {
            state.display.write_all(b"\r\n").await?;
        }
        state.display.flush().await?;
    }
    state.display.cursor_position(0, 0).await?;
    state.display.show_cursor().await?;
    state.display.flush().await?;
    Ok(())
}

fn process_key(key: term::Key) -> Option<Command> {
    use term::Key::{Char, Ctrl};
    match key {
        Char(c) => {
            println!("{}\r", c);
            None
        }
        Ctrl(k) => match k {
            b'q' | b'c' => Some(Command::Quit),
            _ => None,
        },
    }
}
