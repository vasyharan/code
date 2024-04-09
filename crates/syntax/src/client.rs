use anyhow::Result;
use futures::Stream;
use std::thread;
use tokio::sync::mpsc;
use tree_sitter as ts;

use crate::BufferContentsTextProvider;
use crate::{highlighter, Language};
use editor::{BufferContents, BufferId, Highlights};

#[derive(Debug)]
pub enum Command {
    Parse {
        buffer_id: BufferId,
        contents: BufferContents,
        language: Language,
    },
}

#[derive(Debug)]
pub enum Event {
    Parsed(BufferId, ts::Tree),
    Hightlight(BufferId, Highlights),
}

#[derive(Debug)]
struct Worker(thread::JoinHandle<Result<()>>);

impl Worker {
    fn spawn(mut rx: mpsc::Receiver<Command>, tx: mpsc::Sender<Event>) -> Self {
        let thread_handle = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().build()?;
            rt.block_on(async {
                let mut parser = ts::Parser::new();

                while let Some(ev) = rx.recv().await {
                    use Command::*;
                    match ev {
                        Parse {
                            buffer_id,
                            contents,
                            language,
                        } => {
                            let span = tracing::info_span!("parse_ts_tree").entered();
                            parser.set_language(language.ts)?;
                            let ts_text = BufferContentsTextProvider(&contents);
                            let ts_tree = parser.parse_with(&mut ts_text.parse_callback(), None);
                            drop(span);
                            match ts_tree {
                                None => todo!(),
                                Some(tree) => {
                                    tx.send(Event::Parsed(buffer_id, tree.clone())).await?;
                                    let highlights =
                                        highlighter::highlight(&contents, language, tree);
                                    tx.send(Event::Hightlight(buffer_id, highlights)).await?;
                                }
                            }
                        }
                    }
                }
                Ok::<(), anyhow::Error>(())
            })?;

            Ok(())
        });
        Self(thread_handle)
    }
}

#[derive(Debug)]
pub struct Client {
    cmd_tx: mpsc::Sender<Command>,
    event_rx: mpsc::Receiver<Event>,
    worker: Worker,
}

impl Client {
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(1);
        let (event_tx, event_rx) = mpsc::channel(1);
        let worker = Worker::spawn(cmd_rx, event_tx);
        Client {
            cmd_tx,
            event_rx,
            worker,
        }
    }

    pub async fn command(&self, command: Command) -> Result<()> {
        self.cmd_tx.send(command).await?;
        Ok(())
    }

    pub fn join(self) -> Result<()> {
        // FIXME: handle error
        let _ = self.worker.0.join();
        Ok(())
    }
}

impl Stream for Client {
    type Item = Event;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.as_mut().event_rx.poll_recv(cx)
    }
}
