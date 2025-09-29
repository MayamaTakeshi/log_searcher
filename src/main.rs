use rouille::{Response, ResponseBody, router};
use std::io::{self, BufReader, BufRead, Write};
use std::fs::{File, read_dir};
use std::path::Path;
use serde::Deserialize;
use chrono::{Local, NaiveDate, NaiveTime, NaiveDateTime};
use flate2::read::GzDecoder;
use zip::read::ZipArchive;
use std::thread;
use os_pipe;

mod file_selector;

#[derive(Deserialize, Debug)]
struct SearchRequest {
    start_stamp: String,
    end_stamp: String,
    search_string: String,
    folder: String,
}

// Parse timestamp from line
/*
fn parse_timestamp(s: &str) -> Option<NaiveDateTime> {
    let formats = ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y/%m/%d %H:%M:%S"];
    for fmt in &formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(dt);
        }
    }
    None
}
*/

fn parse_timestamp(s: &str) -> Option<NaiveDateTime> {
    if s.len() < 19 { return None; }

    let y: i32 = s[0..4].parse().ok()?;
    let m: u32 = s[5..7].parse().ok()?;
    let d: u32 = s[8..10].parse().ok()?;
    let h: u32 = s[11..13].parse().ok()?;
    let min: u32 = s[14..16].parse().ok()?;
    let sec: u32 = s[17..19].parse().ok()?;

    let date = NaiveDate::from_ymd_opt(y, m, d)?;
    let time = NaiveTime::from_hms_opt(h, min, sec)?;
    Some(NaiveDateTime::new(date, time))
}

// Stream file line by line
fn search_file(path: &Path, start: NaiveDateTime, end: NaiveDateTime, search: &str, results: &mut impl Write) -> io::Result<()> {
    let file = File::open(path)?;
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension {
        "gz" => {
            let decoder = GzDecoder::new(file);
            let reader = BufReader::new(decoder);
            for line in reader.lines() {
                let line = line?;
                if line.len() >= 19 {
                    if let Some(ts) = parse_timestamp(&line[..19]) {
                        if ts > end {
                            break; // Stop processing if the timestamp is after the end time
                        }
                        if ts >= start && line.contains(search) {
                            writeln!(results, "{}", line)?;
                        }
                    }
                }
            }
        }
        "zip" => {
            let mut archive = ZipArchive::new(file)?;
            for i in 0..archive.len() {
                let mut zfile = archive.by_index(i)?;
                let reader = BufReader::new(&mut zfile);
                for line in reader.lines() {
                    let line = line?;
                    if line.len() >= 19 {
                        if let Some(ts) = parse_timestamp(&line[..19]) {
                            if ts > end {
                                break; // Stop processing if the timestamp is after the end time
                            }
                            if ts >= start && line.contains(search) {
                                writeln!(results, "{}", line)?;
                            }
                        }
                    }
                }
            }
        }
        _ => {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if line.len() >= 19 {
                    if let Some(ts) = parse_timestamp(&line[..19]) {
                        if ts > end {
                            break; // Stop processing if the timestamp is after the end time
                        }
                        if ts >= start && line.contains(search) {
                            writeln!(results, "{}", line)?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() {
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
                thread::spawn(move || {
                    let result = (|| -> io::Result<()> {
                        let mut entries: Vec<_> = read_dir(folder_path)?.filter_map(Result::ok).collect();
                        entries.sort_by_key(|e| e.metadata().map(|m| m.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)).unwrap_or(std::time::SystemTime::UNIX_EPOCH));
                        println!("{} - Processing {} files: {:?}", 
                            Local::now().format("%Y-%m-%d %H:%M:%S"), 
                            entries.len(), 
                            entries.iter().map(|e| e.path()).collect::<Vec<_>>());

                        for entry in entries {
                            let path = entry.path();
                            if path.is_file() {
                                println!("{} - Start processing file: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), path.display());
                                if let Err(e) = search_file(&path, start, end, &search_string, &mut writer) {
                                    writeln!(writer, "X-LOG-SEARCHER-ERROR: Failed to process file {}: {}", path.display(), e)?;
                                }
                                println!("{} - End processing file: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), path.display());
                            }
                        }
                        Ok(())
                    })();

                    if let Err(e) = result {
                        let _ = writeln!(writer, "X-LOG-SEARCHER-ERROR: {}", e);
                    }
                });

                Response {
                    status_code: 200,
                    headers: vec![("Content-Type".into(), "text/plain".into())],
                    data: ResponseBody::from_reader(reader),
                    upgrade: None,
                }
            },

            _ => Response::empty_404()
        )
    });
}
