use anyhow::Result;
use crossterm::cursor::{self, SetCursorStyle};
use crossterm::event::Event;
use crossterm::execute;
use slotmap::{new_key_type, SlotMap};
use tokio::sync::mpsc;

use commands::Commands;
use editor::{Buffer, BufferContents, BufferId, Editor, EditorId};
use rope::SlabAllocator;
use tore::CursorPoint;

type BufferMap = SlotMap<BufferId, Buffer>;
type EditorMap = SlotMap<EditorId, Editor>;

#[derive(Debug, Clone)]
pub enum Command {
    Quit,
    FileOpen(Option<EditorId>, std::path::PathBuf),

    Commands(commands::Command),

    Editor(EditorId, editor::EditorCommand),
    Buffer(BufferId, editor::BufferCommand),
}

new_key_type! {
    pub struct PaneId;
}

#[derive(Debug)]
pub enum Pane {
    Commands(PaneId, commands::Mode),
    Editor(PaneId, editor::Mode, EditorId),
}

impl Pane {
    fn id(&self) -> PaneId {
        match self {
            Pane::Commands(id, ..) => *id,
            Pane::Editor(id, ..) => *id,
        }
    }
}

type PaneMap = SlotMap<PaneId, Pane>;

impl Pane {
    fn new_editor(id: PaneId, editor_id: EditorId) -> Self {
        Pane::Editor(id, editor::Mode::default(), editor_id)
    }

    fn new_commands(id: PaneId) -> Self {
        Pane::Commands(id, commands::Mode::Command)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Editor(editor::Mode),
    Commands(commands::Mode),
}

#[derive(Debug)]
pub struct App {
    cmd_tx: mpsc::Sender<Command>,
    cmd_rx: mpsc::Receiver<Command>,
}

#[derive(Debug)]
struct State {
    commands: Commands<Command>,
    buffers: BufferMap,
    editors: EditorMap,
    panes: PaneMap,
    visible_panes: Vec<PaneId>,
    focused_pane: PaneId,
}

impl State {
    fn focused_pane_mut(&mut self) -> &mut Pane {
        let pane = self
            .panes
            .get_mut(self.focused_pane)
            .expect("focused pane does not exist");
        debug_assert!(self.visible_panes.contains(&pane.id()), "focused pane not visible");
        pane
    }
}

impl App {
    pub fn spawn(paths: Option<Vec<std::path::PathBuf>>) -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread().build()?;
        rt.block_on(async move {
            let (cmd_tx, cmd_rx) = mpsc::channel(1);
            let app = Self::new(cmd_rx, cmd_tx.clone());
            let app = tokio::spawn(app.run());
            if let Some(paths) = paths {
                for p in paths.iter() {
                    cmd_tx.send(Command::FileOpen(None, p.clone())).await?;
                }
            }

            let res = app.await;
            res?
        })
    }

    fn new(cmd_rx: mpsc::Receiver<Command>, cmd_tx: mpsc::Sender<Command>) -> Self {
        Self { cmd_rx, cmd_tx }
    }

    async fn run(mut self) -> Result<()> {
        use crossterm::event::EventStream;
        use futures::{future::FutureExt, StreamExt};
        use ratatui::backend::CrosstermBackend;
        use ratatui::Terminal;

        let stdout = std::io::stdout();
        let mut term = Terminal::new(CrosstermBackend::new(stdout))?;
        let mut events = EventStream::new();
        let mut syntax = syntax::Client::spawn();

        let theme = ui::Theme::default();
        let mut alloc = rope::SlabAllocator::new();
        let mut state: State = {
            let mut buffers = BufferMap::with_key();
            let mut editors = EditorMap::with_key();
            let mut panes = PaneMap::with_key();

            let mut commands = Commands::new(self.cmd_tx.clone());
            commands.register("quit", vec![], Command::Quit);
            commands.search();

            let (focused_pane, visible_panes) = {
                let editor_id: EditorId = editors.insert_with_key(|k| {
                    let buffer_id: BufferId = buffers.insert_with_key(Buffer::empty);
                    Editor::new(k, buffer_id)
                });
                let pane_id = panes.insert_with_key(|k| Pane::new_editor(k, editor_id));
                (pane_id, vec![pane_id])
            };
            State { commands, buffers, editors, panes, visible_panes, focused_pane }
        };

        let commands_pane_id = state.panes.insert_with_key(|k| Pane::new_commands(k));
        let default_editor_id: EditorId = state
            .editors
            .keys()
            .next()
            .expect("at least one editor must be active");

        'main: loop {
            let mut cursor: Option<(CursorPoint, SetCursorStyle)> = None;
            term.draw(|frame| {
                let area = frame.size();
                let fb = frame.buffer_mut();
                for pane_id in state.visible_panes.iter() {
                    let pane = state.panes.get(*pane_id).expect("pane not found");
                    match &pane {
                        Pane::Commands(pane_id, ..) => {
                            let widget = ui::CommandsPane::new(&theme, &state.commands);
                            let c = widget.render(area, fb);
                            (cursor.is_none() && state.focused_pane == *pane_id)
                                .then(|| cursor = Some((c, SetCursorStyle::BlinkingBar)));
                        }
                        Pane::Editor(pane_id, mode, editor_id) => {
                            let editor = &state.editors[*editor_id];
                            let buffer = &state.buffers[editor.buffer_id];
                            let widget = ui::EditorPane::new(&theme, buffer, editor);
                            let c = widget.render(area, fb);
                            (cursor.is_none() && state.focused_pane == *pane_id).then(|| {
                                let cursor_style = match mode {
                                    editor::Mode::Normal => SetCursorStyle::BlinkingBlock,
                                    editor::Mode::Insert => SetCursorStyle::BlinkingBar,
                                };
                                cursor = Some((c, cursor_style))
                            });
                        }
                    }
                }
            })?;

            let (cursor, cursor_style) = cursor.expect("cursor must be set");
            execute!(term.backend_mut(), cursor::MoveTo(cursor.x, cursor.y))?;
            execute!(term.backend_mut(), cursor::Show)?;
            execute!(term.backend_mut(), cursor_style)?;

            let mut maybe_command = tokio::select! {
                maybe_command = self.cmd_rx.recv() => { maybe_command }
                maybe_syntax_event = syntax.next().fuse() => {
                    match maybe_syntax_event {
                        Some(event) => process_syntax(event),
                        None => panic!("syntax thread crashed?"),
                    }
                }
                maybe_event = events.next().fuse() => {
                    match maybe_event {
                        Some(event) => {
                            process_event(&mut state, event?)
                        },
                        None => break 'main,
                    }
                }
            };

            while let Some(command) = maybe_command.take() {
                match command {
                    Command::Quit => break 'main,
                    Command::Commands(cmd) => match cmd {
                        commands::Command::Select(entry_id) => {
                            let entry = state.commands.entries.get(entry_id).unwrap();
                            maybe_command = Some(entry.send.clone());
                        }
                        commands::Command::Open => {
                            state.visible_panes.push(commands_pane_id);
                            state.focused_pane = commands_pane_id;
                        }
                        commands::Command::Close => {
                            let pane_id = state.visible_panes.pop();
                            debug_assert_eq!(pane_id, Some(commands_pane_id));
                            state.focused_pane = state
                                .visible_panes
                                .last()
                                .copied()
                                .expect("no panes visible");
                        }
                    },
                    Command::Editor(editor_id, cmd) => {
                        let editor = &mut state.editors[editor_id];
                        let buffer = &mut state.buffers[editor.buffer_id];
                        editor.command(buffer, cmd);
                    }
                    Command::Buffer(buffer_id, cmd) => {
                        let buffer = &mut state.buffers[buffer_id];
                        buffer.command(cmd);
                    }
                    Command::FileOpen(maybe_editor_id, path) => {
                        let contents = file_open(&mut alloc, &path).await?;
                        let buffer_id = state
                            .buffers
                            .insert_with_key(|k| Buffer::new(k, contents.clone()));
                        match syntax::Language::try_from(&state.buffers[buffer_id]) {
                            Ok(language) => {
                                syntax
                                    .command(syntax::Command::Parse {
                                        buffer_id,
                                        contents,
                                        language,
                                    })
                                    .await?;
                            }
                            _ => todo!(),
                        };

                        let editor_id = maybe_editor_id.unwrap_or(default_editor_id);
                        let editor = &mut state.editors[editor_id];
                        editor.swap_buffer(buffer_id);
                    }
                }
            }
        }

        Ok(())
    }
}

#[tracing::instrument(skip(alloc))]
async fn file_open(alloc: &mut SlabAllocator, path: &std::path::PathBuf) -> Result<BufferContents> {
    Buffer::read(alloc, path).await
}

fn process_syntax(ev: syntax::Event) -> Option<Command> {
    match ev {
        syntax::Event::Hightlight(buffer_id, hls) => {
            Some(Command::Buffer(buffer_id, editor::BufferCommand::Highlight(hls)))
        }
        _ => None,
    }
}

#[tracing::instrument(skip(ev, state))]
fn process_event(state: &mut State, ev: Event) -> Option<Command> {
    use crossterm::event::{KeyCode, KeyModifiers};

    match ev {
        Event::FocusGained => todo!(),
        Event::FocusLost => todo!(),
        Event::Paste(_) => todo!(),
        Event::Mouse(_) => todo!(),
        Event::Resize(_, _) => None,
        Event::Key(key) => {
            let command = match key.code {
                KeyCode::Char(c) => {
                    if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) {
                        Some(Command::Quit)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            command.or_else(|| {
                let focused_pane = state
                    .panes
                    .get_mut(state.focused_pane)
                    .expect("focused pane does not exist");

                let command = match focused_pane {
                    Pane::Commands(_, mode) => {
                        let command = state
                            .commands
                            .process_key(key)
                            .map(|cmd| Command::Commands(cmd));
                        let command = command.or_else(|| match mode {
                            commands::Mode::Command => match key.code {
                                KeyCode::Esc => Some(Command::Commands(commands::Command::Close)),
                                _ => None,
                            },
                        });
                        command
                    }
                    Pane::Editor(_, mode, editor_id) => {
                        let editor = &mut state.editors[*editor_id];
                        let buffer = &mut state.buffers[editor.buffer_id];
                        let command = editor
                            .process_key(key, buffer)
                            .map(|cmd| Command::Editor(*editor_id, cmd));
                        let command = command.or_else(|| match mode {
                            editor::Mode::Normal => match key.code {
                                KeyCode::Char(':') => {
                                    Some(Command::Commands(commands::Command::Open))
                                }
                                _ => None,
                            },
                            _ => None,
                        });
                        command
                    }
                };
                command
            })
        }
    }
}
