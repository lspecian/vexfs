/*
 * VexFS v2.0 - Metadata Journaling (Task 3)
 * 
 * This implements comprehensive metadata journaling for VexFS as part of the
 * AI-Native Semantic Substrate roadmap (Phase 1). Builds on the Full FS Journal
 * (Task 1) and Atomic Operations (Task 2) to provide complete metadata integrity
 * and crash recovery for all VexFS metadata structures.
 *
 * Key Features:
 * - Inode metadata journaling with vector-specific fields
 * - Directory entry journaling for namespace operations
 * - Allocation bitmap journaling for space management
 * - Vector metadata journaling for AI-native operations
 * - Ordered writes for metadata-data consistency
 * - Kernel-compatible serialization framework
 * - Integrity verification with checksums
 * - Performance optimization through metadata caching
 */

#ifndef _VEXFS_V2_METADATA_JOURNAL_H
#define _VEXFS_V2_METADATA_JOURNAL_H

#include <linux/types.h>
#include <linux/spinlock.h>
#include <linux/mutex.h>
#include <linux/atomic.h>
#include <linux/workqueue.h>
#include <linux/completion.h>
#include <linux/rbtree.h>
#include <linux/list.h>
#include <linux/buffer_head.h>
#include <linux/fs.h>
#include <linux/dcache.h>
#include <linux/crc32.h>

#include "vexfs_v2_journal.h"
#include "vexfs_v2_atomic.h"

/* Metadata operation types for journaling */
#define VEXFS_META_OP_INODE_CREATE      0x01
#define VEXFS_META_OP_INODE_DELETE      0x02
#define VEXFS_META_OP_INODE_UPDATE      0x03
#define VEXFS_META_OP_DENTRY_CREATE     0x04
#define VEXFS_META_OP_DENTRY_DELETE     0x05
#define VEXFS_META_OP_DENTRY_RENAME     0x06
#define VEXFS_META_OP_BITMAP_ALLOC      0x07
#define VEXFS_META_OP_BITMAP_FREE       0x08
#define VEXFS_META_OP_VECTOR_META       0x09
#define VEXFS_META_OP_INDEX_UPDATE      0x0A
#define VEXFS_META_OP_SUPERBLOCK        0x0B

/* Metadata journaling flags */
#define VEXFS_META_JOURNAL_SYNC         0x01
#define VEXFS_META_JOURNAL_ASYNC        0x02
#define VEXFS_META_JOURNAL_ORDERED      0x04
#define VEXFS_META_JOURNAL_BATCH        0x08
#define VEXFS_META_JOURNAL_CHECKSUM     0x10

/* Metadata serialization types */
#define VEXFS_META_SERIAL_INODE         0x01
#define VEXFS_META_SERIAL_DENTRY        0x02
#define VEXFS_META_SERIAL_BITMAP        0x03
#define VEXFS_META_SERIAL_VECTOR        0x04
#define VEXFS_META_SERIAL_SUPERBLOCK    0x05

/* Maximum values for metadata journaling */
#define VEXFS_META_MAX_BATCH_SIZE       128
#define VEXFS_META_MAX_CACHE_ENTRIES    1024
#define VEXFS_META_MAX_PENDING_OPS      512
#define VEXFS_META_CHECKSUM_SIZE        4

/*
 * Serialized inode metadata for journaling
 */
struct vexfs_meta_serialized_inode {
    __le64 ino;                         /* Inode number */
    __le32 mode;                        /* File mode */
    __le32 uid;                         /* User ID */
    __le32 gid;                         /* Group ID */
    __le64 size;                        /* File size */
    __le64 blocks;                      /* Block count */
    __le64 atime_sec;                   /* Access time seconds */
    __le32 atime_nsec;                  /* Access time nanoseconds */
    __le64 mtime_sec;                   /* Modify time seconds */
    __le32 mtime_nsec;                  /* Modify time nanoseconds */
    __le64 ctime_sec;                   /* Change time seconds */
    __le32 ctime_nsec;                  /* Change time nanoseconds */
    __le64 crtime_sec;                  /* Creation time seconds */
    __le32 crtime_nsec;                 /* Creation time nanoseconds */
    
    /* VexFS-specific inode fields */
    __le32 i_flags;                     /* Inode flags */
    __le32 i_block[15];                 /* Block pointers */
    
    /* Vector-specific metadata */
    __u8 is_vector_file;                /* Vector file flag */
    __u8 vector_element_type;           /* Element type */
    __le16 vector_dimensions;           /* Vector dimensions */
    __le32 vector_count;                /* Number of vectors */
    __le32 vector_alignment;            /* SIMD alignment */
    __le32 vectors_per_block;           /* Vectors per block */
    __le64 vector_data_size;            /* Vector data size */
    __le64 hnsw_graph_block;            /* HNSW graph block */
    __le64 pq_codebook_block;           /* PQ codebook block */
    __le32 hnsw_max_connections;        /* HNSW M parameter */
    __le32 hnsw_ef_construction;        /* HNSW efConstruction */
    __le32 vector_flags;                /* Vector flags */
    __le32 access_pattern;              /* Access pattern */
    __le32 storage_format;              /* Storage format */
    __le32 compression_type;            /* Compression type */
    __le64 data_offset;                 /* Data offset */
    __le64 index_offset;                /* Index offset */
    
    /* Checksum and validation */
    __le32 checksum;                    /* Metadata checksum */
    __le32 reserved[4];                 /* Reserved for future use */
} __packed;

/*
 * Serialized directory entry metadata for journaling
 */
struct vexfs_meta_serialized_dentry {
    __le64 parent_ino;                  /* Parent inode number */
    __le64 child_ino;                   /* Child inode number */
    __le32 name_len;                    /* Name length */
    __le32 entry_type;                  /* Entry type (file, dir, etc.) */
    __le64 hash;                        /* Name hash for fast lookup */
    
    /* Variable-length name follows */
    char name[0];                       /* Entry name (null-terminated) */
} __packed;

/*
 * Serialized allocation bitmap metadata for journaling
 */
struct vexfs_meta_serialized_bitmap {
    __le64 block_group;                 /* Block group number */
    __le64 start_block;                 /* First block in range */
    __le32 block_count;                 /* Number of blocks */
    __le32 operation;                   /* Allocation or free */
    __le64 free_blocks_before;          /* Free blocks before operation */
    __le64 free_blocks_after;           /* Free blocks after operation */
    __le32 checksum;                    /* Bitmap checksum */
    __le32 reserved[3];                 /* Reserved */
} __packed;

/*
 * Serialized vector metadata for journaling
 */
struct vexfs_meta_serialized_vector {
    __le64 vector_id;                   /* Vector ID */
    __le64 inode_number;                /* Associated inode */
    __le32 dimensions;                  /* Vector dimensions */
    __le32 element_type;                /* Element type */
    __le64 data_block;                  /* Data block location */
    __le32 data_offset;                 /* Offset within block */
    __le32 flags;                       /* Vector flags */
    __le64 timestamp;                   /* Creation/modification time */
    __le32 checksum;                    /* Vector metadata checksum */
    __le32 reserved[3];                 /* Reserved */
} __packed;

/*
 * Serialized superblock metadata for journaling
 */
struct vexfs_meta_serialized_superblock {
    __le32 magic;                       /* Filesystem magic */
    __le32 version_major;               /* Major version */
    __le32 version_minor;               /* Minor version */
    __le32 version_patch;               /* Patch version */
    __le64 block_count;                 /* Total blocks */
    __le64 free_blocks;                 /* Free blocks */
    __le64 inode_count;                 /* Total inodes */
    __le64 free_inodes;                 /* Free inodes */
    
    /* Vector-specific superblock fields */
    __le16 default_vector_dim;          /* Default vector dimensions */
    __u8 default_element_type;          /* Default element type */
    __u8 vector_alignment;              /* Vector alignment */
    __le64 hnsw_index_block;            /* HNSW index root */
    __le64 pq_index_block;              /* PQ codebook */
    __le64 ivf_index_block;             /* IVF cluster centers */
    __le64 vector_meta_block;           /* Vector metadata table */
    __le32 max_collections;             /* Max collections */
    __le32 active_collections;          /* Active collections */
    __le64 collection_table_block;      /* Collection metadata */
    
    /* Journal metadata */
    __le64 journal_start_block;         /* Journal start */
    __le64 journal_total_blocks;        /* Journal size */
    __le32 journal_flags;               /* Journal flags */
    __le32 journal_version;             /* Journal version */
    
    __le32 checksum;                    /* Superblock checksum */
    __le32 reserved[8];                 /* Reserved */
} __packed;

/*
 * Metadata operation descriptor
 */
struct vexfs_metadata_operation {
    u32 op_type;                        /* Operation type */
    u32 op_flags;                       /* Operation flags */
    u64 op_id;                          /* Unique operation ID */
    u64 transaction_id;                 /* Associated transaction ID */
    
    /* Target metadata information */
    union {
        struct inode *target_inode;     /* Target inode */
        struct dentry *target_dentry;   /* Target dentry */
        u64 target_block;               /* Target block number */
    };
    
    /* Serialized metadata */
    void *serialized_data;              /* Serialized metadata */
    size_t serialized_size;             /* Size of serialized data */
    u32 serialized_type;                /* Type of serialized data */
    
    /* Before/after state for rollback */
    void *before_state;                 /* State before operation */
    void *after_state;                  /* State after operation */
    size_t state_size;                  /* Size of state data */
    
    /* Integrity verification */
    u32 metadata_checksum;              /* Metadata checksum */
    u32 operation_checksum;             /* Operation checksum */
    
    /* Timing and ordering */
    u64 sequence_number;                /* Sequence number for ordering */
    unsigned long timestamp;            /* Operation timestamp */
    
    /* Completion tracking */
    struct completion op_completion;    /* Operation completion */
    atomic_t op_state;                  /* Operation state */
    int op_result;                      /* Operation result */
    
    /* List management */
    struct list_head op_list;           /* List of operations */
    struct rcu_head rcu;                /* RCU callback head */
};

/*
 * Metadata cache entry for performance optimization
 */
struct vexfs_metadata_cache_entry {
    u64 key;                            /* Cache key (inode number, etc.) */
    u32 entry_type;                     /* Type of cached metadata */
    void *cached_data;                  /* Cached metadata */
    size_t data_size;                   /* Size of cached data */
    
    /* Cache management */
    unsigned long access_time;          /* Last access time */
    atomic_t ref_count;                 /* Reference count */
    u32 flags;                          /* Cache entry flags */
    
    /* Integrity verification */
    u32 checksum;                       /* Cached data checksum */
    
    /* Tree management */
    struct rb_node rb_node;             /* Red-black tree node */
    struct list_head lru_list;          /* LRU list */
    struct rcu_head rcu;                /* RCU callback head */
};

/*
 * Metadata journaling manager
 */
struct vexfs_metadata_journal_manager {
    /* Journal integration */
    struct vexfs_journal *journal;      /* Associated journal */
    struct vexfs_atomic_manager *atomic_mgr; /* Atomic operations manager */
    
    /* Operation management */
    struct list_head pending_ops;       /* Pending operations */
    struct mutex ops_mutex;             /* Operations list mutex */
    atomic_t pending_count;             /* Number of pending operations */
    u64 next_op_id;                     /* Next operation ID */
    
    /* Batch processing */
    struct workqueue_struct *batch_workqueue; /* Batch processing workqueue */
    struct delayed_work batch_work;     /* Batch processing work */
    u32 batch_size;                     /* Current batch size */
    u32 max_batch_size;                 /* Maximum batch size */
    
    /* Metadata cache */
    struct rb_root cache_tree;          /* Cache tree root */
    struct list_head cache_lru;         /* Cache LRU list */
    struct mutex cache_mutex;           /* Cache mutex */
    atomic_t cache_entries;             /* Number of cache entries */
    u32 max_cache_entries;              /* Maximum cache entries */
    
    /* Performance optimization */
    atomic64_t ops_processed;           /* Total operations processed */
    atomic64_t cache_hits;              /* Cache hits */
    atomic64_t cache_misses;            /* Cache misses */
    atomic64_t bytes_journaled;         /* Total bytes journaled */
    
    /* Serialization support */
    struct kmem_cache *inode_serial_cache; /* Inode serialization cache */
    struct kmem_cache *dentry_serial_cache; /* Dentry serialization cache */
    struct kmem_cache *bitmap_serial_cache; /* Bitmap serialization cache */
    struct kmem_cache *vector_serial_cache; /* Vector serialization cache */
    
    /* Memory management */
    struct kmem_cache *op_cache;        /* Operation allocation cache */
    struct kmem_cache *cache_entry_cache; /* Cache entry allocation cache */
    
    /* Configuration */
    u32 journal_flags;                  /* Journaling flags */
    u32 sync_mode;                      /* Synchronization mode */
    u32 batch_timeout;                  /* Batch timeout in ms */
    
    /* Statistics */
    atomic64_t inode_ops;               /* Inode operations */
    atomic64_t dentry_ops;              /* Directory entry operations */
    atomic64_t bitmap_ops;              /* Bitmap operations */
    atomic64_t vector_ops;              /* Vector metadata operations */
    atomic64_t checksum_errors;         /* Checksum errors detected */
    
    /* Error handling */
    atomic_t error_count;               /* Total error count */
    struct list_head error_log;         /* Error log entries */
    
    /* Synchronization */
    struct rw_semaphore manager_rwsem;  /* Manager read-write semaphore */
    spinlock_t stats_lock;              /* Statistics spinlock */
};

/* Function declarations */

/* Manager initialization and cleanup */
struct vexfs_metadata_journal_manager *vexfs_metadata_journal_init(
    struct vexfs_journal *journal,
    struct vexfs_atomic_manager *atomic_mgr);
void vexfs_metadata_journal_destroy(struct vexfs_metadata_journal_manager *mgr);

/* Inode metadata journaling */
int vexfs_metadata_journal_inode_create(struct vexfs_metadata_journal_manager *mgr,
                                        struct inode *inode, u32 flags);
int vexfs_metadata_journal_inode_delete(struct vexfs_metadata_journal_manager *mgr,
                                        struct inode *inode, u32 flags);
int vexfs_metadata_journal_inode_update(struct vexfs_metadata_journal_manager *mgr,
                                        struct inode *inode, u32 flags);

/* Directory entry journaling */
int vexfs_metadata_journal_dentry_create(struct vexfs_metadata_journal_manager *mgr,
                                         struct dentry *dentry, u32 flags);
int vexfs_metadata_journal_dentry_delete(struct vexfs_metadata_journal_manager *mgr,
                                         struct dentry *dentry, u32 flags);
int vexfs_metadata_journal_dentry_rename(struct vexfs_metadata_journal_manager *mgr,
                                         struct dentry *old_dentry,
                                         struct dentry *new_dentry, u32 flags);

/* Allocation bitmap journaling */
int vexfs_metadata_journal_bitmap_alloc(struct vexfs_metadata_journal_manager *mgr,
                                        u64 block_group, u64 start_block,
                                        u32 block_count, u32 flags);
int vexfs_metadata_journal_bitmap_free(struct vexfs_metadata_journal_manager *mgr,
                                       u64 block_group, u64 start_block,
                                       u32 block_count, u32 flags);

/* Vector metadata journaling */
int vexfs_metadata_journal_vector_create(struct vexfs_metadata_journal_manager *mgr,
                                         u64 vector_id, struct inode *inode,
                                         u32 dimensions, u32 element_type, u32 flags);
int vexfs_metadata_journal_vector_delete(struct vexfs_metadata_journal_manager *mgr,
                                         u64 vector_id, u32 flags);
int vexfs_metadata_journal_vector_update(struct vexfs_metadata_journal_manager *mgr,
                                         u64 vector_id, u32 flags);

/* Superblock journaling */
int vexfs_metadata_journal_superblock_update(struct vexfs_metadata_journal_manager *mgr,
                                             struct super_block *sb, u32 flags);

/* Serialization functions */
int vexfs_metadata_serialize_inode(struct inode *inode,
                                   struct vexfs_meta_serialized_inode *serialized);
int vexfs_metadata_deserialize_inode(struct vexfs_meta_serialized_inode *serialized,
                                     struct inode *inode);
int vexfs_metadata_serialize_dentry(struct dentry *dentry,
                                    struct vexfs_meta_serialized_dentry **serialized,
                                    size_t *size);
int vexfs_metadata_deserialize_dentry(struct vexfs_meta_serialized_dentry *serialized,
                                      struct dentry *dentry);

/* Cache management */
int vexfs_metadata_cache_get(struct vexfs_metadata_journal_manager *mgr,
                            u64 key, u32 entry_type, void **data, size_t *size);
int vexfs_metadata_cache_put(struct vexfs_metadata_journal_manager *mgr,
                            u64 key, u32 entry_type, void *data, size_t size);
int vexfs_metadata_cache_invalidate(struct vexfs_metadata_journal_manager *mgr,
                                   u64 key, u32 entry_type);
int vexfs_metadata_cache_flush(struct vexfs_metadata_journal_manager *mgr);

/* Batch processing */
int vexfs_metadata_journal_batch_commit(struct vexfs_metadata_journal_manager *mgr);
int vexfs_metadata_journal_force_sync(struct vexfs_metadata_journal_manager *mgr);

/* Integrity verification */
u32 vexfs_metadata_calculate_checksum(const void *data, size_t size, u32 seed);
int vexfs_metadata_verify_integrity(struct vexfs_metadata_journal_manager *mgr,
                                    struct vexfs_metadata_operation *op);

/* Recovery operations */
int vexfs_metadata_journal_recover(struct vexfs_metadata_journal_manager *mgr);
int vexfs_metadata_journal_replay_operations(struct vexfs_metadata_journal_manager *mgr,
                                             u64 start_seq, u64 end_seq);

/* Statistics and monitoring */
void vexfs_metadata_journal_get_stats(struct vexfs_metadata_journal_manager *mgr,
                                      struct vexfs_metadata_journal_stats *stats);

/* Metadata journaling statistics */
struct vexfs_metadata_journal_stats {
    u64 total_operations;
    u64 inode_operations;
    u64 dentry_operations;
    u64 bitmap_operations;
    u64 vector_operations;
    u64 superblock_operations;
    u64 bytes_journaled;
    u64 cache_hits;
    u64 cache_misses;
    u32 cache_entries;
    u32 pending_operations;
    u32 batch_size;
    u32 checksum_errors;
    u64 recovery_count;
    unsigned long last_batch_time;
    unsigned long last_recovery_time;
};

/* Utility macros */
#define VEXFS_META_OP_ID(op) ((op) ? (op)->op_id : 0)
#define VEXFS_META_IS_VECTOR_INODE(inode) \
    (VEXFS_V2_I(inode)->is_vector_file)
#define VEXFS_META_INODE_VECTOR_COUNT(inode) \
    (VEXFS_V2_I(inode)->vector_count)

/* Error codes specific to metadata journaling */
#define VEXFS_META_ERR_SERIALIZATION    -2001
#define VEXFS_META_ERR_CHECKSUM         -2002
#define VEXFS_META_ERR_CACHE_FULL       -2003
#define VEXFS_META_ERR_INVALID_OP       -2004
#define VEXFS_META_ERR_RECOVERY_FAIL    -2005

#endif /* _VEXFS_V2_METADATA_JOURNAL_H */