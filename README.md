### log_searcher

A simple node.js app providing search of log files via an HTTP API method.

It will search multiple log folders (multiple applications).
Applications must log lines starting with prefix:
```
YYYY-MM-DD HH:mm:ss
```

## Installation

Execute these:
```
npm install
cp config/default.js.sample config/default.js
vim config/default.js # adjust as necessary
```

Then create app log folders under the folder specified by config.base_dir (they would be remote folder accessed by NFS). Ex:
```
  /mnt/nfs/log/myapp1
  /mnt/nfs/log/myapp2
  /mnt/nfs/log/myapp3
```

## Sample API call
```
curl -s -x '' -X POST 'http://192.168.1.1:7000/search' -H 'Content-Type: application/json' -d '{"start": "2020-05-08 06:25:01", "end": "2020-08-07 13:00:00", "pattern": "aa207c94-a89e-11ea-bb37-0242ac130002", "folders": ["myapp1", "myapp2", "myapp3"]}'   
```

## Details
  - based on the folders list specified in the API call, we compose a file list to search.
  - the log files can be compressed: if they end with '.gz' or '.xz' they will be stream-decompressed for processing.
  - to avoid generating an invalid search in case a log rotation happens while the resolved list of files is being processed, we will regenerate the file list after search and if it differs from the original, one, we will search them again.
