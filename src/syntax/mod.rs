use std::thread;

use anyhow::{Error, Result};
use futures::Stream;
use tokio::sync::mpsc;
use tree_sitter as ts;

use crate::buffer;
use crate::rope::{self, Rope};

pub(crate) mod highlighter;
pub(crate) mod language;

pub(crate) use language::Language;

#[derive(Debug)]
struct RopeTextProvider<'a>(&'a Rope);

impl<'a> RopeTextProvider<'a> {
    fn parse_callback(&self) -> impl Fn(usize, ts::Point) -> &'a [u8] {
        |byte, _pos| -> &[u8] {
            if let Some(chunk) = self.0.chunks(byte..).next() {
                chunk
            } else {
                &[]
            }
        }
    }
}

impl<'a> ts::TextProvider<'a> for RopeTextProvider<'a> {
    type I = rope::Chunks<'a>;

    fn text(&mut self, node: ts::Node) -> Self::I {
        self.0.chunks(node.byte_range())
    }
}

#[derive(Debug)]
pub(crate) enum Command {
    Parse {
        buffer_id: buffer::Id,
        contents: Rope,
        language: Language,
    },
}

#[derive(Debug)]
pub(crate) enum SyntaxEvent {
    Parsed(buffer::Id, ts::Tree),
    Hightlight(buffer::Id, highlighter::Highlights),
}

#[derive(Debug)]
struct Worker(thread::JoinHandle<Result<()>>);

impl Worker {
    fn spawn(mut rx: mpsc::Receiver<Command>, tx: mpsc::Sender<SyntaxEvent>) -> Self {
        let thread_handle = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().build()?;
            rt.block_on(async {
                let mut parser = ts::Parser::new();

                while let Some(ev) = rx.recv().await {
                    use Command::*;
                    match ev {
                        Parse { buffer_id, contents, language } => {
                            let span = tracing::info_span!("parse_ts_tree").entered();
                            parser
                                .set_language(language.ts)
                                .expect("Error loading Rust grammar");
                            let ts_text = RopeTextProvider(&contents);
                            let tree = parser
                                .parse_with(&mut ts_text.parse_callback(), None)
                                .expect("expected a valid tree");
                            tx.send(SyntaxEvent::Parsed(buffer_id, tree.clone()))
                                .await?;
                            drop(span);

                            let highlights = highlighter::highlight(&contents, language, tree);
                            tx.send(SyntaxEvent::Hightlight(buffer_id, highlights))
                                .await?;
                        }
                    }
                }
                Ok::<(), Error>(())
            })?;

            Ok::<(), Error>(())
        });
        Self(thread_handle)
    }
}

#[derive(Debug)]
pub(crate) struct Client {
    cmd_tx: mpsc::Sender<Command>,
    event_rx: mpsc::Receiver<SyntaxEvent>,
    // worker: Worker,
}

impl Client {
    pub(crate) fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(1);
        let (event_tx, event_rx) = mpsc::channel(1);
        let _worker = Worker::spawn(cmd_rx, event_tx);
        Client { cmd_tx, event_rx }
    }

    pub(crate) async fn send(
        &self,
        command: Command,
    ) -> std::result::Result<(), mpsc::error::SendError<Command>> {
        self.cmd_tx.send(command).await
    }
}

impl Stream for Client {
    type Item = SyntaxEvent;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.as_mut().event_rx.poll_recv(cx)
    }
}
