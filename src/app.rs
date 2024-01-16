use std::io::Write;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::cursor;
use crossterm::event::{
    Event, EventStream, KeyCode, KeyModifiers, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal;
use crossterm::QueueableCommand;
use futures::{future::FutureExt, StreamExt};
use lazy_static::lazy_static;
use ratatui::backend::CrosstermBackend;
use ratatui::prelude as tui;
use ratatui::Terminal;
use slotmap::SlotMap;
use tokio::fs::File;
use tokio::sync::mpsc;

use crate::buffer::{self, Buffer};
use crate::editor::{self, Editor};
use crate::rope::{self, Rope};
use crate::syntax;
use crate::theme::Theme;

lazy_static! {
    pub(crate) static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_string();
    pub(crate) static ref LOG_ENV: String =
        format!("{}_LOGLEVEL", PROJECT_NAME.clone().to_uppercase());
    pub(crate) static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
}

#[derive(Debug, Parser)]
pub struct Args {
    /// Paths to files to open
    paths: Option<Vec<std::path::PathBuf>>,
}

#[derive(Debug)]
pub(crate) enum Command {
    Quit,
    FileOpen(std::path::PathBuf),

    EditorCommand(editor::Command),
    BufferInsert(buffer::Id, char),
}

type BufferMap = SlotMap<buffer::Id, Buffer>;

#[derive(Debug)]
pub(crate) struct App {
    theme: Theme,
    buffers: BufferMap,
    editor_id: (buffer::Id, editor::Id),
}

impl App {
    fn new() -> Self {
        let mut buffers = BufferMap::with_key();
        let buffer_id: buffer::Id = buffers.insert_with_key(Buffer::empty);
        let buffer = &mut buffers[buffer_id];
        let contents = buffer.contents.clone();
        let buffer_view_id = buffer
            .editors
            .insert_with_key(|id| Editor::new_contents(id, buffer_id, contents));
        let editor_id = (buffer_id, buffer_view_id);

        Self { theme: Default::default(), buffers, editor_id }
    }

    async fn file_open(&mut self, p: std::path::PathBuf) -> Result<&Buffer> {
        let contents = file_open(&p).await?;
        let previous_editor_id = self.editor_id;
        let buffer_id = self
            .buffers
            .insert_with_key(|buffer_id| Buffer::new(buffer_id, Some(p), contents));
        self.buffers.remove(previous_editor_id.0);
        let buffer = &mut self.buffers[buffer_id];
        let contents = buffer.contents.clone();
        let editor_id = buffer
            .editors
            .insert_with_key(|id| Editor::new_contents(id, buffer_id, contents));
        self.editor_id = (buffer_id, editor_id);
        Ok(buffer)
    }
}

pub fn main(args: Args) -> Result<()> {
    let supports_keyboard_enhancement =
        matches!(terminal::supports_keyboard_enhancement(), Ok(true));
    setup_panic_handler(supports_keyboard_enhancement);
    setup_logging()?;
    terminal_enter(supports_keyboard_enhancement)?;

    let rt = tokio::runtime::Builder::new_current_thread().build()?;
    let res = rt.block_on(async move {
        let (cmd_tx, cmd_rx) = mpsc::channel(1);
        let main_loop = tokio::spawn(main_loop(cmd_rx));
        if let Some(paths) = args.paths {
            for p in paths.iter() {
                cmd_tx.send(Command::FileOpen(p.clone())).await?;
            }
        }

        main_loop.await?
    });

    terminal_exit(supports_keyboard_enhancement)?;
    res
}

async fn main_loop(mut app_rx: mpsc::Receiver<Command>) -> Result<()> {
    let mut term = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    let mut input = EventStream::new();
    let mut syntax = syntax::Client::spawn();
    let mut app = App::new();

    'main: loop {
        term.draw(|frame| {
            let area = frame.size();
            let style = tui::Style::default().bg(app.theme.bg().0);
            frame.buffer_mut().set_style(area, style);

            let (buffer_id, editor_id) = app.editor_id;
            let buffer: &Buffer = &app.buffers[buffer_id];
            let editor: &Editor = &buffer.editors[editor_id];
            let pane = crate::widgets::EditorPane::new(&app.theme, buffer, editor);
            let cursor_pos = pane
                .screen_cursor_position((area.width, area.height))
                .unwrap();
            frame.render_widget(pane, area);
            frame.set_cursor(cursor_pos.0, cursor_pos.1);
        })?;

        let mut maybe_command = tokio::select! {
            maybe_input_event = input.next().fuse() => {
                match maybe_input_event {
                    Some(event) => process_event(&app, event?),
                    None => break 'main,
                }
            }
            maybe_syntax_event = syntax.next().fuse() => {
                match maybe_syntax_event {
                    Some(event) => process_syntax(&mut app, event),
                    None => panic!("syntax thread crashed?")
                }
             }
            maybe_command = app_rx.recv() => { maybe_command }
        };

        while let Some(command) = maybe_command.take() {
            use Command::*;

            match command {
                Quit => break 'main,
                FileOpen(p) => {
                    let buffer = app.file_open(p).await?;
                    match syntax::Language::try_from(buffer) {
                        Ok(language) => {
                            syntax
                                .send(syntax::Command::Parse {
                                    buffer_id: buffer.id,
                                    contents: buffer.contents.clone(),
                                    language,
                                })
                                .await?;
                        }
                        _ => todo!(),
                    }
                }
                BufferInsert(..) => {}
                EditorCommand(cmd) => {
                    let buffer = &mut app.buffers[app.editor_id.0];
                    let editor = &mut buffer.editors[app.editor_id.1];
                    editor.command(&cmd);
                }
            }
        }
    }

    Ok(())
}

#[tracing::instrument(skip(ctx, event))]
fn process_syntax(ctx: &mut App, event: syntax::SyntaxEvent) -> Option<Command> {
    match event {
        syntax::SyntaxEvent::Parsed(..) => (),
        syntax::SyntaxEvent::Hightlight(buffer_id, hls) => {
            ctx.buffers[buffer_id].highlights = hls;
        }
    }
    None
}

#[tracing::instrument(skip(ctx, event))]
fn process_event(ctx: &App, event: Event) -> Option<Command> {
    match event {
        Event::FocusGained => todo!(),
        Event::FocusLost => todo!(),
        Event::Paste(_) => todo!(),
        Event::Mouse(_) => todo!(),
        Event::Resize(_, _) => todo!(),
        Event::Key(key) => {
            let global_command = match key.code {
                KeyCode::Char(c) => {
                    if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) {
                        Some(Command::Quit)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            global_command.or_else(|| {
                let buffer = &ctx.buffers[ctx.editor_id.0];
                let editor = &buffer.editors[ctx.editor_id.1];
                editor.process_key(key).map(Command::EditorCommand)
            })
        }
    }
}

#[tracing::instrument]
async fn file_open(path: &std::path::PathBuf) -> Result<Rope> {
    let mut blocks = rope::SlabAllocator::new();
    let mut file = File::open(&path).await?;
    let mut rope = Rope::empty();
    loop {
        let (block, read) = blocks.read(&mut file).await?;
        if read == 0 {
            break Ok(rope);
        }
        rope = rope.append(block)?;
    }
}

fn terminal_enter(supports_keyboard_enhancement: bool) -> Result<()> {
    let mut stdout = std::io::stdout();
    terminal::enable_raw_mode().context("enable raw mode")?;
    let command_queue = stdout.queue(terminal::EnterAlternateScreen)?;
    if supports_keyboard_enhancement {
        command_queue.queue(PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
        ))?;
    }
    command_queue.flush().context("setup terminal")?;
    Ok(())
}

fn terminal_exit(supports_keyboard_enhancement: bool) -> Result<()> {
    let mut stdout = std::io::stdout();
    let command_queue = stdout
        .queue(terminal::Clear(terminal::ClearType::All))?
        .queue(terminal::LeaveAlternateScreen)?
        .queue(cursor::Show)?;
    if supports_keyboard_enhancement {
        command_queue.queue(PopKeyboardEnhancementFlags)?;
    }
    command_queue.flush().context("reset terminal")?;
    terminal::disable_raw_mode().context("disable raw mode")?;
    Ok(())
}

fn setup_panic_handler(supports_keyboard_enhancement: bool) {
    // let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        _ = terminal_exit(supports_keyboard_enhancement);
        better_panic::Settings::auto().create_panic_handler()(panic_info);
        // default_panic(info);
    }));
}

// /// This replaces the standard color_eyre panic and error hooks with hooks that
// /// restore the terminal before printing the panic or error.
// pub fn setup_panic_handler(supports_keyboard_enhancement: bool) -> color_eyre::Result<()> {
//     // add any extra configuration you need to the hook builder
//     let hook_builder = color_eyre::config::HookBuilder::default();
//     let (panic_hook, eyre_hook) = hook_builder.into_hooks();

//     // convert from a color_eyre PanicHook to a standard panic hook
//     let panic_hook = panic_hook.into_panic_hook();
//     std::panic::set_hook(Box::new(move |panic_info| {
//         // tui::restore().unwrap();
//         _ = terminal_exit(supports_keyboard_enhancement);
//         panic_hook(panic_info);
//     }));

//     // convert from a color_eyre EyreHook to a eyre ErrorHook
//     let eyre_hook = eyre_hook.into_eyre_hook();
//     eyre::set_hook(Box::new(move |error| {
//         // tui::restore().unwrap();
//         _ = terminal_exit(supports_keyboard_enhancement);
//         eyre_hook(error)
//     }))?;

//     Ok(())
// }

fn setup_logging() -> Result<()> {
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::Layer;

    let xdg_dirs = xdg::BaseDirectories::with_prefix(PROJECT_NAME.clone())
        .expect("cannot determine XDG paths");
    let log_path = xdg_dirs
        .place_data_file(LOG_FILE.clone())
        .expect("cannot create data file");
    let log_file = std::fs::File::create(log_path)?;

    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG")
            .or_else(|_| std::env::var(LOG_ENV.clone()))
            .unwrap_or_else(|_| format!("{}=warn", env!("CARGO_CRATE_NAME"))),
    );

    // let console_subscriber = console_subscriber::ConsoleLayer::builder()
    //     .with_default_env()
    //     .spawn();
    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(true)
        .with_ansi(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(EnvFilter::from_default_env());
    tracing_subscriber::registry()
        // .with(console_subscriber)
        .with(file_subscriber)
        .init();

    Ok(())
}
