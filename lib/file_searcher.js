const fs = require('fs');
const readline = require('readline');
const m = require('moment')

const fr = require('./file_resolver.js');

async function filter_file(file, acc, start, end, pattern, is_regex) {
	const fileStream = fs.createReadStream(file);

	const rl = readline.createInterface({
		input: fileStream,
		crlfDelay: Infinity
	});
	// Note: we use the crlfDelay option to recognize all instances of CR LF
	// ('\r\n') in input.txt as a single line break.

	for await (const line of rl) {
		var date_str = line.substring(0, 19)
		var epoch = m(date_str).unix() * 1000
		if(epoch >= start && epoch <= end) {
			if(is_regex && line.match(pattern)) {
				acc.push(line)
			} else if(line.indexOf(pattern)) {
				acc.push(line)
			}
		}
	}
}

async function filter_folder(folder, start, end, pattern, is_regex) {
	var acc = []
	var files = await fr.resolve(folder, start, end)
	
	for (const file of files) {
		await filter_file(file[0], acc, start, end, pattern, is_regex)
	}

	return acc
}

async function filter_folders(folders, start, end, pattern, is_regex) {
	var final = []
	for (const folder of folders) {
		var res = await filter_folder(folder, start, end, pattern, is_regex)
		final = final.concat(res)
	}

	return final
}

module.exports = {
	filter_file,
	filter_folder,
	filter_folders,
}
