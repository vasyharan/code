use std::thread;

use futures::Stream;
use tokio::sync::mpsc;
use tree_sitter as ts;

use crate::error::Error;
use crate::rope::{self, Chunks, Rope};

pub(crate) mod highlighter;
pub(crate) mod language;

use language::Language;

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
    type I = Chunks<'a>;

    fn text(&mut self, node: ts::Node) -> Self::I {
        self.0.chunks(node.byte_range())
    }
}

#[derive(Debug)]
pub(crate) enum Command {
    Parse { contents: Rope, language: Language },
}

#[derive(Debug)]
pub(crate) enum SyntaxEvent {
    Parsed(ts::Tree),
    Hightlight(highlighter::Highlights),
}

struct Worker {
    // thread_handle: thread::JoinHandle<Result<()>>,
}

impl Worker {
    fn spawn(mut rx: mpsc::Receiver<Command>, tx: mpsc::Sender<SyntaxEvent>) -> Self {
        let thread_handle = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                // .enable_all()
                .build()?;
            rt.block_on(async {
                let mut parser = ts::Parser::new();

                while let Some(ev) = rx.recv().await {
                    use Command::*;
                    match ev {
                        Parse { contents, language } => {
                            let span = tracing::info_span!("parse_ts_tree").entered();
                            parser
                                .set_language(language.ts)
                                .expect("Error loading Rust grammar");
                            let ts_text = RopeTextProvider(&contents);
                            let tree = parser
                                .parse_with(&mut ts_text.parse_callback(), None)
                                .expect("expected a valid tree");
                            tx.send(SyntaxEvent::Parsed(tree.clone())).await?;
                            drop(span);

                            let highlights = highlighter::highlight(&contents, language, tree);
                            tx.send(SyntaxEvent::Hightlight(highlights)).await?;
                        }
                    }
                }
                Ok::<(), Error>(())
            })?;

            Ok::<(), Error>(())
        });
        Self {}
    }
}

pub(crate) struct Client {
    cmd_tx: mpsc::Sender<Command>,
    event_rx: mpsc::Receiver<SyntaxEvent>,
    // worker: Worker,
}

impl Client {
    pub(crate) fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(1);
        let (event_tx, event_rx) = mpsc::channel(1);
        let _ = Worker::spawn(cmd_rx, event_tx);
        Client { cmd_tx, event_rx }
    }

    pub(crate) async fn parse(
        &self,
        language: Language,
        contents: rope::Rope,
    ) -> std::result::Result<(), mpsc::error::SendError<Command>> {
        self.cmd_tx
            .send(Command::Parse { contents, language })
            .await
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
