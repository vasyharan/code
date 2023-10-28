use toku::error;
use toku::term;

fn main() -> error::Result<()> {
    let mut raw_mode = term::RawMode::new()?;
    raw_mode.enable()?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let mut keyboard = term::KeyboardInput::new();
        'main: loop {
            match keyboard.read_key().await? {
                term::Key::Char(c) => {
                    println!("{}\r", c)
                }
                term::Key::Ctrl(k) => match k {
                    b'q' | b'c' => break 'main,
                    _ => {}
                },
            }
        }
        Ok::<_, error::Error>(())
    })
}
