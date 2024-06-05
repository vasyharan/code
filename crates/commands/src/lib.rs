use bitflags::bitflags;
use crossterm::event::{KeyEvent, KeyModifiers};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use slotmap::{new_key_type, SlotMap};
use tokio::sync::mpsc;

use tore::Point;

#[derive(Debug, Clone)]
pub enum Command {
    Open,
    Close,
    Select(EntryId),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Command,
}

bitflags! {
    #[derive(Debug)]
    pub struct EntryMode: u8 {
        const ALWAYS   = 0b0000;
        const VISIBLE  = 0b0001;
        const FOCUSED  = 0b0010;
    }
}

#[derive(Debug)]
pub struct Entry<T> {
    pub id: EntryId,
    pub name: String,
    pub aliases: Vec<String>,
    pub command: T,
}

#[derive(Debug)]
pub struct ResultEntry<'a, T> {
    pub entry: &'a Entry<T>,
    pub score: i64,
    pub indices: Vec<usize>,
}

#[derive(Debug)]
struct SearchResult {
    pub entry: EntryId,
    pub score: i64,
    pub indices: Vec<usize>,
}

new_key_type! {
    pub struct EntryId;
}

#[derive(Debug)]
pub struct Commands<T> {
    pub sender: mpsc::Sender<T>,
    pub query: String,
    pub cursor: Point,
    pub entries: SlotMap<EntryId, Entry<T>>,

    pub selected: Option<EntryId>,
    filtered: Vec<SearchResult>,
}

const MAX_RESULTS: usize = 32;

impl<T> Commands<T> {
    pub fn new(tx: mpsc::Sender<T>) -> Self {
        Self {
            sender: tx,
            query: String::new(),
            cursor: Point::default(),
            entries: SlotMap::with_key(),
            selected: None,
            filtered: vec![],
        }
    }

    pub fn register(&mut self, command: &str, aliases: Vec<&str>, msg: T) -> EntryId {
        let command = command.to_string();
        let aliases = aliases.iter().map(|s| s.to_string()).collect();
        self.entries
            .insert_with_key(|id| Entry { id, name: command, aliases, command: msg })
    }

    pub fn process_key(&mut self, key: KeyEvent) -> Option<Command> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Backspace => {
                self.query.pop();
                self.cursor.move_prev_column();
                self.search();
            }
            KeyCode::Char(c) => {
                if c == 'p' && key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.select_up();
                } else if c == 'n' && key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.select_down();
                } else {
                    self.query.push(c);
                    self.cursor.move_next_column();
                    self.search();
                }
            }
            KeyCode::Up => self.select_up(),
            KeyCode::Down => self.select_down(),
            KeyCode::Enter => return Some(Command::Select(self.selected?)),
            _ => {}
        }
        None
    }

    pub fn reset(&mut self) {
        self.query = String::new();
        self.cursor = Point::default();
        self.selected = None;
        self.filtered.clear();
        self.search();
    }

    pub fn results(&self) -> Vec<ResultEntry<T>> {
        self.filtered
            .iter()
            .map(|SearchResult { entry, score, indices }| ResultEntry {
                entry: &self.entries[*entry],
                score: *score,
                indices: indices.clone(),
            })
            .collect()
    }

    #[tracing::instrument(skip(self))]
    pub fn search(&mut self) {
        let mut results = vec![];
        if self.query.is_empty() {
            for (id, _) in &self.entries {
                if results.len() > MAX_RESULTS {
                    break;
                }
                results.push(SearchResult { entry: id, score: 0, indices: vec![] });
            }
        } else {
            let matcher = SkimMatcherV2::default();
            for (id, entry) in &self.entries {
                let result = matcher.fuzzy_indices(&entry.name, &self.query);
                if let Some((score, indices)) = result {
                    results.push(SearchResult { entry: id, score, indices });
                }
            }
            results.sort_by_key(|entry| entry.score);
        }

        self.selected = results.first().map(|r| r.entry);
        self.filtered = results;
    }

    fn select_up(&mut self) -> () {
        let idx = self
            .filtered
            .iter()
            .enumerate()
            .find(|(_, r)| Some(r.entry) == self.selected)
            .map(|(i, _)| i)
            .unwrap_or(0);
        if idx > 0 {
            self.selected = self.filtered.get(idx - 1).map(|r| r.entry);
        }
    }

    fn select_down(&mut self) -> () {
        let idx = self
            .filtered
            .iter()
            .enumerate()
            .find(|(_, r)| Some(r.entry) == self.selected)
            .map(|(i, _)| i)
            .unwrap_or(0);
        if idx + 1 < self.filtered.len() {
            self.selected = self.filtered.get(idx + 1).map(|r| r.entry);
        }
    }
}
