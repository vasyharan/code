use anyhow::Result;
use crossterm::event::Event;

#[derive(Debug)]
pub struct App {}

impl App {
    pub fn run() -> Result<()> {
        use crossterm::event::EventStream;
        use futures::{future::FutureExt, StreamExt};
        use ratatui::backend::CrosstermBackend;
        use ratatui::Terminal;

        let rt = tokio::runtime::Builder::new_current_thread().build()?;
        rt.block_on(async move {
            let mut term = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
            let mut events = EventStream::new();
            let app = Self {};

            'main: loop {
                term.draw(|frame| {})?;

                let maybe_command = match events.next().fuse().await {
                    None => break 'main,
                    Some(ev) => app.process_event(ev.unwrap()),
                };
            }

            Ok(())
        })
    }

    #[tracing::instrument(skip(self))]
    fn process_event(&self, ev: Event) -> Option<()> {
        todo!()
    }
}
