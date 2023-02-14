// What I need to do
// 1. Read file and their config
// 2. Loop through file in each folder and make sure it fit schema
// 3. If it don't, "move" it

// How to move
// 1. read import config line by line until match
// if not match, put it in _uncategorized
// 2. if it fit some config line, loop through schema
// 	is current data field fit schema?
// 	if true
// 		if true, remove datafield and loop through scheme again
// 		if false, return error
// 	if false/error go to next schema
