# Docker-Based Development and Testing for VexFS

This document provides instructions on how to use the Docker-based environment for developing and testing VexFS. This setup allows for a reproducible environment to experiment with VexFS, run vector operations, and contribute without needing a full VM or manual kernel rebuilds for userspace testing.

## Prerequisites

*   Docker installed on your system.

## Building the Docker Image

To build the Docker image, navigate to the root directory of the VexFS project and run:

```bash
docker build -t vexfs-dev .
```

This command will build an image tagged `vexfs-dev` using the `Dockerfile` in the project root.

## Running the Docker Container

### Interactive Development Shell

To start an interactive shell within the container for development and experimentation:

```bash
# Non-privileged mode (recommended for most userspace development)
docker run -it --rm -v "$(pwd)":/app -v vexfs_data:/mnt/vexfs_data vexfs-dev

# Privileged mode (potentially needed for kernel module interaction, use with caution)
# docker run -it --rm --privileged -v "$(pwd)":/app -v vexfs_data:/mnt/vexfs_data vexfs-dev
```

**Explanation of options:**

*   `-it`: Runs the container in interactive mode with a pseudo-TTY.
*   `--rm`: Automatically removes the container when it exits.
*   `-v "$(pwd)":/app`: Mounts the current host directory (your VexFS project) into the `/app` directory inside the container. This allows you_to edit files on your host and have the changes reflected immediately in the container.
*   `-v vexfs_data:/mnt/vexfs_data`: Creates or uses a Docker named volume called `vexfs_data` and mounts it to `/mnt/vexfs_data` inside the container. This directory can be used for storing test data, datasets for ingestion, or any persistent artifacts you want to survive container rebuilds.
*   `--privileged`: (Use with caution) Grants the container extended privileges on the host machine. This might be necessary if attempting to interact with or load kernel modules directly from within the container, though this approach has security implications and limitations. For VexFS, the primary focus of this Docker environment is userspace development and testing.

### Running Tests Directly

To execute the test suite directly:

```bash
docker run --rm -v "$(pwd)":/app vexfs-dev run-tests
```
This will execute the `run-tests` command defined in the `docker-entrypoint.sh` script.

### Using `vexctl` (Example)

Once inside the interactive shell, or by passing commands to `docker run`:

```bash
# Inside the interactive container shell:
cargo build # Ensure everything is compiled
./target/debug/vexctl --help # Example: run vexctl

# Or, run vexctl directly (after building the image):
docker run --rm -v "$(pwd)":/app vexfs-dev ./target/debug/vexctl --help
```

## Volume Mounts for Test Files and Data

As shown in the `docker run` examples, you can use the `-v` flag to mount data into the container.

*   **Source Code:** `-v "$(pwd)":/app` is crucial for development, as it syncs your local project files into the container's `/app` directory.
*   **Test Data:** Use the `/mnt/vexfs_data` mount (or define your own) for:
    *   Storing large datasets for vector ingestion and search tests.
    *   Outputting benchmark results or logs that you want to persist.
    *   Experimenting with file system operations on a dedicated, container-managed volume.

**Example: Ingesting data from a host directory**

1.  Place your test data (e.g., `my_vectors.bin`) in a directory on your host, say `~/vexfs_test_data`.
2.  Run the container, mounting this directory:

    ```bash
    docker run -it --rm       -v "$(pwd)":/app       -v ~/vexfs_test_data:/mnt/external_data       -v vexfs_data:/mnt/vexfs_data       vexfs-dev
    ```
3.  Inside the container, you can then access your data from `/mnt/external_data` and use it with `vexctl` or your test scripts, potentially processing it and storing results in `/mnt/vexfs_data`.

## Privileged vs. Non-Privileged Mode

*   **Non-Privileged Mode (Default & Recommended for Userspace)**:
    *   The container runs without elevated permissions on the host.
    *   This is sufficient for compiling VexFS in userspace mode, running `cargo test` for userspace components, using `vexctl`, and developing features that do not require direct kernel interaction.
    *   The VexFS kernel module itself **cannot** be loaded or tested directly from a non-privileged container in most standard Docker setups. The environment will primarily rely on userspace testing (FUSE or simulated layers if available, or the `vector_test_runner`).

*   **Privileged Mode (`--privileged`)**:
    *   Grants the container root-like access to the host's devices and kernel capabilities.
    *   **Security Risk**: This mode significantly reduces container isolation and should be used with extreme caution and only if absolutely necessary.
    *   **Kernel Module Interaction**: While `--privileged` might allow *some* interaction with the host kernel (like trying to load modules built for the *exact same kernel version as the host*), it's generally not a reliable or recommended way to test kernel modules that are part of the project being developed *inside* the container. Kernel module development and testing are best done in a dedicated VM or on a development machine where kernel versions match.
    *   **VexFS Focus**: For VexFS, this Docker environment is primarily aimed at **userspace development and testing**. If kernel module testing is required, the existing VM-based testing infrastructure (`test_env/`) is the more appropriate solution. This Docker setup helps streamline the userspace part of the "Two-Tier Development Strategy."

## Kernel Module Considerations

*   **Host Kernel Module Reuse**: Reusing a VexFS kernel module already installed on the host from *within* the Docker container is complex and generally not feasible without `--privileged` mode and matching kernel versions. Even then, it's not a standard Docker use case.
*   **Building Kernel Module in Docker**: The Dockerfile *could* include steps to compile the kernel module components. However, this compiled module would be for the kernel version *inside the Ubuntu Docker image*, which is unlikely to match your host kernel. Attempting to load such a module on the host would likely fail or cause instability.
*   **Userspace Fallback**: The primary strength of this Docker environment is for robust userspace testing. If the VexFS kernel module is unavailable or cannot be loaded, tests should rely on:
    *   The FUSE-based userspace implementation of VexFS (if available).
    *   Simulated environment or mock layers for kernel interactions.
    *   The `vector_test_runner` which operates in userspace.

## Optimizations

*   **BuildKit**: Docker BuildKit is typically enabled by default in recent Docker versions and helps optimize build times through improved caching and parallel builds.
*   **Multi-stage Builds**: The `Dockerfile` may use multi-stage builds to separate build dependencies from the final runtime image, reducing image size. For example, a build stage could compile VexFS, and a final stage would copy only the compiled binaries and necessary runtime dependencies.

## Quick Start Summary

1.  **Build Image**:
    ```bash
    docker build -t vexfs-dev .
    ```
2.  **Run Interactive Shell (Userspace Development)**:
    ```bash
    docker run -it --rm -v "$(pwd)":/app -v vexfs_data:/mnt/vexfs_data vexfs-dev
    ```
    Inside the container:
    ```bash
    cargo build
    cargo test
    cargo run --bin vector_test_runner
    ./target/debug/vexctl --help
    ```

This Docker environment aims to accelerate the userspace development and testing cycle for VexFS. For kernel module development and testing, please refer to the VM-based workflows described in the project documentation.
