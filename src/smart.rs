use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OcrKind {
    Text,
    Code,
    Table,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OcrMode {
    Auto,
    Text,
    Code,
    Table,
}

pub fn parse_mode(value: &str) -> Result<OcrMode, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "auto" | "" => Ok(OcrMode::Auto),
        "text" => Ok(OcrMode::Text),
        "code" => Ok(OcrMode::Code),
        "table" => Ok(OcrMode::Table),
        _ => Err("PARKER_OCR_MODE must be auto, text, code, or table.".to_string()),
    }
}

#[derive(Clone, Debug)]
struct Word {
    block: i32,
    paragraph: i32,
    line: i32,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    text: String,
}

#[derive(Clone, Debug)]
struct Cell {
    left: i32,
    text: String,
}

#[derive(Clone, Debug)]
struct Row {
    top: i32,
    height: i32,
    cells: Vec<Cell>,
}

pub fn clean_text(text: &str) -> String {
    let normalized = text
        .trim_matches('\u{feff}')
        .replace('\u{000c}', "")
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut lines: Vec<&str> = normalized.lines().collect();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

pub fn normalize_code(text: &str) -> String {
    let mut lines: Vec<String> = text
        .replace('\r', "")
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect();

    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

pub fn looks_like_code(text: &str) -> bool {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.len() < 2 {
        return false;
    }

    let lower = text.to_ascii_lowercase();
    let mut score = 0i32;
    let strong_tokens = [
        "=>",
        "::",
        "#include",
        "</",
        "function ",
        "const ",
        "let ",
        "var ",
        "def ",
        "class ",
        "import ",
        "from ",
        "return ",
        "fn ",
        "use ",
        "public ",
        "private ",
        "SELECT ",
        "FROM ",
        "WHERE ",
    ];

    for token in strong_tokens {
        let present = if token.chars().any(char::is_uppercase) {
            text.contains(token)
        } else {
            lower.contains(token)
        };
        if present {
            score += 2;
        }
    }

    score += lines
        .iter()
        .filter(|line| line.starts_with(' ') || line.starts_with('\t'))
        .count() as i32;
    score += lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.ends_with(';')
                || trimmed.ends_with('{')
                || trimmed.ends_with('}')
                || trimmed.starts_with("//")
                || trimmed.starts_with('#')
        })
        .count() as i32;

    let punctuation = text
        .chars()
        .filter(|character| "{}[]();=<>:&|!".contains(*character))
        .count();
    let non_space = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .count();
    if non_space > 0 && punctuation * 100 / non_space >= 8 {
        score += 3;
    }

    score >= 5
}

pub fn reconstruct_text_from_tsv(tsv: &str) -> String {
    let words = parse_tsv(tsv);
    if words.is_empty() {
        return String::new();
    }

    let mut by_line: BTreeMap<(i32, i32, i32), Vec<Word>> = BTreeMap::new();
    for word in words {
        by_line
            .entry((word.block, word.paragraph, word.line))
            .or_default()
            .push(word);
    }

    let mut lines = Vec::new();
    let global_left = by_line
        .values()
        .flat_map(|line| line.iter().map(|word| word.left))
        .min()
        .unwrap_or(0);
    let mut widths = by_line
        .values()
        .flat_map(|line| line.iter())
        .filter_map(|word| {
            let count = word.text.chars().count() as i32;
            (count > 0).then_some((word.width / count).max(1))
        })
        .collect::<Vec<i32>>();
    let median_character_width = median_i32(&mut widths).max(4);

    for (_, mut words) in by_line {
        words.sort_by_key(|word| word.left);
        let first_left = words.first().map(|word| word.left).unwrap_or(global_left);
        let indent_columns = ((first_left - global_left).max(0) / median_character_width).min(24);
        let mut line = " ".repeat(indent_columns as usize);
        let mut previous_right: Option<i32> = None;

        for word in words {
            if let Some(right) = previous_right {
                let gap = word.left - right;
                let spaces = (gap / median_character_width).clamp(1, 8);
                line.push_str(&" ".repeat(spaces as usize));
            }
            line.push_str(&word.text);
            previous_right = Some(word.left + word.width);
        }
        lines.push(line.trim_end().to_string());
    }

    lines.join("\n")
}

pub fn extract_table(tsv: &str) -> Option<String> {
    let words = parse_tsv(tsv);
    if words.len() < 4 {
        return None;
    }

    let mut by_line: BTreeMap<(i32, i32, i32), Vec<Word>> = BTreeMap::new();
    for word in words {
        by_line
            .entry((word.block, word.paragraph, word.line))
            .or_default()
            .push(word);
    }

    let mut rows = Vec::new();
    for (_, mut words) in by_line {
        words.sort_by_key(|word| word.left);
        if let Some(row) = split_row(words) {
            rows.push(row);
        }
    }
    rows.sort_by_key(|row| row.top);

    if rows.len() < 2 {
        return None;
    }

    let mut frequencies: HashMap<usize, usize> = HashMap::new();
    for row in &rows {
        if (2..=12).contains(&row.cells.len()) {
            *frequencies.entry(row.cells.len()).or_default() += 1;
        }
    }
    let (&column_count, &matching_rows) = frequencies
        .iter()
        .max_by_key(|(columns, count)| (**count, std::cmp::Reverse(**columns)))?;

    if matching_rows < 2 || matching_rows * 10 < rows.len() * 6 {
        return None;
    }

    let rows: Vec<Row> = rows
        .into_iter()
        .filter(|row| row.cells.len() == column_count)
        .collect();
    if rows.len() < 2 {
        return None;
    }

    let mut heights = rows.iter().map(|row| row.height).collect::<Vec<i32>>();
    let median_height = median_i32(&mut heights).max(1);
    let tolerance = (median_height * 2).max(24);

    // Bolt: Reuse a single buffer for median calculations across columns
    // to avoid allocating a new Vec per column.
    let mut starts_buffer = Vec::with_capacity(rows.len());

    for column in 0..column_count {
        starts_buffer.clear();
        starts_buffer.extend(rows.iter().map(|row| row.cells[column].left));
        let median = median_i32(&mut starts_buffer);
        let aligned = starts_buffer
            .iter()
            .filter(|start| (**start - median).abs() <= tolerance)
            .count();
        if aligned * 10 < rows.len() * 7 {
            return None;
        }
    }

    let output = rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .map(|cell| cell.text.replace(['\t', '\r', '\n'], " "))
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n");

    if output.trim().is_empty() {
        None
    } else {
        Some(output)
    }
}

fn split_row(words: Vec<Word>) -> Option<Row> {
    if words.len() < 2 {
        return None;
    }

    let top = words.iter().map(|word| word.top).min()?;
    let mut heights = words.iter().map(|word| word.height).collect::<Vec<i32>>();
    let height = median_i32(&mut heights).max(1);
    let mut character_widths: Vec<i32> = words
        .iter()
        .filter_map(|word| {
            let count = word.text.chars().count() as i32;
            (count > 0).then_some((word.width / count).max(1))
        })
        .collect();
    let character_width = median_i32(&mut character_widths).max(4);
    let gap_threshold = (character_width * 3).max(height).max(14);

    let mut cells = Vec::new();

    // Bolt: Use into_iter() to take ownership of strings directly
    // rather than calling clone() on each word in the loop.
    let mut words_iter = words.into_iter();
    let first = words_iter.next().unwrap(); // safe because words.len() >= 2
    let mut current_left = first.left;
    let mut current_text = first.text;
    let mut previous_right = first.left + first.width;

    for word in words_iter {
        let gap = word.left - previous_right;
        if gap > gap_threshold {
            cells.push(Cell {
                left: current_left,
                text: current_text.trim().to_string(),
            });
            current_left = word.left;
            current_text = word.text;
        } else {
            if !current_text.is_empty() {
                current_text.push(' ');
            }
            current_text.push_str(&word.text);
        }
        previous_right = word.left + word.width;
    }

    cells.push(Cell {
        left: current_left,
        text: current_text.trim().to_string(),
    });
    cells.retain(|cell| !cell.text.is_empty());

    if cells.len() >= 2 {
        Some(Row { top, height, cells })
    } else {
        None
    }
}

fn parse_tsv(tsv: &str) -> Vec<Word> {
    tsv.lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.splitn(12, '\t').collect();
            if fields.len() != 12 || fields[0] != "5" {
                return None;
            }

            let text = fields[11].trim().replace('\t', " ");
            if text.is_empty() {
                return None;
            }

            Some(Word {
                block: fields[2].parse().ok()?,
                paragraph: fields[3].parse().ok()?,
                line: fields[4].parse().ok()?,
                left: fields[6].parse().ok()?,
                top: fields[7].parse().ok()?,
                width: fields[8].parse().ok()?,
                height: fields[9].parse().ok()?,
                text: text.to_string(),
            })
        })
        .collect()
}

// Bolt: Accept a mutable slice instead of Vec to sort in-place
// and prevent unnecessary clone() allocations at the call site.
fn median_i32(values: &mut [i32]) -> i32 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[values.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::{clean_text, extract_table, looks_like_code, normalize_code, parse_mode};

    #[test]
    fn detects_common_code_shapes() {
        assert!(looks_like_code(
            "fn main() {\n    let value = 42;\n    println!(\"{}\", value);\n}"
        ));
        assert!(!looks_like_code(
            "This is a normal paragraph.\nIt contains several ordinary sentences."
        ));
    }

    #[test]
    fn extracts_aligned_table_as_tsv() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
5\t1\t1\t1\t1\t1\t10\t10\t40\t20\t95\tName\n\
5\t1\t1\t1\t1\t2\t180\t10\t30\t20\t95\tAge\n\
5\t1\t1\t1\t2\t1\t10\t50\t50\t20\t95\tNomu\n\
5\t1\t1\t1\t2\t2\t180\t50\t20\t20\t95\t20\n";
        assert_eq!(extract_table(tsv).as_deref(), Some("Name\tAge\nNomu\t20"));
    }

    #[test]
    fn rejects_unaligned_tables() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
5\t1\t1\t1\t1\t1\t10\t10\t40\t20\t95\talpha\n\
5\t1\t1\t1\t1\t2\t900\t10\t30\t20\t95\tbeta\n\
5\t1\t1\t1\t2\t1\t400\t50\t50\t20\t95\tyear\n\
5\t1\t1\t1\t2\t2\t120\t50\t20\t20\t95\t1994\n";
        assert_eq!(extract_table(tsv), None);
    }

    #[test]
    fn cleans_page_artifacts() {
        assert_eq!(
            clean_text("\u{feff}\n\r\nhello\r\nworld\u{c}\n"),
            "hello\nworld"
        );
    }

    #[test]
    fn normalizes_code_indentation() {
        assert_eq!(normalize_code("\n  fn a() {}\n\n"), "  fn a() {}");
    }

    #[test]
    fn parses_modes() {
        assert!(parse_mode("AUTO").is_ok());
        assert!(parse_mode("bogus").is_err());
        assert_eq!(parse_mode("").unwrap(), super::OcrMode::Auto);
    }
}
