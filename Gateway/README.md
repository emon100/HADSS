# Gateway

This layer locates between user and internal storage
layer.

This Gateway use a module to communicate with 


command:
| GET <id> ;返回文件内容
| HEAD <id> ;返回metadata
| POST LENGTH <data>
| DELETE <id>+ ; 可一次删除多个文件
| UPDATEHEAD <id> <metadata-data>
;

