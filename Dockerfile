# Stage 1: Build environment
FROM ubuntu:22.04 AS builder

# Prevent interactive prompts during build
ENV DEBIAN_FRONTEND=noninteractive

# Install build dependencies
RUN apt-get update &&     apt-get install -y --no-install-recommends     build-essential     curl     git     pkg-config     libfuse3-dev     libssl-dev     ca-certificates     && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
ENV RUSTUP_HOME=/usr/local/rustup     CARGO_HOME=/usr/local/cargo     PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal

# Create a non-root user for building and running the application
# Running as non-root is a security best practice.
# USER_ID and GROUP_ID can be passed as build arguments to match host user
ARG USER_ID=1000
ARG GROUP_ID=1000
RUN groupadd --gid $GROUP_ID vexfs_user || true # || true to handle existing group
RUN useradd --uid $USER_ID --gid $GROUP_ID --create-home --shell /bin/bash vexfs_user

# Application directory
WORKDIR /app

# Copy source code
# First copy only files necessary for dependency fetching to leverage Docker cache
COPY Cargo.toml Cargo.lock ./
# If you have a workspace, you might need to copy all Cargo.toml files
# COPY Cargo.toml Cargo.lock ./
# COPY vexctl/Cargo.toml ./vexctl/
# COPY fs/Cargo.toml ./fs/ # Assuming 'fs' is the main crate or workspace member

# Create dummy src/lib.rs and potentially other main files if needed for `cargo fetch`
# This helps in caching dependencies if these files don't change often
RUN mkdir -p src vexctl/src fs/src &&     echo "fn main() {}" > src/lib.rs &&     ( [ -f vexctl/Cargo.toml ] && echo "fn main() {}" > vexctl/src/main.rs || true ) &&     ( [ -f fs/Cargo.toml ] && echo "fn main() {}" > fs/src/lib.rs || true )

# Fetch dependencies (as app_user to ensure permissions are fine for later build steps)
# Ensure correct ownership for cargo directories if they were created by root before switching user
RUN chown -R vexfs_user:vexfs_user /usr/local/rustup /usr/local/cargo
USER vexfs_user
# RUN cargo fetch # Temporarily disabling cargo fetch as it might fail with dummy files if workspace structure is complex. Build will fetch.

# Copy the rest of the application code
USER root # Switch back to root to copy files owned by host user, then chown
COPY . .
RUN chown -R vexfs_user:vexfs_user /app

# Switch to non-root user for building
USER vexfs_user

# Build the project (optional, can be done in entrypoint or a final stage)
# RUN cargo build --release

# Stage 2: Final image (runtime)
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# Install runtime dependencies (e.g., libfuse3 for userspace execution)
RUN apt-get update &&     apt-get install -y --no-install-recommends     libfuse3     ca-certificates     && rm -rf /var/lib/apt/lists/*

# Create the same non-root user as in the builder stage
ARG USER_ID=1000
ARG GROUP_ID=1000
RUN groupadd --gid $GROUP_ID vexfs_user || true
RUN useradd --uid $USER_ID --gid $GROUP_ID --create-home --shell /bin/bash vexfs_user

WORKDIR /app

# Copy entrypoint script
COPY --chown=vexfs_user:vexfs_user docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Copy compiled artifacts from builder stage
# Ensure the paths here match where cargo build places them (e.g., target/release/ or target/debug/)
# If building in entrypoint, this step might not be needed or will be different.
# For now, we assume compilation will happen inside the container via entrypoint or manually.
# If you pre-compile in builder stage:
# COPY --from=builder --chown=vexfs_user:vexfs_user /app/target/release/vexfs ./vexfs # Example
# COPY --from=builder --chown=vexfs_user:vexfs_user /app/target/release/vector_test_runner ./vector_test_runner
# COPY --from=builder --chown=vexfs_user:vexfs_user /app/target/release/vexctl ./vexctl

# Copy the application source for development and on-the-fly compilation
# This is useful if the entrypoint script compiles the code or for development.
COPY --chown=vexfs_user:vexfs_user . .

# Volume for persistent data (e.g., test datasets, indexes)
VOLUME /mnt/vexfs_data
RUN mkdir -p /mnt/vexfs_data && chown vexfs_user:vexfs_user /mnt/vexfs_data

USER vexfs_user

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

# Default command (starts a shell)
CMD ["shell"]
