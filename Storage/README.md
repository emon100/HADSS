# StorageNode Overview
# HTTP endpoints
## /health

This endpoint shows health information of the Storage node.

## /slice/:id
* GET <id> ;返回文件内容
* HEAD <id> ;返回metadata
* DELETE <id>+ ; 可一次删除多个文件
