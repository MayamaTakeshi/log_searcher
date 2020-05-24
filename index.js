const express = require('express')
const bodyParser = require('body-parser')
const m = require('moment')
const config = require('config')

const fs = require('./lib/file_searcher.js')

const _ = require('lodash')

const app = express()
app.use(bodyParser.json())

app.post("/search", async (req, res) => {
	console.log(req.body)

	var start = m(req.body.start).unix() * 1000
	var end = m(req.body.end).unix() * 1000 + 999

	//console.log(start, end)

	var pattern = req.body.pattern
	var is_regex = req.body.is_regex
	if(is_regex) {
		pattern = new RegExp(pattern)
	}	

	var folders = _.map(req.body.folders, (folder) => config.base_dir + "/" + folder)

	var result = await fs.filter_folders(folders, start, end, pattern, is_regex)
	//console.log(result)

	res.status(200)

	for (const line of result) {
		res.write(line)
		res.write("\n")
	}

	res.end()
})

app.listen(config.local_port, config.local_ip, () => console.log(`log_searcher listening at ${config.local_port}`))
