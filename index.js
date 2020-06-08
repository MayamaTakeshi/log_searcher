const express = require('express')
const bodyParser = require('body-parser')
const m = require('moment')
const config = require('config')

const fr = require('./lib/file_resolver.js')
const fs = require('./lib/file_searcher.js')

const _ = require('lodash')

const app = express()
app.use(bodyParser.json())

app.post("/search", async (req, res, next) => {
	try {
		console.log("/search")
		console.log(req.body)
		var start = m(req.body.start).unix() * 1000
		var end = m(req.body.end).unix() * 1000 + 999

		console.log(`start=${start} end=${end}`)

		var pattern = req.body.pattern
		var is_regex = req.body.is_regex
		if(is_regex) {
			pattern = new RegExp(pattern)
		}	

		var folders = _.map(req.body.folders, (folder) => config.base_dir + "/" + folder)

		var files = await fr.resolve_files(folders, start, end)

		var result = await fs.search(files, start, end, pattern, is_regex)

		var files2 = await fr.resolve_files(folders, start, end)
		if(!_.isEqual(files, files2)) {
			// files to search changed. Refetch
			result = await fs.search(files, start, end, pattern, is_regex)
			if(!_.isEqual(files, files2)) {
				var msg = "List to files to check changed during check more than once"
				console.error(msg)
				res.statusMessage = msg	
				res.status(500).end()
				return
			}
		}

		result.sort() // need to sort it because we might have lines from different apps
		//console.log(result)

		res.status(200)

		for (const line of result) {
			res.write(line)
			res.write("\n")
		}

		res.end()
		console.log(`Succefully found and sent ${result.length} lines`)
	} catch (error) {
		console.log(error)
		return next(error)
	}
})

app.listen(config.local_port, config.local_ip, () => console.log(`log_searcher listening at ${config.local_port}`))
