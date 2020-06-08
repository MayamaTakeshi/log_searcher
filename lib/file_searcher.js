const fs = require('fs');
const readline = require('readline');
const m = require('moment')
const zlib = require('zlib')
const xz = require('xz')

const fr = require('./file_resolver.js');

async function search_file(file, acc, start, end, pattern, is_regex) {
	return new Promise((resolve, reject) => {	
		const fileStream = fs.createReadStream(file);

		var decomp = null

		if(file.endsWith(".gz")) {
			decomp = zlib.createUnzip()
			fileStream.pipe(decomp)
		} else if(file.endsWith(".xz")) {
			decomp = new xz.Decompressor()
			fileStream.pipe(decomp)
		}

		const rl = readline.createInterface({
			input: decomp ? decomp : fileStream,
			crlfDelay: Infinity
		});
		// Note: we use the crlfDelay option to recognize all instances of CR LF
		// ('\r\n') in input.txt as a single line break.
	
		rl.on('line', line => {
			var date_str = line.substring(0, 19)
			//console.log(date_str)
			var epoch;
			try { 
				epoch = m(date_str).unix() * 1000
			} catch (e) {
				//date_str might not be date
				epoch = 0
			}
				
			if(epoch >= start && epoch <= end) {
				if(is_regex && line.match(pattern)) {
					acc.push(line)
				} else if(line.indexOf(pattern) >= 0) {
					acc.push(line)
				}
			} else if(epoch > end) {
				rl.close()
				resolve()
			}
		})

		rl.on('close', () => {
			//console.log(`${file} rl close`)
			resolve()
		})
	})

}

async function search(files, start, end, pattern, is_regex) {
	var acc = []
	
	for (const file of files) {
		await search_file(file, acc, start, end, pattern, is_regex)
	}

	return acc
}

module.exports = {
	search_file,
	search,
}
