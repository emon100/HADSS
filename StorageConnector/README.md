# StorageConnector

This module builds connections to StorageNode and
to warps basic operations on the StorageNode.

## How to build connection
BasicConnection: Connection By http, builds from `dns lookable name` and `consistency policy`.

## Operations
1. GetSlice
2. PutSlice
3. DeleteSlice (TBD)

Every Operation needs a `handler` to locate the file slice.

###
