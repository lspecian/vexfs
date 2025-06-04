"""
Batch Operations Engine

This module implements optimized batch operations for maximum throughput,
leveraging VexFS v2's high-performance capabilities and asyncio for parallel execution.
"""

import asyncio
import time
import logging
from typing import Dict, List, Any, Optional, Union, Tuple
from concurrent.futures import ThreadPoolExecutor, as_completed
from ..core.vexfs_client import VexFSClient, VexFSError
from ..filters.filter_engine import FilterEngine

logger = logging.getLogger(__name__)


class BatchOperations:
    """
    Optimized batch operations for maximum throughput.
    
    Provides high-performance batch search, grouped search, and optimized
    batch upsert operations using VexFS v2's capabilities and parallel execution.
    
    Performance targets:
    - Batch search: >50 queries/sec with parallel execution
    - Batch upsert: Maintain 95,117 ops/sec performance
    - Memory efficiency: Configurable batch sizes and memory limits
    """
    
    def __init__(self, vexfs_client: VexFSClient, max_workers: int = 4):
        """
        Initialize batch operations engine.
        
        Args:
            vexfs_client: VexFS client for vector operations
            max_workers: Maximum number of worker threads for parallel execution
        """
        self.vexfs_client = vexfs_client
        self.filter_engine = FilterEngine(vexfs_client)
        self.max_workers = max_workers
        self.executor = ThreadPoolExecutor(max_workers=max_workers)
        
        # Performance tracking
        self._batch_stats = {
            'total_batch_operations': 0,
            'successful_operations': 0,
            'failed_operations': 0,
            'total_execution_time': 0.0,
            'total_queries_processed': 0,
            'total_points_processed': 0,
            'avg_queries_per_second': 0.0,
            'avg_points_per_second': 0.0
        }
    
    async def batch_search(self, collection: str, queries: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """
        Execute multiple search queries in parallel.
        
        Args:
            collection: Collection name
            queries: List of search query dictionaries
            
        Returns:
            List of search results corresponding to each query
            
        Raises:
            ValueError: If queries are invalid
            RuntimeError: If batch search fails
        """
        start_time = time.time()
        self._batch_stats['total_batch_operations'] += 1
        
        try:
            if not queries:
                return []
            
            # Validate queries
            for i, query in enumerate(queries):
                if 'vector' not in query:
                    raise ValueError(f"Query {i} missing 'vector' field")
                if not isinstance(query['vector'], list):
                    raise ValueError(f"Query {i} 'vector' must be a list")
            
            # Execute searches in parallel
            search_tasks = []
            for i, query in enumerate(queries):
                task = self._execute_single_search(collection, query, i)
                search_tasks.append(task)
            
            # Wait for all searches to complete
            results = await asyncio.gather(*search_tasks, return_exceptions=True)
            
            # Process results and handle exceptions
            processed_results = []
            successful_queries = 0
            
            for i, result in enumerate(results):
                if isinstance(result, Exception):
                    logger.error(f"Query {i} failed: {result}")
                    processed_results.append({
                        'query_id': i,
                        'error': str(result),
                        'result': []
                    })
                else:
                    processed_results.append(result)
                    successful_queries += 1
            
            execution_time = time.time() - start_time
            
            # Update statistics
            self._update_batch_stats(
                execution_time=execution_time,
                success=successful_queries > 0,
                queries_processed=len(queries),
                points_processed=sum(len(r.get('result', [])) for r in processed_results)
            )
            
            logger.info(
                "Batch search completed",
                collection=collection,
                total_queries=len(queries),
                successful_queries=successful_queries,
                execution_time=execution_time,
                queries_per_second=len(queries) / execution_time if execution_time > 0 else 0
            )
            
            return processed_results
            
        except Exception as e:
            self._update_batch_stats(time.time() - start_time, False, len(queries), 0)
            logger.error(f"Batch search failed: {e}")
            raise RuntimeError(f"Batch search failed: {e}")
    
    async def grouped_search(self, collection: str, query_vector: List[float],
                           group_by: str, limit: int = 10, group_size: int = 3,
                           filter_condition: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """
        Search with result grouping by field.
        
        Args:
            collection: Collection name
            query_vector: Query vector
            group_by: Field to group results by
            limit: Maximum number of groups
            group_size: Maximum points per group
            filter_condition: Optional filter to apply
            
        Returns:
            Grouped search results
        """
        start_time = time.time()
        
        try:
            # Perform initial search with larger limit
            search_limit = limit * group_size * 3  # Get more results for grouping
            
            search_results = self.vexfs_client.search_vectors(
                collection=collection,
                query_vector=query_vector,
                limit=search_limit,
                distance="Cosine"
            )
            
            # Apply filters if specified
            if filter_condition:
                candidate_ids = [str(result.vector_id) for result in search_results]
                filtered_ids = self.filter_engine.apply_filter(
                    collection, filter_condition, candidate_ids
                )
                search_results = [
                    result for result in search_results
                    if str(result.vector_id) in filtered_ids
                ]
            
            # Get metadata for grouping
            point_ids = [str(result.vector_id) for result in search_results]
            metadata_list = self.vexfs_client.get_vector_metadata(collection, [int(pid) for pid in point_ids])
            
            # Create metadata lookup
            metadata_lookup = {str(meta['id']): meta for meta in metadata_list}
            
            # Group results
            groups = {}
            for result in search_results:
                point_id = str(result.vector_id)
                metadata = metadata_lookup.get(point_id, {})
                
                # Get group value
                group_value = metadata.get('payload', {}).get(group_by)
                if group_value is None:
                    group_value = "null"
                else:
                    group_value = str(group_value)
                
                # Add to group
                if group_value not in groups:
                    groups[group_value] = []
                
                if len(groups[group_value]) < group_size:
                    groups[group_value].append({
                        'id': point_id,
                        'score': float(result.score),
                        'payload': metadata.get('payload', {})
                    })
                
                # Stop if we have enough groups
                if len(groups) >= limit:
                    break
            
            # Format result
            grouped_results = []
            for group_value, points in list(groups.items())[:limit]:
                grouped_results.append({
                    'group_id': group_value,
                    'hits': points
                })
            
            execution_time = time.time() - start_time
            total_points = sum(len(group['hits']) for group in grouped_results)
            
            self._update_batch_stats(execution_time, True, 1, total_points)
            
            logger.info(
                "Grouped search completed",
                collection=collection,
                groups_found=len(grouped_results),
                total_points=total_points,
                execution_time=execution_time
            )
            
            return {
                'result': grouped_results,
                'group_by': group_by,
                'total_groups': len(grouped_results)
            }
            
        except Exception as e:
            self._update_batch_stats(time.time() - start_time, False, 1, 0)
            logger.error(f"Grouped search failed: {e}")
            raise RuntimeError(f"Grouped search failed: {e}")
    
    async def batch_upsert_optimized(self, collection: str, points: List[Dict[str, Any]],
                                   batch_size: int = 1000) -> Dict[str, Any]:
        """
        Optimized batch upsert using VexFS batch insert IOCTL.
        
        Args:
            collection: Collection name
            points: List of points to upsert
            batch_size: Batch size for processing
            
        Returns:
            Upsert operation result
        """
        start_time = time.time()
        
        try:
            if not points:
                return {'operation_id': 0, 'status': 'completed', 'points_processed': 0}
            
            total_points = len(points)
            processed_points = 0
            operation_ids = []
            
            # Process in batches
            for i in range(0, total_points, batch_size):
                batch = points[i:i + batch_size]
                
                # Use VexFS batch insert for optimal performance
                result = self.vexfs_client.insert_points(collection, batch)
                operation_ids.append(result.get('operation_id', 0))
                processed_points += len(batch)
                
                logger.debug(f"Processed batch {i//batch_size + 1}, points: {len(batch)}")
            
            execution_time = time.time() - start_time
            
            self._update_batch_stats(execution_time, True, 0, processed_points)
            
            logger.info(
                "Batch upsert completed",
                collection=collection,
                total_points=total_points,
                batches_processed=len(operation_ids),
                execution_time=execution_time,
                points_per_second=total_points / execution_time if execution_time > 0 else 0
            )
            
            return {
                'operation_id': max(operation_ids) if operation_ids else 0,
                'status': 'completed',
                'points_processed': processed_points,
                'batches_processed': len(operation_ids),
                'execution_time': execution_time
            }
            
        except Exception as e:
            self._update_batch_stats(time.time() - start_time, False, 0, len(points))
            logger.error(f"Batch upsert failed: {e}")
            raise RuntimeError(f"Batch upsert failed: {e}")
    
    async def parallel_collection_search(self, collections: List[str], 
                                       query_vector: List[float],
                                       limit: int = 10) -> Dict[str, List[Dict[str, Any]]]:
        """
        Search across multiple collections in parallel.
        
        Args:
            collections: List of collection names
            query_vector: Query vector
            limit: Limit per collection
            
        Returns:
            Dictionary mapping collection names to search results
        """
        start_time = time.time()
        
        try:
            # Create search tasks for each collection
            search_tasks = []
            for collection in collections:
                task = self._search_single_collection(collection, query_vector, limit)
                search_tasks.append((collection, task))
            
            # Execute searches in parallel
            results = {}
            for collection, task in search_tasks:
                try:
                    search_result = await task
                    results[collection] = search_result
                except Exception as e:
                    logger.error(f"Search failed for collection {collection}: {e}")
                    results[collection] = []
            
            execution_time = time.time() - start_time
            total_points = sum(len(result) for result in results.values())
            
            self._update_batch_stats(execution_time, True, len(collections), total_points)
            
            logger.info(
                "Parallel collection search completed",
                collections=len(collections),
                total_results=total_points,
                execution_time=execution_time
            )
            
            return results
            
        except Exception as e:
            self._update_batch_stats(time.time() - start_time, False, len(collections), 0)
            logger.error(f"Parallel collection search failed: {e}")
            raise RuntimeError(f"Parallel collection search failed: {e}")
    
    async def _execute_single_search(self, collection: str, query: Dict[str, Any], 
                                   query_id: int) -> Dict[str, Any]:
        """Execute a single search query"""
        
        try:
            # Extract query parameters
            vector = query['vector']
            limit = query.get('limit', 10)
            filter_condition = query.get('filter')
            with_payload = query.get('with_payload', True)
            with_vector = query.get('with_vector', False)
            
            # Perform search
            search_results = self.vexfs_client.search_vectors(
                collection=collection,
                query_vector=vector,
                limit=limit,
                distance="Cosine"
            )
            
            # Apply filters if specified
            if filter_condition:
                candidate_ids = [str(result.vector_id) for result in search_results]
                filtered_ids = self.filter_engine.apply_filter(
                    collection, filter_condition, candidate_ids
                )
                search_results = [
                    result for result in search_results
                    if str(result.vector_id) in filtered_ids
                ]
            
            # Format results
            formatted_results = []
            for result in search_results:
                point = {
                    'id': str(result.vector_id),
                    'score': float(result.score)
                }
                
                if with_payload:
                    # Get payload from metadata
                    metadata = self.vexfs_client.get_vector_metadata(collection, [int(result.vector_id)])
                    if metadata:
                        point['payload'] = metadata[0].get('payload', {})
                
                if with_vector:
                    point['vector'] = None  # Would be populated from VexFS
                
                formatted_results.append(point)
            
            return {
                'query_id': query_id,
                'result': formatted_results
            }
            
        except Exception as e:
            raise RuntimeError(f"Search query {query_id} failed: {e}")
    
    async def _search_single_collection(self, collection: str, query_vector: List[float], 
                                      limit: int) -> List[Dict[str, Any]]:
        """Search a single collection"""
        
        try:
            search_results = self.vexfs_client.search_vectors(
                collection=collection,
                query_vector=query_vector,
                limit=limit,
                distance="Cosine"
            )
            
            return [
                {
                    'id': str(result.vector_id),
                    'score': float(result.score),
                    'collection': collection
                }
                for result in search_results
            ]
            
        except Exception as e:
            logger.error(f"Collection search failed for {collection}: {e}")
            return []
    
    def _update_batch_stats(self, execution_time: float, success: bool, 
                           queries_processed: int, points_processed: int):
        """Update batch operation statistics"""
        
        self._batch_stats['total_execution_time'] += execution_time
        
        if success:
            self._batch_stats['successful_operations'] += 1
        else:
            self._batch_stats['failed_operations'] += 1
        
        self._batch_stats['total_queries_processed'] += queries_processed
        self._batch_stats['total_points_processed'] += points_processed
        
        # Update averages
        total_time = self._batch_stats['total_execution_time']
        if total_time > 0:
            self._batch_stats['avg_queries_per_second'] = (
                self._batch_stats['total_queries_processed'] / total_time
            )
            self._batch_stats['avg_points_per_second'] = (
                self._batch_stats['total_points_processed'] / total_time
            )
    
    def get_batch_statistics(self) -> Dict[str, Any]:
        """Get batch operations performance statistics"""
        return {
            'batch_stats': self._batch_stats.copy(),
            'performance_targets': {
                'target_queries_per_second': 50,
                'target_points_per_second': 95117,
                'max_workers': self.max_workers
            },
            'current_performance': {
                'queries_per_second': self._batch_stats['avg_queries_per_second'],
                'points_per_second': self._batch_stats['avg_points_per_second']
            }
        }
    
    def clear_statistics(self):
        """Clear all batch operation statistics"""
        self._batch_stats = {
            'total_batch_operations': 0,
            'successful_operations': 0,
            'failed_operations': 0,
            'total_execution_time': 0.0,
            'total_queries_processed': 0,
            'total_points_processed': 0,
            'avg_queries_per_second': 0.0,
            'avg_points_per_second': 0.0
        }
    
    def __del__(self):
        """Cleanup executor on destruction"""
        if hasattr(self, 'executor'):
            self.executor.shutdown(wait=False)