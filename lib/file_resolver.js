const fsUtils = require("nodejs-fs-utils")
const util = require('util')
const _ = require('lodash')


const walk = util.promisify(fsUtils.walk)

function list_files(folder) {
	var res = []

	return walk(folder, function (err, path, stats, next, cache) {
		if(err) {
			return next(err)
		}
		if(stats.isFile()) {
			res.push([path, stats.mtime.getTime()])
		}
		next();
	})
	.then(cache => {
		return res
	})
}

function sort_files(files) {
	return files.sort((fa, fb) => {
		if( fa[1] < fb[1] ) return -1;
		if( fa[1] > fb[1] ) return 1;
		return 0;
	})
}

function select_files(files, start, end) {
	var res = []
	
	var tail = _.dropWhile(files, item => {
		return item[1] < start
	})

	if(tail[0]) {
		res.push(tail[0])
	}

	if(tail[1]) {
		var rest = _.takeWhile(tail.slice(1), item => {
			return item[1] < end
		})
		rest.forEach(f => {
			res.push(f)
		})
		if(tail[rest.length+1]) {
			res.push(tail[rest.length+1])
		}
	}
	return res
}

async function resolve_files(folders, start, end) {
	var results = await Promise.all(folders.map(async (folder) => {
		return await list_files(folder)
		.then(files => {
			files = sort_files(files)
			return select_files(files, start, end)
		})
	}))

	return _.chain(results)
		.flatten()
		.map(entry => { return entry[0] }) //discard mtime as we don't need it anymore
		.uniq()
		.value()
}

module.exports = {
	list_files,
	sort_files,
	select_files,
	resolve_files,
}
