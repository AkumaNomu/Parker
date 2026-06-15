use crate::win::{BELOW_NORMAL_PRIORITY_CLASS, CREATE_NO_WINDOW};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct OcrCapturePath {
    pub path: PathBuf,
    pub temporary: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OcrKind {
    Text,
    Code,
    Table,
}

pub struct OcrResult {
    pub kind: OcrKind,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OcrMode {
    Auto,
    Text,
    Code,
    Table,
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

pub fn create_capture_path() -> Result<OcrCapturePath, String> {
    let keep_capture = env_flag("PARKER_KEEP_OCR_CAPTURE");
    let directory = if keep_capture {
        if let Some(profile) = env::var_os("USERPROFILE") {
            PathBuf::from(profile).join("Pictures").join("Parker")
        } else {
            env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("Parker")
        }
    } else {
        env::temp_dir().join("Parker")
    };

    fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "Could not create the OCR capture directory {}: {error}",
            directory.display()
        )
    })?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("The system clock is invalid: {error}"))?
        .as_millis();

    Ok(OcrCapturePath {
        path: directory.join(format!("parker-capture-{timestamp}.bmp")),
        temporary: !keep_capture,
    })
}

pub fn recognize_smart(path: &Path) -> Result<OcrResult, String> {
    let mode = configured_mode()?;

    match mode {
        OcrMode::Text => {
            let plain = clean_text(&run_tesseract(path, false)?);
            if plain.is_empty() {
                return Err("No text was detected in the selected region.".to_string());
            }
            Ok(OcrResult {
                kind: OcrKind::Text,
                text: plain,
            })
        }
        OcrMode::Code => {
            let plain = clean_text(&run_tesseract(path, false)?);
            if plain.is_empty() {
                return Err("No code text was detected in the selected region.".to_string());
            }
            Ok(OcrResult {
                kind: OcrKind::Code,
                text: normalize_code(&plain),
            })
        }
        OcrMode::Table => {
            let tsv = run_tesseract(path, true)?;
            extract_table(&tsv)
                .map(|text| OcrResult {
                    kind: OcrKind::Table,
                    text,
                })
                .ok_or_else(|| {
                    "The selected region did not contain a consistently aligned table.".to_string()
                })
        }
        OcrMode::Auto => {
            // Automatic mode uses a single Tesseract process. TSV provides both
            // word geometry for table inference and enough layout data to
            // rebuild normal/code text without running OCR twice.
            let tsv = run_tesseract(path, true)?;
            if let Some(table) = extract_table(&tsv) {
                return Ok(OcrResult {
                    kind: OcrKind::Table,
                    text: table,
                });
            }

            let plain = clean_text(&reconstruct_text_from_tsv(&tsv));
            if plain.is_empty() {
                return Err(
                    "No text, code, table, or QR code was detected in the selected region."
                        .to_string(),
                );
            }

            if looks_like_code(&plain) {
                Ok(OcrResult {
                    kind: OcrKind::Code,
                    text: normalize_code(&plain),
                })
            } else {
                Ok(OcrResult {
                    kind: OcrKind::Text,
                    text: plain,
                })
            }
        }
    }
}

fn run_tesseract(path: &Path, tsv: bool) -> Result<String, String> {
    let tesseract = locate_tesseract().ok_or_else(|| {
        "Tesseract OCR was not found. Run install.ps1, install Tesseract with winget, or set PARKER_TESSERACT to tesseract.exe."
            .to_string()
    })?;
    let language = env::var("PARKER_OCR_LANG").unwrap_or_else(|_| "eng".to_string());
    let psm = configured_psm()?;

    let mut command = Command::new(&tesseract);
    command
        .creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS)
        .arg(path)
        .arg("stdout")
        .arg("-l")
        .arg(language)
        .arg("--psm")
        .arg(psm.to_string())
        .arg("-c")
        .arg("preserve_interword_spaces=1");

    if tsv {
        command.arg("tsv");
    }

    let output = command.output().map_err(|error| {
        format!(
            "Could not start Tesseract at {}: {error}",
            tesseract.display()
        )
    })?;

    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if details.is_empty() {
            format!("Tesseract exited with {}.", output.status)
        } else {
            format!("Tesseract failed: {details}")
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn configured_mode() -> Result<OcrMode, String> {
    let value = env::var("PARKER_OCR_MODE").unwrap_or_else(|_| "auto".to_string());
    match value.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(OcrMode::Auto),
        "text" => Ok(OcrMode::Text),
        "code" => Ok(OcrMode::Code),
        "table" => Ok(OcrMode::Table),
        _ => Err("PARKER_OCR_MODE must be auto, text, code, or table.".to_string()),
    }
}

fn configured_psm() -> Result<u8, String> {
    let value = env::var("PARKER_OCR_PSM").unwrap_or_else(|_| "6".to_string());
    let psm = value
        .parse::<u8>()
        .map_err(|_| "PARKER_OCR_PSM must be an integer between 0 and 13.".to_string())?;
    if psm > 13 {
        Err("PARKER_OCR_PSM must be an integer between 0 and 13.".to_string())
    } else {
        Ok(psm)
    }
}

fn clean_text(text: &str) -> String {
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

fn normalize_code(text: &str) -> String {
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

fn looks_like_code(text: &str) -> bool {
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

fn reconstruct_text_from_tsv(tsv: &str) -> String {
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
    let median_character_width = median_i32(
        by_line
            .values()
            .flat_map(|line| line.iter())
            .filter_map(|word| {
                let count = word.text.chars().count() as i32;
                (count > 0).then_some((word.width / count).max(1))
            })
            .collect(),
    )
    .max(4);

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

    lines.join(
        "
",
    )
}

fn extract_table(tsv: &str) -> Option<String> {
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

    let median_height = median_i32(rows.iter().map(|row| row.height).collect()).max(1);
    let tolerance = (median_height * 2).max(24);
    for column in 0..column_count {
        let starts: Vec<i32> = rows.iter().map(|row| row.cells[column].left).collect();
        let median = median_i32(starts.clone());
        let aligned = starts
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
    let height = median_i32(words.iter().map(|word| word.height).collect()).max(1);
    let character_widths: Vec<i32> = words
        .iter()
        .filter_map(|word| {
            let count = word.text.chars().count() as i32;
            (count > 0).then_some((word.width / count).max(1))
        })
        .collect();
    let character_width = median_i32(character_widths).max(4);
    let gap_threshold = (character_width * 3).max(height).max(14);

    let mut cells = Vec::new();
    let mut current_left = words[0].left;
    let mut current_text = words[0].text.clone();
    let mut previous_right = words[0].left + words[0].width;

    for word in words.into_iter().skip(1) {
        let gap = word.left - previous_right;
        if gap > gap_threshold {
            cells.push(Cell {
                left: current_left,
                text: current_text.trim().to_string(),
            });
            current_left = word.left;
            current_text = word.text.clone();
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

            let text = fields[11].trim();
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

fn median_i32(mut values: Vec<i32>) -> i32 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[values.len() / 2]
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn locate_tesseract() -> Option<PathBuf> {
    if let Some(path) = env::var_os("PARKER_TESSERACT") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(executable) = env::current_exe() {
        if let Some(parent) = executable.parent() {
            let bundled = parent.join("tesseract.exe");
            if bundled.is_file() {
                return Some(bundled);
            }
        }
    }

    let mut candidates = Vec::new();
    for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = env::var_os(variable) {
            candidates.push(
                PathBuf::from(root)
                    .join("Tesseract-OCR")
                    .join("tesseract.exe"),
            );
        }
    }
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        candidates.push(
            local
                .join("Programs")
                .join("Tesseract-OCR")
                .join("tesseract.exe"),
        );
        candidates.push(
            local
                .join("Microsoft")
                .join("WinGet")
                .join("Links")
                .join("tesseract.exe"),
        );
    }

    if let Some(found) = candidates.into_iter().find(|path| path.is_file()) {
        return Some(found);
    }

    let output = Command::new("where.exe")
        .creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS)
        .arg("tesseract.exe")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::{extract_table, looks_like_code};

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
}
