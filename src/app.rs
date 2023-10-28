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
    buffer: rope::Rope,
}

async fn app_main(mut command_rx: mpsc::Receiver<Command>) -> error::Result<()> {
    let mut keyboard = term::KeyboardInput::new();
    let mut state = State { buffer: rope::Rope::empty() };
    'main: loop {
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
                        state.buffer = state.buffer.insert_at(state.buffer.len(), block)?;
                    }
                }
            }
        }
    }

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
