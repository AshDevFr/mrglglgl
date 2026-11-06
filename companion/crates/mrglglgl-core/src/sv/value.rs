use std::collections::BTreeMap;

/// A Lua value as WoW writes it into SavedVariables. Functions and userdata never appear there.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Table(Table),
}

/// A table key. WoW writes string and integer keys; other key types are kept so that a file
/// written by some other addon in the same file still parses.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    Integer(i64),
    String(String),
    Bool(bool),
    /// A float key, kept as its bit pattern so keys stay totally ordered.
    Float(u64),
}

/// A Lua table. Positional entries are stored under integer keys counting from 1, exactly as Lua
/// would see them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Table {
    entries: BTreeMap<Key, Value>,
}

impl Table {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: Key, value: Value) {
        // Assigning nil removes a key in Lua.
        if value == Value::Nil {
            self.entries.remove(&key);
        } else {
            self.entries.insert(key, value);
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.get(&Key::String(key.to_string()))
    }

    pub fn get_index(&self, index: i64) -> Option<&Value> {
        self.entries.get(&Key::Integer(index))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Key, &Value)> {
        self.entries.iter()
    }

    /// The values at 1, 2, 3, ... up to the first gap, like Lua's `ipairs`.
    pub fn sequence(&self) -> impl Iterator<Item = &Value> {
        (1..).map_while(move |i| self.get_index(i))
    }
}

impl Value {
    pub fn as_table(&self) -> Option<&Table> {
        match self {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// An integer, also accepting a float with no fractional part (Lua 5.1 numbers are all
    /// doubles, and WoW writes some whole numbers with a decimal point).
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Value::Integer(i) => Some(i),
            Value::Float(f) if f.fract() == 0.0 && f.is_finite() => Some(f as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Value::Integer(i) => Some(i as f64),
            Value::Float(f) => Some(f),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }
}
