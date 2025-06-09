#!/bin/bash
# Minimal workload for baseline measurements

MOUNT_POINT="$1"
DURATION="$2"

log() {
    echo "[BASELINE] $1"
}

log "Starting minimal baseline workload on $MOUNT_POINT for ${DURATION}s"

# Basic filesystem operations to establish baseline
basic_operations() {
    local end_time=$(($(date +%s) + DURATION))
    local operation_count=0
    
    while [[ $(date +%s) -lt $end_time ]]; do
        # Simple file creation
        echo "Baseline test data $operation_count" > "$MOUNT_POINT/baseline_$operation_count.txt"
        
        # Simple file read
        cat "$MOUNT_POINT/baseline_$operation_count.txt" > /dev/null
        
        # Simple directory listing
        ls -la "$MOUNT_POINT" > /dev/null
        
        # Simple file metadata access
        stat "$MOUNT_POINT/baseline_$operation_count.txt" > /dev/null
        
        # Simple file deletion (every 5th file to maintain some files)
        if [[ $((operation_count % 5)) -eq 0 ]] && [[ $operation_count -gt 0 ]]; then
            rm -f "$MOUNT_POINT/baseline_$((operation_count - 1)).txt" 2>/dev/null || true
        fi
        
        operation_count=$((operation_count + 1))
        
        # Gentle pacing to avoid overwhelming the minimal implementation
        sleep 0.2
        
        if [[ $((operation_count % 50)) -eq 0 ]]; then
            log "Completed $operation_count baseline operations"
        fi
    done
    
    log "Baseline workload completed with $operation_count operations"
}

# Execute baseline operations
basic_operations
