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
use futures::TryFutureExt;
use futures::{future::FutureExt, StreamExt};
use lazy_static::lazy_static;
use ratatui::backend::CrosstermBackend;
use ratatui::prelude as tui;
use ratatui::Terminal;
use slotmap::{new_key_type, SlotMap};
use tokio::fs::File;
use tokio::sync::mpsc;

use crate::buffer::Buffer;
use crate::editor::{self, EditorContext};
use crate::rope::{self, Rope};
use crate::syntax::{self, language::Language};
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
}

new_key_type! {
    pub(crate) struct BufferId;
}

#[derive(Debug)]
pub(crate) struct AppContext {
    pub(crate) theme: Theme,
    pub(crate) editor: EditorContext,

    buffers: SlotMap<BufferId, Buffer>,
}

impl AppContext {
    pub(crate) fn buffer_create(&mut self, buffer: Buffer) -> BufferId {
        self.buffers.insert(buffer)
    }
}

impl Default for AppContext {
    fn default() -> Self {
        let mut buffers = SlotMap::with_key();
        let buffer_id: BufferId = buffers.insert(Buffer::default());

        Self { theme: Default::default(), buffers, editor: EditorContext::new_buffer(buffer_id) }
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

        return main_loop.await?;
    });

    terminal_exit(supports_keyboard_enhancement)?;
    res
}

async fn main_loop(mut app_rx: mpsc::Receiver<Command>) -> Result<()> {
    let mut term = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    let mut input = EventStream::new();
    let mut syntax = syntax::Client::spawn();
    let mut context = AppContext::default();

    'main: loop {
        term.draw(|frame| {
            let area = frame.size();
            let style = tui::Style::default().bg(context.theme.bg().0);
            frame.buffer_mut().set_style(area, style);

            // let buffer = context.buffers[]
            let buffer = &context.buffers[context.editor.buffer_id];
            let root = crate::widgets::EditorPane {
                buffer,
                theme: &context.theme,
                context: &context.editor,
            };
            let cursor_pos = root.screen_cursor_position((area.width, area.height));
            frame.render_widget(root, area);
            frame.set_cursor(cursor_pos.0, cursor_pos.1);
        })?;

        let maybe_command = tokio::select! {
            maybe_input_event = input.next().fuse() => {
                match maybe_input_event {
                    Some(event) => process_event(&context, event?),
                    None => break 'main,
                }
            }
            maybe_syntax_event = syntax.next().fuse() => {
                match maybe_syntax_event {
                    Some(event) => process_syntax(&mut context, event),
                    None => panic!("syntax thread crashed?")
                }
             }
            maybe_command = app_rx.recv() => { maybe_command }
        };

        if let Some(command) = maybe_command {
            use Command::*;

            match command {
                Quit => break 'main,
                FileOpen(p) => {
                    let buffer = file_open(p).await?;
                    let contents = buffer.contents.clone();
                    let buffer_id = context.buffer_create(buffer);
                    let buffer = &context.buffers[buffer_id];
                    match Language::try_from(buffer) {
                        Ok(language) => {
                            syntax
                                .send(syntax::Command::Parse { buffer_id, contents, language })
                                .await?;
                            ()
                        }
                        _ => (),
                    }

                    context.editor = EditorContext::new_buffer(buffer_id);
                }
                EditorCommand(command) => context.editor.command(command),
            }
        }
    }

    Ok(())
}

#[tracing::instrument]
fn process_syntax(context: &mut AppContext, event: syntax::SyntaxEvent) -> Option<Command> {
    match event {
        syntax::SyntaxEvent::Parsed(..) => (),
        syntax::SyntaxEvent::Hightlight(buffer_id, hls) => {
            context.buffers[buffer_id].highlights = hls;
        }
    }
    None
}

#[tracing::instrument]
fn process_event(ctx: &AppContext, event: Event) -> Option<Command> {
    match event {
        Event::FocusGained => todo!(),
        Event::FocusLost => todo!(),
        Event::Paste(_) => todo!(),
        Event::Mouse(_) => todo!(),
        Event::Resize(_, _) => None,
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
            global_command.or_else(|| ctx.editor.process_key(key).map(Command::EditorCommand))
        }
    }
}

#[tracing::instrument]
async fn file_open(path: std::path::PathBuf) -> Result<Buffer> {
    let mut blocks = rope::BlockBuffer::new();
    let mut file = File::open(&path).await?;
    let mut rope = Rope::empty();
    loop {
        let (block, read) = blocks.read(&mut file).await?;
        if read == 0 {
            break Ok(Buffer::new(path, rope));
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
        better_panic::Settings::auto()
            // .most_recent_first(false)
            // .lineno_suffix(true)
            .create_panic_handler()(panic_info);
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
