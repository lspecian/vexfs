"""
Scroll API

This module implements the main Scroll API for efficient pagination of large collections,
providing cursor-based pagination with session management and filter integration.
"""

from typing import Dict, List, Any, Optional, Union
import time
import logging
from ..core.vexfs_client import VexFSClient, VexFSError
from ..filters.filter_engine import FilterEngine
from .scroll_session import ScrollSessionManager

logger = logging.getLogger(__name__)


class ScrollAPI:
    """
    Efficient pagination for large collections.
    
    Provides cursor-based pagination with stateful sessions, memory management,
    and filter integration. Optimized for >10K points/sec with <100MB memory usage.
    """
    
    def __init__(self, vexfs_client: VexFSClient,
                 max_memory_mb: int = 100,
                 session_timeout_seconds: int = 3600):
        """
        Initialize scroll API.
        
        Args:
            vexfs_client: VexFS client for data operations
            max_memory_mb: Maximum memory usage for scroll sessions
            session_timeout_seconds: Session timeout in seconds
        """
        self.vexfs_client = vexfs_client
        self.filter_engine = FilterEngine(vexfs_client)
        self.session_manager = ScrollSessionManager(
            vexfs_client=vexfs_client,
            max_memory_mb=max_memory_mb,
            session_timeout_seconds=session_timeout_seconds
        )
        
        # Performance tracking
        self._scroll_stats = {
            'total_scroll_requests': 0,
            'successful_scrolls': 0,
            'failed_scrolls': 0,
            'total_points_scrolled': 0,
            'total_execution_time': 0.0,
            'avg_execution_time': 0.0,
            'avg_points_per_second': 0.0
        }
    
    def scroll_points(self, collection: str,
                     limit: int = 100,
                     offset: Optional[str] = None,
                     filter_condition: Optional[Dict[str, Any]] = None,
                     with_payload: bool = True,
                     with_vectors: bool = False,
                     order_by: Optional[str] = None) -> Dict[str, Any]:
        """
        Scroll through collection with cursor-based pagination.
        
        Args:
            collection: Collection name
            limit: Number of points to return (1-10000)
            offset: Cursor offset for pagination (None for first page)
            filter_condition: Optional filter to apply
            with_payload: Include payload in results
            with_vectors: Include vectors in results
            order_by: Optional field to order by
            
        Returns:
            Scroll result with points and next cursor
            
        Raises:
            ValueError: If parameters are invalid
            RuntimeError: If scroll operation fails
        """
        start_time = time.time()
        self._scroll_stats['total_scroll_requests'] += 1
        
        try:
            # Validate parameters
            if limit <= 0 or limit > 10000:
                raise ValueError("Limit must be between 1 and 10000")
            
            # Handle first page (no offset)
            if offset is None:
                return self._start_new_scroll(
                    collection=collection,
                    limit=limit,
                    filter_condition=filter_condition,
                    with_payload=with_payload,
                    with_vectors=with_vectors,
                    order_by=order_by
                )
            
            # Handle continuation (with offset)
            else:
                return self._continue_existing_scroll(
                    offset=offset,
                    limit=limit
                )
                
        except Exception as e:
            self._update_stats(start_time, False, 0)
            logger.error(f"Scroll operation failed: {e}")
            raise RuntimeError(f"Scroll failed: {e}")
    
    def create_scroll_session(self, collection: str,
                             filter_condition: Optional[Dict[str, Any]] = None,
                             with_payload: bool = True,
                             with_vectors: bool = False,
                             batch_size: int = 100) -> str:
        """
        Create new scroll session for large operations.
        
        Args:
            collection: Collection name
            filter_condition: Optional filter to apply
            with_payload: Include payload in results
            with_vectors: Include vectors in results
            batch_size: Default batch size for this session
            
        Returns:
            Session ID for subsequent scroll operations
            
        Raises:
            RuntimeError: If session creation fails
        """
        try:
            session_id = self.session_manager.create_session(
                collection=collection,
                filter_condition=filter_condition,
                with_payload=with_payload,
                with_vector=with_vectors,
                batch_size=batch_size
            )
            
            logger.info(
                "Scroll session created",
                session_id=session_id,
                collection=collection,
                has_filter=filter_condition is not None
            )
            
            return session_id
            
        except Exception as e:
            logger.error(f"Failed to create scroll session: {e}")
            raise RuntimeError(f"Session creation failed: {e}")
    
    def continue_scroll(self, session_id: str, limit: int = 100) -> Dict[str, Any]:
        """
        Continue existing scroll session.
        
        Args:
            session_id: Session ID from create_scroll_session
            limit: Number of points to return
            
        Returns:
            Scroll result with points and continuation info
            
        Raises:
            ValueError: If session not found
            RuntimeError: If scroll operation fails
        """
        start_time = time.time()
        
        try:
            result = self.session_manager.continue_scroll(session_id, limit)
            
            execution_time = time.time() - start_time
            points_count = len(result.get('points', []))
            
            self._update_stats(start_time, True, points_count)
            
            logger.debug(
                "Scroll continued",
                session_id=session_id,
                points_returned=points_count,
                execution_time=execution_time,
                has_more=result.get('has_more', False)
            )
            
            return result
            
        except Exception as e:
            self._update_stats(start_time, False, 0)
            logger.error(f"Failed to continue scroll: {e}")
            raise
    
    def close_scroll_session(self, session_id: str) -> bool:
        """
        Clean up scroll session resources.
        
        Args:
            session_id: Session ID to close
            
        Returns:
            True if session was found and closed
        """
        try:
            result = self.session_manager.close_session(session_id)
            
            if result:
                logger.info(f"Scroll session {session_id} closed")
            else:
                logger.warning(f"Scroll session {session_id} not found")
            
            return result
            
        except Exception as e:
            logger.error(f"Failed to close scroll session {session_id}: {e}")
            return False
    
    def get_scroll_session_info(self, session_id: str) -> Optional[Dict[str, Any]]:
        """
        Get information about a scroll session.
        
        Args:
            session_id: Session ID
            
        Returns:
            Session information or None if not found
        """
        try:
            return self.session_manager.get_session_info(session_id)
        except Exception as e:
            logger.error(f"Failed to get session info for {session_id}: {e}")
            return None
    
    def list_scroll_sessions(self) -> List[Dict[str, Any]]:
        """
        List all active scroll sessions.
        
        Returns:
            List of session information dictionaries
        """
        try:
            return self.session_manager.list_sessions()
        except Exception as e:
            logger.error(f"Failed to list scroll sessions: {e}")
            return []
    
    def scroll_with_search(self, collection: str,
                          query_vector: List[float],
                          limit: int = 100,
                          offset: Optional[str] = None,
                          filter_condition: Optional[Dict[str, Any]] = None,
                          score_threshold: Optional[float] = None) -> Dict[str, Any]:
        """
        Scroll through search results with pagination.
        
        Args:
            collection: Collection name
            query_vector: Query vector for similarity search
            limit: Number of results to return
            offset: Cursor for pagination
            filter_condition: Optional filter to apply
            score_threshold: Minimum score threshold
            
        Returns:
            Paginated search results
        """
        start_time = time.time()
        
        try:
            # Perform vector search with larger limit for pagination
            search_limit = limit * 10  # Get more results for pagination
            
            search_results = self.vexfs_client.search_vectors(
                collection=collection,
                query_vector=query_vector,
                limit=search_limit,
                distance="Cosine"
            )
            
            # Convert to point IDs
            candidate_ids = [str(result.vector_id) for result in search_results]
            
            # Apply filters if specified
            if filter_condition:
                filtered_ids = self.filter_engine.apply_filter(
                    collection, filter_condition, candidate_ids
                )
                # Keep only filtered results in original order
                search_results = [
                    result for result in search_results
                    if str(result.vector_id) in filtered_ids
                ]
            
            # Apply score threshold
            if score_threshold is not None:
                search_results = [
                    result for result in search_results
                    if result.score >= score_threshold
                ]
            
            # Handle pagination
            start_idx = int(offset) if offset else 0
            end_idx = start_idx + limit
            
            paginated_results = search_results[start_idx:end_idx]
            
            # Format results
            points = []
            for result in paginated_results:
                point = {
                    'id': str(result.vector_id),
                    'score': float(result.score)
                }
                points.append(point)
            
            # Determine next offset
            next_offset = str(end_idx) if end_idx < len(search_results) else None
            
            execution_time = time.time() - start_time
            self._update_stats(start_time, True, len(points))
            
            return {
                'points': points,
                'next_page_offset': next_offset,
                'total_results': len(search_results),
                'has_more': next_offset is not None
            }
            
        except Exception as e:
            self._update_stats(start_time, False, 0)
            logger.error(f"Scroll with search failed: {e}")
            raise RuntimeError(f"Search scroll failed: {e}")
    
    def get_scroll_statistics(self) -> Dict[str, Any]:
        """
        Get scroll API performance statistics.
        
        Returns:
            Performance statistics and session information
        """
        session_stats = self.session_manager.get_statistics()
        
        return {
            'scroll_stats': self._scroll_stats.copy(),
            'session_stats': session_stats,
            'performance_targets': {
                'target_points_per_second': 10000,
                'target_memory_usage_mb': 100,
                'target_session_timeout_seconds': 3600
            }
        }
    
    def cleanup_all_sessions(self):
        """Clean up all scroll sessions (for shutdown)"""
        try:
            self.session_manager.cleanup_all_sessions()
            logger.info("All scroll sessions cleaned up")
        except Exception as e:
            logger.error(f"Failed to cleanup scroll sessions: {e}")
    
    def _start_new_scroll(self, collection: str, limit: int,
                         filter_condition: Optional[Dict[str, Any]],
                         with_payload: bool, with_vectors: bool,
                         order_by: Optional[str]) -> Dict[str, Any]:
        """Start a new scroll operation"""
        
        # For simple scrolling without session, use direct approach
        if not filter_condition and not order_by:
            return self._simple_scroll(collection, 0, limit, with_payload, with_vectors)
        
        # For complex scrolling, create a session
        session_id = self.create_scroll_session(
            collection=collection,
            filter_condition=filter_condition,
            with_payload=with_payload,
            with_vectors=with_vectors,
            batch_size=limit
        )
        
        # Get first batch
        result = self.continue_scroll(session_id, limit)
        
        # Add session info to result
        result['session_id'] = session_id
        
        return result
    
    def _continue_existing_scroll(self, offset: str, limit: int) -> Dict[str, Any]:
        """Continue an existing scroll operation"""
        
        # Try to parse offset as session ID
        if len(offset) > 10 and '-' in offset:  # Looks like UUID
            return self.continue_scroll(offset, limit)
        
        # Try to parse offset as numeric offset
        try:
            numeric_offset = int(offset)
            # This would need collection info - for now, return error
            raise ValueError("Numeric offset requires session-based scrolling")
        except ValueError:
            raise ValueError(f"Invalid offset format: {offset}")
    
    def _simple_scroll(self, collection: str, offset: int, limit: int,
                      with_payload: bool, with_vectors: bool) -> Dict[str, Any]:
        """Simple scroll without filters or sessions"""
        
        try:
            # Get collection info to determine total points
            collection_info = self.vexfs_client.get_collection_info(collection)
            total_points = collection_info.get('points_count', 0)
            
            # Generate point IDs for this batch
            start_id = offset
            end_id = min(offset + limit, total_points)
            
            if start_id >= total_points:
                return {
                    'points': [],
                    'next_page_offset': None,
                    'total_points': total_points,
                    'has_more': False
                }
            
            # Get point IDs for this range
            point_ids = [str(i) for i in range(start_id, end_id)]
            
            # Get point data
            points = self._get_points_data(collection, point_ids, with_payload, with_vectors)
            
            # Determine next offset
            next_offset = str(end_id) if end_id < total_points else None
            
            return {
                'points': points,
                'next_page_offset': next_offset,
                'total_points': total_points,
                'has_more': next_offset is not None
            }
            
        except VexFSError as e:
            raise RuntimeError(f"VexFS operation failed: {e}")
    
    def _get_points_data(self, collection: str, point_ids: List[str],
                        with_payload: bool, with_vectors: bool) -> List[Dict[str, Any]]:
        """Get point data for a list of point IDs"""
        try:
            # Convert to integers for VexFS
            int_point_ids = [int(pid) for pid in point_ids]
            
            # Get metadata from VexFS
            points_data = self.vexfs_client.get_vector_metadata(collection, int_point_ids)
            
            # Format results
            formatted_points = []
            for point_data in points_data:
                point = {
                    'id': point_data['id']
                }
                
                if with_payload:
                    point['payload'] = point_data.get('payload', {})
                
                if with_vectors:
                    point['vector'] = point_data.get('vector')
                
                formatted_points.append(point)
            
            return formatted_points
            
        except Exception as e:
            logger.error(f"Failed to get points data: {e}")
            return []
    
    def _update_stats(self, start_time: float, success: bool, points_count: int):
        """Update performance statistics"""
        execution_time = time.time() - start_time
        self._scroll_stats['total_execution_time'] += execution_time
        
        if success:
            self._scroll_stats['successful_scrolls'] += 1
            self._scroll_stats['total_points_scrolled'] += points_count
        else:
            self._scroll_stats['failed_scrolls'] += 1
        
        # Update averages
        total_requests = self._scroll_stats['total_scroll_requests']
        if total_requests > 0:
            self._scroll_stats['avg_execution_time'] = (
                self._scroll_stats['total_execution_time'] / total_requests
            )
        
        total_time = self._scroll_stats['total_execution_time']
        if total_time > 0:
            self._scroll_stats['avg_points_per_second'] = (
                self._scroll_stats['total_points_scrolled'] / total_time
            )