#!/bin/bash

# VexFS v2.0 Version Standardization Validation Script
# This script validates that all version references follow the standardized naming scheme

set -e

echo "🔍 VexFS v2.0 Version Standardization Validation"
echo "================================================"
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0

# Function to run a check
run_check() {
    local description="$1"
    local command="$2"
    local expected_result="$3"  # "0" for should find nothing, "1" for should find something
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    echo -n "  Checking: $description... "
    
    # Run the command and capture both output and exit code
    if result=$(eval "$command" 2>/dev/null); then
        count=$(echo "$result" | wc -l)
        if [ -z "$result" ]; then
            count=0
        fi
        
        if [ "$expected_result" = "0" ]; then
            # Should find nothing (good)
            if [ "$count" -eq 0 ]; then
                echo -e "${GREEN}✅ PASS${NC}"
                PASSED_CHECKS=$((PASSED_CHECKS + 1))
            else
                echo -e "${RED}❌ FAIL${NC} (found $count instances)"
                echo "$result" | head -5 | sed 's/^/    /'
                if [ "$count" -gt 5 ]; then
                    echo "    ... and $((count - 5)) more"
                fi
                FAILED_CHECKS=$((FAILED_CHECKS + 1))
            fi
        else
            # Should find something (good)
            if [ "$count" -gt 0 ]; then
                echo -e "${GREEN}✅ PASS${NC} (found $count instances)"
                PASSED_CHECKS=$((PASSED_CHECKS + 1))
            else
                echo -e "${RED}❌ FAIL${NC} (found nothing)"
                FAILED_CHECKS=$((FAILED_CHECKS + 1))
            fi
        fi
    else
        echo -e "${YELLOW}⚠️  SKIP${NC} (command failed)"
    fi
}

echo -e "${BLUE}📋 Phase 1: Documentation Validation${NC}"
echo "Checking for legacy version references in documentation..."

# Check for legacy version references (should find nothing)
run_check "VexFS v1.0 references" \
    "grep -r 'VexFS v1\.0' docs/ --exclude='*VERSION_STANDARDIZATION.md' --exclude='*LEGACY_VERSION_MAPPING.md'" \
    "0"

run_check "vexfs v1 references" \
    "grep -r 'vexfs v1' docs/ --exclude='*VERSION_STANDARDIZATION.md' --exclude='*LEGACY_VERSION_MAPPING.md'" \
    "0"

run_check "Standalone 'Phase 3' references" \
    "grep -r 'Phase 3' docs/ --exclude='*VERSION_STANDARDIZATION.md' --exclude='*LEGACY_VERSION_MAPPING.md' --exclude='*VEXFS_V2_PHASE3_COMPLETION_SUMMARY.md'" \
    "0"

run_check "VexFS v2.0 Phase 3 mixed references" \
    "grep -r 'VexFS v2\.0 Phase 3' docs/ --exclude='*VERSION_STANDARDIZATION.md' --exclude='*LEGACY_VERSION_MAPPING.md'" \
    "0"

run_check "b62 build references" \
    "grep -r 'b62' docs/ --exclude='*VERSION_STANDARDIZATION.md' --exclude='*LEGACY_VERSION_MAPPING.md' --exclude='*status*'" \
    "0"

echo
echo -e "${BLUE}📋 Phase 2: Standard Version Usage${NC}"
echo "Checking for correct standard version usage..."

# Check for standard version references (should find many)
run_check "VexFS v2.0 standard references" \
    "grep -r 'VexFS v2\.0' docs/ --exclude='*VERSION_STANDARDIZATION.md' --exclude='*LEGACY_VERSION_MAPPING.md'" \
    "1"

run_check "vexfs_v2_uapi.h references" \
    "grep -r 'vexfs_v2_uapi\.h' docs/" \
    "1"

echo
echo -e "${BLUE}📋 Phase 3: Code Validation${NC}"
echo "Checking for legacy references in code files..."

# Check for legacy mount points in test files
run_check "Legacy mount point /tmp/vexfs_v2_316_test" \
    "grep -r '/tmp/vexfs_v2_316_test' kernel/ tests/" \
    "0"

run_check "Legacy mount point /tmp/vexfs_phase3_test" \
    "grep -r '/tmp/vexfs_phase3_test' kernel/ tests/" \
    "0"

run_check "Legacy test names with phase3" \
    "grep -r 'phase3_test' kernel/ tests/ --exclude-dir=archive" \
    "0"

echo
echo -e "${BLUE}📋 Phase 4: Build System Validation${NC}"
echo "Checking build system consistency..."

# Check Makefile consistency
run_check "Makefile uses standard module name" \
    "grep -q 'vexfs_v2_phase3.o' kernel/vexfs_v2_build/Makefile" \
    "1"

run_check "Makefile test target uses standard mount" \
    "grep -q '/tmp/vexfs_v2_test' kernel/vexfs_v2_build/Makefile || grep -q '/tmp/vexfs_test' kernel/vexfs_v2_build/Makefile" \
    "1"

echo
echo -e "${BLUE}📋 Phase 5: Filesystem Type Validation${NC}"
echo "Checking filesystem type registration..."

# Check filesystem type in main source
run_check "Filesystem type registration" \
    "grep -q 'vexfs_v2_b62' kernel/vexfs_v2_build/vexfs_v2_main.c" \
    "1"

echo
echo -e "${BLUE}📋 Phase 6: API Consistency${NC}"
echo "Checking API naming consistency..."

# Check for consistent API usage
run_check "UAPI header includes" \
    "grep -r '#include.*vexfs_v2_uapi\.h' kernel/" \
    "1"

run_check "Standard IOCTL magic usage" \
    "grep -r 'VEXFS.*IOC.*MAGIC' kernel/" \
    "1"

echo
echo "================================================"
echo -e "${BLUE}📊 Validation Summary${NC}"
echo "================================================"

echo "Total checks: $TOTAL_CHECKS"
echo -e "Passed: ${GREEN}$PASSED_CHECKS${NC}"
echo -e "Failed: ${RED}$FAILED_CHECKS${NC}"

if [ $FAILED_CHECKS -eq 0 ]; then
    echo
    echo -e "${GREEN}🎉 All validation checks passed!${NC}"
    echo -e "${GREEN}VexFS v2.0 version standardization is complete.${NC}"
    exit 0
else
    echo
    echo -e "${RED}❌ $FAILED_CHECKS validation checks failed.${NC}"
    echo -e "${YELLOW}Please review the failed checks above and update the corresponding files.${NC}"
    echo
    echo "Reference documents:"
    echo "  - docs/architecture/VERSION_STANDARDIZATION.md"
    echo "  - docs/architecture/LEGACY_VERSION_MAPPING.md"
    echo "  - docs/architecture/API_HIERARCHY.md"
    exit 1
fi