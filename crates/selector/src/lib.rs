use tore::Point;

#[derive(Debug, Clone)]
pub enum Mode {
    Single,
}

#[derive(Debug, Clone)]
pub enum Direction {
    Next,
    Prev,
}

#[derive(Debug, Clone)]
pub enum Command<Id> {
    Focus(Direction),
    Delete(Direction),
    Insert(char),
    SetEntries(Vec<Id>),
}

#[derive(Debug)]
pub struct Selector<Id: Eq + Copy> {
    pub query_prefix: &'static str,
    pub query: String,
    pub cursor: Point,
    pub entries: Vec<Id>,
    pub focused: Option<Id>,
}

impl<Id: Eq + Copy> Selector<Id> {
    pub fn new(query_prefix: &'static str) -> Self {
        let query = String::default();
        let cursor = Point::default();
        let focused = None;
        let entries = vec![];
        Self { query_prefix, query, cursor, entries, focused }
    }

    pub fn command(&mut self, command: Command<Id>) {
        match command {
            Command::Focus(dir) => self.focus(dir),
            Command::Delete(dir) => self.delete(dir),
            Command::Insert(c) => self.insert(c),
            Command::SetEntries(es) => self.set_entries(es),
        }
    }

    pub fn insert(&mut self, c: char) {
        if self.cursor.column == self.query.len() {
            self.query.push(c);
        } else {
            self.query.insert(self.cursor.column, c);
        }
        self.cursor.move_next_column();
    }

    fn delete(&mut self, dir: Direction) {
        let range = match dir {
            Direction::Next => self.cursor.column..self.cursor.column + 1,
            Direction::Prev => self.cursor.column - 1..self.cursor.column,
        };
        self.query.drain(range);
        self.cursor.move_prev_column();
    }

    fn focus(&mut self, direction: Direction) {
        let mut prev: Option<&Id> = None;
        let mut iter = self.entries.iter();
        loop {
            match iter.next() {
                None => break,
                Some(res) => {
                    if Some(*res) == self.focused {
                        let focused = match direction {
                            Direction::Prev => prev,
                            Direction::Next => iter.next(),
                        };
                        if focused.is_some() {
                            self.focused = focused.copied();
                        }
                        break;
                    }
                    prev = Some(res);
                }
            }
        }
    }

    fn set_entries(&mut self, entries: Vec<Id>) {
        self.entries = entries;
    }
}
