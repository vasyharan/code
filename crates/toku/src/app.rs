use anyhow::Result;
use crossterm::event::Event;
use editor::{Buffer, BufferContents, BufferId, Editor, EditorId};
use slotmap::SlotMap;
use tokio::sync::mpsc;

type BufferMap = SlotMap<BufferId, Buffer>;
type EditorMap = SlotMap<EditorId, Editor>;

#[derive(Debug)]
pub enum Command {
    Quit,
    FileOpen(std::path::PathBuf),

    EditorCommand(editor::Command),
}

#[derive(Debug)]
pub struct App {
    cmd_rx: mpsc::Receiver<Command>,
}

impl App {
    pub fn spawn(paths: Option<Vec<std::path::PathBuf>>) -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread().build()?;
        rt.block_on(async move {
            let (cmd_tx, cmd_rx) = mpsc::channel(1);
            let app = Self::new(cmd_rx);
            let run = tokio::spawn(app.run());
            if let Some(paths) = paths {
                for p in paths.iter() {
                    cmd_tx.send(Command::FileOpen(p.clone())).await?;
                }
            }

            run.await?
        })
    }

    fn new(cmd_rx: mpsc::Receiver<Command>) -> Self {
        Self { cmd_rx }
    }

    async fn run(mut self) -> Result<()> {
        use crossterm::event::EventStream;
        use futures::{future::FutureExt, StreamExt};
        use ratatui::backend::CrosstermBackend;
        use ratatui::Terminal;

        let mut term = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
        let mut events = EventStream::new();

        let mut buffers = BufferMap::with_key();
        let mut editors = EditorMap::with_key();
        let mut editor_id: EditorId = editors.insert_with_key(|k| {
            let buffer_id: BufferId = buffers.insert_with_key(Buffer::empty);
            Editor::new(k, buffer_id)
        });

        'main: loop {
            term.draw(|frame| {
                let area = frame.size();
                let editor = &editors[editor_id];
                let buffer = &buffers[editor.buffer_id];
                let pane = ui::EditorPane::new(buffer, editor);
                let cursor = editor.cursor;
                frame.render_widget(pane, area);
                frame.set_cursor((cursor.column - 1) as u16, (cursor.line - 1) as u16);
            })?;

            let mut maybe_command = tokio::select! {
                maybe_command = self.cmd_rx.recv() => { maybe_command }
                maybe_event = events.next().fuse() => {
                    match maybe_event {
                        Some(event) => {
                            let editor = &mut editors[editor_id];
                            let buffer = &mut buffers[editor.buffer_id];
                            self.process_event(editor, buffer, event?)
                        },
                        None => break 'main,
                    }
                }
            };

            if let Some(command) = maybe_command.take() {
                use Command::*;

                match command {
                    Quit => break 'main,
                    EditorCommand(command) => match command {
                        editor::Command::Editor(editor_id, cmd) => {
                            let editor = &mut editors[editor_id];
                            let buffer = &mut buffers[editor.buffer_id];
                            editor.command(&buffer, cmd);
                        }
                    },
                    FileOpen(path) => {
                        let contents = self.file_open(&path).await?;
                        let buffer_id = buffers.insert_with_key(|k| Buffer::new(k, contents));
                        editor_id = editors.insert_with_key(|k| Editor::new(k, buffer_id));
                    }
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn file_open(&self, path: &std::path::PathBuf) -> Result<BufferContents> {
        Buffer::read(&path).await
    }

    #[tracing::instrument(skip(self, editor, buffer, ev))]
    fn process_event(
        &self,
        editor: &mut Editor,
        buffer: &mut Buffer,
        ev: Event,
    ) -> Option<Command> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match ev {
            Event::FocusGained => todo!(),
            Event::FocusLost => todo!(),
            Event::Paste(_) => todo!(),
            Event::Mouse(_) => todo!(),
            Event::Resize(_, _) => todo!(),
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
                command.or_else(|| editor.process_key(key, buffer).map(Command::EditorCommand))
            }
        }
    }
}
