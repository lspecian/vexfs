#!/usr/bin/env python3
"""
VexFS Real-time Performance Monitor

Monitors VexFS performance metrics in real-time and displays a dashboard
"""

import time
import requests
import psutil
import os
import sys
import threading
from datetime import datetime
from collections import deque
import curses

class VexFSMonitor:
    def __init__(self, api_url="http://localhost:7680"):
        self.api_url = api_url
        self.metrics = {
            'requests_per_sec': deque(maxlen=60),
            'latency_ms': deque(maxlen=60),
            'cpu_percent': deque(maxlen=60),
            'memory_mb': deque(maxlen=60),
            'collections': 0,
            'documents': 0,
            'uptime': 0,
            'errors': 0
        }
        self.running = True
        self.last_request_count = 0
        self.request_times = deque(maxlen=100)
        
    def check_api_health(self):
        """Check if API server is healthy"""
        try:
            response = requests.get(f"{self.api_url}/health", timeout=1)
            return response.status_code == 200
        except:
            return False
    
    def get_metrics(self):
        """Fetch current metrics from API"""
        try:
            response = requests.get(f"{self.api_url}/metrics", timeout=1)
            if response.status_code == 200:
                data = response.json()
                self.metrics['collections'] = data.get('collections_count', 0)
                self.metrics['documents'] = data.get('total_documents', 0)
                self.metrics['uptime'] = data.get('uptime_seconds', 0)
        except:
            self.metrics['errors'] += 1
    
    def measure_latency(self):
        """Measure API latency"""
        try:
            start = time.perf_counter()
            requests.get(f"{self.api_url}/health", timeout=1)
            latency = (time.perf_counter() - start) * 1000
            self.metrics['latency_ms'].append(latency)
            self.request_times.append(time.time())
        except:
            self.metrics['latency_ms'].append(0)
    
    def get_process_metrics(self):
        """Get process-level metrics"""
        # Find VexFS processes
        for proc in psutil.process_iter(['pid', 'name', 'cpu_percent', 'memory_info']):
            try:
                if 'vexfs' in proc.info['name'].lower():
                    self.metrics['cpu_percent'].append(proc.info['cpu_percent'])
                    self.metrics['memory_mb'].append(
                        proc.info['memory_info'].rss / 1024 / 1024
                    )
                    break
            except:
                pass
        else:
            # No process found, append zeros
            self.metrics['cpu_percent'].append(0)
            self.metrics['memory_mb'].append(0)
    
    def calculate_rps(self):
        """Calculate requests per second"""
        now = time.time()
        recent_requests = [t for t in self.request_times if now - t <= 1.0]
        self.metrics['requests_per_sec'].append(len(recent_requests))
    
    def monitor_loop(self):
        """Background monitoring loop"""
        while self.running:
            self.get_metrics()
            self.measure_latency()
            self.get_process_metrics()
            self.calculate_rps()
            time.sleep(1)
    
    def format_uptime(self, seconds):
        """Format uptime nicely"""
        hours = int(seconds // 3600)
        minutes = int((seconds % 3600) // 60)
        secs = int(seconds % 60)
        return f"{hours:02d}:{minutes:02d}:{secs:02d}"
    
    def draw_dashboard(self, stdscr):
        """Draw the monitoring dashboard"""
        curses.curs_set(0)  # Hide cursor
        stdscr.nodelay(1)    # Non-blocking input
        
        # Color pairs
        curses.init_pair(1, curses.COLOR_GREEN, curses.COLOR_BLACK)
        curses.init_pair(2, curses.COLOR_YELLOW, curses.COLOR_BLACK)
        curses.init_pair(3, curses.COLOR_RED, curses.COLOR_BLACK)
        curses.init_pair(4, curses.COLOR_CYAN, curses.COLOR_BLACK)
        
        # Start monitoring thread
        monitor_thread = threading.Thread(target=self.monitor_loop)
        monitor_thread.daemon = True
        monitor_thread.start()
        
        while self.running:
            try:
                height, width = stdscr.getmaxyx()
                stdscr.clear()
                
                # Header
                header = "VexFS Performance Monitor"
                stdscr.addstr(0, (width - len(header)) // 2, header, 
                             curses.A_BOLD | curses.color_pair(4))
                stdscr.addstr(1, 0, "=" * width)
                
                # Status
                row = 3
                status = "ONLINE" if self.check_api_health() else "OFFLINE"
                status_color = curses.color_pair(1) if status == "ONLINE" else curses.color_pair(3)
                stdscr.addstr(row, 0, f"Status: ")
                stdscr.addstr(row, 8, status, status_color | curses.A_BOLD)
                
                uptime = self.format_uptime(self.metrics['uptime'])
                stdscr.addstr(row, 25, f"Uptime: {uptime}")
                
                stdscr.addstr(row, 50, f"Time: {datetime.now().strftime('%H:%M:%S')}")
                
                # Metrics
                row += 2
                stdscr.addstr(row, 0, "Performance Metrics:", curses.A_BOLD)
                row += 1
                stdscr.addstr(row, 0, "-" * 50)
                
                # Request rate
                row += 1
                rps = self.metrics['requests_per_sec'][-1] if self.metrics['requests_per_sec'] else 0
                avg_rps = sum(self.metrics['requests_per_sec']) / max(len(self.metrics['requests_per_sec']), 1)
                stdscr.addstr(row, 0, f"Requests/sec: {rps:6.1f}  (avg: {avg_rps:6.1f})")
                
                # Latency
                row += 1
                latency = self.metrics['latency_ms'][-1] if self.metrics['latency_ms'] else 0
                avg_latency = sum(self.metrics['latency_ms']) / max(len(self.metrics['latency_ms']), 1)
                latency_color = curses.color_pair(1) if latency < 10 else (
                    curses.color_pair(2) if latency < 50 else curses.color_pair(3)
                )
                stdscr.addstr(row, 0, f"Latency (ms): ")
                stdscr.addstr(row, 14, f"{latency:6.1f}", latency_color)
                stdscr.addstr(row, 22, f"(avg: {avg_latency:6.1f})")
                
                # Resource usage
                row += 2
                stdscr.addstr(row, 0, "Resource Usage:", curses.A_BOLD)
                row += 1
                stdscr.addstr(row, 0, "-" * 50)
                
                # CPU
                row += 1
                cpu = self.metrics['cpu_percent'][-1] if self.metrics['cpu_percent'] else 0
                cpu_color = curses.color_pair(1) if cpu < 50 else (
                    curses.color_pair(2) if cpu < 80 else curses.color_pair(3)
                )
                stdscr.addstr(row, 0, f"CPU Usage:    ")
                stdscr.addstr(row, 14, f"{cpu:5.1f}%", cpu_color)
                
                # Memory
                row += 1
                mem = self.metrics['memory_mb'][-1] if self.metrics['memory_mb'] else 0
                stdscr.addstr(row, 0, f"Memory (MB):  {mem:6.1f}")
                
                # Data metrics
                row += 2
                stdscr.addstr(row, 0, "Data Metrics:", curses.A_BOLD)
                row += 1
                stdscr.addstr(row, 0, "-" * 50)
                
                row += 1
                stdscr.addstr(row, 0, f"Collections:  {self.metrics['collections']:6d}")
                row += 1
                stdscr.addstr(row, 0, f"Documents:    {self.metrics['documents']:6d}")
                row += 1
                stdscr.addstr(row, 0, f"Errors:       {self.metrics['errors']:6d}")
                
                # Graph (simple ASCII)
                row += 2
                if row < height - 5 and self.metrics['latency_ms']:
                    stdscr.addstr(row, 0, "Latency Graph (last 60s):", curses.A_BOLD)
                    row += 1
                    
                    # Draw simple graph
                    graph_height = min(10, height - row - 2)
                    graph_width = min(60, width - 2)
                    
                    if len(self.metrics['latency_ms']) > 0:
                        max_lat = max(self.metrics['latency_ms']) or 1
                        for i in range(graph_height):
                            threshold = max_lat * (graph_height - i) / graph_height
                            line = ""
                            for val in list(self.metrics['latency_ms'])[-graph_width:]:
                                if val >= threshold:
                                    line += "█"
                                else:
                                    line += " "
                            stdscr.addstr(row + i, 0, line[:width-1])
                
                # Footer
                footer_row = height - 1
                footer = "Press 'q' to quit, 'r' to reset metrics"
                stdscr.addstr(footer_row, 0, footer)
                
                stdscr.refresh()
                
                # Handle input
                key = stdscr.getch()
                if key == ord('q'):
                    self.running = False
                elif key == ord('r'):
                    # Reset metrics
                    for key in self.metrics:
                        if isinstance(self.metrics[key], deque):
                            self.metrics[key].clear()
                        elif isinstance(self.metrics[key], int):
                            self.metrics[key] = 0
                
                time.sleep(0.1)
                
            except KeyboardInterrupt:
                self.running = False
            except curses.error:
                pass  # Ignore curses errors
    
    def run(self):
        """Run the monitor with curses interface"""
        try:
            curses.wrapper(self.draw_dashboard)
        except KeyboardInterrupt:
            print("\nMonitoring stopped.")
        finally:
            self.running = False

def main():
    import argparse
    
    parser = argparse.ArgumentParser(description="VexFS Performance Monitor")
    parser.add_argument("--url", default="http://localhost:7680",
                       help="VexFS API server URL")
    parser.add_argument("--simple", action="store_true",
                       help="Simple text output instead of dashboard")
    
    args = parser.parse_args()
    
    monitor = VexFSMonitor(args.url)
    
    if args.simple:
        # Simple monitoring mode
        print("VexFS Performance Monitor (Simple Mode)")
        print("Press Ctrl+C to stop")
        print("-" * 50)
        
        try:
            while True:
                monitor.get_metrics()
                monitor.measure_latency()
                monitor.get_process_metrics()
                
                latency = monitor.metrics['latency_ms'][-1] if monitor.metrics['latency_ms'] else 0
                cpu = monitor.metrics['cpu_percent'][-1] if monitor.metrics['cpu_percent'] else 0
                mem = monitor.metrics['memory_mb'][-1] if monitor.metrics['memory_mb'] else 0
                
                print(f"\r[{datetime.now().strftime('%H:%M:%S')}] "
                      f"Latency: {latency:5.1f}ms | "
                      f"CPU: {cpu:5.1f}% | "
                      f"Mem: {mem:6.1f}MB | "
                      f"Collections: {monitor.metrics['collections']} | "
                      f"Docs: {monitor.metrics['documents']}", end="")
                
                time.sleep(1)
        except KeyboardInterrupt:
            print("\n\nMonitoring stopped.")
    else:
        # Full dashboard mode
        monitor.run()

if __name__ == "__main__":
    main()