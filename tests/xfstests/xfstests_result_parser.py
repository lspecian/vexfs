#!/usr/bin/env python3
"""
VexFS v2.0 xfstests Result Parser

This script parses xfstests output and generates comprehensive reports
for VexFS POSIX compliance testing. It analyzes test results, identifies
patterns, and generates both text and HTML reports.

Usage: python3 xfstests_result_parser.py <results_directory>
"""

import os
import sys
import json
import re
import argparse
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, asdict
from collections import defaultdict

@dataclass
class TestResult:
    """Represents a single test result"""
    test_id: str
    test_name: str
    status: str  # pass, fail, skip, notrun
    duration: float
    output: str
    error_message: Optional[str] = None
    category: Optional[str] = None

@dataclass
class TestSummary:
    """Represents overall test summary"""
    total_tests: int
    passed: int
    failed: int
    skipped: int
    notrun: int
    total_duration: float
    pass_rate: float
    categories: Dict[str, Dict[str, int]]

class XFSTestsResultParser:
    """Parser for xfstests results"""
    
    def __init__(self, results_dir: str):
        self.results_dir = Path(results_dir)
        self.test_results: List[TestResult] = []
        self.summary: Optional[TestSummary] = None
        
        # Test result patterns
        self.result_patterns = {
            'pass': re.compile(r'^(\S+)\s+(\d+)s\s*$'),
            'fail': re.compile(r'^(\S+)\s+\[failed, exit status (\d+)\]\s*$'),
            'skip': re.compile(r'^(\S+)\s+\[not run\]\s+(.*)$'),
            'notrun': re.compile(r'^(\S+)\s+\[notrun\]\s+(.*)$')
        }
        
        # Test categories
        self.test_categories = {
            'generic': 'Generic filesystem tests',
            'vexfs': 'VexFS-specific tests',
            'posix': 'POSIX compliance tests',
            'stress': 'Stress and performance tests',
            'quick': 'Quick smoke tests'
        }
    
    def parse_results(self) -> None:
        """Parse all test results from the results directory"""
        print(f"Parsing results from: {self.results_dir}")
        
        # Look for xfstests check output
        check_log = self.results_dir / "test_execution.log"
        if check_log.exists():
            self._parse_check_log(check_log)
        
        # Look for individual test logs
        self._parse_individual_logs()
        
        # Generate summary
        self._generate_summary()
        
        print(f"Parsed {len(self.test_results)} test results")
    
    def _parse_check_log(self, log_file: Path) -> None:
        """Parse the main xfstests check log"""
        print(f"Parsing check log: {log_file}")
        
        with open(log_file, 'r') as f:
            content = f.read()
        
        # Extract test results from check output
        lines = content.split('\n')
        current_test = None
        test_output = []
        
        for line in lines:
            line = line.strip()
            if not line:
                continue
            
            # Check for test start
            test_match = re.match(r'^(\S+/\d+)\s+', line)
            if test_match:
                # Save previous test if exists
                if current_test:
                    self._add_test_result(current_test, '\n'.join(test_output))
                
                current_test = line
                test_output = []
            else:
                test_output.append(line)
        
        # Save last test
        if current_test:
            self._add_test_result(current_test, '\n'.join(test_output))
    
    def _add_test_result(self, result_line: str, output: str) -> None:
        """Add a test result from a result line"""
        test_id = None
        status = 'unknown'
        duration = 0.0
        error_message = None
        
        # Try to match different result patterns
        for status_type, pattern in self.result_patterns.items():
            match = pattern.match(result_line)
            if match:
                test_id = match.group(1)
                status = status_type
                
                if status_type == 'pass':
                    duration = float(match.group(2))
                elif status_type in ['skip', 'notrun']:
                    error_message = match.group(2) if len(match.groups()) > 1 else None
                elif status_type == 'fail':
                    error_message = f"Exit status {match.group(2)}"
                
                break
        
        if not test_id:
            # Try to extract test ID from line
            match = re.match(r'^(\S+)', result_line)
            if match:
                test_id = match.group(1)
                if 'failed' in result_line.lower():
                    status = 'fail'
                elif 'not run' in result_line.lower():
                    status = 'skip'
        
        if test_id:
            # Determine category
            category = self._get_test_category(test_id)
            
            test_result = TestResult(
                test_id=test_id,
                test_name=self._get_test_name(test_id),
                status=status,
                duration=duration,
                output=output,
                error_message=error_message,
                category=category
            )
            
            self.test_results.append(test_result)
    
    def _parse_individual_logs(self) -> None:
        """Parse individual test log files"""
        # Look for .out and .full files
        for log_file in self.results_dir.glob("*.out"):
            test_id = log_file.stem
            
            # Check if we already have this test
            existing = next((t for t in self.test_results if t.test_id == test_id), None)
            if existing:
                # Update with detailed output
                with open(log_file, 'r') as f:
                    existing.output = f.read()
            else:
                # Create new test result
                with open(log_file, 'r') as f:
                    output = f.read()
                
                # Try to determine status from output
                status = 'pass'  # Default assumption
                if 'FAIL' in output or 'ERROR' in output:
                    status = 'fail'
                elif 'SKIP' in output or 'not run' in output:
                    status = 'skip'
                
                test_result = TestResult(
                    test_id=test_id,
                    test_name=self._get_test_name(test_id),
                    status=status,
                    duration=0.0,
                    output=output,
                    category=self._get_test_category(test_id)
                )
                
                self.test_results.append(test_result)
    
    def _get_test_category(self, test_id: str) -> str:
        """Determine test category from test ID"""
        if '/' in test_id:
            category = test_id.split('/')[0]
            return category if category in self.test_categories else 'other'
        return 'other'
    
    def _get_test_name(self, test_id: str) -> str:
        """Get human-readable test name"""
        # For now, just return the test ID
        # Could be enhanced to read test descriptions
        return test_id
    
    def _generate_summary(self) -> None:
        """Generate test summary statistics"""
        if not self.test_results:
            self.summary = TestSummary(0, 0, 0, 0, 0, 0.0, 0.0, {})
            return
        
        total_tests = len(self.test_results)
        passed = sum(1 for t in self.test_results if t.status == 'pass')
        failed = sum(1 for t in self.test_results if t.status == 'fail')
        skipped = sum(1 for t in self.test_results if t.status == 'skip')
        notrun = sum(1 for t in self.test_results if t.status == 'notrun')
        
        total_duration = sum(t.duration for t in self.test_results)
        pass_rate = (passed / total_tests * 100) if total_tests > 0 else 0.0
        
        # Category breakdown
        categories = defaultdict(lambda: defaultdict(int))
        for test in self.test_results:
            category = test.category or 'other'
            categories[category][test.status] += 1
            categories[category]['total'] += 1
        
        self.summary = TestSummary(
            total_tests=total_tests,
            passed=passed,
            failed=failed,
            skipped=skipped,
            notrun=notrun,
            total_duration=total_duration,
            pass_rate=pass_rate,
            categories=dict(categories)
        )
    
    def generate_text_report(self, output_file: Optional[str] = None) -> str:
        """Generate a text report"""
        if not self.summary:
            return "No test results to report"
        
        report_lines = [
            "VexFS v2.0 xfstests Results Report",
            "=" * 40,
            f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
            "",
            "SUMMARY",
            "-" * 20,
            f"Total tests: {self.summary.total_tests}",
            f"Passed: {self.summary.passed} ({self.summary.pass_rate:.1f}%)",
            f"Failed: {self.summary.failed}",
            f"Skipped: {self.summary.skipped}",
            f"Not run: {self.summary.notrun}",
            f"Total duration: {self.summary.total_duration:.1f}s",
            ""
        ]
        
        # Category breakdown
        if self.summary.categories:
            report_lines.extend([
                "CATEGORY BREAKDOWN",
                "-" * 20
            ])
            
            for category, stats in self.summary.categories.items():
                category_name = self.test_categories.get(category, category)
                total = stats.get('total', 0)
                passed = stats.get('pass', 0)
                failed = stats.get('fail', 0)
                
                pass_rate = (passed / total * 100) if total > 0 else 0.0
                
                report_lines.extend([
                    f"{category_name}:",
                    f"  Total: {total}, Passed: {passed} ({pass_rate:.1f}%), Failed: {failed}",
                    ""
                ])
        
        # Failed tests
        failed_tests = [t for t in self.test_results if t.status == 'fail']
        if failed_tests:
            report_lines.extend([
                "FAILED TESTS",
                "-" * 20
            ])
            
            for test in failed_tests:
                report_lines.extend([
                    f"Test: {test.test_id}",
                    f"Error: {test.error_message or 'Unknown error'}",
                    ""
                ])
        
        # Skipped tests
        skipped_tests = [t for t in self.test_results if t.status in ['skip', 'notrun']]
        if skipped_tests:
            report_lines.extend([
                "SKIPPED/NOT RUN TESTS",
                "-" * 20
            ])
            
            for test in skipped_tests:
                report_lines.extend([
                    f"Test: {test.test_id} ({test.status})",
                    f"Reason: {test.error_message or 'No reason given'}",
                    ""
                ])
        
        report = '\n'.join(report_lines)
        
        if output_file:
            with open(output_file, 'w') as f:
                f.write(report)
            print(f"Text report saved to: {output_file}")
        
        return report
    
    def generate_html_report(self, output_file: str) -> None:
        """Generate an HTML report"""
        if not self.summary:
            return
        
        html_content = f"""
<!DOCTYPE html>
<html>
<head>
    <title>VexFS v2.0 xfstests Results</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background-color: #f0f0f0; padding: 20px; border-radius: 5px; }}
        .summary {{ margin: 20px 0; }}
        .category {{ margin: 10px 0; }}
        .pass {{ color: green; font-weight: bold; }}
        .fail {{ color: red; font-weight: bold; }}
        .skip {{ color: orange; font-weight: bold; }}
        .notrun {{ color: gray; font-weight: bold; }}
        table {{ border-collapse: collapse; width: 100%; margin: 20px 0; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #f2f2f2; }}
        .test-output {{ background-color: #f8f8f8; padding: 10px; font-family: monospace; white-space: pre-wrap; max-height: 200px; overflow-y: auto; }}
        .progress-bar {{ width: 100%; height: 20px; background-color: #f0f0f0; border-radius: 10px; overflow: hidden; }}
        .progress-fill {{ height: 100%; background-color: #4CAF50; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>VexFS v2.0 xfstests Results</h1>
        <p><strong>Generated:</strong> {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}</p>
        <p><strong>Results Directory:</strong> {self.results_dir}</p>
    </div>
    
    <div class="summary">
        <h2>Test Summary</h2>
        <div class="progress-bar">
            <div class="progress-fill" style="width: {self.summary.pass_rate}%"></div>
        </div>
        <p><strong>Pass Rate:</strong> {self.summary.pass_rate:.1f}% ({self.summary.passed}/{self.summary.total_tests})</p>
        
        <table>
            <tr>
                <th>Status</th>
                <th>Count</th>
                <th>Percentage</th>
            </tr>
            <tr>
                <td class="pass">Passed</td>
                <td>{self.summary.passed}</td>
                <td>{self.summary.passed/self.summary.total_tests*100:.1f}%</td>
            </tr>
            <tr>
                <td class="fail">Failed</td>
                <td>{self.summary.failed}</td>
                <td>{self.summary.failed/self.summary.total_tests*100:.1f}%</td>
            </tr>
            <tr>
                <td class="skip">Skipped</td>
                <td>{self.summary.skipped}</td>
                <td>{self.summary.skipped/self.summary.total_tests*100:.1f}%</td>
            </tr>
            <tr>
                <td class="notrun">Not Run</td>
                <td>{self.summary.notrun}</td>
                <td>{self.summary.notrun/self.summary.total_tests*100:.1f}%</td>
            </tr>
        </table>
        
        <p><strong>Total Duration:</strong> {self.summary.total_duration:.1f} seconds</p>
    </div>
"""
        
        # Category breakdown
        if self.summary.categories:
            html_content += """
    <div class="categories">
        <h2>Category Breakdown</h2>
        <table>
            <tr>
                <th>Category</th>
                <th>Total</th>
                <th>Passed</th>
                <th>Failed</th>
                <th>Skipped</th>
                <th>Pass Rate</th>
            </tr>
"""
            
            for category, stats in self.summary.categories.items():
                category_name = self.test_categories.get(category, category)
                total = stats.get('total', 0)
                passed = stats.get('pass', 0)
                failed = stats.get('fail', 0)
                skipped = stats.get('skip', 0) + stats.get('notrun', 0)
                pass_rate = (passed / total * 100) if total > 0 else 0.0
                
                html_content += f"""
            <tr>
                <td>{category_name}</td>
                <td>{total}</td>
                <td class="pass">{passed}</td>
                <td class="fail">{failed}</td>
                <td class="skip">{skipped}</td>
                <td>{pass_rate:.1f}%</td>
            </tr>
"""
            
            html_content += """
        </table>
    </div>
"""
        
        # Detailed test results
        html_content += """
    <div class="test-details">
        <h2>Detailed Test Results</h2>
        <table>
            <tr>
                <th>Test ID</th>
                <th>Category</th>
                <th>Status</th>
                <th>Duration</th>
                <th>Error Message</th>
            </tr>
"""
        
        for test in sorted(self.test_results, key=lambda t: t.test_id):
            status_class = test.status
            error_msg = test.error_message or ""
            
            html_content += f"""
            <tr>
                <td>{test.test_id}</td>
                <td>{test.category or 'other'}</td>
                <td class="{status_class}">{test.status}</td>
                <td>{test.duration:.1f}s</td>
                <td>{error_msg}</td>
            </tr>
"""
        
        html_content += """
        </table>
    </div>
</body>
</html>
"""
        
        with open(output_file, 'w') as f:
            f.write(html_content)
        
        print(f"HTML report saved to: {output_file}")
    
    def generate_json_report(self, output_file: str) -> None:
        """Generate a JSON report for programmatic analysis"""
        report_data = {
            'timestamp': datetime.now().isoformat(),
            'results_directory': str(self.results_dir),
            'summary': asdict(self.summary) if self.summary else None,
            'test_results': [asdict(test) for test in self.test_results]
        }
        
        with open(output_file, 'w') as f:
            json.dump(report_data, f, indent=2)
        
        print(f"JSON report saved to: {output_file}")

def main():
    parser = argparse.ArgumentParser(description='Parse VexFS xfstests results')
    parser.add_argument('results_dir', help='Directory containing test results')
    parser.add_argument('--text-report', help='Output file for text report')
    parser.add_argument('--html-report', help='Output file for HTML report')
    parser.add_argument('--json-report', help='Output file for JSON report')
    parser.add_argument('--print-summary', action='store_true', help='Print summary to stdout')
    
    args = parser.parse_args()
    
    if not os.path.exists(args.results_dir):
        print(f"Error: Results directory '{args.results_dir}' does not exist")
        sys.exit(1)
    
    # Parse results
    parser = XFSTestsResultParser(args.results_dir)
    parser.parse_results()
    
    # Generate reports
    if args.text_report:
        parser.generate_text_report(args.text_report)
    
    if args.html_report:
        parser.generate_html_report(args.html_report)
    
    if args.json_report:
        parser.generate_json_report(args.json_report)
    
    if args.print_summary:
        print(parser.generate_text_report())

if __name__ == '__main__':
    main()