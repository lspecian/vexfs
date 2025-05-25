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

#include <linux/init.h>
#include <linux/module.h>
#include <linux/kernel.h>

// Declare the Rust functions that will be called.
// Their actual Rust names might be mangled, so we'll need to use `#[no_mangle]`
// and potentially `extern "C"` in the Rust code.
extern int vexfs_rust_init(void);
extern void vexfs_rust_exit(void);

static int __init vexfs_init_module(void)
{
    printk(KERN_INFO "VexFS: vexfs_module_entry: Calling vexfs_rust_init()\n");
    return vexfs_rust_init();
}

static void __exit vexfs_exit_module(void)
{
    printk(KERN_INFO "VexFS: vexfs_module_entry: Calling vexfs_rust_exit()\n");
    vexfs_rust_exit();
}

module_init(vexfs_init_module);
module_exit(vexfs_exit_module);

MODULE_LICENSE("GPL");
MODULE_AUTHOR("VexFS Contributors");
MODULE_DESCRIPTION("VexFS: Vector-Native File System (C Entry Point)");
MODULE_VERSION("0.1.0");
