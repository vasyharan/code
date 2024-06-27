use anyhow::Result;
use crossterm::cursor::{self, SetCursorStyle};
use crossterm::event::{Event, EventStream, KeyEvent};
use futures::{Future, FutureExt};
use ratatui::backend::CrosstermBackend;
use ratatui::prelude as tui;
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use std::io::Stdout;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tree_sitter as ts;

use editor::{Buffer, BufferCommand, BufferId, Editor, EditorCommand, EditorId};
use selector::Selector;
use syntax::Syntax;
use tore::CursorPoint;

type BufferMap = SlotMap<BufferId, Buffer>;
type EditorMap = SlotMap<EditorId, Editor>;
type SyntaxTreeMap = SecondaryMap<BufferId, ts::Tree>;
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;

#[derive(Debug, Clone)]
pub enum PaneCommand {
    Open,
    Close,
}

#[derive(Debug, Clone)]
pub enum Command {
    Quit,
    FileOpen(Option<EditorId>, std::path::PathBuf),
    Pane(PaneId, PaneCommand),
    Buffer(BufferId, BufferCommand),
    Editor(EditorId, EditorCommand),
    FocusedEditor(EditorCommand),
    Commands(selector::Command<CommandId>),
}

new_key_type! {
    pub struct PaneId;
}

#[derive(Debug, Clone)]
pub enum Pane {
    Commands(PaneId),
    Editor(PaneId, EditorId),
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
        Pane::Editor(id, editor_id)
    }

    fn new_commands(id: PaneId) -> Self {
        Pane::Commands(id)
    }
}

new_key_type! {
    pub struct CommandId;
}

#[derive(Debug)]
struct Entry {
    name: &'static str,
    aliases: Vec<&'static str>,
    command: Command,
}

#[derive(Debug)]
struct CommandRegistry {
    entries: SlotMap<CommandId, Entry>,
    selector: Selector<CommandId>,
    // filtered: Vec<CommandId>,
}

impl CommandRegistry {
    fn new() -> Self {
        let selector = Selector::new(":");
        let entries = SlotMap::with_key();
        Self { entries, selector }
    }

    // fn focused(&self) -> Option<Command> {
    //     self.selector
    //         .focused
    //         .map(|id| self.entries[id].command.clone())
    // }

    fn register(
        &mut self,
        name: &'static str,
        aliases: Vec<&'static str>,
        command: Command,
    ) -> CommandId {
        self.entries.insert(Entry { name, aliases, command })
    }

    async fn update(&mut self, query: &str) {
    }

    // async fn update(&mut self, query: &str) {
    //     tokio::spawn(async move {
    //         let mut results = vec![];
    //         // if self.selector.query.is_empty() {
    //         // for (id, _) in &self.entries {
    //         //     // todo: handle max results
    //         //     if results.len() > 32 {
    //         //         break;
    //         //     }
    //         //     results.push(id);
    //         // }
    //         // } else {
    //         //     let matcher = SkimMatcherV2::default();
    //         //     for (id, entry) in &self.entries {
    //         //         let result = matcher.fuzzy_indices(&entry.name, &self.query);
    //         //         if let Some((score, indices)) = result {
    //         //             results.push(id)
    //         //         }
    //         //     }
    //         //     // results.sort_by_key(|entry| entry.score);
    //         // }

    //         // self.selected = results.first().map(|r| r.entry);
    //         // self.filtered = results;
    //         results
    //     });
    // }

    // fn results(&self) -> Vec<CommandId> {
    //     let mut results = vec![];
    //     // if self.selector.query.is_empty() {
    //     for (id, _) in &self.entries {
    //         // todo: handle max results
    //         if results.len() > 32 {
    //             break;
    //         }
    //         results.push(id);
    //     }
    //     // } else {
    //     //     let matcher = SkimMatcherV2::default();
    //     //     for (id, entry) in &self.entries {
    //     //         let result = matcher.fuzzy_indices(&entry.name, &self.query);
    //     //         if let Some((score, indices)) = result {
    //     //             results.push(id)
    //     //         }
    //     //     }
    //     //     // results.sort_by_key(|entry| entry.score);
    //     // }

    //     // self.selected = results.first().map(|r| r.entry);
    //     // self.filtered = results;
    //     results
    // }

    fn render(
        &self,
        buf: &mut tui::Buffer,
        area: tui::Rect,
        theme: &ui::Theme,
    ) -> (CursorPoint, SetCursorStyle) {
        let widget = ui::SelectorPane::new(theme, &self.selector);
        widget.render(buf, area, &self.selector.entries, |area, buf, id| {
            self.render_result(area, buf, id)
        })
    }

    fn render_result(&self, area: tui::Rect, buf: &mut tui::Buffer, id: CommandId) {
        use bstr::ByteSlice;
        let entry = &self.entries[id];
        let content = entry.name;
        let mut graphemes = content.as_bytes().as_bstr().graphemes();
        for (idx, x) in (area.left()..area.right()).enumerate() {
            let symbol = graphemes.next().unwrap_or(" ");
            let style = tui::Style::reset();
            buf.get_mut(x, area.top())
                .set_style(style)
                .set_symbol(symbol);
        }
    }
}

#[derive(Debug)]
struct State {
    theme: ui::Theme,

    buffers: BufferMap,
    editors: EditorMap,
    syntax_trees: SyntaxTreeMap,

    panes: PaneMap,
    visible_panes: Vec<PaneId>,
    focused_pane: PaneId,

    default_editor_id: EditorId,

    command_registry: CommandRegistry,
}

impl State {
    fn new() -> Self {
        let theme = ui::Theme::default();
        let syntax_trees = SecondaryMap::new();
        // let commands = Selector::new(":");

        let mut buffers = BufferMap::with_key();
        let mut editors = EditorMap::with_key();
        let mut panes = PaneMap::with_key();

        // create a empty editor pane.
        let (focused_pane, visible_panes) = {
            let editor_id: EditorId = editors.insert_with_key(|k| {
                let buffer_id: BufferId = buffers.insert_with_key(Buffer::empty);
                Editor::new(k, buffer_id)
            });
            let pane_id = panes.insert_with_key(|k| Pane::new_editor(k, editor_id));
            (pane_id, vec![pane_id])
        };

        let default_editor_id: EditorId = editors
            .keys()
            .next()
            .expect("at least one editor must be active");

        let mut command_registry = CommandRegistry::new();
        register_commands(&mut command_registry);
        let commands_pane_id = panes.insert_with_key(Pane::new_commands);

        State {
            theme,
            buffers,
            editors,
            syntax_trees,
            panes,
            visible_panes,
            focused_pane,
            default_editor_id,
            command_registry,
            commands_pane_id,
        }
    }

    fn focused_pane(&self) -> Pane {
        let pane = self
            .panes
            .get(self.focused_pane)
            .expect("focused pane does not exist");
        debug_assert!(self.visible_panes.contains(&pane.id()), "focused pane not visible");
        pane.clone()
    }

    fn close_focused_pane(&mut self) {
        let pane_id = self.visible_panes.pop();
        debug_assert_eq!(pane_id, Some(self.focused_pane));
        // self.commands.reset();
        self.restore_focus_to_last_pane();
    }

    fn focus_pane(&mut self, pane_id: PaneId) {
        if let Some(idx) = self.visible_panes.iter().position(|id| *id == pane_id) {
            self.visible_panes.remove(idx);
        }
        self.visible_panes.push(pane_id);
        self.focused_pane = pane_id;
    }

    fn restore_focus_to_last_pane(&mut self) {
        let last_pane = self.visible_panes.last().expect("visible panes is empty");
        self.focused_pane = *last_pane;
    }

    #[tracing::instrument(skip(self, frame))]
    fn draw_frame(&self, frame: &mut ratatui::Frame) -> Option<(CursorPoint, SetCursorStyle)> {
        let mut cursor: Option<(CursorPoint, SetCursorStyle)> = None;

        let area = frame.size();
        let fb = frame.buffer_mut();
        for pane_id in self.visible_panes.iter() {
            let pane = self.panes.get(*pane_id).expect("pane not found");
            match &pane {
                Pane::Commands(pane_id, ..) => {
                    let c = self.command_registry.render(fb, area, &self.theme);
                    (cursor.is_none() && self.focused_pane == *pane_id).then(|| cursor = Some(c));
                }
                Pane::Editor(pane_id, editor_id) => {
                    let editor = &self.editors[*editor_id];
                    let buffer = &self.buffers[editor.buffer_id];
                    let widget = ui::EditorPane::new(&self.theme, buffer, editor);
                    let c = widget.render(fb, area);
                    (cursor.is_none() && self.focused_pane == *pane_id).then(|| cursor = Some(c));
                }
            }
        }

        cursor
    }

    #[tracing::instrument(skip(ev, self))]
    fn process_event(&mut self, ev: Event) -> Option<Command> {
        match ev {
            Event::FocusGained => todo!(),
            Event::FocusLost => todo!(),
            Event::Paste(_) => todo!(),
            Event::Mouse(_) => todo!(),
            Event::Resize(_, _) => None,
            Event::Key(key) => {
                let command = self.process_key(key);
                command
            }
        }
    }

    fn process_key(&mut self, key: KeyEvent) -> Option<Command> {
        use crossterm::event::{KeyCode, KeyModifiers};

        let focused_pane = self
            .panes
            .get_mut(self.focused_pane)
            .expect("focused pane does not exist");

        match focused_pane {
            Pane::Commands(_) => match key.code {
                KeyCode::Up => {
                    Some(Command::Commands(selector::Command::Focus(selector::Direction::Prev)))
                }
                KeyCode::Down => {
                    Some(Command::Commands(selector::Command::Focus(selector::Direction::Next)))
                }
                KeyCode::Backspace => {
                    Some(Command::Commands(selector::Command::Delete(selector::Direction::Prev)))
                }
                KeyCode::Enter => self.command_registry.focused(),
                KeyCode::Char(c) => {
                    let ctrl = key.modifiers == KeyModifiers::CONTROL;
                    if ctrl && c == 'p' {
                        Some(Command::Commands(selector::Command::Focus(selector::Direction::Prev)))
                    } else if ctrl && c == 'n' {
                        Some(Command::Commands(selector::Command::Focus(selector::Direction::Next)))
                    } else {
                        Some(Command::Commands(selector::Command::Insert(c)))
                    }
                }
                _ => None,
            },
            Pane::Editor(_, editor_id) => {
                let editor = &mut self.editors[*editor_id];
                let command = match editor.mode {
                    editor::Mode::Normal => match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            Some(EditorCommand::CursorMove(editor::Direction::Up))
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            Some(EditorCommand::CursorMove(editor::Direction::Down))
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            Some(EditorCommand::CursorMove(editor::Direction::Left))
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            Some(EditorCommand::CursorMove(editor::Direction::Right))
                        }
                        KeyCode::Char('w') => {
                            Some(EditorCommand::CursorJump(editor::CursorJump::StartOfNextWord))
                        }
                        KeyCode::Char('e') => {
                            Some(EditorCommand::CursorJump(editor::CursorJump::EndOfNearestWord))
                        }
                        KeyCode::Char('b') => {
                            Some(EditorCommand::CursorJump(editor::CursorJump::StartOfNearestWord))
                        }
                        KeyCode::Char('0') => {
                            Some(EditorCommand::CursorJump(editor::CursorJump::StartOfNearestWord))
                        }
                        KeyCode::Char('i') => Some(EditorCommand::SetMode(editor::Mode::Insert)),
                        _ => None,
                    },
                    editor::Mode::Insert => match key.code {
                        KeyCode::Esc => Some(EditorCommand::SetMode(editor::Mode::Normal)),
                        KeyCode::Up => Some(EditorCommand::CursorMove(editor::Direction::Up)),
                        KeyCode::Down => Some(EditorCommand::CursorMove(editor::Direction::Down)),
                        KeyCode::Left => Some(EditorCommand::CursorMove(editor::Direction::Left)),
                        KeyCode::Right => Some(EditorCommand::CursorMove(editor::Direction::Right)),
                        KeyCode::Char(c) => Some(EditorCommand::InsertChar(c)),
                        _ => None,
                    },
                };
                command
                    .map(|c| Command::Editor(*editor_id, c))
                    .or_else(|| match editor.mode {
                        editor::Mode::Normal => match key.code {
                            KeyCode::Char(':') => {
                                Some(Command::Pane(self.commands_pane_id, PaneCommand::Open))
                            }
                            _ => None,
                        },
                        _ => None,
                    })
            }
        }
    }

    fn process_syntax(&mut self, ev: syntax::Event) -> Option<Command> {
        match ev {
            syntax::Event::Hightlight(buffer_id, hls) => {
                Some(Command::Buffer(buffer_id, BufferCommand::Highlight(hls)))
            }
            syntax::Event::Parsed(buffer_id, tree) => {
                self.syntax_trees.insert(buffer_id, tree);
                None
            }
        }
    }
}

struct BackgroundExecutor(tokio::runtime::Handle);

impl BackgroundExecutor {
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        tokio::task::spawn(future)
    }
}

struct AppContext {
    background: BackgroundExecutor,
}

impl AppContext {
    pub fn new() -> Result<Self> {
        let background_rt = tokio::runtime::Builder::new_multi_thread().build()?;
        let background = BackgroundExecutor(background_rt.handle().clone());
        Ok(Self { background })
    }

    pub fn background_executor(&self) -> &BackgroundExecutor {
        &self.background
    }
}

#[derive()]
pub struct App {
    ctx: AppContext,
    cmd_rx: mpsc::Receiver<Command>,
    cmd_tx: mpsc::Sender<Command>,
    term: Terminal,
    events: EventStream,
    syntax: syntax::Syntax,
    state: State,
}

impl App {
    pub fn spawn(paths: Option<Vec<std::path::PathBuf>>) -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread().build()?;
        let ctx = AppContext::new()?;
        rt.block_on(async move {
            let stdout = std::io::stdout();
            let term = Terminal::new(CrosstermBackend::new(stdout))?;

            let (cmd_tx, cmd_rx) = mpsc::channel(1);
            let app = Self::new(ctx, term, cmd_tx.clone(), cmd_rx);
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

    fn new(ctx: AppContext, term: Terminal, cmd_tx: mpsc::Sender<Command>, cmd_rx: mpsc::Receiver<Command>) -> Self {
        let events = EventStream::new();
        let syntax = Syntax::spawn();
        let state = State::new();
        Self { ctx, cmd_tx, cmd_rx, term, events, syntax, state }
    }

    async fn run(mut self) -> Result<()> {
        'main: loop {
            self.draw_frame()?;
            let maybe_command = self.select_command().await?;

            if let Some(command) = maybe_command {
                if let Command::Quit = command {
                    break 'main;
                }
                self.process_command(command).await?;
            }
        }

        Ok(())
    }

    fn draw_frame(&mut self) -> Result<()> {
        use crossterm::QueueableCommand;
        use std::io::Write;

        let mut cursor: Option<(CursorPoint, SetCursorStyle)> = None;
        self.term.draw(|frame| {
            cursor = self.state.draw_frame(frame);
        })?;

        let (cursor, cursor_style) = cursor.expect("cursor must be set");
        let backend = self.term.backend_mut();
        backend
            .queue(cursor_style)?
            .queue(cursor::MoveTo(cursor.x, cursor.y))?
            .queue(cursor::Show)?
            .flush()?;
        Ok(())
    }

    async fn select_command(&mut self) -> Result<Option<Command>> {
        use futures::{future::FutureExt, StreamExt};

        let maybe_command = tokio::select! {
            maybe_command = self.cmd_rx.recv() => { maybe_command }
            maybe_syntax = self.syntax.next().fuse() => {
                let syntax = maybe_syntax.expect("syntax thread crashed?");
                self.state.process_syntax(syntax)
            },
            maybe_event = self.events.next().fuse() => match maybe_event {
                None => Some(Command::Quit),
                Some(event) => self.state.process_event(event?),
            },
        };
        Ok(maybe_command)
    }

    async fn process_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Quit => unreachable!("handled in main loop"),
            Command::Commands(cmd) => match cmd {
                _ => todo!(),
                // selector::Command::Select(entry_id) => {
                //     let entry = self.state.commands.entries.get(entry_id).unwrap();
                //     self.cmd_tx.send(entry.command.clone()).await?;
                //     self.state.close_focused_pane();
                // }
                // selector::Command::Open => {
                //     self.state.focus_pane(self.state.commands_pane_id);
                // }
                // selector::Command::Close => {
                //     debug_assert_eq!(self.state.focused_pane, self.state.commands_pane_id);
                //     self.state.close_focused_pane();
                // }
            },
            Command::Pane(pane_id, cmd) => match cmd {
                PaneCommand::Open => {
                    self.state.focus_pane(pane_id);
                }
                PaneCommand::Close => {
                    debug_assert_eq!(self.state.focused_pane, pane_id);
                    self.state.close_focused_pane()
                }
            },
            Command::Editor(editor_id, cmd) => {
                let editor = &mut self.state.editors[editor_id];
                let buffer = &mut self.state.buffers[editor.buffer_id];
                editor.command(buffer, cmd);
            }
            Command::Buffer(buffer_id, cmd) => {
                let buffer = &mut self.state.buffers[buffer_id];
                buffer.command(cmd);
            }

            Command::FocusedEditor(cmd) => {
                let pane_id = match self.state.focused_pane() {
                    Pane::Commands(..) => {
                        if let [.., pane_id, _] = self.state.visible_panes[..] {
                            match self.state.panes[pane_id] {
                                Pane::Editor(..) => pane_id,
                                _ => unreachable!("no focused editor"),
                            }
                        } else {
                            unreachable!("no visible panes")
                        }
                    }
                    Pane::Editor(..) => self.state.focused_pane,
                };
                let pane = &self.state.panes[pane_id];
                match pane {
                    Pane::Commands(..) => {
                        unreachable!("focused pane is not an editor")
                    }
                    Pane::Editor(_, editor_id) => {
                        let editor = &mut self.state.editors[*editor_id];
                        let buffer = &mut self.state.buffers[editor.buffer_id];
                        editor.command(buffer, cmd);
                    }
                }
            }

            Command::FileOpen(maybe_editor_id, path) => {
                let contents = Buffer::read(&path).await?;
                let buffer_id = self
                    .state
                    .buffers
                    .insert_with_key(|k| Buffer::new(k, contents.clone()));

                let editor_id = maybe_editor_id.unwrap_or(self.state.default_editor_id);
                let editor = &mut self.state.editors[editor_id];
                editor.swap_buffer(buffer_id);

                match syntax::Language::try_from(&self.state.buffers[buffer_id]) {
                    Ok(language) => {
                        self.syntax
                            .command(syntax::Command::Parse { buffer_id, contents, language })
                            .await?;
                    }
                    _ => todo!(),
                };
            }
        };

        Ok(())
    }
}

fn register_commands(registry: &mut CommandRegistry) {
    use editor::EditorCommand::*;
    use editor::{CursorJump, Direction};

    registry.register("quit", vec![], Command::Quit);

    let cmds = [
        ("cursor.up", vec![], CursorMove(Direction::Up)),
        ("cursor.down", vec![], CursorMove(Direction::Down)),
        ("cursor.left", vec![], CursorMove(Direction::Left)),
        ("cursor.right", vec![], CursorMove(Direction::Right)),
        ("cursor.startOfNextWord", vec![], CursorJump(CursorJump::StartOfNextWord)),
        ("cursor.startOfLastWord", vec![], CursorJump(CursorJump::StartOfLastWord)),
        ("cursor.startOfNearestWord", vec![], CursorJump(CursorJump::StartOfNearestWord)),
        ("cursor.endOfNearestWord", vec![], CursorJump(CursorJump::EndOfNearestWord)),
    ];
    for (name, aliases, cmd) in cmds {
        registry.register(name, aliases, Command::FocusedEditor(cmd));
    }

    // commands.reset();
}
