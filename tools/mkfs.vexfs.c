/*
 * mkfs.vexfs - VexFS Filesystem Formatter
 *
 * This utility creates a VexFS filesystem on a block device with proper
 * on-disk layout including superblock, bitmaps, inode table, and root directory.
 */

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <time.h>
#include <errno.h>
#include <getopt.h>
#include <sys/ioctl.h>
#include <linux/fs.h>
#include <endian.h>

/* VexFS Constants */
#define VEXFS_MAGIC 0x56455846  /* "VEXF" */
#define VEXFS_BLOCK_SIZE 4096
#define VEXFS_BLOCK_SIZE_BITS 12
#define VEXFS_ROOT_INO 1
#define VEXFS_BITMAP_BLOCKS 1
#define VEXFS_INODE_TABLE_BLOCKS 64
#define VEXFS_MAX_INODES 1024
#define VEXFS_DIRECT_BLOCKS 12
#define VEXFS_MAX_NAME_LEN 255

/* File type constants for directory entries */
#define VEXFS_FT_UNKNOWN     0
#define VEXFS_FT_REG_FILE    1
#define VEXFS_FT_DIR         2
#define VEXFS_FT_CHRDEV      3
#define VEXFS_FT_BLKDEV      4
#define VEXFS_FT_FIFO        5
#define VEXFS_FT_SOCK        6
#define VEXFS_FT_SYMLINK     7

/* On-disk superblock structure */
struct vexfs_super_block {
    uint32_t s_magic;           /* Magic number */
    uint32_t s_block_size;      /* Block size */
    uint32_t s_blocks_count;    /* Total blocks */
    uint32_t s_free_blocks;     /* Free blocks */
    uint32_t s_inodes_count;    /* Total inodes */
    uint32_t s_free_inodes;     /* Free inodes */
    uint32_t s_first_data_block; /* First data block */
    uint32_t s_log_block_size;  /* Block size = 1024 << s_log_block_size */
    uint32_t s_blocks_per_group; /* Blocks per group */
    uint32_t s_inodes_per_group; /* Inodes per group */
    uint32_t s_mtime;           /* Mount time */
    uint32_t s_wtime;           /* Write time */
    uint16_t s_mnt_count;       /* Mount count */
    uint16_t s_max_mnt_count;   /* Maximum mount count */
    uint16_t s_state;           /* Filesystem state */
    uint16_t s_errors;          /* Error handling */
    uint16_t s_minor_rev_level; /* Minor revision level */
    uint32_t s_lastcheck;       /* Last check time */
    uint32_t s_checkinterval;   /* Check interval */
    uint32_t s_creator_os;      /* Creator OS */
    uint32_t s_rev_level;       /* Revision level */
    uint16_t s_def_resuid;      /* Default reserved user ID */
    uint16_t s_def_resgid;      /* Default reserved group ID */
    uint32_t s_first_ino;       /* First non-reserved inode */
    uint16_t s_inode_size;      /* Inode size */
    uint16_t s_block_group_nr;  /* Block group number */
    uint32_t s_feature_compat;  /* Compatible features */
    uint32_t s_feature_incompat; /* Incompatible features */
    uint32_t s_feature_ro_compat; /* Read-only compatible features */
    uint8_t  s_uuid[16];        /* Filesystem UUID */
    char     s_volume_name[16]; /* Volume name */
    char     s_last_mounted[64]; /* Last mount point */
    uint32_t s_algorithm_usage_bitmap; /* Compression algorithms */
    uint8_t  s_prealloc_blocks; /* Preallocated blocks */
    uint8_t  s_prealloc_dir_blocks; /* Preallocated directory blocks */
    uint16_t s_reserved_gdt_blocks; /* Reserved GDT blocks */
    uint8_t  s_journal_uuid[16]; /* Journal UUID */
    uint32_t s_journal_inum;    /* Journal inode number */
    uint32_t s_journal_dev;     /* Journal device */
    uint32_t s_last_orphan;     /* Last orphaned inode */
    uint32_t s_hash_seed[4];    /* Hash seed */
    uint8_t  s_def_hash_version; /* Default hash version */
    uint8_t  s_jnl_backup_type; /* Journal backup type */
    uint16_t s_desc_size;       /* Group descriptor size */
    uint32_t s_default_mount_opts; /* Default mount options */
    uint32_t s_first_meta_bg;   /* First meta block group */
    uint32_t s_mkfs_time;       /* Filesystem creation time */
    uint32_t s_jnl_blocks[17];  /* Journal backup blocks */
    uint32_t s_blocks_count_hi; /* High 32 bits of block count */
    uint32_t s_r_blocks_count_hi; /* High 32 bits of reserved blocks */
    uint32_t s_free_blocks_count_hi; /* High 32 bits of free blocks */
    uint16_t s_min_extra_isize; /* Minimum extra inode size */
    uint16_t s_want_extra_isize; /* Desired extra inode size */
    uint32_t s_flags;           /* Miscellaneous flags */
    uint16_t s_raid_stride;     /* RAID stride */
    uint16_t s_mmp_update_interval; /* MMP update interval */
    uint64_t s_mmp_block;       /* MMP block number */
    uint32_t s_raid_stripe_width; /* RAID stripe width */
    uint8_t  s_log_groups_per_flex; /* Groups per flex group */
    uint8_t  s_checksum_type;   /* Metadata checksum algorithm */
    uint16_t s_reserved_pad;    /* Padding */
    uint64_t s_kbytes_written;  /* Kilobytes written */
    uint32_t s_snapshot_inum;   /* Snapshot inode number */
    uint32_t s_snapshot_id;     /* Snapshot ID */
    uint64_t s_snapshot_r_blocks_count; /* Reserved blocks for snapshot */
    uint32_t s_snapshot_list;   /* Snapshot list head */
    uint32_t s_error_count;     /* Error count */
    uint32_t s_first_error_time; /* First error time */
    uint32_t s_first_error_ino; /* First error inode */
    uint64_t s_first_error_block; /* First error block */
    uint8_t  s_first_error_func[32]; /* First error function */
    uint32_t s_first_error_line; /* First error line */
    uint32_t s_last_error_time; /* Last error time */
    uint32_t s_last_error_ino;  /* Last error inode */
    uint32_t s_last_error_line; /* Last error line */
    uint64_t s_last_error_block; /* Last error block */
    uint8_t  s_last_error_func[32]; /* Last error function */
    uint8_t  s_mount_opts[64];  /* Mount options */
    uint32_t s_usr_quota_inum;  /* User quota inode */
    uint32_t s_grp_quota_inum;  /* Group quota inode */
    uint32_t s_overhead_clusters; /* Overhead clusters */
    uint32_t s_backup_bgs[2];   /* Backup block groups */
    uint8_t  s_encrypt_algos[4]; /* Encryption algorithms */
    uint8_t  s_encrypt_pw_salt[16]; /* Encryption password salt */
    uint32_t s_lpf_ino;         /* Lost+found inode */
    uint32_t s_prj_quota_inum;  /* Project quota inode */
    uint32_t s_checksum_seed;   /* Checksum seed */
    uint8_t  s_wtime_hi;        /* High byte of write time */
    uint8_t  s_mtime_hi;        /* High byte of mount time */
    uint8_t  s_mkfs_time_hi;    /* High byte of mkfs time */
    uint8_t  s_lastcheck_hi;    /* High byte of last check time */
    uint8_t  s_first_error_time_hi; /* High byte of first error time */
    uint8_t  s_last_error_time_hi; /* High byte of last error time */
    uint8_t  s_pad[2];          /* Padding */
    uint16_t s_encoding;        /* Filename charset encoding */
    uint16_t s_encoding_flags;  /* Filename charset encoding flags */
    uint32_t s_reserved[95];    /* Reserved for future use */
    uint32_t s_checksum;        /* Superblock checksum */
};

/* On-disk inode structure - FIXED to match kernel module layout */
struct vexfs_inode {
    uint16_t i_mode;            /* File mode */
    uint16_t i_links_count;     /* Links count */
    uint32_t i_uid;             /* Owner UID - 32-bit to match kernel */
    uint32_t i_gid;             /* Group ID - 32-bit to match kernel */
    uint64_t i_size;            /* File size - 64-bit to match kernel */
    uint32_t i_atime;           /* Access time */
    uint32_t i_ctime;           /* Creation time */
    uint32_t i_mtime;           /* Modification time */
    uint32_t i_blocks;          /* Blocks count */
    uint32_t i_block[12];       /* Pointers to blocks - 12 blocks to match kernel */
    uint32_t i_flags;           /* File flags */
    uint32_t i_generation;      /* File version */
    uint32_t i_reserved[3];     /* Reserved for future use */
};

/* Directory entry structure */
struct vexfs_dir_entry {
    uint32_t inode;             /* Inode number */
    uint16_t rec_len;           /* Directory entry length */
    uint8_t  name_len;          /* Name length */
    uint8_t  file_type;         /* File type */
    char     name[];            /* File name */
};

/* Global variables */
static int verbose = 0;
static int force = 0;
static char *volume_label = NULL;

/* Function prototypes */
static void usage(const char *progname);
static int check_device(const char *device);
static int write_superblock(int fd, uint64_t total_blocks);
static int write_block_bitmap(int fd, uint64_t total_blocks);
static int write_inode_table(int fd);
static int write_root_directory(int fd);
static int verify_filesystem(int fd);
static void print_filesystem_info(uint64_t total_blocks);

/**
 * Print usage information
 */
static void usage(const char *progname)
{
    printf("Usage: %s [options] <device>\n", progname);
    printf("\nOptions:\n");
    printf("  -f, --force        Force formatting even if device appears to contain data\n");
    printf("  -L, --label LABEL  Set volume label\n");
    printf("  -v, --verbose      Verbose output\n");
    printf("  -h, --help         Show this help message\n");
    printf("\nExamples:\n");
    printf("  %s /dev/sdb1\n", progname);
    printf("  %s -L \"MyVexFS\" -v /dev/loop0\n", progname);
    printf("\n");
}

/**
 * Check if device is suitable for formatting
 */
static int check_device(const char *device)
{
    struct stat st;
    int fd;
    char buffer[VEXFS_BLOCK_SIZE];
    
    if (stat(device, &st) < 0) {
        perror("stat");
        return -1;
    }
    
    if (!S_ISBLK(st.st_mode) && !S_ISREG(st.st_mode)) {
        fprintf(stderr, "Error: %s is not a block device or regular file\n", device);
        return -1;
    }
    
    fd = open(device, O_RDONLY);
    if (fd < 0) {
        perror("open");
        return -1;
    }
    
    /* Check if device already contains data */
    if (!force && read(fd, buffer, VEXFS_BLOCK_SIZE) == VEXFS_BLOCK_SIZE) {
        int i, has_data = 0;
        for (i = 0; i < VEXFS_BLOCK_SIZE; i++) {
            if (buffer[i] != 0) {
                has_data = 1;
                break;
            }
        }
        
        if (has_data) {
            fprintf(stderr, "Error: Device appears to contain data. Use -f to force formatting.\n");
            close(fd);
            return -1;
        }
    }
    
    close(fd);
    return 0;
}

/**
 * Write VexFS superblock
 */
static int write_superblock(int fd, uint64_t total_blocks)
{
    struct vexfs_super_block sb;
    time_t now = time(NULL);
    
    memset(&sb, 0, sizeof(sb));
    
    /* Basic filesystem parameters - convert to little-endian for disk storage */
    sb.s_magic = htole32(VEXFS_MAGIC);  /* Magic number must be little-endian for kernel */
    sb.s_block_size = htole32(VEXFS_BLOCK_SIZE);
    sb.s_blocks_count = htole32(total_blocks);
    sb.s_free_blocks = htole32(total_blocks - (1 + VEXFS_BITMAP_BLOCKS + VEXFS_INODE_TABLE_BLOCKS + 1)); /* -superblock -bitmap -inode_table -root_dir */
    sb.s_inodes_count = htole32(VEXFS_MAX_INODES);
    sb.s_free_inodes = htole32(VEXFS_MAX_INODES - 1); /* Root inode is used */
    sb.s_first_data_block = htole32(1 + VEXFS_BITMAP_BLOCKS + VEXFS_INODE_TABLE_BLOCKS);
    sb.s_log_block_size = htole32(VEXFS_BLOCK_SIZE_BITS - 10); /* 4096 = 1024 << 2 */
    sb.s_blocks_per_group = htole32(8192);
    sb.s_inodes_per_group = htole32(VEXFS_MAX_INODES);
    
    /* Timestamps - convert to little-endian */
    sb.s_mkfs_time = htole32(now);
    sb.s_wtime = htole32(now);
    sb.s_mtime = htole32(0);
    sb.s_lastcheck = htole32(now);
    
    /* State and error handling - convert to little-endian */
    sb.s_mnt_count = htole16(0);
    sb.s_max_mnt_count = htole16(20);
    sb.s_state = htole16(1); /* Clean */
    sb.s_errors = htole16(1); /* Continue on errors */
    sb.s_minor_rev_level = htole16(0);
    sb.s_checkinterval = htole32(0);
    sb.s_creator_os = htole32(0); /* Linux */
    sb.s_rev_level = htole32(1);
    
    /* Default values - convert to little-endian */
    sb.s_def_resuid = htole16(0);
    sb.s_def_resgid = htole16(0);
    sb.s_first_ino = htole32(11);
    sb.s_inode_size = htole16(sizeof(struct vexfs_inode));
    sb.s_block_group_nr = htole16(0);
    
    /* Volume label */
    if (volume_label) {
        strncpy(sb.s_volume_name, volume_label, sizeof(sb.s_volume_name) - 1);
    } else {
        strcpy(sb.s_volume_name, "VexFS");
    }
    
    /* Write superblock to block 0 */
    if (lseek(fd, 0, SEEK_SET) < 0) {
        perror("lseek");
        return -1;
    }
    
    if (write(fd, &sb, sizeof(sb)) != sizeof(sb)) {
        perror("write superblock");
        return -1;
    }
    
    if (verbose) {
        printf("Superblock written:\n");
        printf("  Magic: 0x%08x (stored as little-endian)\n", VEXFS_MAGIC);
        printf("  Block size: %u bytes\n", VEXFS_BLOCK_SIZE);
        printf("  Total blocks: %u\n", (uint32_t)total_blocks);
        printf("  Free blocks: %u\n", (uint32_t)(total_blocks - (1 + VEXFS_BITMAP_BLOCKS + VEXFS_INODE_TABLE_BLOCKS + 1)));
        printf("  Total inodes: %u\n", VEXFS_MAX_INODES);
        printf("  Free inodes: %u\n", VEXFS_MAX_INODES - 1);
        printf("  First data block: %u\n", 1 + VEXFS_BITMAP_BLOCKS + VEXFS_INODE_TABLE_BLOCKS);
        printf("  Volume label: %s\n", sb.s_volume_name);
    }
    
    return 0;
}

/**
 * Write block bitmap
 */
static int write_block_bitmap(int fd, uint64_t total_blocks)
{
    uint8_t *bitmap;
    uint32_t bitmap_size = VEXFS_BLOCK_SIZE;
    uint32_t used_blocks = 1 + VEXFS_BITMAP_BLOCKS + VEXFS_INODE_TABLE_BLOCKS + 1; /* superblock + bitmap + inode_table + root_dir */
    uint32_t i;
    
    bitmap = calloc(1, bitmap_size);
    if (!bitmap) {
        perror("malloc bitmap");
        return -1;
    }
    
    /* Mark used blocks in bitmap */
    for (i = 0; i < used_blocks && i < total_blocks; i++) {
        bitmap[i / 8] |= (1 << (i % 8));
    }
    
    /* Write bitmap to block 1 */
    if (lseek(fd, VEXFS_BLOCK_SIZE, SEEK_SET) < 0) {
        perror("lseek bitmap");
        free(bitmap);
        return -1;
    }
    
    if (write(fd, bitmap, bitmap_size) != bitmap_size) {
        perror("write bitmap");
        free(bitmap);
        return -1;
    }
    
    if (verbose) {
        printf("Block bitmap written:\n");
        printf("  Used blocks marked: %u\n", used_blocks);
        printf("  Bitmap size: %u bytes\n", bitmap_size);
    }
    
    free(bitmap);
    return 0;
}

/**
 * Write inode table
 */
static int write_inode_table(int fd)
{
    struct vexfs_inode *inode_table;
    uint32_t table_size = VEXFS_INODE_TABLE_BLOCKS * VEXFS_BLOCK_SIZE;
    time_t now = time(NULL);
    
    inode_table = calloc(1, table_size);
    if (!inode_table) {
        perror("malloc inode table");
        return -1;
    }
    
    /* Initialize root inode (inode #1) - FIXED: Match kernel module structure layout */
    struct vexfs_inode *root_inode = &inode_table[VEXFS_ROOT_INO - 1];
    root_inode->i_mode = htole16(S_IFDIR | 0755); /* S_IFDIR | 0755 */
    root_inode->i_links_count = htole16(2); /* . and .. */
    root_inode->i_uid = htole32(0);
    root_inode->i_gid = htole32(0);
    root_inode->i_size = htole64(VEXFS_BLOCK_SIZE);
    root_inode->i_atime = htole32(now);
    root_inode->i_ctime = htole32(now);
    root_inode->i_mtime = htole32(now);
    root_inode->i_blocks = htole32(1);
    root_inode->i_block[0] = htole32(1 + VEXFS_BITMAP_BLOCKS + VEXFS_INODE_TABLE_BLOCKS); /* First data block */
    root_inode->i_flags = htole32(0);
    root_inode->i_generation = htole32(1);
    /* Clear reserved fields */
    root_inode->i_reserved[0] = htole32(0);
    root_inode->i_reserved[1] = htole32(0);
    root_inode->i_reserved[2] = htole32(0);
    
    /* Write inode table starting at block 2 */
    if (lseek(fd, (1 + VEXFS_BITMAP_BLOCKS) * VEXFS_BLOCK_SIZE, SEEK_SET) < 0) {
        perror("lseek inode table");
        free(inode_table);
        return -1;
    }
    
    if (write(fd, inode_table, table_size) != table_size) {
        perror("write inode table");
        free(inode_table);
        return -1;
    }
    
    if (verbose) {
        printf("Inode table written:\n");
        printf("  Table size: %u bytes (%u blocks)\n", table_size, VEXFS_INODE_TABLE_BLOCKS);
        printf("  Root inode initialized (inode #%u)\n", VEXFS_ROOT_INO);
        printf("  Root inode mode: 0%o\n", root_inode->i_mode);
        printf("  Root inode size: %lu bytes\n", (unsigned long)root_inode->i_size);
        printf("  Root inode data block: %u\n", root_inode->i_block[0]);
    }
    
    free(inode_table);
    return 0;
}

/**
 * Write root directory
 */
static int write_root_directory(int fd)
{
    uint8_t *dir_block;
    struct vexfs_dir_entry *entry;
    uint32_t offset = 0;
    uint32_t root_data_block = 1 + VEXFS_BITMAP_BLOCKS + VEXFS_INODE_TABLE_BLOCKS;
    
    dir_block = calloc(1, VEXFS_BLOCK_SIZE);
    if (!dir_block) {
        perror("malloc directory block");
        return -1;
    }
    
    /* Create "." entry */
    entry = (struct vexfs_dir_entry *)(dir_block + offset);
    entry->inode = VEXFS_ROOT_INO;
    entry->name_len = 1;
    entry->file_type = VEXFS_FT_DIR;
    entry->rec_len = 12; /* 8 bytes header + 1 byte name + 3 bytes padding */
    strcpy(entry->name, ".");
    offset += entry->rec_len;
    
    /* Create ".." entry */
    entry = (struct vexfs_dir_entry *)(dir_block + offset);
    entry->inode = VEXFS_ROOT_INO; /* Root's parent is itself */
    entry->name_len = 2;
    entry->file_type = VEXFS_FT_DIR;
    entry->rec_len = VEXFS_BLOCK_SIZE - offset; /* Rest of the block */
    strcpy(entry->name, "..");
    
    /* Write root directory to its data block */
    if (lseek(fd, root_data_block * VEXFS_BLOCK_SIZE, SEEK_SET) < 0) {
        perror("lseek root directory");
        free(dir_block);
        return -1;
    }
    
    if (write(fd, dir_block, VEXFS_BLOCK_SIZE) != VEXFS_BLOCK_SIZE) {
        perror("write root directory");
        free(dir_block);
        return -1;
    }
    
    if (verbose) {
        printf("Root directory written:\n");
        printf("  Directory block: %u\n", root_data_block);
        printf("  Entries: . and ..\n");
        printf("  Directory size: %u bytes\n", VEXFS_BLOCK_SIZE);
    }
    
    free(dir_block);
    return 0;
}

/**
 * Verify the created filesystem
 */
static int verify_filesystem(int fd)
{
    struct vexfs_super_block sb;
    
    /* Read and verify superblock */
    if (lseek(fd, 0, SEEK_SET) < 0) {
        perror("lseek verify");
        return -1;
    }
    
    if (read(fd, &sb, sizeof(sb)) != sizeof(sb)) {
        perror("read verify superblock");
        return -1;
    }
    
    /* Convert from little-endian for verification */
    uint32_t magic = le32toh(sb.s_magic);
    uint32_t block_size = le32toh(sb.s_block_size);
    
    if (magic != VEXFS_MAGIC) {
        fprintf(stderr, "Verification failed: Invalid magic number 0x%08x (expected 0x%08x)\n", magic, VEXFS_MAGIC);
        return -1;
    }
    
    if (block_size != VEXFS_BLOCK_SIZE) {
        fprintf(stderr, "Verification failed: Invalid block size %u\n", block_size);
        return -1;
    }
    
    if (verbose) {
        printf("Filesystem verification: PASSED\n");
        printf("  Magic number: 0x%08x (correct)\n", magic);
        printf("  Block size: %u bytes (correct)\n", block_size);
        printf("  Total blocks: %u\n", le32toh(sb.s_blocks_count));
        printf("  Free blocks: %u\n", le32toh(sb.s_free_blocks));
    }
    
    return 0;
}

/**
 * Print filesystem information
 */
static void print_filesystem_info(uint64_t total_blocks)
{
    uint64_t total_size = total_blocks * VEXFS_BLOCK_SIZE;
    uint32_t used_blocks = 1 + VEXFS_BITMAP_BLOCKS + VEXFS_INODE_TABLE_BLOCKS + 1;
    uint64_t used_size = used_blocks * VEXFS_BLOCK_SIZE;
    uint64_t available_size = total_size - used_size;
    
    printf("\nVexFS filesystem created successfully!\n");
    printf("\nFilesystem Information:\n");
    printf("  Filesystem type: VexFS\n");
    printf("  Block size: %u bytes\n", VEXFS_BLOCK_SIZE);
    printf("  Total size: %llu bytes (%.2f MB)\n", 
           (unsigned long long)total_size, 
           (double)total_size / (1024 * 1024));
    printf("  Available space: %llu bytes (%.2f MB)\n", 
           (unsigned long long)available_size, 
           (double)available_size / (1024 * 1024));
    printf("  Total blocks: %llu\n", (unsigned long long)total_blocks);
    printf("  Used blocks: %u (metadata)\n", used_blocks);
    printf("  Available blocks: %llu\n", (unsigned long long)(total_blocks - used_blocks));
    printf("  Total inodes: %u\n", VEXFS_MAX_INODES);
    printf("  Available inodes: %u\n", VEXFS_MAX_INODES - 1);
    
    if (volume_label) {
        printf("  Volume label: %s\n", volume_label);
    }
    
    printf("\nLayout:\n");
    printf("  Block 0: Superblock\n");
    printf("  Block 1: Block bitmap\n");
    printf("  Blocks 2-%u: Inode table\n", 1 + VEXFS_INODE_TABLE_BLOCKS);
    printf("  Block %u+: Data blocks\n", 1 + VEXFS_BITMAP_BLOCKS + VEXFS_INODE_TABLE_BLOCKS);
    
    printf("\nTo mount this filesystem:\n");
    printf("  sudo mount -t vexfs_fixed <device> <mountpoint>\n");
}

/**
 * Main function
 */
int main(int argc, char *argv[])
{
    int fd;
    const char *device;
    struct stat st;
    uint64_t total_blocks;
    int opt;
    
    static struct option long_options[] = {
        {"force", no_argument, 0, 'f'},
        {"label", required_argument, 0, 'L'},
        {"verbose", no_argument, 0, 'v'},
        {"help", no_argument, 0, 'h'},
        {0, 0, 0, 0}
    };
    
    /* Parse command line options */
    while ((opt = getopt_long(argc, argv, "fL:vh", long_options, NULL)) != -1) {
        switch (opt) {
        case 'f':
            force = 1;
            break;
        case 'L':
            volume_label = optarg;
            if (strlen(volume_label) >= 16) {
                fprintf(stderr, "Error: Volume label too long (max 15 characters)\n");
                exit(1);
            }
            break;
        case 'v':
            verbose = 1;
            break;
        case 'h':
            usage(argv[0]);
            exit(0);
        default:
            usage(argv[0]);
            exit(1);
        }
    }
    
    if (optind >= argc) {
        fprintf(stderr, "Error: Device not specified\n");
        usage(argv[0]);
        exit(1);
    }
    
    device = argv[optind];
    
    printf("mkfs.vexfs - VexFS Filesystem Formatter\n");
    printf("Device: %s\n", device);
    
    /* Check device */
    if (check_device(device) < 0) {
        exit(1);
    }
    
    /* Get device size */
    if (stat(device, &st) < 0) {
        perror("stat");
        exit(1);
    }
    
    fd = open(device, O_RDWR);
    if (fd < 0) {
        perror("open");
        exit(1);
    }
    
    /* Calculate total blocks */
    if (S_ISBLK(st.st_mode)) {
        /* Block device - get size via ioctl */
        uint64_t size;
        if (ioctl(fd, BLKGETSIZE64, &size) < 0) {
            perror("ioctl BLKGETSIZE64");
            close(fd);
            exit(1);
        }
        total_blocks = size / VEXFS_BLOCK_SIZE;
    } else {
        /* Regular file */
        total_blocks = st.st_size / VEXFS_BLOCK_SIZE;
    }
    
    if (total_blocks < 100) {
        fprintf(stderr, "Error: Device too small (minimum 100 blocks = %u bytes)\n",
                100 * VEXFS_BLOCK_SIZE);
        close(fd);
        exit(1);
    }
    
    if (verbose) {
        printf("Device size: %llu blocks (%llu bytes)\n",
               (unsigned long long)total_blocks,
               (unsigned long long)(total_blocks * VEXFS_BLOCK_SIZE));
    }
    
    /* Confirm formatting */
    if (!force) {
        printf("This will destroy all data on %s.\n", device);
        printf("Use -f/--force to skip this check.\n");
        printf("Proceeding with formatting...\n");
    }
    
    printf("Creating VexFS filesystem...\n");
    
    /* Write filesystem structures */
    if (write_superblock(fd, total_blocks) < 0) {
        fprintf(stderr, "Failed to write superblock\n");
        close(fd);
        exit(1);
    }
    
    if (write_block_bitmap(fd, total_blocks) < 0) {
        fprintf(stderr, "Failed to write block bitmap\n");
        close(fd);
        exit(1);
    }
    
    if (write_inode_table(fd) < 0) {
        fprintf(stderr, "Failed to write inode table\n");
        close(fd);
        exit(1);
    }
    
    if (write_root_directory(fd) < 0) {
        fprintf(stderr, "Failed to write root directory\n");
        close(fd);
        exit(1);
    }
    
    /* Sync to disk */
    if (fsync(fd) < 0) {
        perror("fsync");
        close(fd);
        exit(1);
    }
    
    /* Verify filesystem */
    if (verify_filesystem(fd) < 0) {
        fprintf(stderr, "Filesystem verification failed\n");
        close(fd);
        exit(1);
    }
    
    close(fd);
    
    /* Print success information */
    print_filesystem_info(total_blocks);
    
    return 0;
}