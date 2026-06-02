# log_searcher
A simple http server that permits to search inside log files

It was written in rust and is built to target [musl](https://en.wikipedia.org/wiki/Musl) to permit to run a single binary in any linux system old or new.

## Usage

Start the server (listens on port 8078):
```sh
./log_searcher
```

## API

### POST /search_logs

Search log files in a folder within a time range.

**Request body (JSON):**

| Field | Type | Required | Description |
|---|---|---|---|
| `folder` | string | yes | Path to the folder containing log files |
| `start_stamp` | string | yes | Start timestamp (`YYYY-MM-DD HH:MM:SS`) |
| `end_stamp` | string | yes | End timestamp (`YYYY-MM-DD HH:MM:SS`) |
| `search_string` | string | no | Only return lines containing this string |
| `file_name_regex` | string | no | Regex to filter which files to search |

Returns matching log lines as plain text.

## Examples

Search all files in a folder for a time range:
```sh
curl -s -X POST http://localhost:8078/search_logs \
  -H 'Content-Type: application/json' \
  -d '{"folder":"/var/log/myapp","start_stamp":"2026-06-02 10:00:00","end_stamp":"2026-06-02 11:00:00"}'
```

Filter lines containing a specific string:
```sh
curl -s -X POST http://localhost:8078/search_logs \
  -H 'Content-Type: application/json' \
  -d '{"folder":"/var/log/myapp","start_stamp":"2026-06-02 10:00:00","end_stamp":"2026-06-02 11:00:00","search_string":"ERROR"}'
```

Also filter files by name using a regex (e.g. only `.log` files):
```sh
curl -s -X POST http://localhost:8078/search_logs \
  -H 'Content-Type: application/json' \
  -d '{"folder":"/var/log/myapp","start_stamp":"2026-06-02 10:00:00","end_stamp":"2026-06-02 11:00:00","search_string":"ERROR","file_name_regex":"\\.log$"}'
```
