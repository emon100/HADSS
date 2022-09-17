# StorageNode Overview

## Architecture

StorageNodes behave like a replicated state machine.
StorageNodes use Consensus module to maintain log consistency.
Then use the log to maintain file consistency.

write:
log update -> local storage pull log -> local storage fetch data -> local storage write
receive: filename, file content
write filename into a append only log.
write file content into a folder.

read:
check log -> check if local storage have the file version -> read

## HTTP endpoints

### /health

This endpoint shows health information of the Storage node.

### /slice/:id

* GET <id> ;return the file by its id
* HEAD <id> ; return metadata of the file by its id
* DELETE <id>+ ;return if the operation is successful
