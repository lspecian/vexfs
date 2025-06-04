/**
 * VexFS Python Bindings - Filesystem Interface
 *
 * Low-level filesystem operations for VexFS Python bindings
 */

#include <string>
#include <vector>
#include <fstream>
#include <sstream>
#include <sys/stat.h>
#include <sys/statvfs.h>
#include <sys/mount.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <cstring>
#include <cstdint>

namespace vexfs {
namespace fs_interface {

/**
 * Check if a path exists
 */
bool path_exists(const std::string& path) {
    struct stat st;
    return stat(path.c_str(), &st) == 0;
}

/**
 * Check if VexFS is mounted at the given path
 */
bool is_vexfs_mounted(const std::string& path) {
    if (!path_exists(path)) {
        return false;
    }
    
    // Read /proc/mounts to check filesystem type
    std::ifstream mounts("/proc/mounts");
    std::string line;
    
    while (std::getline(mounts, line)) {
        if (line.find(path) != std::string::npos && 
            line.find("vexfs") != std::string::npos) {
            return true;
        }
    }
    
    return false;
}

/**
 * Get filesystem statistics
 */
struct FilesystemStats {
    uint64_t total_space;
    uint64_t free_space;
    uint64_t used_space;
    uint64_t total_inodes;
    uint64_t free_inodes;
    uint32_t block_size;
};

FilesystemStats get_filesystem_stats(const std::string& path) {
    FilesystemStats stats = {};
    
    struct statvfs st;
    if (statvfs(path.c_str(), &st) == 0) {
        stats.total_space = st.f_blocks * st.f_frsize;
        stats.free_space = st.f_bavail * st.f_frsize;
        stats.used_space = stats.total_space - stats.free_space;
        stats.total_inodes = st.f_files;
        stats.free_inodes = st.f_favail;
        stats.block_size = st.f_bsize;
    }
    
    return stats;
}

/**
 * List mount points that could be VexFS
 */
std::vector<std::string> list_potential_mount_points() {
    std::vector<std::string> mount_points;
    
    // Common VexFS mount points
    std::vector<std::string> candidates = {
        "/mnt/vexfs",
        "/tmp/vexfs",
        "/opt/vexfs",
        "/var/lib/vexfs"
    };
    
    for (const auto& candidate : candidates) {
        if (path_exists(candidate)) {
            mount_points.push_back(candidate);
        }
    }
    
    // Also check /proc/mounts for any vexfs entries
    std::ifstream mounts("/proc/mounts");
    std::string line;
    
    while (std::getline(mounts, line)) {
        if (line.find("vexfs") != std::string::npos) {
            // Extract mount point (second field)
            size_t start = line.find(' ') + 1;
            size_t end = line.find(' ', start);
            if (start != std::string::npos && end != std::string::npos) {
                std::string mount_point = line.substr(start, end - start);
                mount_points.push_back(mount_point);
            }
        }
    }
    
    return mount_points;
}

/**
 * Create directory if it doesn't exist
 */
bool create_directory(const std::string& path, mode_t mode = 0755) {
    if (path_exists(path)) {
        return true;
    }
    
    return mkdir(path.c_str(), mode) == 0;
}

/**
 * Check if we have read/write access to a path
 */
bool check_access(const std::string& path, int mode = R_OK | W_OK) {
    return access(path.c_str(), mode) == 0;
}

/**
 * Get VexFS module information
 */
struct ModuleInfo {
    bool loaded;
    std::string version;
    std::string description;
    uint32_t ref_count;
};

ModuleInfo get_module_info() {
    ModuleInfo info = {};
    
    std::ifstream modules("/proc/modules");
    std::string line;
    
    while (std::getline(modules, line)) {
        if (line.find("vexfs") != std::string::npos) {
            info.loaded = true;
            
            // Parse module line: name size used_by_count used_by_list state address
            std::istringstream iss(line);
            std::string name, size, used_count;
            iss >> name >> size >> used_count;
            
            try {
                info.ref_count = std::stoul(used_count);
            } catch (...) {
                info.ref_count = 0;
            }
            
            break;
        }
    }
    
    // Try to get version from modinfo if available
    if (info.loaded) {
        FILE* pipe = popen("modinfo vexfs_v2_phase3 2>/dev/null | grep version", "r");
        if (pipe) {
            char buffer[256];
            if (fgets(buffer, sizeof(buffer), pipe)) {
                std::string version_line(buffer);
                size_t pos = version_line.find(':');
                if (pos != std::string::npos) {
                    info.version = version_line.substr(pos + 1);
                    // Trim whitespace
                    info.version.erase(0, info.version.find_first_not_of(" \t\n\r"));
                    info.version.erase(info.version.find_last_not_of(" \t\n\r") + 1);
                }
            }
            pclose(pipe);
        }
        
        // Get description
        pipe = popen("modinfo vexfs_v2_phase3 2>/dev/null | grep description", "r");
        if (pipe) {
            char buffer[256];
            if (fgets(buffer, sizeof(buffer), pipe)) {
                std::string desc_line(buffer);
                size_t pos = desc_line.find(':');
                if (pos != std::string::npos) {
                    info.description = desc_line.substr(pos + 1);
                    // Trim whitespace
                    info.description.erase(0, info.description.find_first_not_of(" \t\n\r"));
                    info.description.erase(info.description.find_last_not_of(" \t\n\r") + 1);
                }
            }
            pclose(pipe);
        }
    }
    
    return info;
}

/**
 * Check VexFS health
 */
struct HealthStatus {
    bool module_loaded;
    bool filesystem_mounted;
    bool accessible;
    std::string mount_point;
    std::string error_message;
};

HealthStatus check_health(const std::string& expected_mount = "") {
    HealthStatus status = {};
    
    // Check if module is loaded
    ModuleInfo mod_info = get_module_info();
    status.module_loaded = mod_info.loaded;
    
    if (!status.module_loaded) {
        status.error_message = "VexFS kernel module not loaded";
        return status;
    }
    
    // Check for mounted filesystems
    std::vector<std::string> mount_points = list_potential_mount_points();
    
    if (!expected_mount.empty()) {
        // Check specific mount point
        if (is_vexfs_mounted(expected_mount)) {
            status.filesystem_mounted = true;
            status.mount_point = expected_mount;
            status.accessible = check_access(expected_mount);
        } else {
            status.error_message = "VexFS not mounted at " + expected_mount;
        }
    } else {
        // Check any mount point
        for (const auto& mp : mount_points) {
            if (is_vexfs_mounted(mp)) {
                status.filesystem_mounted = true;
                status.mount_point = mp;
                status.accessible = check_access(mp);
                break;
            }
        }
        
        if (!status.filesystem_mounted) {
            status.error_message = "No VexFS filesystem found mounted";
        }
    }
    
    return status;
}

} // namespace fs_interface
} // namespace vexfs