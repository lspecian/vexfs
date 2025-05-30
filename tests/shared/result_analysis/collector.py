"""
Test Result Collection Framework
Collects, aggregates, and stores test results from multiple domains and VMs
"""

import asyncio
import json
import logging
import time
from dataclasses import dataclass, field, asdict
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional, Any, Union
import sqlite3
import aiofiles
import aiohttp
from enum import Enum


class StorageBackend(Enum):
    """Available storage backends for test results"""
    FILESYSTEM = "filesystem"
    DATABASE = "database"
    S3 = "s3"
    ELASTICSEARCH = "elasticsearch"


@dataclass
class TestResultMetadata:
    """Metadata for test result collection"""
    collection_id: str
    timestamp: float
    domain: str
    vm_hostname: str
    vm_specs: Dict[str, Any]
    environment: str
    version: str
    git_commit: Optional[str] = None
    test_suite_version: str = "1.0.0"


@dataclass
class AggregatedResult:
    """Aggregated test results across multiple executions"""
    domain: str
    test_name: str
    total_executions: int
    successful_executions: int
    failed_executions: int
    error_executions: int
    timeout_executions: int
    success_rate: float
    average_duration: float
    min_duration: float
    max_duration: float
    last_execution: float
    trend_direction: str  # "improving", "degrading", "stable"
    performance_metrics: Dict[str, Any] = field(default_factory=dict)


@dataclass
class TrendAnalysis:
    """Trend analysis for test results over time"""
    domain: str
    test_name: str
    time_period: str  # "1h", "24h", "7d", "30d"
    data_points: List[Dict[str, Any]]
    trend_direction: str
    confidence_score: float
    anomalies: List[Dict[str, Any]]
    recommendations: List[str]


class ResultCollector:
    """
    Main result collection and storage system
    
    Handles:
    - Collection from multiple VMs and domains
    - Storage in various backends
    - Real-time aggregation and analysis
    - Trend detection and alerting
    """
    
    def __init__(self, config: Dict[str, Any]):
        self.config = config
        self.logger = logging.getLogger(f"{__name__}.ResultCollector")
        
        # Storage configuration
        self.storage_backend = StorageBackend(config.get("storage_backend", "filesystem"))
        self.storage_path = Path(config.get("storage_path", "/tmp/vexfs_test_results"))
        self.database_url = config.get("database_url", "sqlite:///vexfs_test_results.db")
        
        # Collection state
        self.active_collections: Dict[str, Dict[str, Any]] = {}
        self.result_cache: Dict[str, List[Dict[str, Any]]] = {}
        
        # Analysis configuration
        self.enable_real_time_analysis = config.get("enable_real_time_analysis", True)
        self.trend_analysis_window = config.get("trend_analysis_window", "24h")
        self.anomaly_detection_threshold = config.get("anomaly_detection_threshold", 2.0)
        
        # Initialize storage
        asyncio.create_task(self._initialize_storage())
    
    async def _initialize_storage(self):
        """Initialize storage backend"""
        try:
            if self.storage_backend == StorageBackend.FILESYSTEM:
                await self._initialize_filesystem_storage()
            elif self.storage_backend == StorageBackend.DATABASE:
                await self._initialize_database_storage()
            elif self.storage_backend == StorageBackend.S3:
                await self._initialize_s3_storage()
            elif self.storage_backend == StorageBackend.ELASTICSEARCH:
                await self._initialize_elasticsearch_storage()
                
            self.logger.info(f"Storage backend {self.storage_backend.value} initialized")
            
        except Exception as e:
            self.logger.error(f"Failed to initialize storage backend: {str(e)}")
            raise
    
    async def _initialize_filesystem_storage(self):
        """Initialize filesystem storage"""
        self.storage_path.mkdir(parents=True, exist_ok=True)
        
        # Create directory structure
        directories = [
            "raw_results",
            "aggregated_results", 
            "trend_analysis",
            "reports",
            "artifacts"
        ]
        
        for directory in directories:
            (self.storage_path / directory).mkdir(exist_ok=True)
    
    async def _initialize_database_storage(self):
        """Initialize database storage"""
        # For SQLite, create tables
        if self.database_url.startswith("sqlite"):
            db_path = self.database_url.replace("sqlite:///", "")
            conn = sqlite3.connect(db_path)
            
            # Create tables
            conn.executescript("""
                CREATE TABLE IF NOT EXISTS test_results (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    collection_id TEXT NOT NULL,
                    domain TEXT NOT NULL,
                    test_name TEXT NOT NULL,
                    status TEXT NOT NULL,
                    duration REAL NOT NULL,
                    timestamp REAL NOT NULL,
                    vm_hostname TEXT NOT NULL,
                    metadata TEXT,
                    metrics TEXT,
                    artifacts TEXT,
                    logs TEXT
                );
                
                CREATE TABLE IF NOT EXISTS aggregated_results (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    domain TEXT NOT NULL,
                    test_name TEXT NOT NULL,
                    total_executions INTEGER NOT NULL,
                    successful_executions INTEGER NOT NULL,
                    failed_executions INTEGER NOT NULL,
                    success_rate REAL NOT NULL,
                    average_duration REAL NOT NULL,
                    last_updated REAL NOT NULL,
                    trend_direction TEXT,
                    performance_metrics TEXT
                );
                
                CREATE TABLE IF NOT EXISTS trend_analysis (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    domain TEXT NOT NULL,
                    test_name TEXT NOT NULL,
                    time_period TEXT NOT NULL,
                    trend_direction TEXT NOT NULL,
                    confidence_score REAL NOT NULL,
                    data_points TEXT,
                    anomalies TEXT,
                    recommendations TEXT,
                    created_at REAL NOT NULL
                );
                
                CREATE INDEX IF NOT EXISTS idx_test_results_domain_test ON test_results(domain, test_name);
                CREATE INDEX IF NOT EXISTS idx_test_results_timestamp ON test_results(timestamp);
                CREATE INDEX IF NOT EXISTS idx_aggregated_domain_test ON aggregated_results(domain, test_name);
            """)
            
            conn.close()
    
    async def _initialize_s3_storage(self):
        """Initialize S3 storage"""
        # Implementation for S3 storage
        pass
    
    async def _initialize_elasticsearch_storage(self):
        """Initialize Elasticsearch storage"""
        # Implementation for Elasticsearch storage
        pass
    
    async def start_collection(self, collection_id: str, metadata: TestResultMetadata) -> bool:
        """Start a new test result collection session"""
        try:
            self.active_collections[collection_id] = {
                "metadata": asdict(metadata),
                "start_time": time.time(),
                "results": [],
                "status": "active"
            }
            
            self.logger.info(f"Started collection session: {collection_id}")
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to start collection {collection_id}: {str(e)}")
            return False
    
    async def store_domain_results(self, domain: str, results: Dict[str, Any]) -> bool:
        """Store results for a specific domain"""
        try:
            collection_id = results.get("collection_id", f"{domain}_{int(time.time())}")
            
            # Store raw results
            await self._store_raw_results(collection_id, domain, results)
            
            # Update aggregated results
            if self.enable_real_time_analysis:
                await self._update_aggregated_results(domain, results)
            
            # Trigger trend analysis
            await self._trigger_trend_analysis(domain)
            
            self.logger.info(f"Stored results for domain {domain}, collection {collection_id}")
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to store domain results: {str(e)}")
            return False
    
    async def _store_raw_results(self, collection_id: str, domain: str, results: Dict[str, Any]):
        """Store raw test results"""
        if self.storage_backend == StorageBackend.FILESYSTEM:
            await self._store_raw_results_filesystem(collection_id, domain, results)
        elif self.storage_backend == StorageBackend.DATABASE:
            await self._store_raw_results_database(collection_id, domain, results)
    
    async def _store_raw_results_filesystem(self, collection_id: str, domain: str, results: Dict[str, Any]):
        """Store raw results to filesystem"""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        filename = f"{domain}_{collection_id}_{timestamp}.json"
        filepath = self.storage_path / "raw_results" / filename
        
        async with aiofiles.open(filepath, 'w') as f:
            await f.write(json.dumps(results, indent=2))
    
    async def _store_raw_results_database(self, collection_id: str, domain: str, results: Dict[str, Any]):
        """Store raw results to database"""
        # For SQLite implementation
        if self.database_url.startswith("sqlite"):
            db_path = self.database_url.replace("sqlite:///", "")
            
            # Use asyncio to run database operations in thread pool
            def store_to_db():
                conn = sqlite3.connect(db_path)
                
                for result in results.get("results", []):
                    conn.execute("""
                        INSERT INTO test_results 
                        (collection_id, domain, test_name, status, duration, timestamp, 
                         vm_hostname, metadata, metrics, artifacts, logs)
                        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """, (
                        collection_id,
                        domain,
                        result.get("test_name", "unknown"),
                        result.get("status", "unknown"),
                        result.get("duration", 0.0),
                        result.get("timestamp", time.time()),
                        results.get("vm_hostname", "unknown"),
                        json.dumps(result.get("metadata", {})),
                        json.dumps(result.get("metrics", {})),
                        json.dumps(result.get("artifacts", [])),
                        json.dumps(result.get("logs", []))
                    ))
                
                conn.commit()
                conn.close()
            
            await asyncio.get_event_loop().run_in_executor(None, store_to_db)
    
    async def _update_aggregated_results(self, domain: str, results: Dict[str, Any]):
        """Update aggregated results for real-time analysis"""
        try:
            for result in results.get("results", []):
                test_name = result.get("test_name", "unknown")
                status = result.get("status", "unknown")
                duration = result.get("duration", 0.0)
                
                # Get or create aggregated result
                key = f"{domain}:{test_name}"
                if key not in self.result_cache:
                    self.result_cache[key] = []
                
                # Add new result
                self.result_cache[key].append({
                    "status": status,
                    "duration": duration,
                    "timestamp": time.time(),
                    "metrics": result.get("metrics", {})
                })
                
                # Keep only recent results (last 1000 executions)
                if len(self.result_cache[key]) > 1000:
                    self.result_cache[key] = self.result_cache[key][-1000:]
                
                # Calculate aggregated metrics
                aggregated = self._calculate_aggregated_metrics(self.result_cache[key])
                
                # Store aggregated results
                await self._store_aggregated_result(domain, test_name, aggregated)
                
        except Exception as e:
            self.logger.error(f"Failed to update aggregated results: {str(e)}")
    
    def _calculate_aggregated_metrics(self, results: List[Dict[str, Any]]) -> AggregatedResult:
        """Calculate aggregated metrics from result list"""
        total = len(results)
        successful = len([r for r in results if r["status"] == "passed"])
        failed = len([r for r in results if r["status"] == "failed"])
        errors = len([r for r in results if r["status"] == "error"])
        timeouts = len([r for r in results if r["status"] == "timeout"])
        
        durations = [r["duration"] for r in results if r["duration"] > 0]
        avg_duration = sum(durations) / len(durations) if durations else 0
        min_duration = min(durations) if durations else 0
        max_duration = max(durations) if durations else 0
        
        success_rate = (successful / total * 100) if total > 0 else 0
        
        # Calculate trend direction
        trend_direction = self._calculate_trend_direction(results)
        
        return AggregatedResult(
            domain="",  # Will be set by caller
            test_name="",  # Will be set by caller
            total_executions=total,
            successful_executions=successful,
            failed_executions=failed,
            error_executions=errors,
            timeout_executions=timeouts,
            success_rate=success_rate,
            average_duration=avg_duration,
            min_duration=min_duration,
            max_duration=max_duration,
            last_execution=max(r["timestamp"] for r in results) if results else 0,
            trend_direction=trend_direction
        )
    
    def _calculate_trend_direction(self, results: List[Dict[str, Any]]) -> str:
        """Calculate trend direction from recent results"""
        if len(results) < 10:
            return "stable"
        
        # Take last 20 results and compare first 10 vs last 10
        recent = results[-20:]
        first_half = recent[:10]
        second_half = recent[10:]
        
        first_success_rate = len([r for r in first_half if r["status"] == "passed"]) / len(first_half)
        second_success_rate = len([r for r in second_half if r["status"] == "passed"]) / len(second_half)
        
        if second_success_rate > first_success_rate + 0.1:
            return "improving"
        elif second_success_rate < first_success_rate - 0.1:
            return "degrading"
        else:
            return "stable"
    
    async def _store_aggregated_result(self, domain: str, test_name: str, aggregated: AggregatedResult):
        """Store aggregated result"""
        aggregated.domain = domain
        aggregated.test_name = test_name
        
        if self.storage_backend == StorageBackend.FILESYSTEM:
            filepath = self.storage_path / "aggregated_results" / f"{domain}_{test_name}.json"
            async with aiofiles.open(filepath, 'w') as f:
                await f.write(json.dumps(asdict(aggregated), indent=2))
        
        elif self.storage_backend == StorageBackend.DATABASE:
            # Store to database
            if self.database_url.startswith("sqlite"):
                def store_to_db():
                    db_path = self.database_url.replace("sqlite:///", "")
                    conn = sqlite3.connect(db_path)
                    
                    # Upsert aggregated result
                    conn.execute("""
                        INSERT OR REPLACE INTO aggregated_results
                        (domain, test_name, total_executions, successful_executions, 
                         failed_executions, success_rate, average_duration, last_updated,
                         trend_direction, performance_metrics)
                        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """, (
                        domain, test_name, aggregated.total_executions,
                        aggregated.successful_executions, aggregated.failed_executions,
                        aggregated.success_rate, aggregated.average_duration,
                        time.time(), aggregated.trend_direction,
                        json.dumps(aggregated.performance_metrics)
                    ))
                    
                    conn.commit()
                    conn.close()
                
                await asyncio.get_event_loop().run_in_executor(None, store_to_db)
    
    async def _trigger_trend_analysis(self, domain: str):
        """Trigger trend analysis for domain"""
        try:
            # Get recent results for analysis
            recent_results = await self._get_recent_results(domain, self.trend_analysis_window)
            
            # Perform trend analysis
            trend_analysis = await self._perform_trend_analysis(domain, recent_results)
            
            # Store trend analysis
            await self._store_trend_analysis(trend_analysis)
            
            # Check for anomalies and alerts
            await self._check_anomalies_and_alerts(trend_analysis)
            
        except Exception as e:
            self.logger.error(f"Failed to trigger trend analysis for {domain}: {str(e)}")
    
    async def _get_recent_results(self, domain: str, time_window: str) -> List[Dict[str, Any]]:
        """Get recent results for trend analysis"""
        # Convert time window to seconds
        window_seconds = self._parse_time_window(time_window)
        cutoff_time = time.time() - window_seconds
        
        results = []
        
        if self.storage_backend == StorageBackend.DATABASE and self.database_url.startswith("sqlite"):
            def get_from_db():
                db_path = self.database_url.replace("sqlite:///", "")
                conn = sqlite3.connect(db_path)
                
                cursor = conn.execute("""
                    SELECT * FROM test_results 
                    WHERE domain = ? AND timestamp > ?
                    ORDER BY timestamp DESC
                """, (domain, cutoff_time))
                
                results = []
                for row in cursor.fetchall():
                    results.append({
                        "test_name": row[2],
                        "status": row[3],
                        "duration": row[4],
                        "timestamp": row[5],
                        "vm_hostname": row[6],
                        "metadata": json.loads(row[7] or "{}"),
                        "metrics": json.loads(row[8] or "{}"),
                    })
                
                conn.close()
                return results
            
            results = await asyncio.get_event_loop().run_in_executor(None, get_from_db)
        
        return results
    
    def _parse_time_window(self, time_window: str) -> int:
        """Parse time window string to seconds"""
        if time_window.endswith('h'):
            return int(time_window[:-1]) * 3600
        elif time_window.endswith('d'):
            return int(time_window[:-1]) * 86400
        elif time_window.endswith('m'):
            return int(time_window[:-1]) * 60
        else:
            return int(time_window)
    
    async def _perform_trend_analysis(self, domain: str, results: List[Dict[str, Any]]) -> List[TrendAnalysis]:
        """Perform trend analysis on results"""
        # Group results by test name
        test_groups = {}
        for result in results:
            test_name = result["test_name"]
            if test_name not in test_groups:
                test_groups[test_name] = []
            test_groups[test_name].append(result)
        
        trend_analyses = []
        
        for test_name, test_results in test_groups.items():
            if len(test_results) < 5:  # Need minimum data points
                continue
            
            # Calculate trend metrics
            trend_direction = self._calculate_trend_direction(test_results)
            confidence_score = self._calculate_confidence_score(test_results)
            anomalies = self._detect_anomalies(test_results)
            recommendations = self._generate_recommendations(test_results, trend_direction, anomalies)
            
            trend_analysis = TrendAnalysis(
                domain=domain,
                test_name=test_name,
                time_period=self.trend_analysis_window,
                data_points=test_results,
                trend_direction=trend_direction,
                confidence_score=confidence_score,
                anomalies=anomalies,
                recommendations=recommendations
            )
            
            trend_analyses.append(trend_analysis)
        
        return trend_analyses
    
    def _calculate_confidence_score(self, results: List[Dict[str, Any]]) -> float:
        """Calculate confidence score for trend analysis"""
        # Simple confidence based on data points and consistency
        data_points = len(results)
        if data_points < 5:
            return 0.0
        elif data_points < 10:
            return 0.5
        elif data_points < 20:
            return 0.7
        else:
            return 0.9
    
    def _detect_anomalies(self, results: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Detect anomalies in test results"""
        anomalies = []
        
        # Calculate baseline metrics
        durations = [r["duration"] for r in results if r["duration"] > 0]
        if not durations:
            return anomalies
        
        avg_duration = sum(durations) / len(durations)
        std_duration = (sum((d - avg_duration) ** 2 for d in durations) / len(durations)) ** 0.5
        
        # Detect duration anomalies
        threshold = self.anomaly_detection_threshold * std_duration
        
        for result in results:
            if abs(result["duration"] - avg_duration) > threshold:
                anomalies.append({
                    "type": "duration_anomaly",
                    "timestamp": result["timestamp"],
                    "test_name": result["test_name"],
                    "expected_duration": avg_duration,
                    "actual_duration": result["duration"],
                    "deviation": abs(result["duration"] - avg_duration)
                })
        
        return anomalies
    
    def _generate_recommendations(self, results: List[Dict[str, Any]], trend_direction: str, anomalies: List[Dict[str, Any]]) -> List[str]:
        """Generate recommendations based on analysis"""
        recommendations = []
        
        if trend_direction == "degrading":
            recommendations.append("Test performance is degrading. Consider investigating recent changes.")
        
        if len(anomalies) > len(results) * 0.1:  # More than 10% anomalies
            recommendations.append("High number of anomalies detected. Review test environment stability.")
        
        # Calculate failure rate
        failures = len([r for r in results if r["status"] in ["failed", "error", "timeout"]])
        failure_rate = failures / len(results) if results else 0
        
        if failure_rate > 0.1:  # More than 10% failures
            recommendations.append(f"High failure rate ({failure_rate:.1%}). Review test implementation and environment.")
        
        return recommendations
    
    async def _store_trend_analysis(self, trend_analyses: List[TrendAnalysis]):
        """Store trend analysis results"""
        for analysis in trend_analyses:
            if self.storage_backend == StorageBackend.FILESYSTEM:
                timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
                filename = f"{analysis.domain}_{analysis.test_name}_{timestamp}.json"
                filepath = self.storage_path / "trend_analysis" / filename
                
                async with aiofiles.open(filepath, 'w') as f:
                    await f.write(json.dumps(asdict(analysis), indent=2, default=str))
    
    async def _check_anomalies_and_alerts(self, trend_analyses: List[TrendAnalysis]):
        """Check for anomalies and send alerts if configured"""
        for analysis in trend_analyses:
            if analysis.anomalies or analysis.trend_direction == "degrading":
                await self._send_alert(analysis)
    
    async def _send_alert(self, analysis: TrendAnalysis):
        """Send alert for anomalies or degrading trends"""
        alert_config = self.config.get("alerts", {})
        if not alert_config.get("enabled", False):
            return
        
        alert_message = f"""
        VexFS Test Alert - {analysis.domain}
        
        Test: {analysis.test_name}
        Trend: {analysis.trend_direction}
        Confidence: {analysis.confidence_score:.2f}
        Anomalies: {len(analysis.anomalies)}
        
        Recommendations:
        {chr(10).join(f"- {rec}" for rec in analysis.recommendations)}
        """
        
        # Send to configured webhook
        webhook_url = alert_config.get("webhook_url")
        if webhook_url:
            try:
                async with aiohttp.ClientSession() as session:
                    await session.post(webhook_url, json={
                        "text": alert_message,
                        "domain": analysis.domain,
                        "test_name": analysis.test_name,
                        "trend_direction": analysis.trend_direction,
                        "anomaly_count": len(analysis.anomalies)
                    })
            except Exception as e:
                self.logger.error(f"Failed to send alert: {str(e)}")
    
    async def get_domain_statistics(self, domain: str) -> Dict[str, Any]:
        """Get statistics for a specific domain"""
        # Implementation to retrieve and calculate domain statistics
        pass
    
    async def get_trend_analysis(self, domain: str, test_name: str = None) -> List[TrendAnalysis]:
        """Get trend analysis for domain or specific test"""
        # Implementation to retrieve trend analysis
        pass
    
    async def generate_report(self, domains: List[str] = None, format: str = "json") -> str:
        """Generate comprehensive test report"""
        # Implementation to generate reports
        pass