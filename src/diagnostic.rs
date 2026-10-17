//! Compiler diagnostics: source spans, a source map and a rustc-style renderer.
//!
//! Everything here is dependency-free and deliberately simple (plain `Vec`s,
//! `String`s and indices, no traits or generics beyond what is strictly
//! needed) so that it can be rewritten in Marrow itself later on.
//!
//! Offsets in a [`Span`] are *character* offsets (not bytes) into the file's
//! text, which is what the lexer works with.
//!
//! Error code ranges:
//!   E00xx  driver / imports        E01xx  lexer
//!   E02xx  parser                  E03xx  names, items, decorators
//!   E04xx  types                   W0xxx  warnings

use std::io::IsTerminal;

// ---------------------------------------------------------------------------
// Spans and source files
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub file: usize,
    pub start: usize,
    pub end: usize,
}

#[allow(dead_code)]
impl Span {
    /// A span that points nowhere (used for compiler-internal diagnostics).
    pub const DUMMY: Span = Span { file: usize::MAX, start: 0, end: 0 };

    pub fn new(file: usize, start: usize, end: usize) -> Span {
        Span { file, start, end: end.max(start) }
    }

    pub fn is_dummy(&self) -> bool {
        self.file == usize::MAX
    }

    /// Smallest span covering both `self` and `other` (same file assumed).
    pub fn to(self, other: Span) -> Span {
        if self.is_dummy() {
            return other;
        }
        if other.is_dummy() {
            return self;
        }
        Span { file: self.file, start: self.start.min(other.start), end: self.end.max(other.end) }
    }

    /// Zero-width span at the start of `self`.
    pub fn start_point(self) -> Span {
        Span { end: self.start, ..self }
    }

    /// Zero-width span at the end of `self`.
    pub fn end_point(self) -> Span {
        Span { start: self.end, ..self }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

pub struct SourceFile {
    pub name: String,
    pub chars: Vec<char>,
    /// Files found through the installed standard library directory
    /// (`~/.marrow`): their warnings are not shown, like dependencies in Cargo.
    pub external: bool,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(name: String, text: &str, external: bool) -> SourceFile {
        let chars: Vec<char> = text.chars().collect();
        let mut line_starts = vec![0];
        for (i, c) in chars.iter().enumerate() {
            if *c == '\n' {
                line_starts.push(i + 1);
            }
        }
        SourceFile { name, chars, external, line_starts }
    }

    /// 0-based line index containing `offset`.
    pub fn line_index(&self, offset: usize) -> usize {
        self.line_starts.partition_point(|&s| s <= offset).saturating_sub(1)
    }

    /// 1-based (line, column) of `offset`; the column counts characters.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.chars.len());
        let li = self.line_index(offset);
        (li + 1, offset - self.line_starts[li] + 1)
    }

    /// Character range `[start, end)` of a line, without its line terminator.
    pub fn line_range(&self, line_idx: usize) -> (usize, usize) {
        let start = self.line_starts[line_idx];
        let mut end = match self.line_starts.get(line_idx + 1) {
            Some(next) => next - 1,
            None => self.chars.len(),
        };
        if end > start && self.chars.get(end - 1) == Some(&'\r') {
            end -= 1;
        }
        (start, end.max(start))
    }

    pub fn text(&self, start: usize, end: usize) -> String {
        let end = end.min(self.chars.len());
        let start = start.min(end);
        self.chars[start..end].iter().collect()
    }
}

#[derive(Default)]
pub struct SourceMap {
    pub files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> SourceMap {
        SourceMap { files: Vec::new() }
    }

    pub fn add_file(&mut self, name: String, text: &str, external: bool) -> usize {
        self.files.push(SourceFile::new(name, text, external));
        self.files.len() - 1
    }

    pub fn file(&self, id: usize) -> &SourceFile {
        &self.files[id]
    }

    pub fn get(&self, id: usize) -> Option<&SourceFile> {
        self.files.get(id)
    }
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub text: String,
    pub primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    Note,
    Help,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub kind: NoteKind,
    pub text: String,
}

/// A machine-applicable-looking suggestion: replace `span` by `replacement`
/// (an empty span means "insert").
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub msg: String,
    pub span: Span,
    pub replacement: String,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: Level,
    pub code: Option<&'static str>,
    pub message: String,
    pub labels: Vec<Label>,
    pub notes: Vec<Note>,
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(Level::Error, message)
    }

    pub fn warning(message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(Level::Warning, message)
    }

    fn new(level: Level, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            level,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// A bug in the compiler itself (the type checker should have caught the
    /// problem before code generation).
    pub fn ice(span: Span, message: impl Into<String>) -> Diagnostic {
        let mut d = Diagnostic::error(format!("internal compiler error: {}", message.into()));
        if !span.is_dummy() {
            d = d.primary(span, "while compiling this");
        }
        d.note("this is a bug in the Marrow compiler: the type checker should have reported it earlier")
            .note("please report it at https://github.com/zuygui/marrow/issues")
    }

    pub fn code(mut self, code: &'static str) -> Diagnostic {
        self.code = Some(code);
        self
    }

    pub fn primary(mut self, span: Span, text: impl Into<String>) -> Diagnostic {
        self.labels.push(Label { span, text: text.into(), primary: true });
        self
    }

    pub fn secondary(mut self, span: Span, text: impl Into<String>) -> Diagnostic {
        self.labels.push(Label { span, text: text.into(), primary: false });
        self
    }

    pub fn note(mut self, text: impl Into<String>) -> Diagnostic {
        self.notes.push(Note { kind: NoteKind::Note, text: text.into() });
        self
    }

    pub fn help(mut self, text: impl Into<String>) -> Diagnostic {
        self.notes.push(Note { kind: NoteKind::Help, text: text.into() });
        self
    }

    pub fn suggest(mut self, msg: impl Into<String>, span: Span, replacement: impl Into<String>) -> Diagnostic {
        self.suggestions.push(Suggestion { msg: msg.into(), span, replacement: replacement.into() });
        self
    }

    pub fn is_error(&self) -> bool {
        self.level == Level::Error
    }

    /// Span of the first primary label (or of the first label).
    pub fn main_span(&self) -> Option<Span> {
        self.labels
            .iter()
            .find(|l| l.primary && !l.span.is_dummy())
            .or_else(|| self.labels.iter().find(|l| !l.span.is_dummy()))
            .map(|l| l.span)
    }
}

// ---------------------------------------------------------------------------
// "Did you mean ...?"
// ---------------------------------------------------------------------------

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Returns the candidate closest to `name`, if it is close enough to be a
/// plausible typo (same threshold as rustc: a third of the name's length).
pub fn suggest_similar<'a>(name: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let max = (name.chars().count() / 3).max(1);
    let mut best: Option<(usize, &'a str)> = None;
    for c in candidates {
        if c == name {
            continue;
        }
        let d = edit_distance(name, c);
        // Case-only differences are always worth suggesting.
        let d = if name.eq_ignore_ascii_case(c) { 0 } else { d };
        if d > max {
            continue;
        }
        match best {
            Some((bd, bc)) if bd < d || (bd == d && bc <= c) => {}
            _ => best = Some((d, c)),
        }
    }
    best.map(|(_, c)| c)
}

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

pub struct Style {
    pub color: bool,
}

impl Style {
    pub fn new(choice: ColorChoice) -> Style {
        let color = match choice {
            ColorChoice::Always => true,
            ColorChoice::Never => false,
            ColorChoice::Auto => {
                std::io::stderr().is_terminal()
                    && std::env::var_os("NO_COLOR").is_none()
                    && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
            }
        };
        Style { color }
    }

    fn paint(&self, text: &str, code: &str) -> String {
        if self.color && !text.is_empty() {
            format!("\x1b[{}m{}\x1b[0m", code, text)
        } else {
            text.to_string()
        }
    }

    fn bold(&self, t: &str) -> String {
        self.paint(t, "1")
    }
    fn blue(&self, t: &str) -> String {
        self.paint(t, "1;34")
    }
    fn level(&self, level: Level, t: &str) -> String {
        match level {
            Level::Error => self.paint(t, "1;31"),
            Level::Warning => self.paint(t, "1;33"),
        }
    }
    fn note_kind(&self, kind: NoteKind, t: &str) -> String {
        match kind {
            NoteKind::Note => self.paint(t, "1;32"),
            NoteKind::Help => self.paint(t, "1;36"),
        }
    }
    fn green(&self, t: &str) -> String {
        self.paint(t, "1;32")
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

const TAB_WIDTH: usize = 4;

/// Expands tabs and maps every character index of the line to its display
/// column. The returned map has `chars.len() + 1` entries.
fn expand_line(chars: &[char]) -> (String, Vec<usize>) {
    let mut text = String::new();
    let mut cols = Vec::with_capacity(chars.len() + 1);
    let mut col = 0;
    for &c in chars {
        cols.push(col);
        if c == '\t' {
            for _ in 0..TAB_WIDTH {
                text.push(' ');
            }
            col += TAB_WIDTH;
        } else {
            text.push(c);
            col += 1;
        }
    }
    cols.push(col);
    (text, cols)
}

struct LineLabel {
    start: usize,
    end: usize,
    text: String,
    primary: bool,
}

fn digits(mut n: usize) -> usize {
    let mut d = 1;
    while n >= 10 {
        n /= 10;
        d += 1;
    }
    d
}

fn put(row: &mut Vec<char>, col: usize, ch: char) {
    while row.len() <= col {
        row.push(' ');
    }
    row[col] = ch;
}

fn put_str(row: &mut Vec<char>, col: usize, s: &str) {
    for (i, ch) in s.chars().enumerate() {
        put(row, col + i, ch);
    }
}

/// Builds the rows printed under a source line to underline `labels`.
/// Each returned row is a plain (uncolored) string made of columns; `kinds`
/// tells for every column whether it belongs to a primary or secondary
/// underline so that the caller can color it.
fn underline_rows(labels: &[LineLabel]) -> Vec<(Vec<char>, Vec<u8>)> {
    // `kind` per column: 0 = plain, 1 = primary mark, 2 = secondary mark.
    let mut rows: Vec<(Vec<char>, Vec<u8>)> = Vec::new();

    let mut main: Vec<char> = Vec::new();
    let mut kinds: Vec<u8> = Vec::new();
    let set_kind = |kinds: &mut Vec<u8>, col: usize, k: u8| {
        while kinds.len() <= col {
            kinds.push(0);
        }
        kinds[col] = k;
    };

    // Secondary underlines first so that primary marks win on overlaps.
    for pass in [false, true] {
        for l in labels.iter().filter(|l| l.primary == pass) {
            let ch = if l.primary { '^' } else { '-' };
            for c in l.start..l.end {
                put(&mut main, c, ch);
                set_kind(&mut kinds, c, if l.primary { 1 } else { 2 });
            }
        }
    }

    // The label with the right-most end gets its text on the underline row.
    let inline = labels
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.text.is_empty())
        .max_by_key(|(_, l)| (l.end, l.start))
        .map(|(i, _)| i);
    if let Some(i) = inline {
        let l = &labels[i];
        let at = l.end + 1;
        put_str(&mut main, at, &l.text);
        let k = if l.primary { 1 } else { 2 };
        for c in at..at + l.text.chars().count() {
            set_kind(&mut kinds, c, k);
        }
    }
    rows.push((main, kinds));

    // Remaining labelled underlines are stacked below, right to left.
    let mut rest: Vec<&LineLabel> = labels
        .iter()
        .enumerate()
        .filter(|(i, l)| Some(*i) != inline && !l.text.is_empty())
        .map(|(_, l)| l)
        .collect();
    rest.sort_by_key(|l| std::cmp::Reverse(l.start));

    if !rest.is_empty() {
        // Connector row: a `|` under the start of every pending label.
        let mut conn: Vec<char> = Vec::new();
        let mut ckinds: Vec<u8> = Vec::new();
        for l in &rest {
            put(&mut conn, l.start, '|');
            set_kind(&mut ckinds, l.start, if l.primary { 1 } else { 2 });
        }
        rows.push((conn, ckinds));

        for i in 0..rest.len() {
            let mut row: Vec<char> = Vec::new();
            let mut rk: Vec<u8> = Vec::new();
            for l in &rest[i + 1..] {
                put(&mut row, l.start, '|');
                set_kind(&mut rk, l.start, if l.primary { 1 } else { 2 });
            }
            let l = rest[i];
            put_str(&mut row, l.start, &l.text);
            for c in l.start..l.start + l.text.chars().count() {
                set_kind(&mut rk, c, if l.primary { 1 } else { 2 });
            }
            rows.push((row, rk));
        }
    }
    rows
}

fn paint_row(row: &[char], kinds: &[u8], level: Level, st: &Style) -> String {
    let mut out = String::new();
    let mut run = String::new();
    let mut run_kind = 0u8;
    let flush = |out: &mut String, run: &mut String, kind: u8| {
        if run.is_empty() {
            return;
        }
        match kind {
            1 => out.push_str(&st.level(level, run)),
            2 => out.push_str(&st.blue(run)),
            _ => out.push_str(run),
        }
        run.clear();
    };
    for (i, ch) in row.iter().enumerate() {
        let k = if *ch == ' ' { 0 } else { kinds.get(i).copied().unwrap_or(0) };
        if k != run_kind {
            flush(&mut out, &mut run, run_kind);
            run_kind = k;
        }
        run.push(*ch);
    }
    flush(&mut out, &mut run, run_kind);
    out.trim_end().to_string()
}

fn render_multiline(prefix: &str, first_line_prefix: &str, text: &str) -> String {
    let mut out = String::new();
    for (i, line) in text.split('\n').enumerate() {
        if i == 0 {
            out.push_str(first_line_prefix);
        } else {
            out.push_str(prefix);
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub fn render(d: &Diagnostic, sm: &SourceMap, st: &Style) -> String {
    let mut out = String::new();

    // Header: `error[E0001]: message`
    let mut head = match d.level {
        Level::Error => "error".to_string(),
        Level::Warning => "warning".to_string(),
    };
    if let Some(code) = d.code {
        head.push_str(&format!("[{}]", code));
    }
    out.push_str(&st.level(d.level, &head));
    out.push_str(&st.bold(": "));
    out.push_str(&st.bold(&d.message));
    out.push('\n');

    let labels: Vec<&Label> = d
        .labels
        .iter()
        .filter(|l| !l.span.is_dummy() && sm.get(l.span.file).is_some())
        .collect();

    // Gutter width: widest line number we are going to print.
    let mut max_line = 1;
    for l in &labels {
        let f = sm.file(l.span.file);
        max_line = max_line.max(f.line_col(l.span.start).0);
    }
    for s in &d.suggestions {
        if !s.span.is_dummy() && sm.get(s.span.file).is_some() {
            max_line = max_line.max(sm.file(s.span.file).line_col(s.span.start).0);
        }
    }
    let width = digits(max_line);
    let pad = " ".repeat(width);
    let bar = st.blue("|");

    if labels.is_empty() {
        if !d.notes.is_empty() {
            out.push_str(&format!("{} {}\n", pad, bar));
        }
    } else {
        // Files in order of appearance, the primary file first.
        let main = d.main_span();
        let mut file_order: Vec<usize> = Vec::new();
        if let Some(m) = main {
            file_order.push(m.file);
        }
        for l in &labels {
            if !file_order.contains(&l.span.file) {
                file_order.push(l.span.file);
            }
        }

        for (fi, &file_id) in file_order.iter().enumerate() {
            let file = sm.file(file_id);
            let in_file: Vec<&&Label> = labels.iter().filter(|l| l.span.file == file_id).collect();

            // Location line.
            let loc_span = if fi == 0 {
                main.unwrap()
            } else {
                in_file.iter().find(|l| l.primary).unwrap_or(&in_file[0]).span
            };
            let (ll, lc) = file.line_col(loc_span.start);
            let arrow = if fi == 0 { "-->" } else { ":::" };
            out.push_str(&format!("{}{} {}:{}:{}\n", pad, st.blue(arrow), file.name, ll, lc));
            out.push_str(&format!("{} {}\n", pad, bar));

            // Group labels per line.
            let mut line_ids: Vec<usize> = in_file.iter().map(|l| file.line_index(l.span.start)).collect();
            line_ids.sort();
            line_ids.dedup();

            let mut prev_line: Option<usize> = None;
            for &li in &line_ids {
                if let Some(p) = prev_line {
                    if li == p + 2 {
                        out.push_str(&render_source_line(file, p + 1, width, st));
                    } else if li > p + 2 {
                        out.push_str(&format!("{}\n", st.blue("...")));
                    }
                }
                prev_line = Some(li);

                let (ls, le) = file.line_range(li);
                let (text, cols) = expand_line(&file.chars[ls..le]);
                out.push_str(&format!("{} {} {}\n", st.blue(&format!("{:>w$}", li + 1, w = width)), bar, text));

                let mut line_labels: Vec<LineLabel> = Vec::new();
                for l in in_file.iter().filter(|l| file.line_index(l.span.start) == li) {
                    let s = l.span.start.clamp(ls, le);
                    let e = l.span.end.clamp(s, le);
                    let ds = cols[s - ls];
                    let mut de = cols[e - ls];
                    if de <= ds {
                        de = ds + 1;
                    }
                    line_labels.push(LineLabel { start: ds, end: de, text: l.text.clone(), primary: l.primary });
                }
                for (row, kinds) in underline_rows(&line_labels) {
                    let body = paint_row(&row, &kinds, d.level, st);
                    out.push_str(&format!("{} {} {}\n", pad, bar, body));
                }
            }
        }
        if !d.notes.is_empty() {
            out.push_str(&format!("{} {}\n", pad, bar));
        }
    }

    // Notes and help messages.
    for n in &d.notes {
        let kind = match n.kind {
            NoteKind::Note => "note",
            NoteKind::Help => "help",
        };
        let prefix_plain = format!("{} = {}: ", pad, kind);
        let continuation = " ".repeat(prefix_plain.chars().count());
        let first = format!("{} {} {}{} ", pad, st.blue("="), st.note_kind(n.kind, kind), st.bold(":"));
        out.push_str(&render_multiline(&continuation, &first, &n.text));
    }

    // Suggestions.
    for s in &d.suggestions {
        out.push_str(&render_suggestion(s, sm, width, st));
    }

    out
}

fn render_source_line(file: &SourceFile, li: usize, width: usize, st: &Style) -> String {
    let (ls, le) = file.line_range(li);
    let (text, _) = expand_line(&file.chars[ls..le]);
    let text = text.trim_end().to_string();
    format!("{} {} {}\n", st.blue(&format!("{:>w$}", li + 1, w = width)), st.blue("|"), text)
}

fn render_suggestion(s: &Suggestion, sm: &SourceMap, width: usize, st: &Style) -> String {
    let mut out = String::new();
    let pad = " ".repeat(width);
    let bar = st.blue("|");
    out.push_str(&format!("{}{} {}\n", st.note_kind(NoteKind::Help, "help"), st.bold(":"), st.bold(&s.msg)));

    let file = match sm.get(s.span.file) {
        Some(f) if !s.span.is_dummy() => f,
        _ => return out,
    };
    let li = file.line_index(s.span.start);
    let (ls, le) = file.line_range(li);
    if s.span.end > le {
        // Multi-line replacements are not rendered as a snippet.
        return out;
    }
    let before: Vec<char> = file.chars[ls..s.span.start].to_vec();
    let after: Vec<char> = file.chars[s.span.end..le].to_vec();
    let repl: Vec<char> = s.replacement.chars().collect();

    let mut new_line: Vec<char> = Vec::new();
    new_line.extend(&before);
    new_line.extend(&repl);
    new_line.extend(&after);

    let (text, _) = expand_line(&new_line);
    let (_, before_cols) = expand_line(&before);
    let start_col = *before_cols.last().unwrap_or(&0);
    let (_, repl_cols) = expand_line(&repl);
    let repl_width = *repl_cols.last().unwrap_or(&0);

    out.push_str(&format!("{} {}\n", pad, bar));
    out.push_str(&format!("{} {} {}\n", st.blue(&format!("{:>w$}", li + 1, w = width)), bar, text));
    let mark = if s.span.is_empty_span() { '+' } else { '~' };
    let marks: String = std::iter::repeat(mark).take(repl_width.max(1)).collect();
    out.push_str(&format!("{} {} {}{}\n", pad, bar, " ".repeat(start_col), st.green(&marks)));
    out
}

impl Span {
    fn is_empty_span(&self) -> bool {
        self.start == self.end
    }
}

// ---------------------------------------------------------------------------
// Emitter
// ---------------------------------------------------------------------------

pub struct Emitter {
    style: Style,
    pub errors: usize,
    pub warnings: usize,
}

impl Emitter {
    pub fn new(choice: ColorChoice) -> Emitter {
        Emitter { style: Style::new(choice), errors: 0, warnings: 0 }
    }

    /// Prints a diagnostic to stderr (warnings located in "external" files are
    /// dropped).
    pub fn emit(&mut self, d: &Diagnostic, sm: &SourceMap) {
        if d.level == Level::Warning {
            if let Some(span) = d.main_span() {
                if sm.get(span.file).map(|f| f.external).unwrap_or(false) {
                    return;
                }
            }
        }
        match d.level {
            Level::Error => self.errors += 1,
            Level::Warning => self.warnings += 1,
        }
        eprintln!("{}", render(d, sm, &self.style));
    }

    pub fn emit_all(&mut self, diags: &[Diagnostic], sm: &SourceMap) {
        for d in diags {
            self.emit(d, sm);
        }
    }

    /// Prints the closing summary line (`error: aborting due to ...`).
    pub fn finish(&self) {
        let plural = |n: usize, word: &str| if n == 1 { format!("{} {}", n, word) } else { format!("{} {}s", n, word) };
        if self.errors > 0 {
            let mut msg = format!("aborting due to {}", if self.errors == 1 { "1 previous error".to_string() } else { format!("{} previous errors", self.errors) });
            if self.warnings > 0 {
                msg.push_str(&format!("; {} emitted", plural(self.warnings, "warning")));
            }
            eprintln!("{}{} {}", self.style.level(Level::Error, "error"), self.style.bold(":"), self.style.bold(&msg));
        } else if self.warnings > 0 {
            eprintln!(
                "{}{} {}",
                self.style.level(Level::Warning, "warning"),
                self.style.bold(":"),
                self.style.bold(&format!("{} emitted", plural(self.warnings, "warning")))
            );
        }
    }

    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }
}
