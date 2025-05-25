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
MODULE_AUTHOR("Your Name/AI Agent");
MODULE_DESCRIPTION("VDBHAX/VexFS: Vector-Native File System (C Entry)");
