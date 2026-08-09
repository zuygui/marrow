use std::fmt;

#[derive(Debug, Clone)]
pub struct CompileError {
    pub line: usize,
    pub col: usize,
    pub len: usize,
    pub message: String,
}

impl CompileError {
    pub fn new(line: usize, col: usize, len: usize, message: impl Into<String>) -> Self {
        CompileError {
            line,
            col,
            len: len.max(1),
            message: message.into(),
        }
    }

    pub fn render(&self, filename: &str, source: &str) -> String {
        let lines: Vec<&str> = source.split('\n').collect();
        let line_idx = self.line.saturating_sub(1);
        let src_line = lines.get(line_idx).copied().unwrap_or("").trim_end_matches('\r');

        let gutter_width = self.line.to_string().len();
        let pad = " ".repeat(gutter_width);

        let mut out = String::new();
        out.push_str(&format!("error: {}\n", self.message));
        out.push_str(&format!("{}--> {}:{}:{}\n", pad, filename, self.line, self.col));
        out.push_str(&format!("{} |\n", pad));
        out.push_str(&format!("{} | {}\n", self.line, src_line));

        let col0 = self.col.saturating_sub(1).min(src_line.chars().count());
        let underline_indent = " ".repeat(col0);
        let carets = "^".repeat(self.len.max(1));
        out.push_str(&format!("{} | {}{}\n", pad, underline_indent, carets));

        out
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for CompileError {}