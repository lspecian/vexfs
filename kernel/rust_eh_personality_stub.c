/*
 * VexFS - Vector Extended File System
 * Copyright (C) 2025 VexFS Contributors
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 */

// Stub implementation for rust_eh_personality to resolve kernel module linking
// This should never be called in kernel space with panic=abort

#include <linux/kernel.h>

void rust_eh_personality(void) {
    // In kernel space with panic=abort, this should never be reached
    // If it is reached, it indicates a serious configuration problem
    panic("rust_eh_personality called in kernel space - this should never happen with panic=abort");
}