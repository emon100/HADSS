# StorageConnector

This module provides a client interface to connect to Storage Layer nodes (StorageNodes) and wrap basic storage operations (GetSlice, PutSlice). It simplifies interaction with the Storage Layer for components like the Gateway.

## How to build connection
The primary connection type provided is `BasicStorageConnection`. It communicates with StorageNodes over HTTP. To create a connection, you typically need:

-   **Address:** The network address (e.g., `hostname:port` or `ip_address:port`) of the target StorageNode. This address should be resolvable.
-   **Consistency Policy:** Specifies the desired consistency level for operations (e.g., strong consistency, eventual consistency).

## Operations
1. GetSlice
2. PutSlice
3. DeleteSlice (TBD)

Every Operation needs a `handler` to locate the file slice.
