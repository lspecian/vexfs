#!/bin/bash

# VexFS Version Cleanup Script
# Removes "v2" and "phase3" version noise from filenames

echo "Starting VexFS version cleanup..."

# Core source files
mv vexfs_v2_main.c vexfs_main.c
mv vexfs_v2_search.c vexfs_search.c
mv vexfs_v2_search.h vexfs_search.h
mv vexfs_v2_monitoring.c vexfs_monitoring.c
mv vexfs_v2_monitoring.h vexfs_monitoring.h
mv vexfs_v2_multi_model.c vexfs_multi_model.c
mv vexfs_v2_advanced_search.c vexfs_advanced_search.c
mv vexfs_v2_hnsw.c vexfs_hnsw.c
mv vexfs_v2_lsh.c vexfs_lsh.c
mv vexfs_v2_phase3_integration.c vexfs_integration.c

# Header files
mv vexfs_v2_phase3.h vexfs.h
mv vexfs_v2_internal.h vexfs_internal.h
mv vexfs_v2_public_api.h vexfs_public_api.h
mv vexfs_v2_uapi.h vexfs_uapi.h

# Additional component files (if they exist)
if [ -f vexfs_v2_memory_manager.c ]; then
    mv vexfs_v2_memory_manager.c vexfs_memory_manager.c
fi
if [ -f vexfs_v2_memory_manager.h ]; then
    mv vexfs_v2_memory_manager.h vexfs_memory_manager.h
fi
if [ -f vexfs_v2_vector_cache.c ]; then
    mv vexfs_v2_vector_cache.c vexfs_vector_cache.c
fi
if [ -f vexfs_v2_vector_cache.h ]; then
    mv vexfs_v2_vector_cache.h vexfs_vector_cache.h
fi
if [ -f vexfs_v2_vector_processing.c ]; then
    mv vexfs_v2_vector_processing.c vexfs_vector_processing.c
fi
if [ -f vexfs_v2_vector_processing.h ]; then
    mv vexfs_v2_vector_processing.h vexfs_vector_processing.h
fi
if [ -f vexfs_v2_locking.c ]; then
    mv vexfs_v2_locking.c vexfs_locking.c
fi
if [ -f vexfs_v2_locking.h ]; then
    mv vexfs_v2_locking.h vexfs_locking.h
fi
if [ -f vexfs_v2_ann_index_cache.c ]; then
    mv vexfs_v2_ann_index_cache.c vexfs_ann_index_cache.c
fi
if [ -f vexfs_v2_ann_index_cache.h ]; then
    mv vexfs_v2_ann_index_cache.h vexfs_ann_index_cache.h
fi

echo "File renaming complete!"
echo "Next: Update Makefile and test build"