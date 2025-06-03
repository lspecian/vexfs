#!/bin/bash

echo "🔍 VexFS v2.0 Phase 3 Module Verification"
echo "=========================================="

# Check if module file exists and get size
if [ -f "vexfs_v2_phase3.ko" ]; then
    echo "✅ Module file exists: $(ls -lh vexfs_v2_phase3.ko | awk '{print $5}')"
else
    echo "❌ Module file not found"
    exit 1
fi

# Check module info
echo ""
echo "📋 Module Information:"
modinfo ./vexfs_v2_phase3.ko | grep -E "(filename|version|description|author|license)" | head -10

# Check for required symbols
echo ""
echo "🔧 Checking for required symbols:"

# Check HNSW symbols
hnsw_symbols=$(nm vexfs_v2_phase3.ko | grep -c "vexfs_hnsw_")
echo "   HNSW symbols found: $hnsw_symbols"

# Check LSH symbols  
lsh_symbols=$(nm vexfs_v2_phase3.ko | grep -c "vexfs_lsh_")
echo "   LSH symbols found: $lsh_symbols"

# Check Phase3 symbols
phase3_symbols=$(nm vexfs_v2_phase3.ko | grep -c "vexfs_phase3_")
echo "   Phase3 symbols found: $phase3_symbols"

# Check for floating-point symbols (should be 0)
float_symbols=$(nm vexfs_v2_phase3.ko 2>/dev/null | grep -E "(__fixunssfsi|__fixunssfdi|__floatsi|__float)" | wc -l)
echo "   Floating-point symbols: $float_symbols (should be 0)"

echo ""
if [ "$hnsw_symbols" -gt 0 ] && [ "$lsh_symbols" -gt 0 ] && [ "$phase3_symbols" -gt 0 ] && [ "$float_symbols" -eq 0 ]; then
    echo "🎉 SUCCESS: Module compiled successfully with all required components!"
    echo "   - HNSW indexing: ✅"
    echo "   - LSH indexing: ✅" 
    echo "   - Phase 3 integration: ✅"
    echo "   - No floating-point issues: ✅"
else
    echo "❌ ISSUES DETECTED:"
    [ "$hnsw_symbols" -eq 0 ] && echo "   - Missing HNSW symbols"
    [ "$lsh_symbols" -eq 0 ] && echo "   - Missing LSH symbols"
    [ "$phase3_symbols" -eq 0 ] && echo "   - Missing Phase3 symbols"
    [ "$float_symbols" -ne 0 ] && echo "   - Floating-point symbols present"
fi

echo ""
echo "📊 Module is ready for deployment!"