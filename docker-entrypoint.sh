#!/bin/bash
set -e

# Navigate to the application directory if it exists
if [ -d "/app" ]; then
  cd /app
else
  echo "Application directory /app not found. Current directory: $(pwd)"
fi

# Function to compile the project
compile_project() {
  echo "Compiling VexFS..."
  cargo build
  echo "Compilation complete."
}

# Function to run main tests
run_main_tests() {
  echo "Running main tests (cargo test)..."
  cargo test
  echo "Main tests complete."
}

# Function to run vector_test_runner
run_vector_tests() {
  echo "Running vector_test_runner..."
  cargo run --bin vector_test_runner
  echo "vector_test_runner complete."
}

# Default action: Start an interactive shell
if [ "$1" = "" ] || [ "$1" = "bash" ] || [ "$1" = "shell" ]; then
  echo "Starting interactive shell..."
  # Compile project first to make sure binaries are available
  compile_project
  exec /bin/bash
elif [ "$1" = "compile" ]; then
  compile_project
elif [ "$1" = "run-tests" ]; then
  compile_project
  run_main_tests
  run_vector_tests
elif [ "$1" = "run-main-tests" ]; then
  compile_project
  run_main_tests
elif [ "$1" = "run-vector-tests" ]; then
  compile_project
  run_vector_tests
elif [ "$1" = "vexctl" ]; then
  compile_project
  shift # Remove 'vexctl' from arguments
  echo "Running vexctl with arguments: $@"
  ./target/debug/vexctl "$@"
else
  # Execute any other command passed to the entrypoint
  echo "Executing command: $@"
  exec "$@"
fi
