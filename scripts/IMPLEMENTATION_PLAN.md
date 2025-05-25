# VexFS Implementation Execution Plan

## 🎯 **Execution Sequence with Dependencies**

### **Phase 1: Foundation Fixes (Critical Path)**

#### **Task 1.1: Fix vexctl ioctl Issue**
**Owner**: Code Mode  
**Priority**: Critical  
**Dependencies**: None  

**Technical Specifications:**
- **File**: `vexctl/src/main.rs`
- **Line**: 62
- **Issue**: `nix::sys::ioctl::ioctl` function removed in nix 0.27.1
- **Solution**: Replace with direct `libc::ioctl` call

**Required Changes:**
1. **Update imports** (lines 1-5):
   ```rust
   // Remove: use nix::sys::ioctl::ioctl;
   // Add: extern crate libc;
   ```

2. **Fix ioctl call** (line 62):
   ```rust
   // Replace:
   match unsafe { nix::sys::ioctl::ioctl(fd, VEXFS_IOCTL_GET_STATUS_FULL_CMD as u64, 0 as *mut _) }
   
   // With:
   match unsafe { libc::ioctl(fd, VEXFS_IOCTL_GET_STATUS_FULL_CMD as libc::c_ulong, 0) }
   ```

3. **Update Cargo.toml**:
   ```toml
   [dependencies]
   clap = { version = "4.0", features = ["derive"] }
   nix = "0.27"
   libc = "0.2"  # Add this dependency
   ```

**Validation Steps:**
```bash
cd vexctl
cargo check  # Should compile without errors
cargo build  # Should build successfully
cargo clippy # Should pass linting
```

#### **Task 1.2: Create Host Development Script**
**Owner**: Code Mode  
**Priority**: High  
**Dependencies**: Task 1.1 complete  

**File**: `scripts/dev-host.sh`
**Purpose**: Enable rapid host-based development

**Script Specifications:**
```bash
#!/bin/bash
set -e

echo "🏠 VexFS Host Development Environment"
echo "===================================="

# Navigate to project root
cd "$(dirname "$0")/.."

echo "📦 Building vexctl..."
cd vexctl
cargo build --release
echo "✅ vexctl build complete"

echo "🔍 Running static analysis..."
cargo clippy -- -D warnings
echo "✅ Static analysis passed"

echo "🧪 Running unit tests..."
cargo test
echo "✅ Unit tests passed"

echo "📊 Running cargo check on kernel module..."
cd ../vexfs
cargo check --lib
echo "✅ Kernel module syntax check complete"

echo "🎉 Host development validation complete!"
echo "📝 Next: Run 'scripts/test-vm.sh' for full integration testing"
```

#### **Task 1.3: Test Current VM Setup**
**Owner**: Test Mode  
**Priority**: High  
**Dependencies**: None (parallel with 1.1-1.2)  

**Validation Checklist:**
- Verify Packer configuration builds VM
- Confirm kernel headers are available
- Test Rust-for-Linux compilation capability
- Validate QEMU execution environment

### **Phase 2: Automation Enhancement**

#### **Task 2.1: Create VM Testing Script**
**Owner**: Code Mode  
**Priority**: High  
**Dependencies**: Task 1.3 complete  

**File**: `scripts/test-vm.sh`
**Purpose**: Automate full VM-based testing

**Script Specifications:**
```bash
#!/bin/bash
set -e

echo "🖥️  VexFS VM Testing Environment"
echo "==============================="

# Navigate to project root
cd "$(dirname "$0")/.."

echo "🏗️  Building VM with Packer..."
cd test_env
packer build vexfs.pkr.hcl
echo "✅ VM build complete"

echo "🚀 Starting VM and running tests..."
# Mount source directory and run tests
./run_qemu.sh --source-mount ../vexfs --test-mode
echo "✅ VM testing complete"

echo "📊 Generating test report..."
# Copy test results from VM
echo "✅ Test results available in test_env/results/"

echo "🎉 Full integration testing complete!"
```

#### **Task 2.2: Enhance Packer Configuration**
**Owner**: Code Mode  
**Priority**: Medium  
**Dependencies**: Task 2.1 design complete  

**File**: `test_env/vexfs.pkr.hcl`
**Enhancements Needed:**
- Add development tools (gdb, strace, perf)
- Configure source directory mounting
- Add automated test runner
- Optimize build caching
- Add result extraction capability

#### **Task 2.3: Create Setup Automation**
**Owner**: Code Mode  
**Priority**: Medium  
**Dependencies**: Tasks 2.1-2.2 complete  

**File**: `scripts/setup-env.sh`
**Purpose**: One-command environment setup

### **Phase 3: Documentation & Integration**

#### **Task 3.1: Developer Quick Start Guide**
**Owner**: Architect Mode  
**Priority**: Medium  
**Dependencies**: Phase 2 complete  

**File**: `docs/DEVELOPMENT.md`

#### **Task 3.2: Testing Procedures Documentation**
**Owner**: Architect Mode  
**Priority**: Medium  
**Dependencies**: Task 3.1 complete  

**File**: `docs/TESTING.md`

#### **Task 3.3: Troubleshooting Guide**
**Owner**: Architect Mode  
**Priority**: Low  
**Dependencies**: Tasks 3.1-3.2 complete  

**File**: `docs/TROUBLESHOOTING.md`

## 🔄 **Execution Strategy**

### **Immediate Actions (Today)**
1. **Fix vexctl ioctl issue** (Code Mode) - 30 minutes
2. **Create host development script** (Code Mode) - 45 minutes
3. **Test current VM setup** (Test Mode) - 60 minutes

### **Near-term Actions (This Week)**
4. **Create VM testing script** (Code Mode) - 2 hours
5. **Enhance Packer configuration** (Code Mode) - 3 hours
6. **Create setup automation** (Code Mode) - 1 hour

### **Documentation Phase (Next Week)**
7. **Create developer guides** (Architect Mode) - 4 hours
8. **Create testing documentation** (Architect Mode) - 2 hours
9. **Create troubleshooting guide** (Architect Mode) - 2 hours

## 🎯 **Success Criteria**

### **Phase 1 Success**
- [ ] vexctl compiles and runs on host
- [ ] Host development script executes successfully
- [ ] VM environment proven functional

### **Phase 2 Success**
- [ ] VM testing script automates full workflow
- [ ] Packer configuration supports development workflow
- [ ] Setup automation enables one-command environment

### **Phase 3 Success**
- [ ] New developers can follow documentation to contribute
- [ ] Testing procedures are clearly documented
- [ ] Common issues have documented solutions

## 🚀 **Mode Delegation Strategy**

1. **Code Mode Tasks**: All script creation, code fixes, configuration updates
2. **Test Mode Tasks**: VM validation, integration testing, test automation
3. **Architect Mode Tasks**: Documentation, process design, strategy updates

This ensures each mode operates within its expertise while maintaining clear dependencies and deliverables.