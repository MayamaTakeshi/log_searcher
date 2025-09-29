#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub mtime: u64, // Unix timestamp
}

pub fn select_candidate_files(
    all_files: &[FileEntry],
    start: u64,
    end: u64,
) -> Vec<String> {
    let mut selected = Vec::new();

    for file in all_files {
        if file.mtime < start {
            continue;
        }
        if file.mtime > end {
            selected.push(file.name.clone());
            break; // list is sorted, no need to check further
        }
        selected.push(file.name.clone());
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_file(name: &str, mtime: u64) -> FileEntry {
        FileEntry {
            name: name.into(),
            mtime,
        }
    }

    #[test]
    fn test_minimal_selection() {
        let all_files = vec![
            make_file("a.log", 100),
            make_file("b.log", 200),
            make_file("c.log", 300),
        ];

        let result = select_candidate_files(&all_files, 150, 250);
        assert_eq!(result, vec!["b.log", "c.log"]);
    }

    #[test]
    fn test_multiple_selection() {
        let all_files = vec![
            make_file("a.log", 100),
            make_file("b.log", 200),
            make_file("c.log", 250),
            make_file("d.log", 300),
            make_file("e.log", 350),
            make_file("f.log", 400),
        ];

        let result = select_candidate_files(&all_files, 150, 300);
        assert_eq!(result, vec!["b.log", "c.log", "d.log", "e.log"]);
    }

    #[test]
    fn test_no_selection() {
        let all_files = vec![make_file("a.log", 100), make_file("b.log", 200)];

        let result = select_candidate_files(&all_files, 300, 400);
        assert!(result.is_empty());
    }
}