use crate::Level;

/// Options for `read_opts`.
#[derive(Debug, Clone)]
pub struct Options {
    pub level: Level,
    /// When non-empty, every function/method/class whose name appears here is
    /// rendered with its body kept in full, regardless of the base `level`.
    /// A class name in this list expands to "every method of that class".
    pub focus: Vec<String>,
}

impl Options {
    pub fn new(level: Level) -> Self {
        Self {
            level,
            focus: Vec::new(),
        }
    }

    pub fn focus<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.focus = names.into_iter().map(Into::into).collect();
        self
    }
}
