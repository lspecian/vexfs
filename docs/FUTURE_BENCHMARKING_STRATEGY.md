# Future Benchmarking Strategy for VexFS

To accurately assess and showcase the performance of VexFS, future benchmarking efforts should adhere to the following principles:

1.  **Benchmark the Core Engine**:
    *   Benchmarks must utilize the actual `VectorSearchEngine` (from `vexfs/src/vector_search.rs`) and its associated components, including `HnswGraph` (from `vexfs/src/anns/hnsw.rs`) for ANN searches and the `KnnSearchEngine`.
    *   Avoid using the standalone `TestVectorSearchEngine` from `vexfs/src/vector_test.rs` for performance claims about VexFS itself. This test engine is suitable for basic functional checks or as a very simple baseline, but not for representing the performance of the main system.

2.  **Accurate Timing Mechanisms**:
    *   For benchmarks intended to run in userspace (e.g., testing the `vexfs` library components before full kernel integration), use reliable timing mechanisms like `std::time::Instant`.
    *   If and when kernel-level benchmarks are developed, the `get_current_time_us()` function within `vexfs/src/vector_search.rs` (and similar timing functions) must be implemented to provide accurate, high-resolution timestamps from the kernel's perspective. Placeholder implementations should be replaced.

3.  **Realistic Data Storage and Operations**:
    *   The `TODO` comments and placeholder implementations in `vector_storage.rs` (and related modules like `file_ops.rs`, `dir_ops.rs` if they interact with vector data) need to be completed.
    *   Benchmarks should reflect genuine data operations:
        *   **Insertion**: This should involve the full pipeline of storing vector data, including any serialization, metadata updates, and index updates (e.g., adding nodes and edges to `HnswGraph`).
        *   **Search**: This should involve querying the actual index and retrieving data through the storage layer.
    *   Initially, an in-memory implementation of the `VectorStorageManager`'s backend could be used, but this should be clearly stated. The ultimate goal is to benchmark with persistence to a block device as expected of a file system.

4.  **Implement Core Algorithms**:
    *   Ensure that critical algorithms, like the layer assignment and search strategy in HNSW (`vexfs/src/anns/hnsw.rs`), are fully implemented according to established methods, replacing any simplified or placeholder logic.

5.  **Clear Distinction in Reporting**:
    *   Clearly differentiate between benchmarks for:
        *   The userspace `vexfs` library (testing components outside the kernel).
        *   The full VexFS kernel module (once integrated and testable within a VM).
    *   Specify the environment, dataset size, vector dimensionality, hardware, and exact software configuration used for any published benchmarks.

6.  **Comparative Benchmarks**:
    *   Once VexFS is more mature, comparisons to other vector databases or search libraries should be done carefully, ensuring that the VexFS setup (userspace library or kernel module) is appropriate for the comparison being made.

By following these guidelines, the VexFS project can build credible and informative performance metrics that accurately reflect its capabilities as it develops.
