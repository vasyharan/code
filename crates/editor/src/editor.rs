use anyhow::Result;

use crate::{Buffer, BufferContents, BufferId};
use crossterm::event::KeyEvent;
use rope::SlabAllocator;
use slotmap::new_key_type;
use tore::Point;

new_key_type! {
    pub struct Id;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
}

#[derive(Debug, Clone)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub enum CursorJump {
    ForwardWordEnd,
    ForwardNextWordStart,
    BackwardWordStart,
    BackwardPrevWordEnd,
}

#[derive(Debug, Clone)]
pub enum Command {
    ModeSet(Mode),
    SwapBuffer(BufferId),
    CursorMove(Direction),
    CursorJump(CursorJump),
}

#[derive(Debug)]
pub struct Editor {
    pub id: Id,
    pub buffer_id: BufferId,
    pub mode: Mode,
    pub cursor: Point,
}

impl Editor {
    pub fn new(id: Id, buffer_id: BufferId) -> Self {
        Self { id, buffer_id, mode: Default::default(), cursor: Default::default() }
    }

    pub fn process_key(&mut self, key: KeyEvent, buffer: &Buffer) -> Option<Command> {
        use crossterm::event::KeyCode;
        debug_assert!(buffer.id == self.buffer_id);

        match self.mode {
            Mode::Normal => match key.code {
                KeyCode::Up | KeyCode::Char('k') => self.cursor_move_up(buffer),
                KeyCode::Down | KeyCode::Char('j') => self.cursor_move_down(buffer),
                KeyCode::Left | KeyCode::Char('h') => self.cursor_move_left(buffer),
                KeyCode::Right | KeyCode::Char('l') => self.cursor_move_right(buffer),
                KeyCode::Char('w') => self.cursor_jump_forward_next_word_start(buffer),
                KeyCode::Char('e') => self.cursor_jump_forward_word_end(buffer),
                KeyCode::Char('0') => self.cursor_jump_line_zero(buffer),
                // KeyCode::Char('i') => Some(Command::ModeSet(Mode::Insert)),
                _ => (),
            },
            Mode::Insert => match key.code {
                // KeyCode::Esc => Some(Command::ModeSet(Mode::Normal)),
                KeyCode::Up => self.cursor_move_up(buffer),
                KeyCode::Down => self.cursor_move_down(buffer),
                KeyCode::Left => self.cursor_move_left(buffer),
                KeyCode::Right => self.cursor_move_right(buffer),
                // KeyCode::Char(c) => Some(Command::Insert(c)),
                _ => (),
            },
        }
        None
    }

    pub fn swap_buffer(&mut self, buffer_id: BufferId) {
        self.buffer_id = buffer_id;
    }

    pub fn command(&mut self, buffer: &Buffer, command: Command) {
        debug_assert!(buffer.id == self.buffer_id);
        match command {
            Command::SwapBuffer(buffer_id) => self.swap_buffer(buffer_id),
            Command::ModeSet(mode) => self.mode = mode,
            Command::CursorMove(direction) => match direction {
                Direction::Up => self.cursor_move_up(buffer),
                Direction::Down => self.cursor_move_down(buffer),
                Direction::Left => self.cursor_move_left(buffer),
                Direction::Right => self.cursor_move_right(buffer),
            },
            Command::CursorJump(jump) => match jump {
                CursorJump::ForwardWordEnd => self.cursor_jump_forward_word_end(buffer),
                CursorJump::ForwardNextWordStart => {
                    self.cursor_jump_forward_next_word_start(buffer)
                }
                CursorJump::BackwardWordStart => self.cursor_jump_backward_word_start(buffer),
                CursorJump::BackwardPrevWordEnd => self.cursor_jump_backward_prev_word_end(buffer),
            },
        };
    }

    // Entry { command: "cursor.left".to_string(), aliases: vec![] },
    // Entry { command: "cursor.right".to_string(), aliases: vec![] },
    // Entry { command: "cursor.up".to_string(), aliases: vec![] },
    // Entry { command: "cursor.down".to_string(), aliases: vec![] },
    // Entry { command: "cursor.forwardWord".to_string(), aliases: vec![] },
    // Entry { command: "cursor.backwardWord".to_string(), aliases: vec![] },
    // Entry { command: "cursor.forwardWordEnd".to_string(), aliases: vec![] },
    // Entry { command: "cursor.backwardWordEnd".to_string(), aliases: vec![] },
    // pub fn commands() -> Vec<(commands::Entry, Command)> {
    //     vec![
    //         (
    //             commands::Entry {
    //                 command: "cursor.jump.forwardWordEnd".to_string(),
    //                 aliases: vec![],
    //             },
    //             Command::CursorJump(CursorJump::ForwardWordEnd),
    //         ),
    //         (
    //             commands::Entry {
    //                 command: "cursor.jump.forwardNextWordStart".to_string(),
    //                 aliases: vec![],
    //             },
    //             Command::CursorJump(CursorJump::ForwardNextWordStart),
    //         ),
    //         (
    //             commands::Entry {
    //                 command: "cursor.jump.backwardWordStart".to_string(),
    //                 aliases: vec![],
    //             },
    //             Command::CursorJump(CursorJump::BackwardWordStart),
    //         ),
    //         (
    //             commands::Entry {
    //                 command: "cursor.jump.backwardPrevWordEnd".to_string(),
    //                 aliases: vec![],
    //             },
    //             Command::CursorJump(CursorJump::BackwardPrevWordEnd),
    //         ),
    //     ]
    // }
}

#[tracing::instrument(skip(alloc))]
async fn file_open(alloc: &mut SlabAllocator, path: &std::path::PathBuf) -> Result<BufferContents> {
    Buffer::read(alloc, path).await
}
