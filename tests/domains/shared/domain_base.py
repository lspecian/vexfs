"""
Shared Domain Base Classes
Base classes and interfaces for Domain-Driven Design test architecture
"""

import asyncio
import logging
import time
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional, Any, Callable, Union
import json
from pathlib import Path


class TestStatus(Enum):
    """Test execution status"""
    PENDING = "pending"
    RUNNING = "running"
    PASSED = "passed"
    FAILED = "failed"
    SKIPPED = "skipped"
    TIMEOUT = "timeout"
    ERROR = "error"


class Priority(Enum):
    """Test priority levels"""
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    CRITICAL = "critical"


@dataclass
class TestMetrics:
    """Test execution metrics"""
    execution_time: float = 0.0
    memory_usage: Dict[str, int] = field(default_factory=dict)
    cpu_usage: float = 0.0
    disk_io: Dict[str, int] = field(default_factory=dict)
    network_io: Dict[str, int] = field(default_factory=dict)
    custom_metrics: Dict[str, Any] = field(default_factory=dict)
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert metrics to dictionary"""
        return {
            "execution_time": self.execution_time,
            "memory_usage": self.memory_usage,
            "cpu_usage": self.cpu_usage,
            "disk_io": self.disk_io,
            "network_io": self.network_io,
            "custom_metrics": self.custom_metrics
        }


@dataclass
class TestResult:
    """Test execution result"""
    status: TestStatus
    message: str
    metrics: Dict[str, Any] = field(default_factory=dict)
    artifacts: List[str] = field(default_factory=list)
    logs: List[str] = field(default_factory=list)
    timestamp: float = field(default_factory=time.time)
    duration: float = 0.0
    error_details: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert result to dictionary"""
        return {
            "status": self.status.value,
            "message": self.message,
            "metrics": self.metrics,
            "artifacts": self.artifacts,
            "logs": self.logs,
            "timestamp": self.timestamp,
            "duration": self.duration,
            "error_details": self.error_details
        }


@dataclass
class TestCase:
    """Test case definition"""
    name: str
    description: str
    test_func: Callable
    timeout: int = 300
    priority: Priority = Priority.MEDIUM
    dependencies: List[str] = field(default_factory=list)
    tags: List[str] = field(default_factory=list)
    safety_level: Optional[Any] = None  # Domain-specific safety level
    retry_count: int = 0
    setup_func: Optional[Callable] = None
    teardown_func: Optional[Callable] = None
    
    def __post_init__(self):
        """Post-initialization validation"""
        if not callable(self.test_func):
            raise ValueError(f"test_func must be callable for test case '{self.name}'")


@dataclass
class DomainTestSuite:
    """Test suite for a specific domain"""
    domain_name: str
    description: str
    test_cases: List[TestCase] = field(default_factory=list)
    setup_func: Optional[Callable] = None
    teardown_func: Optional[Callable] = None
    parallel_execution: bool = False
    max_parallel_tests: int = 4
    
    def add_test_case(self, test_case: TestCase):
        """Add a test case to the suite"""
        self.test_cases.append(test_case)
    
    def get_test_case(self, name: str) -> Optional[TestCase]:
        """Get test case by name"""
        for test_case in self.test_cases:
            if test_case.name == name:
                return test_case
        return None


class DomainBase(ABC):
    """
    Base class for all domain implementations
    
    Provides common functionality for:
    - Test execution and management
    - Result collection and reporting
    - Resource management
    - Error handling and recovery
    """
    
    def __init__(self, domain_name: str, vm_manager, result_collector):
        self.domain_name = domain_name
        self.vm_manager = vm_manager
        self.result_collector = result_collector
        self.logger = logging.getLogger(f"{__name__}.{domain_name}")
        
        # Test execution state
        self.test_suite = DomainTestSuite(domain_name, self.get_domain_description())
        self.current_test: Optional[TestCase] = None
        self.test_results: List[TestResult] = []
        
        # Resource tracking
        self.resources_allocated: List[str] = []
        self.cleanup_functions: List[Callable] = []
        
        # Configuration
        self.config = self._load_domain_config()
        
    @abstractmethod
    def get_domain_description(self) -> str:
        """Get description of this domain"""
        pass
    
    @abstractmethod
    async def setup_domain(self) -> bool:
        """Set up domain-specific resources and configuration"""
        pass
    
    @abstractmethod
    async def teardown_domain(self) -> bool:
        """Clean up domain-specific resources"""
        pass
    
    @abstractmethod
    async def validate_domain_constraints(self) -> bool:
        """Validate domain-specific constraints and safety requirements"""
        pass
    
    def _load_domain_config(self) -> Dict[str, Any]:
        """Load domain-specific configuration"""
        config_path = Path(f"tests/domains/{self.domain_name}/config.json")
        if config_path.exists():
            with open(config_path, 'r') as f:
                return json.load(f)
        return {}
    
    async def execute_test_suite(self) -> List[TestResult]:
        """Execute all tests in the domain test suite"""
        self.logger.info(f"Starting test suite execution for domain: {self.domain_name}")
        
        try:
            # Domain setup
            if not await self.setup_domain():
                raise RuntimeError(f"Domain setup failed for {self.domain_name}")
            
            # Validate constraints
            if not await self.validate_domain_constraints():
                raise RuntimeError(f"Domain constraints validation failed for {self.domain_name}")
            
            # Execute tests
            if self.test_suite.parallel_execution:
                results = await self._execute_tests_parallel()
            else:
                results = await self._execute_tests_sequential()
            
            self.test_results.extend(results)
            
            # Collect and store results
            await self._collect_and_store_results(results)
            
            return results
            
        except Exception as e:
            self.logger.error(f"Test suite execution failed: {str(e)}")
            error_result = TestResult(
                status=TestStatus.ERROR,
                message=f"Test suite execution failed: {str(e)}",
                error_details=str(e)
            )
            return [error_result]
            
        finally:
            # Always attempt cleanup
            await self.teardown_domain()
    
    async def execute_single_test(self, test_name: str) -> TestResult:
        """Execute a single test case by name"""
        test_case = self.test_suite.get_test_case(test_name)
        if not test_case:
            return TestResult(
                status=TestStatus.ERROR,
                message=f"Test case '{test_name}' not found in domain '{self.domain_name}'"
            )
        
        return await self._execute_test_case(test_case)
    
    async def _execute_tests_sequential(self) -> List[TestResult]:
        """Execute tests sequentially"""
        results = []
        
        for test_case in self.test_suite.test_cases:
            result = await self._execute_test_case(test_case)
            results.append(result)
            
            # Stop on critical failures if configured
            if (result.status == TestStatus.FAILED and 
                test_case.priority == Priority.CRITICAL and
                self.config.get("stop_on_critical_failure", False)):
                self.logger.warning(f"Stopping test execution due to critical failure in {test_case.name}")
                break
        
        return results
    
    async def _execute_tests_parallel(self) -> List[TestResult]:
        """Execute tests in parallel"""
        semaphore = asyncio.Semaphore(self.test_suite.max_parallel_tests)
        
        async def execute_with_semaphore(test_case: TestCase) -> TestResult:
            async with semaphore:
                return await self._execute_test_case(test_case)
        
        tasks = [execute_with_semaphore(test_case) for test_case in self.test_suite.test_cases]
        results = await asyncio.gather(*tasks, return_exceptions=True)
        
        # Convert exceptions to error results
        processed_results = []
        for i, result in enumerate(results):
            if isinstance(result, Exception):
                processed_results.append(TestResult(
                    status=TestStatus.ERROR,
                    message=f"Test execution failed with exception: {str(result)}",
                    error_details=str(result)
                ))
            else:
                processed_results.append(result)
        
        return processed_results
    
    async def _execute_test_case(self, test_case: TestCase) -> TestResult:
        """Execute a single test case with full lifecycle management"""
        self.current_test = test_case
        start_time = time.time()
        
        self.logger.info(f"Executing test case: {test_case.name}")
        
        try:
            # Setup
            if test_case.setup_func:
                await test_case.setup_func()
            
            # Execute with timeout
            result = await asyncio.wait_for(
                test_case.test_func(),
                timeout=test_case.timeout
            )
            
            # Ensure result is a TestResult object
            if not isinstance(result, TestResult):
                result = TestResult(
                    status=TestStatus.PASSED,
                    message=str(result) if result else "Test completed successfully"
                )
            
            result.duration = time.time() - start_time
            
            self.logger.info(f"Test case {test_case.name} completed: {result.status.value}")
            
            return result
            
        except asyncio.TimeoutError:
            result = TestResult(
                status=TestStatus.TIMEOUT,
                message=f"Test case timed out after {test_case.timeout} seconds",
                duration=time.time() - start_time
            )
            self.logger.warning(f"Test case {test_case.name} timed out")
            return result
            
        except Exception as e:
            result = TestResult(
                status=TestStatus.ERROR,
                message=f"Test case failed with exception: {str(e)}",
                duration=time.time() - start_time,
                error_details=str(e)
            )
            self.logger.error(f"Test case {test_case.name} failed: {str(e)}")
            return result
            
        finally:
            # Teardown
            try:
                if test_case.teardown_func:
                    await test_case.teardown_func()
            except Exception as e:
                self.logger.warning(f"Test case teardown failed: {str(e)}")
            
            self.current_test = None
    
    async def _collect_and_store_results(self, results: List[TestResult]):
        """Collect and store test results"""
        try:
            # Create domain result summary
            domain_result = {
                "domain": self.domain_name,
                "timestamp": time.time(),
                "total_tests": len(results),
                "passed": len([r for r in results if r.status == TestStatus.PASSED]),
                "failed": len([r for r in results if r.status == TestStatus.FAILED]),
                "errors": len([r for r in results if r.status == TestStatus.ERROR]),
                "timeouts": len([r for r in results if r.status == TestStatus.TIMEOUT]),
                "total_duration": sum(r.duration for r in results),
                "results": [r.to_dict() for r in results]
            }
            
            # Store results using result collector
            await self.result_collector.store_domain_results(self.domain_name, domain_result)
            
        except Exception as e:
            self.logger.error(f"Failed to collect and store results: {str(e)}")
    
    def add_cleanup_function(self, cleanup_func: Callable):
        """Add a cleanup function to be called during teardown"""
        self.cleanup_functions.append(cleanup_func)
    
    def allocate_resource(self, resource_id: str):
        """Track allocated resource for cleanup"""
        self.resources_allocated.append(resource_id)
    
    def deallocate_resource(self, resource_id: str):
        """Remove resource from tracking"""
        if resource_id in self.resources_allocated:
            self.resources_allocated.remove(resource_id)
    
    async def cleanup_resources(self):
        """Clean up all allocated resources"""
        self.logger.info(f"Cleaning up {len(self.resources_allocated)} resources")
        
        # Execute cleanup functions
        for cleanup_func in reversed(self.cleanup_functions):
            try:
                if asyncio.iscoroutinefunction(cleanup_func):
                    await cleanup_func()
                else:
                    cleanup_func()
            except Exception as e:
                self.logger.warning(f"Cleanup function failed: {str(e)}")
        
        # Clear tracking
        self.resources_allocated.clear()
        self.cleanup_functions.clear()
    
    def get_test_statistics(self) -> Dict[str, Any]:
        """Get test execution statistics"""
        if not self.test_results:
            return {}
        
        total_tests = len(self.test_results)
        passed = len([r for r in self.test_results if r.status == TestStatus.PASSED])
        failed = len([r for r in self.test_results if r.status == TestStatus.FAILED])
        errors = len([r for r in self.test_results if r.status == TestStatus.ERROR])
        timeouts = len([r for r in self.test_results if r.status == TestStatus.TIMEOUT])
        
        return {
            "domain": self.domain_name,
            "total_tests": total_tests,
            "passed": passed,
            "failed": failed,
            "errors": errors,
            "timeouts": timeouts,
            "success_rate": (passed / total_tests * 100) if total_tests > 0 else 0,
            "total_duration": sum(r.duration for r in self.test_results),
            "average_duration": sum(r.duration for r in self.test_results) / total_tests if total_tests > 0 else 0
        }


class DomainRegistry:
    """Registry for managing domain implementations"""
    
    def __init__(self):
        self.domains: Dict[str, type] = {}
        self.domain_instances: Dict[str, DomainBase] = {}
    
    def register_domain(self, domain_name: str, domain_class: type):
        """Register a domain implementation"""
        if not issubclass(domain_class, DomainBase):
            raise ValueError(f"Domain class must inherit from DomainBase")
        
        self.domains[domain_name] = domain_class
    
    def get_domain_class(self, domain_name: str) -> Optional[type]:
        """Get domain class by name"""
        return self.domains.get(domain_name)
    
    def create_domain_instance(self, domain_name: str, vm_manager, result_collector) -> Optional[DomainBase]:
        """Create domain instance"""
        domain_class = self.get_domain_class(domain_name)
        if not domain_class:
            return None
        
        instance = domain_class(vm_manager, result_collector)
        self.domain_instances[domain_name] = instance
        return instance
    
    def get_domain_instance(self, domain_name: str) -> Optional[DomainBase]:
        """Get existing domain instance"""
        return self.domain_instances.get(domain_name)
    
    def list_domains(self) -> List[str]:
        """List all registered domains"""
        return list(self.domains.keys())


# Global domain registry
domain_registry = DomainRegistry()