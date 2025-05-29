#!/bin/bash

# Create a simple mkfs.vexfs utility for testing
# This is a minimal implementation that creates a basic VexFS superblock

set -e

echo "Creating simple mkfs.vexfs utility..."

cat > /tmp/mkfs_vexfs_simple.c << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <stdint.h>
#include <sys/stat.h>
#include <errno.h>

// VexFS constants
#define VEXFS_MAGIC 0x5645584653555045ULL  // "VEXFSUPE" in ASCII
#define VEXFS_DEFAULT_BLOCK_SIZE 4096
#define VEXFS_VERSION_MAJOR 1
#define VEXFS_VERSION_MINOR 0

// Simplified VexFS superblock structure
struct vexfs_superblock {
    uint64_t s_magic;              // Magic number
    uint64_t s_blocks_count;       // Total blocks
    uint64_t s_free_blocks_count;  // Free blocks
    uint32_t s_inodes_count;       // Total inodes
    uint32_t s_free_inodes_count;  // Free inodes
    uint32_t s_block_size;         // Block size
    uint16_t s_inode_size;         // Inode size
    uint16_t s_version_major;      // Major version
    uint16_t s_version_minor;      // Minor version
    uint64_t s_mkfs_time;          // Creation time
    uint64_t s_mount_time;         // Last mount time
    uint64_t s_wtime;              // Last write time
    uint16_t s_mount_count;        // Mount count
    uint16_t s_max_mount_count;    // Max mount count
    uint16_t s_state;              // Filesystem state
    uint16_t s_errors;             // Error behavior
    uint32_t s_feature_compat;     // Compatible features
    uint32_t s_feature_incompat;   // Incompatible features
    uint32_t s_feature_ro_compat;  // Read-only compatible features
    uint8_t  s_uuid[16];           // UUID
    char     s_volume_name[64];    // Volume name
    uint64_t s_first_data_block;   // First data block
    uint32_t s_blocks_per_group;   // Blocks per group
    uint32_t s_inodes_per_group;   // Inodes per group
    uint32_t s_group_count;        // Number of block groups
    uint32_t s_journal_inum;       // Journal inode
    uint32_t s_journal_blocks;     // Journal blocks
    uint64_t s_journal_first_block; // First journal block
    uint32_t s_vector_magic;       // Vector magic
    uint16_t s_vector_version;     // Vector version
    uint16_t s_vector_dimensions;  // Vector dimensions
    uint8_t  s_vector_algorithm;   // Vector algorithm
    uint8_t  s_vector_metric;      // Vector metric
    uint16_t s_vector_params[4];   // Vector parameters
    uint64_t s_vector_index_block; // Vector index block
    uint32_t s_vector_index_blocks; // Vector index blocks
    uint64_t s_vector_count;       // Vector count
    uint32_t s_vector_features;    // Vector features
    uint32_t s_checksum;           // Checksum
    uint32_t s_reserved[126];      // Reserved space
} __attribute__((packed));

void print_usage(const char *prog) {
    printf("Usage: %s [-f] [-L label] <device>\n", prog);
    printf("  -f        Force creation\n");
    printf("  -L label  Set volume label\n");
    printf("  -h        Show this help\n");
}

uint64_t get_device_size(int fd) {
    struct stat st;
    if (fstat(fd, &st) < 0) {
        return 0;
    }
    
    if (S_ISREG(st.st_mode)) {
        return st.st_size;
    } else if (S_ISBLK(st.st_mode)) {
        // For block devices, seek to end
        off_t size = lseek(fd, 0, SEEK_END);
        lseek(fd, 0, SEEK_SET);
        return size > 0 ? size : 0;
    }
    
    return 0;
}

uint32_t simple_crc32(const void *data, size_t len) {
    const uint8_t *bytes = (const uint8_t *)data;
    uint32_t crc = 0xFFFFFFFF;
    
    for (size_t i = 0; i < len; i++) {
        crc ^= bytes[i];
        for (int j = 0; j < 8; j++) {
            if (crc & 1) {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    
    return ~crc;
}

int main(int argc, char *argv[]) {
    const char *device = NULL;
    const char *label = NULL;
    int force = 0;
    int opt;
    
    while ((opt = getopt(argc, argv, "fL:h")) != -1) {
        switch (opt) {
            case 'f':
                force = 1;
                break;
            case 'L':
                label = optarg;
                break;
            case 'h':
                print_usage(argv[0]);
                return 0;
            default:
                print_usage(argv[0]);
                return 1;
        }
    }
    
    if (optind >= argc) {
        fprintf(stderr, "Error: Device path required\n");
        print_usage(argv[0]);
        return 1;
    }
    
    device = argv[optind];
    
    printf("mkfs.vexfs: Creating VexFS filesystem on %s\n", device);
    
    // Open device
    int fd = open(device, O_RDWR);
    if (fd < 0) {
        fprintf(stderr, "Error: Cannot open %s: %s\n", device, strerror(errno));
        return 1;
    }
    
    // Get device size
    uint64_t device_size = get_device_size(fd);
    if (device_size == 0) {
        fprintf(stderr, "Error: Cannot determine device size\n");
        close(fd);
        return 1;
    }
    
    if (device_size < 1024 * 1024) {
        fprintf(stderr, "Error: Device too small (minimum 1MB)\n");
        close(fd);
        return 1;
    }
    
    printf("Device size: %llu bytes (%.1f MB)\n", 
           (unsigned long long)device_size, 
           device_size / (1024.0 * 1024.0));
    
    // Calculate filesystem parameters
    uint64_t total_blocks = device_size / VEXFS_DEFAULT_BLOCK_SIZE;
    uint32_t blocks_per_group = 32768; // 128MB groups at 4KB blocks
    uint32_t inodes_per_group = 8192;  // 8K inodes per group
    uint32_t group_count = (total_blocks + blocks_per_group - 1) / blocks_per_group;
    uint32_t total_inodes = group_count * inodes_per_group;
    
    printf("Filesystem layout:\n");
    printf("  Block size: %d bytes\n", VEXFS_DEFAULT_BLOCK_SIZE);
    printf("  Total blocks: %llu\n", (unsigned long long)total_blocks);
    printf("  Block groups: %u\n", group_count);
    printf("  Blocks per group: %u\n", blocks_per_group);
    printf("  Inodes per group: %u\n", inodes_per_group);
    printf("  Total inodes: %u\n", total_inodes);
    
    // Create superblock
    struct vexfs_superblock sb;
    memset(&sb, 0, sizeof(sb));
    
    sb.s_magic = VEXFS_MAGIC;
    sb.s_blocks_count = total_blocks;
    sb.s_free_blocks_count = total_blocks - 100; // Reserve some blocks
    sb.s_inodes_count = total_inodes;
    sb.s_free_inodes_count = total_inodes - 10; // Reserve some inodes
    sb.s_block_size = VEXFS_DEFAULT_BLOCK_SIZE;
    sb.s_inode_size = 256;
    sb.s_version_major = VEXFS_VERSION_MAJOR;
    sb.s_version_minor = VEXFS_VERSION_MINOR;
    sb.s_mkfs_time = time(NULL);
    sb.s_mount_time = 0;
    sb.s_wtime = time(NULL);
    sb.s_mount_count = 0;
    sb.s_max_mount_count = 20;
    sb.s_state = 1; // Clean
    sb.s_errors = 1; // Continue on errors
    sb.s_feature_compat = 0;
    sb.s_feature_incompat = 0;
    sb.s_feature_ro_compat = 0;
    sb.s_first_data_block = 1;
    sb.s_blocks_per_group = blocks_per_group;
    sb.s_inodes_per_group = inodes_per_group;
    sb.s_group_count = group_count;
    sb.s_journal_inum = 0;
    sb.s_journal_blocks = 1024; // 4MB journal
    sb.s_journal_first_block = 0;
    
    // Set volume label if provided
    if (label) {
        strncpy(sb.s_volume_name, label, sizeof(sb.s_volume_name) - 1);
        sb.s_volume_name[sizeof(sb.s_volume_name) - 1] = '\0';
    }
    
    // Generate simple UUID (not cryptographically secure)
    for (int i = 0; i < 16; i++) {
        sb.s_uuid[i] = rand() & 0xFF;
    }
    
    // Calculate checksum
    sb.s_checksum = 0;
    sb.s_checksum = simple_crc32(&sb, sizeof(sb));
    
    printf("Writing superblock...\n");
    
    // Write superblock to block 0
    if (lseek(fd, 0, SEEK_SET) < 0) {
        fprintf(stderr, "Error: Cannot seek to start: %s\n", strerror(errno));
        close(fd);
        return 1;
    }
    
    if (write(fd, &sb, sizeof(sb)) != sizeof(sb)) {
        fprintf(stderr, "Error: Cannot write superblock: %s\n", strerror(errno));
        close(fd);
        return 1;
    }
    
    // Pad the rest of the block with zeros
    char zero_block[VEXFS_DEFAULT_BLOCK_SIZE];
    memset(zero_block, 0, sizeof(zero_block));
    
    size_t remaining = VEXFS_DEFAULT_BLOCK_SIZE - sizeof(sb);
    if (write(fd, zero_block, remaining) != remaining) {
        fprintf(stderr, "Error: Cannot pad superblock: %s\n", strerror(errno));
        close(fd);
        return 1;
    }
    
    // Sync to ensure data is written
    if (fsync(fd) < 0) {
        fprintf(stderr, "Error: Cannot sync: %s\n", strerror(errno));
        close(fd);
        return 1;
    }
    
    close(fd);
    
    printf("VexFS filesystem created successfully!\n");
    printf("Magic: 0x%llx\n", (unsigned long long)sb.s_magic);
    printf("Version: %u.%u\n", sb.s_version_major, sb.s_version_minor);
    if (label) {
        printf("Label: %s\n", label);
    }
    
    return 0;
}
EOF

echo "Compiling mkfs.vexfs..."
gcc -o /tmp/mkfs_vexfs_simple /tmp/mkfs_vexfs_simple.c

echo "mkfs.vexfs utility created at /tmp/mkfs_vexfs_simple"
echo "Usage: /tmp/mkfs_vexfs_simple [-f] [-L label] <device>"