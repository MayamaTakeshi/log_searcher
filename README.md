### log_searcher

A simple node.js app providing search of log files via an HTTP API method.

It will search multiple log folders (multiple applications).
Applications must log lines starting with prefix:
```
YYYY-MM-DD HH:mm:ss
```

## Installation
```
npm install
cp config/default.js.sample config/default.js
vim config/default.js # adjust as necessary
```

## Sample API call
```
curl -s -x '' -X POST 'http://192.168.1.1:7000/search' -H 'Content-Type: application/json' -d '{"start": "2020-05-08 06:25:01", "end": "2020-08-07 13:00:00", "pattern": "aa207c94-a89e-11ea-bb37-0242ac130002", "folders": ["myapp1", "myapp2", "myapp3"]}'   
```
