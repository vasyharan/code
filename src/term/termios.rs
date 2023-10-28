use crate::error;

pub struct RawMode {
    stdin: std::io::Stdin,
    is_enabled: bool,
    original_termios: termios::Termios,
}

impl RawMode {
    pub fn new() -> error::Result<Self> {
        #[cfg(any(unix))]
        use std::os::unix::io::AsRawFd;

        let stdin = std::io::stdin();
        let termios = termios::Termios::from_fd(stdin.as_raw_fd())?;
        Ok(Self { stdin, is_enabled: false, original_termios: termios })
    }

    pub fn enable(&mut self) -> error::Result<()> {
        #[cfg(any(unix))]
        use std::os::unix::io::AsRawFd;
        use termios::*;

        let mut raw = self.original_termios;
        // Disable control flow mode (Ctrl+Q/Ctrl+S) and CR-to-NL translation
        raw.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        // Disable output processing such as \n to \r\n translation
        raw.c_oflag &= !(OPOST);
        // Ensure character size is 8bits
        raw.c_cflag |= CS8;
        // Set terminal raw mode. Disable echo back, canonical mode, signals (SIGINT, SIGTSTP) and Ctrl+V.
        raw.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        tcsetattr(self.stdin.as_raw_fd(), TCSAFLUSH, &raw)?;
        self.is_enabled = true;
        Ok(())
    }

    pub fn disable(&mut self) -> error::Result<()> {
        #[cfg(any(unix))]
        use std::os::unix::io::AsRawFd;
        use termios::*;

        if self.is_enabled {
            tcsetattr(self.stdin.as_raw_fd(), TCSAFLUSH, &self.original_termios)?;
            self.is_enabled = false;
        }
        Ok(())
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        self.disable().unwrap();
    }
}
