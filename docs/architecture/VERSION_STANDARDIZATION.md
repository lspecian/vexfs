# VexFS Version Standardization Guide

**Status**: ✅ **OFFICIAL VERSION STANDARD**  
**Date**: June 4, 2025  
**Scope**: All VexFS components, documentation, and APIs

## Official Version Standard

### **Canonical Version: VexFS v2.0**

All VexFS components, documentation, and references MUST use the standardized naming scheme defined below.

## Standardized Naming Scheme

### **1. Project Version**
- **Official Name**: `VexFS v2.0`
- **Usage**: All documentation, README files, and user-facing content
- **Format**: Always include "v" prefix and major.minor format

### **2. Kernel Module**
- **Module Name**: `vexfs_v2.ko`
- **Module Object**: `vexfs_v2`
- **Internal Name**: `vexfs_v2_phase3` (legacy compatibility during transition)

### **3. Filesystem Type**
- **Filesystem Type**: `vexfs_v2`
- **Mount Command**: `mount -t vexfs_v2 <device> <mountpoint>`
- **Legacy Type**: `vexfs_v2_b62` (deprecated, maintained for compatibility)

### **4. API Version**
- **UAPI Header**: `vexfs_v2_uapi.h`
- **API Version**: `v2.0`
- **IOCTL Magic**: `VEXFS_V2_IOC_MAGIC`

### **5. Documentation References**
- **Standard Format**: `VexFS v2.0`
- **File Naming**: `VEXFS_V2_*` for technical documents
- **Directory Structure**: `docs/v2.0/` for version-specific docs

## Legacy Version Mapping

### **Deprecated Names → Standard Names**

| **Deprecated** | **Standard** | **Context** |
|----------------|--------------|-------------|
| `VexFS v1.0` | `VexFS v2.0` | All documentation |
| `vexfs v1` | `vexfs v2` | Technical references |
| `vexfs_v2_phase3` | `vexfs_v2` | Kernel module name |
| `vexfs_v2_b62` | `vexfs_v2` | Filesystem type |
| `phase3` | `v2.0` | Feature references |
| `b62` | `v2.0` | Build/version tags |

### **Component Mapping**

| **Component** | **Legacy Name** | **Standard Name** |
|---------------|-----------------|-------------------|
| Kernel Module | `vexfs_v2_phase3.ko` | `vexfs_v2.ko` |
| Filesystem Type | `vexfs_v2_b62` | `vexfs_v2` |
| UAPI Header | `vexfs_v2_uapi.h` | `vexfs_v2_uapi.h` ✅ |
| Main Source | `vexfs_v2_main.c` | `vexfs_v2_main.c` ✅ |
| Search Module | `vexfs_v2_search.c` | `vexfs_v2_search.c` ✅ |

## Implementation Guidelines

### **For Documentation**
```markdown
✅ CORRECT: "VexFS v2.0 provides kernel-native vector operations"
❌ INCORRECT: "VexFS v1.0 provides..." or "vexfs_v2_phase3 provides..."
```

### **For Code Comments**
```c
✅ CORRECT: /* VexFS v2.0 vector search implementation */
❌ INCORRECT: /* Phase 3 advanced search */ or /* VexFS v1 search */
```

### **For File Headers**
```c
✅ CORRECT: 
/*
 * VexFS v2.0 - Kernel-Native Vector Database Filesystem
 * Component: Vector Search Operations
 */
```

### **For Mount Commands**
```bash
✅ STANDARD: sudo mount -t vexfs_v2 /dev/sda1 /mnt/vexfs
✅ LEGACY: sudo mount -t vexfs_v2_b62 /dev/sda1 /mnt/vexfs  # Still works
```

## Transition Plan

### **Phase 1: Documentation Standardization** ✅ **IN PROGRESS**
- Update all `.md` files to use `VexFS v2.0`
- Replace legacy version references
- Create version mapping documents

### **Phase 2: Code Standardization** (Future)
- Update filesystem type registration
- Standardize module naming
- Maintain backward compatibility

### **Phase 3: Legacy Deprecation** (Future)
- Mark legacy names as deprecated
- Provide migration warnings
- Plan removal timeline

## Validation Rules

### **Documentation Validation**
```bash
# Check for non-standard version references
grep -r "VexFS v1\|vexfs v1\|phase3\|b62" docs/ --exclude="*VERSION_STANDARDIZATION.md" --exclude="*LEGACY_VERSION_MAPPING.md"

# Should return no results after standardization
```

### **Code Validation**
```bash
# Check for legacy filesystem type usage
grep -r "vexfs_v2_b62" kernel/ --exclude="*.md"

# Check for legacy module references
grep -r "vexfs_v2_phase3" . --exclude-dir=docs --exclude="*.md"
```

## Enforcement

### **Required Actions**
1. **All new documentation** MUST use `VexFS v2.0`
2. **All new code** MUST use standardized naming
3. **All updates** MUST convert legacy references
4. **All reviews** MUST check version consistency

### **Prohibited Actions**
1. **DO NOT** introduce new legacy version references
2. **DO NOT** use inconsistent naming schemes
3. **DO NOT** mix version formats in single documents
4. **DO NOT** create new `phase3` or `b62` references

## Benefits of Standardization

### **User Benefits**
- **Clear versioning**: No confusion about current version
- **Consistent documentation**: Uniform experience across all docs
- **Predictable naming**: Standard patterns for all components

### **Developer Benefits**
- **Reduced confusion**: Single source of truth for naming
- **Easier maintenance**: Consistent patterns across codebase
- **Better collaboration**: Clear standards for all contributors

### **Project Benefits**
- **Professional appearance**: Consistent branding and naming
- **Easier support**: Clear version identification
- **Future scalability**: Established patterns for future versions

## References

- [Legacy Version Mapping](LEGACY_VERSION_MAPPING.md)
- [API Hierarchy Documentation](API_HIERARCHY.md)
- [VexFS v2.0 Architecture Overview](VEXFS_V2_FLOATING_POINT_ARCHITECTURE.md)

---

**This document is the official standard for all VexFS version references. All components must comply with these guidelines.**

**Last Updated**: June 4, 2025  
**Next Review**: September 2025  
**Maintained By**: VexFS Development Team