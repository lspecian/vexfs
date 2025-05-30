"""
Shared Infrastructure Components
VM management and result collection for domain testing
"""

import asyncio
import json
import logging
import subprocess
import time
from pathlib import Path
from typing import Dict, List, Optional, Any
from dataclasses import dataclass, field


@dataclass
class VMConfig:
    """VM configuration for testing"""
    name: str
    memory_mb: int = 2048
    cpu_cores: int = 2
    disk_size_gb: int = 20
    os_image: str = "ubuntu-22.04"
    network: str = "default"
    ssh_key_path: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "memory_mb": self.memory_mb,
            "cpu_cores": self.cpu_cores,
            "disk_size_gb": self.disk_size_gb,
            "os_image": self.os_image,
            "network": self.network,
            "ssh_key_path": self.ssh_key_path
        }


@dataclass
class VMInstance:
    """Running VM instance"""
    config: VMConfig
    vm_id: str
    ip_address: Optional[str] = None
    status: str = "unknown"
    created_at: float = field(default_factory=time.time)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "config": self.config.to_dict(),
            "vm_id": self.vm_id,
            "ip_address": self.ip_address,
            "status": self.status,
            "created_at": self.created_at
        }


class VMManager:
    """
    VM Manager for test infrastructure
    Handles VM lifecycle for kernel module testing
    """
    
    def __init__(self, infrastructure_path: str = "infrastructure"):
        self.infrastructure_path = Path(infrastructure_path)
        self.logger = logging.getLogger(f"{__name__}.VMManager")
        self.active_vms: Dict[str, VMInstance] = {}
        
        # Ensure infrastructure directory exists
        self.infrastructure_path.mkdir(exist_ok=True)
        
    async def create_vm(self, config: VMConfig) -> VMInstance:
        """Create a new VM instance"""
        self.logger.info(f"Creating VM: {config.name}")
        
        try:
            # For demonstration, we'll simulate VM creation
            # In a real implementation, this would use libvirt/QEMU
            vm_id = f"vm_{config.name}_{int(time.time())}"
            
            # Simulate VM creation process
            await asyncio.sleep(1)  # Simulate creation time
            
            instance = VMInstance(
                config=config,
                vm_id=vm_id,
                ip_address=f"192.168.122.{len(self.active_vms) + 10}",
                status="running"
            )
            
            self.active_vms[vm_id] = instance
            self.logger.info(f"VM created successfully: {vm_id}")
            
            return instance
            
        except Exception as e:
            self.logger.error(f"Failed to create VM {config.name}: {str(e)}")
            raise
    
    async def destroy_vm(self, vm_id: str) -> bool:
        """Destroy a VM instance"""
        if vm_id not in self.active_vms:
            self.logger.warning(f"VM {vm_id} not found in active VMs")
            return False
        
        try:
            self.logger.info(f"Destroying VM: {vm_id}")
            
            # Simulate VM destruction
            await asyncio.sleep(0.5)
            
            del self.active_vms[vm_id]
            self.logger.info(f"VM destroyed successfully: {vm_id}")
            
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to destroy VM {vm_id}: {str(e)}")
            return False
    
    async def get_vm_status(self, vm_id: str) -> Optional[str]:
        """Get VM status"""
        if vm_id in self.active_vms:
            return self.active_vms[vm_id].status
        return None
    
    async def execute_command(self, vm_id: str, command: str, timeout: int = 300) -> Dict[str, Any]:
        """Execute command in VM"""
        if vm_id not in self.active_vms:
            raise ValueError(f"VM {vm_id} not found")
        
        vm = self.active_vms[vm_id]
        self.logger.info(f"Executing command in VM {vm_id}: {command}")
        
        try:
            # Simulate command execution
            await asyncio.sleep(0.1)  # Simulate execution time
            
            # For demonstration, return success for most commands
            if "error" in command.lower():
                return {
                    "exit_code": 1,
                    "stdout": "",
                    "stderr": "Simulated error",
                    "execution_time": 0.1
                }
            else:
                return {
                    "exit_code": 0,
                    "stdout": f"Command executed successfully: {command}",
                    "stderr": "",
                    "execution_time": 0.1
                }
                
        except Exception as e:
            self.logger.error(f"Command execution failed in VM {vm_id}: {str(e)}")
            return {
                "exit_code": -1,
                "stdout": "",
                "stderr": str(e),
                "execution_time": 0.0
            }
    
    async def copy_file_to_vm(self, vm_id: str, local_path: str, remote_path: str) -> bool:
        """Copy file to VM"""
        if vm_id not in self.active_vms:
            raise ValueError(f"VM {vm_id} not found")
        
        self.logger.info(f"Copying {local_path} to VM {vm_id}:{remote_path}")
        
        try:
            # Simulate file copy
            await asyncio.sleep(0.1)
            return True
            
        except Exception as e:
            self.logger.error(f"File copy failed: {str(e)}")
            return False
    
    async def copy_file_from_vm(self, vm_id: str, remote_path: str, local_path: str) -> bool:
        """Copy file from VM"""
        if vm_id not in self.active_vms:
            raise ValueError(f"VM {vm_id} not found")
        
        self.logger.info(f"Copying VM {vm_id}:{remote_path} to {local_path}")
        
        try:
            # Simulate file copy
            await asyncio.sleep(0.1)
            return True
            
        except Exception as e:
            self.logger.error(f"File copy failed: {str(e)}")
            return False
    
    def list_active_vms(self) -> List[VMInstance]:
        """List all active VMs"""
        return list(self.active_vms.values())
    
    async def cleanup_all_vms(self):
        """Clean up all active VMs"""
        self.logger.info(f"Cleaning up {len(self.active_vms)} active VMs")
        
        vm_ids = list(self.active_vms.keys())
        for vm_id in vm_ids:
            await self.destroy_vm(vm_id)


class ResultCollector:
    """
    Result collection and analysis for test domains
    Handles structured result storage and reporting
    """
    
    def __init__(self, results_path: str = "test_results"):
        self.results_path = Path(results_path)
        self.logger = logging.getLogger(f"{__name__}.ResultCollector")
        
        # Ensure results directory exists
        self.results_path.mkdir(exist_ok=True)
        
        # Initialize result storage
        self.domain_results: Dict[str, List[Dict[str, Any]]] = {}
        self.session_id = f"session_{int(time.time())}"
    
    async def store_domain_results(self, domain_name: str, results: Dict[str, Any]):
        """Store results for a specific domain"""
        self.logger.info(f"Storing results for domain: {domain_name}")
        
        try:
            # Add session metadata
            results["session_id"] = self.session_id
            results["stored_at"] = time.time()
            
            # Store in memory
            if domain_name not in self.domain_results:
                self.domain_results[domain_name] = []
            self.domain_results[domain_name].append(results)
            
            # Store to file
            domain_file = self.results_path / f"{domain_name}_results.json"
            with open(domain_file, 'w') as f:
                json.dump(self.domain_results[domain_name], f, indent=2)
            
            self.logger.info(f"Results stored successfully for domain: {domain_name}")
            
        except Exception as e:
            self.logger.error(f"Failed to store results for domain {domain_name}: {str(e)}")
            raise
    
    async def get_domain_results(self, domain_name: str) -> List[Dict[str, Any]]:
        """Get results for a specific domain"""
        return self.domain_results.get(domain_name, [])
    
    async def generate_summary_report(self) -> Dict[str, Any]:
        """Generate summary report across all domains"""
        self.logger.info("Generating summary report")
        
        total_tests = 0
        total_passed = 0
        total_failed = 0
        total_errors = 0
        total_duration = 0.0
        
        domain_summaries = {}
        
        for domain_name, results_list in self.domain_results.items():
            if not results_list:
                continue
                
            latest_results = results_list[-1]  # Get latest results
            
            domain_summaries[domain_name] = {
                "total_tests": latest_results.get("total_tests", 0),
                "passed": latest_results.get("passed", 0),
                "failed": latest_results.get("failed", 0),
                "errors": latest_results.get("errors", 0),
                "total_duration": latest_results.get("total_duration", 0.0),
                "success_rate": (latest_results.get("passed", 0) / max(latest_results.get("total_tests", 1), 1)) * 100
            }
            
            total_tests += latest_results.get("total_tests", 0)
            total_passed += latest_results.get("passed", 0)
            total_failed += latest_results.get("failed", 0)
            total_errors += latest_results.get("errors", 0)
            total_duration += latest_results.get("total_duration", 0.0)
        
        summary = {
            "session_id": self.session_id,
            "generated_at": time.time(),
            "overall_summary": {
                "total_tests": total_tests,
                "total_passed": total_passed,
                "total_failed": total_failed,
                "total_errors": total_errors,
                "total_duration": total_duration,
                "overall_success_rate": (total_passed / max(total_tests, 1)) * 100
            },
            "domain_summaries": domain_summaries
        }
        
        # Store summary report
        summary_file = self.results_path / f"summary_report_{self.session_id}.json"
        with open(summary_file, 'w') as f:
            json.dump(summary, f, indent=2)
        
        return summary
    
    async def export_results(self, format_type: str = "json") -> str:
        """Export all results in specified format"""
        if format_type.lower() != "json":
            raise ValueError("Only JSON format is currently supported")
        
        export_data = {
            "session_id": self.session_id,
            "exported_at": time.time(),
            "domain_results": self.domain_results
        }
        
        export_file = self.results_path / f"export_{self.session_id}.json"
        with open(export_file, 'w') as f:
            json.dump(export_data, f, indent=2)
        
        return str(export_file)
    
    def get_session_id(self) -> str:
        """Get current session ID"""
        return self.session_id


# Infrastructure factory functions
def create_vm_manager(infrastructure_path: str = "infrastructure") -> VMManager:
    """Create VM manager instance"""
    return VMManager(infrastructure_path)


def create_result_collector(results_path: str = "test_results") -> ResultCollector:
    """Create result collector instance"""
    return ResultCollector(results_path)