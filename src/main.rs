use rouille::{Response, ResponseBody, router};
use std::io::{BufReader, BufRead, Write};
use std::fs::{File, read_dir};
use std::path::Path;
use serde::Deserialize;
use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
use flate2::read::GzDecoder;
use zip::read::ZipArchive;
use std::thread;
use os_pipe;

#[derive(Deserialize)]
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
fn search_file(path: &Path, start: NaiveDateTime, end: NaiveDateTime, search: &str, results: &mut impl Write) {
    if let Ok(file) = File::open(path) {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "gz" => {
                let decoder = GzDecoder::new(file);
                let reader = BufReader::new(decoder);
                for line in reader.lines().flatten() {
                    if line.len() >= 19 {
                        if let Some(ts) = parse_timestamp(&line[..19]) {
                            if ts >= start && ts <= end && line.contains(search) {
                                let _ = writeln!(results, "{}", line);
                            }
                        }
                    }
                }
            }
            "zip" => {
                if let Ok(mut archive) = ZipArchive::new(file) {
                    for i in 0..archive.len() {
                        if let Ok(mut zfile) = archive.by_index(i) {
                            let reader = BufReader::new(&mut zfile);
                            for line in reader.lines().flatten() {
                                if line.len() >= 19 {
                                    if let Some(ts) = parse_timestamp(&line[..19]) {
                                        if ts >= start && ts <= end && line.contains(search) {
                                            let _ = writeln!(results, "{}", line);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                let reader = BufReader::new(file);
                for line in reader.lines().flatten() {
                    if line.len() >= 19 {
                        if let Some(ts) = parse_timestamp(&line[..19]) {
                            if ts >= start && ts <= end && line.contains(search) {
                                let _ = writeln!(results, "{}", line);
                            }
                        }
                    }
                }
            }
        }
    }
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
                    if let Ok(entries) = read_dir(folder_path) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                search_file(&path, start, end, &search_string, &mut writer);
                            }
                        }
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