use tiny_http::{Server, Response, Request};
use std::io::{BufReader, BufRead};
use std::fs::{File, read_dir};
use std::path::Path;
use serde::Deserialize;
use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
use flate2::read::GzDecoder;
use zip::read::ZipArchive;

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
fn search_file(path: &Path, start: NaiveDateTime, end: NaiveDateTime, search: &str, results: &mut Vec<String>) {
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
                                results.push(line);
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
                                            results.push(line);
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
                                results.push(line);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn handle_request(mut request: Request) {
    if request.method() != &tiny_http::Method::Post || request.url() != "/search_logs" {
        let _ = request.respond(Response::from_string("Not Found").with_status_code(404));
        return;
    }

    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        let _ = request.respond(Response::from_string("Failed to read body").with_status_code(400));
        return;
    }

    let req: SearchRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(_) => {
            let _ = request.respond(Response::from_string("Invalid JSON").with_status_code(400));
            return;
        }
    };

    let start = match parse_timestamp(&req.start_stamp) {
        Some(t) => t,
        None => {
            let _ = request.respond(Response::from_string("Invalid start_stamp").with_status_code(400));
            return;
        }
    };

    let end = match parse_timestamp(&req.end_stamp) {
        Some(t) => t,
        None => {
            let _ = request.respond(Response::from_string("Invalid end_stamp").with_status_code(400));
            return;
        }
    };

    let folder_path = Path::new(&req.folder);
    if !folder_path.is_dir() {
        let _ = request.respond(Response::from_string("Folder does not exist").with_status_code(400));
        return;
    }

    // Collect all files in the folder
    let mut results = Vec::new();
    if let Ok(entries) = read_dir(folder_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                search_file(&path, start, end, &req.search_string, &mut results);
            }
        }
    }

    let json = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    let _ = request.respond(Response::from_string(json).with_status_code(200));
}

fn main() {
    let server = Server::http("0.0.0.0:8078").expect("Failed to start server");
    println!("Server running on http://0.0.0.0:8078");

    for request in server.incoming_requests() {
        handle_request(request);
    }
}

