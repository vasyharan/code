use tokio::io::{stdout, AsyncWriteExt, BufWriter, Stdout};

#[derive(Debug)]
pub struct Dimensions {
    pub rows: usize,
    pub cols: usize,
}

fn dimensions() -> Dimensions {
    if let Some((cols, rows)) = term_size::dimensions() {
        return Dimensions { rows, cols };
    }
    unimplemented!()
}

#[derive(Debug)]
pub struct Display {
    pub dimensions: Dimensions,
    out: BufWriter<Stdout>,
}

#[derive(Debug)]
pub enum EraseInMode {
    FromPos,
    ToPos,
    All,
}

impl Display {
    pub fn new() -> Self {
        let dimensions = dimensions();
        let out = BufWriter::new(stdout());
        Self { dimensions, out }
    }

    pub async fn write_all(&mut self, contents: &[u8]) -> std::io::Result<()> {
        self.out.write_all(contents).await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> std::io::Result<()> {
        self.out.flush().await
    }

    pub async fn hide_cursor(&mut self) -> std::io::Result<()> {
        self.out.write_all(b"\x1b[?25l").await
    }

    pub async fn show_cursor(&mut self) -> std::io::Result<()> {
        self.out.write_all(b"\x1b[?25h").await
    }

    pub async fn erase_in(&mut self, mode: EraseInMode) -> std::io::Result<()> {
        // https://vt100.net/docs/vt100-ug/chapter3.html#ED
        match mode {
            EraseInMode::FromPos => self.out.write_all(b"\x1b[J").await,
            EraseInMode::ToPos => self.out.write_all(b"\x1b[1J").await,
            EraseInMode::All => self.out.write_all(b"\x1b[2J").await,
        }
    }

    pub async fn erase_in_line(&mut self, mode: EraseInMode) -> std::io::Result<()> {
        // https://vt100.net/docs/vt100-ug/chapter3.html#ED
        match mode {
            EraseInMode::FromPos => self.out.write_all(b"\x1b[K").await,
            EraseInMode::ToPos => self.out.write_all(b"\x1b[1K").await,
            EraseInMode::All => self.out.write_all(b"\x1b[2K").await,
        }
    }

    pub async fn cursor_position(&mut self, row: usize, col: usize) -> std::io::Result<()> {
        // https://vt100.net/docs/vt100-ug/chapter3.html#CUP
        if row == 0 && col == 0 {
            self.out.write_all(b"\x1b[H").await
        } else {
            self.out
                .write_all(format!("\x1b[{};{}H", row, col).as_bytes())
                .await
        }
    }

    pub(crate) async fn enable_alternate_screen(&mut self) -> std::io::Result<()> {
        self.out.write_all(b"\x1b[?1049h").await
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        // use std::io::Write;
        // let mut out = std::io::stdout();
        // // TODO: do this sync so that we can handle panics
        // let disable_alternate_screen = out.write(b"\x1b[?1049l\x1b[H");
        // let clear_screen = out.write(b"\x1b[2J");
        // let reset_cursor = out.write(b"\x1b[H");
        // _ = out.flush();
        // disable_alternate_screen.expect("error disabling alternate screen on exit");
        // clear_screen.expect("error clearing screen on exit");
        // reset_cursor.expect("error resetting cursor on exit");
    }
}
