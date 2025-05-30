# VexFS Licensing Guide

VexFS uses a dual licensing structure to accommodate both userland and kernel components while ensuring compatibility with the Linux kernel's GPL requirements.

## License Structure Overview

### Apache License 2.0 (Default)
Applies to all userland components and libraries:
- **Root library** (`src/lib.rs` and related files)
- **Control tool** (`vexctl/` directory)
- **Documentation** (`docs/` directory)
- **Build scripts** and configuration files
- **Test environments** (`tests/legacy/` directory)
- **Examples** and sample code

### GNU General Public License v2
Applies to kernel module components:
- **Kernel module entry point** (`fs/vexfs_module_entry.c`)
- **Kernel build files** (`fs/Kbuild`, `fs/Makefile`)
- **Kernel-specific Rust code** (when compiled with `kernel` feature)

## File-by-File Breakdown

### Apache 2.0 Licensed Files
```
LICENSE                     # Apache 2.0 license text
src/lib.rs                  # Main userland library
src/bin/                    # Test binaries
vexctl/                     # Control tool (all files)
docs/                       # Documentation (all files)
tests/legacy/                   # Testing environment (all files)
scripts/                    # Build and utility scripts
Cargo.toml                  # Main workspace manifest
README.md                   # Project documentation
.gitignore                  # Git configuration
```

### GPL v2 Licensed Files
```
LICENSE.kernel              # GPL v2 license text
fs/vexfs_module_entry.c  # Kernel module C entry point
fs/Kbuild                # Kernel build configuration
fs/Makefile              # Kernel module makefile
```

### Dual Licensed Files
```
fs/src/                  # Core filesystem implementation
fs/Cargo.toml           # Module manifest (dual licensed)
```

The `fs/src/` directory contains code that can be compiled for both userland testing and kernel module usage. When compiled with the `kernel` feature, these files operate under GPL v2. When compiled for userland testing, they operate under Apache 2.0.

## License Headers

All source files include appropriate license headers:

### Apache 2.0 Header (Userland)
```rust
/*
 * VexFS - Vector Extended File System
 * Copyright 2025 VexFS Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
```

### GPL v2 Header (Kernel)
```c
/*
 * VexFS - Vector Extended File System
 * Copyright (C) 2025 VexFS Contributors
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 */
```

## Compilation Features and Licensing

### Userland Compilation (Default)
```bash
cargo build                    # Apache 2.0 licensing
cargo build --features std    # Apache 2.0 licensing
```

### Kernel Module Compilation
```bash
# In fs/ directory
make                          # GPL v2 licensing
```

### Mixed Environment Testing
The project supports testing kernel logic in userland while maintaining proper licensing boundaries. The `fs/src/` code is structured to be license-aware based on compilation target.

## Contributing Guidelines

### License Compatibility
- **Userland contributions**: Must be compatible with Apache 2.0
- **Kernel contributions**: Must be compatible with GPL v2
- **Core algorithm contributions**: Should work under both licenses

### Code Organization
- Place userland-only code in `src/` or `vexctl/`
- Place kernel-only code in `fs/` with appropriate C wrappers
- Place shared algorithms in `fs/src/` with conditional compilation

### Third-Party Dependencies
- **Userland dependencies**: Apache 2.0, MIT, or BSD compatible licenses
- **Kernel dependencies**: GPL v2 compatible only
- Document all dependencies in `NOTICE` file

## License Compliance

### Distribution Requirements

#### Apache 2.0 Components
- Include `LICENSE` file
- Include `NOTICE` file
- Preserve copyright notices
- Include attribution for modifications

#### GPL v2 Components
- Include `LICENSE.kernel` file
- Provide source code access
- Include GPL headers in distributed files
- Document any modifications

### Binary Distribution
- Userland binaries: Apache 2.0 requirements apply
- Kernel modules: GPL v2 requirements apply
- Combined distributions: Both license requirements apply

## Questions and Clarifications

For licensing questions:
1. Check this document first
2. Review the `NOTICE` file
3. Examine individual file headers
4. Consult the full license texts in `LICENSE` and `LICENSE.kernel`

The dual licensing ensures maximum compatibility while respecting both the open source ecosystem and Linux kernel requirements.