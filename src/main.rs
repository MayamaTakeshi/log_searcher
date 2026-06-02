use rouille::{Response, ResponseBody, router};
use std::io::{self, BufReader, BufRead, Write};
use std::fs::{File, read_dir};
use std::path::{Path, PathBuf};
use serde::Deserialize;
use chrono::{Local, NaiveDate, NaiveTime, NaiveDateTime};
use flate2::read::GzDecoder;
use zip::read::ZipArchive;
use std::thread;
use os_pipe;
use regex::Regex;

mod file_selector;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize, Debug)]
struct SearchRequest {
    start_stamp: String,
    end_stamp: String,
    search_string: Option<String>,
    folder: String,
    file_name_regex: Option<String>,
}

pub(crate) fn parse_timestamp(s: &str) -> Option<NaiveDateTime> {
    if s.len() < 19 { return None; }

    let y: i32 = s[0..4].parse().ok()?;
    if y < 2000 || y >= 2100 {
        return None;
    }
    let m: u32 = s[5..7].parse().ok()?;
    let d: u32 = s[8..10].parse().ok()?;
    let h: u32 = s[11..13].parse().ok()?;
    let min: u32 = s[14..16].parse().ok()?;
    let sec: u32 = s[17..19].parse().ok()?;

    let date = NaiveDate::from_ymd_opt(y, m, d)?;
    let time = NaiveTime::from_hms_opt(h, min, sec)?;
    Some(NaiveDateTime::new(date, time))
}

pub(crate) fn search_file(path: &Path, start: NaiveDateTime, end: NaiveDateTime, search: &Option<String>, results: &mut impl Write) -> io::Result<()> {
    let file = File::open(path)?;
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    fn process_lines(reader: impl BufRead, start: NaiveDateTime, end: NaiveDateTime, search: &Option<String>, results: &mut impl Write) -> io::Result<()> {
        let mut capture = false;
        let search_term = search.as_deref().unwrap_or("");
        let perform_search = !search_term.is_empty();

        for line in reader.lines() {
            let line = line?;
            if let Some(ts) = parse_timestamp(&line) {
                if ts > end {
                    break;
                }
                capture = ts >= start && (!perform_search || line.contains(search_term));
                if capture {
                    writeln!(results, "{}", line)?;
                }
            } else if capture {
                writeln!(results, "{}", line)?;
            }
        }
        Ok(())
    }

    match extension {
        "gz" => {
            let decoder = GzDecoder::new(file);
            let reader = BufReader::new(decoder);
            process_lines(reader, start, end, search, results)?;
        }
        "zip" => {
            let mut archive = ZipArchive::new(file)?;
            for i in 0..archive.len() {
                let mut zfile = archive.by_index(i)?;
                let reader = BufReader::new(&mut zfile);
                process_lines(reader, start, end, search, results)?;
            }
        }
        _ => {
            let reader = BufReader::new(file);
            process_lines(reader, start, end, search, results)?;
        }
    }
    Ok(())
}

fn main() {
    if std::env::args().any(|a| a == "-v") {
        println!("{}", VERSION);
        return;
    }
    println!("Server running on http://0.0.0.0:8078");
    rouille::start_server("0.0.0.0:8078", move |request| {
        router!(request,            
            (POST) (/search_logs) => {
                let req: SearchRequest = match rouille::input::json_input(request) {
                    Ok(r) => r,
                    Err(_) => {
                        return Response::text("Invalid JSON").with_status_code(400);
                    }
                };

                println!("{} - {} {} from {} - Body: {:?}", 
                    Local::now().format("%Y-%m-%d %H:%M:%S"), 
                    request.method(), 
                    request.url(), 
                    request.remote_addr(), 
                    &req);

                let start = match parse_timestamp(&req.start_stamp) {
                    Some(t) => t,
                    None => {
                        return Response::text("Invalid start_stamp").with_status_code(400);
                    }
                };

                let end = match parse_timestamp(&req.end_stamp) {
                    Some(t) => t,
                    None => {
                        return Response::text("Invalid end_stamp").with_status_code(400);
                    }
                };

                let folder_path = Path::new(&req.folder).to_path_buf();
                if !folder_path.is_dir() {
                    return Response::text("Folder does not exist").with_status_code(400);
                }

                let (reader, mut writer) = os_pipe::pipe().expect("Failed to create pipe");

                let search_string = req.search_string.clone();
                let file_name_regex = req.file_name_regex.clone();

                thread::spawn(move || {
                    let result = (|| -> io::Result<()> {
                        let file_name_re = match file_name_regex {
                            Some(s) => match Regex::new(&s) {
                                Ok(re) => Some(re),
                                Err(e) => {
                                    let _ = writeln!(writer, "X-LOG-SEARCHER-ERROR: Invalid regex: {}", e);
                                    return Ok(());
                                }
                            },
                            None => None,
                        };

                        let mut entries: Vec<_> = read_dir(&folder_path)?.filter_map(Result::ok).collect();
                        entries.sort_by_key(|e| e.metadata().map(|m| m.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)).unwrap_or(std::time::SystemTime::UNIX_EPOCH));
                        
                        let mut files_to_process = Vec::new();

                        if let Some(re) = &file_name_re {
                            let mut file_entries = Vec::new();
                            for entry in entries {
                                if let Ok(metadata) = entry.metadata() {
                                    if metadata.is_file() {
                                        if let Some(file_name) = entry.path().file_name().and_then(|n| n.to_str()) {
                                            let mtime = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                                                .duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs();
                                            file_entries.push(file_selector::FileEntry { name: file_name.to_string(), mtime });
                                        }
                                    }
                                }
                            }

                            let candidate_files = file_selector::select_candidate_files(&file_entries, start.and_utc().timestamp() as u64, end.and_utc().timestamp() as u64);

                            for file_name in candidate_files {
                                if re.is_match(&file_name) {
                                    files_to_process.push(PathBuf::from(&folder_path).join(file_name));
                                }
                            }
                        } else {
                            // If no file_name_regex, process all files in the folder, subject to time-based filtering within search_file
                            for entry in entries {
                                if entry.path().is_file() {
                                    files_to_process.push(entry.path());
                                }
                            }
                        }

                        println!("{} - Processing {} files: {:?}", 
                            Local::now().format("%Y-%m-%d %H:%M:%S"), 
                            files_to_process.len(), 
                            &files_to_process);

                        for path in files_to_process {
                            println!("{} - Start processing file: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), path.display());
                            if let Err(e) = search_file(&path, start, end, &search_string, &mut writer) {
                                writeln!(writer, "X-LOG-SEARCHER-ERROR: Failed to process file {}: {}", path.display(), e)?;
                            }
                            println!("{} - End processing file: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), path.display());
                        }
                        Ok(())
                    })();

                    if let Err(e) = result {
                        let _ = writeln!(writer, "X-LOG-SEARCHER-ERROR: {}", e);
                    }
                });

                Response {
                    status_code: 200,
                    headers: vec![("Content-Type".into(), "text/plain; charset=utf-8".into())],
                    data: ResponseBody::from_reader(reader),
                    upgrade: None,
                }
            },

            _ => Response::empty_404()
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use zip::write::{ZipWriter, FileOptions};

    fn ts(s: &str) -> NaiveDateTime {
        parse_timestamp(s).unwrap()
    }

    fn search(path: &Path, start: &str, end: &str, search: Option<&str>) -> String {
        let mut out = Vec::new();
        search_file(path, ts(start), ts(end), &search.map(str::to_string), &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn tmp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    // --- parse_timestamp ---

    #[test]
    fn test_parse_timestamp_space_separator() {
        assert!(parse_timestamp("2025-09-27 19:33:10 some text").is_some());
    }

    #[test]
    fn test_parse_timestamp_t_separator() {
        assert!(parse_timestamp("2025-09-16T00:29:30").is_some());
    }

    #[test]
    fn test_parse_timestamp_too_short() {
        assert!(parse_timestamp("2025-09-27").is_none());
    }

    #[test]
    fn test_parse_timestamp_year_out_of_range() {
        assert!(parse_timestamp("1999-01-01 00:00:00").is_none());
        assert!(parse_timestamp("2100-01-01 00:00:00").is_none());
    }

    #[test]
    fn test_parse_timestamp_invalid_date() {
        assert!(parse_timestamp("2025-13-01 00:00:00").is_none());
    }

    // --- search_file (plain text) ---

    #[test]
    fn test_plain_basic_range() {
        let path = tmp_path("ls_test_plain.log");
        fs::write(&path, "\
2025-01-01 00:00:00 before\n\
2025-01-01 01:00:00 match1\n\
2025-01-01 02:00:00 match2\n\
2025-01-01 03:00:00 after\n").unwrap();

        let out = search(&path, "2025-01-01 01:00:00", "2025-01-01 02:00:00", None);
        assert_eq!(out, "2025-01-01 01:00:00 match1\n2025-01-01 02:00:00 match2\n");
    }

    #[test]
    fn test_plain_continuation_lines() {
        let path = tmp_path("ls_test_continuation.log");
        fs::write(&path, "\
2025-01-01 01:00:00 error occurred\n\
at line 42\n\
at line 99\n\
2025-01-01 02:00:00 next entry\n").unwrap();

        let out = search(&path, "2025-01-01 01:00:00", "2025-01-01 01:30:00", None);
        assert_eq!(out, "2025-01-01 01:00:00 error occurred\nat line 42\nat line 99\n");
    }

    #[test]
    fn test_plain_search_string_filter() {
        let path = tmp_path("ls_test_search.log");
        fs::write(&path, "\
2025-01-01 01:00:00 user=alice action=login\n\
2025-01-01 01:01:00 user=bob action=login\n\
2025-01-01 01:02:00 user=alice action=logout\n").unwrap();

        let out = search(&path, "2025-01-01 01:00:00", "2025-01-01 01:02:00", Some("alice"));
        assert!(out.contains("alice"));
        assert!(!out.contains("bob"));
    }

    #[test]
    fn test_plain_no_results_outside_range() {
        let path = tmp_path("ls_test_norange.log");
        fs::write(&path, "2025-01-01 01:00:00 only line\n").unwrap();

        let out = search(&path, "2025-01-01 02:00:00", "2025-01-01 03:00:00", None);
        assert!(out.is_empty());
    }

    // --- search_file (.gz) ---

    #[test]
    fn test_gz_basic_range() {
        let path = tmp_path("ls_test.log.gz");
        let file = fs::File::create(&path).unwrap();
        let mut enc = GzEncoder::new(file, Compression::default());
        enc.write_all(b"2025-01-01 01:00:00 gz line\n2025-01-01 02:00:00 gz after\n").unwrap();
        enc.finish().unwrap();

        let out = search(&path, "2025-01-01 01:00:00", "2025-01-01 01:30:00", None);
        assert_eq!(out, "2025-01-01 01:00:00 gz line\n");
    }

    // --- search_file (.zip) ---

    #[test]
    fn test_zip_basic_range() {
        let path = tmp_path("ls_test.zip");
        let file = fs::File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        let opts = FileOptions::default();
        zip.start_file("app.log", opts).unwrap();
        zip.write_all(b"2025-01-01 01:00:00 zip line\n2025-01-01 02:00:00 zip after\n").unwrap();
        zip.finish().unwrap();

        let out = search(&path, "2025-01-01 01:00:00", "2025-01-01 01:30:00", None);
        assert_eq!(out, "2025-01-01 01:00:00 zip line\n");
    }
}
