use tokio::io::AsyncReadExt;

use crate::error;
use crate::term::stdin::AsyncStdin;

pub enum Key {
    Char(u8),
    Ctrl(u8),
}

pub struct KeyboardInput {
    stdin: AsyncStdin,
}

impl KeyboardInput {
    pub fn new() -> Self {
        Self { stdin: AsyncStdin::new() }
    }

    pub async fn read_key(&mut self) -> error::Result<Key> {
        match self.read_byte().await? {
            0x1b => match self.read_byte().await? {
                b'[' => unimplemented!(),
                b if b.is_ascii_control() => Ok(Key::Char(0x1b)),
                _ => unimplemented!(),
            },
            b if b.is_ascii_control() => Ok(Key::Ctrl(b | 0b0110_0000)),
            b => Ok(Key::Char(b)),
        }
    }

    async fn read_byte(&mut self) -> error::Result<u8> {
        let mut buf: [u8; 1] = [0; 1];
        let read = self.stdin.read(&mut buf[0..1]).await?;
        assert_eq!(read, 1); // block until read
        Ok(buf[0])
    }
}
