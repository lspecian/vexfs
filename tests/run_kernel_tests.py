#!/usr/bin/env python3
"""
VexFS Kernel Module Test Runner
Demonstrates the Infrastructure-as-Code testing framework for kernel module validation
"""

import asyncio
import logging
import sys
from pathlib import Path

# Add the project root to Python path
sys.path.insert(0, str(Path(__file__).parent.parent))

from tests.domains.kernel_module.domain_model import KernelModuleDomain
from tests.domains.shared.infrastructure import create_vm_manager, create_result_collector


async def main():
    """Main test execution function"""
    
    # Configure logging
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    )
    
    logger = logging.getLogger(__name__)
    logger.info("Starting VexFS Kernel Module Test Suite")
    
    try:
        # Initialize infrastructure components
        logger.info("Initializing test infrastructure...")
        vm_manager = create_vm_manager("infrastructure")
        result_collector = create_result_collector("test_results")
        
        # Create kernel module domain
        logger.info("Creating kernel module domain...")
        kernel_domain = KernelModuleDomain(vm_manager, result_collector)
        
        # Execute domain setup
        logger.info("Setting up kernel module domain...")
        setup_success = await kernel_domain.setup_domain()
        if not setup_success:
            logger.error("Domain setup failed")
            return 1
        
        # Validate domain constraints
        logger.info("Validating domain constraints...")
        constraints_valid = await kernel_domain.validate_domain_constraints()
        if not constraints_valid:
            logger.error("Domain constraints validation failed")
            return 1
        
        # Execute test suite
        logger.info("Executing kernel module test suite...")
        test_results = await kernel_domain.execute_test_suite()
        
        # Display results
        logger.info("Test execution completed. Results:")
        
        passed = len([r for r in test_results if r.status.value == "passed"])
        failed = len([r for r in test_results if r.status.value == "failed"])
        errors = len([r for r in test_results if r.status.value == "error"])
        total = len(test_results)
        
        print(f"\n{'='*60}")
        print(f"VexFS Kernel Module Test Results")
        print(f"{'='*60}")
        print(f"Total Tests: {total}")
        print(f"Passed: {passed}")
        print(f"Failed: {failed}")
        print(f"Errors: {errors}")
        print(f"Success Rate: {(passed/total*100):.1f}%" if total > 0 else "N/A")
        print(f"{'='*60}")
        
        # Display individual test results
        for i, result in enumerate(test_results, 1):
            status_symbol = "✅" if result.status.value == "passed" else "❌"
            print(f"{i:2d}. {status_symbol} {result.message}")
            if result.metrics:
                for key, value in result.metrics.items():
                    if isinstance(value, (int, float)):
                        print(f"    {key}: {value}")
        
        # Generate summary report
        logger.info("Generating summary report...")
        summary = await result_collector.generate_summary_report()
        
        print(f"\nSummary Report Generated:")
        print(f"Session ID: {summary['session_id']}")
        print(f"Overall Success Rate: {summary['overall_summary']['overall_success_rate']:.1f}%")
        
        # Export results
        export_file = await result_collector.export_results()
        print(f"Results exported to: {export_file}")
        
        # Return appropriate exit code
        return 0 if failed == 0 and errors == 0 else 1
        
    except Exception as e:
        logger.error(f"Test execution failed: {str(e)}")
        return 1
    
    finally:
        # Cleanup
        try:
            await kernel_domain.teardown_domain()
            logger.info("Domain teardown completed")
        except Exception as e:
            logger.warning(f"Domain teardown failed: {str(e)}")


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)