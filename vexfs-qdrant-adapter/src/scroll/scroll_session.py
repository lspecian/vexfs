"""
Scroll Session Management

This module manages scroll sessions for efficient pagination of large collections,
providing stateful scrolling with memory management and cleanup.
"""

from typing import Dict, List, Any, Optional, Set
import time
import uuid
import threading
import logging
from dataclasses import dataclass
from ..core.vexfs_client import VexFSClient
from ..filters.filter_engine import FilterEngine

logger = logging.getLogger(__name__)


@dataclass
class ScrollSession:
    """
    Scroll session for stateful pagination.
    
    Maintains state for efficient cursor-based pagination through large collections.
    """
    session_id: str
    collection: str
    filter_condition: Optional[Dict[str, Any]]
    with_payload: bool
    with_vector: bool
    created_at: float
    last_accessed: float
    current_offset: int
    total_points: Optional[int]
    filtered_point_ids: Optional[List[str]]
    batch_size: int
    memory_usage: int  # Estimated memory usage in bytes
    
    def update_access_time(self):
        """Update last accessed timestamp"""
        self.last_accessed = time.time()
    
    def is_expired(self, timeout_seconds: int = 3600) -> bool:
        """Check if session has expired"""
        return (time.time() - self.last_accessed) > timeout_seconds
    
    def get_next_batch_range(self, limit: int) -> tuple[int, int]:
        """Get the range for the next batch"""
        start = self.current_offset
        end = min(start + limit, len(self.filtered_point_ids) if self.filtered_point_ids else start + limit)
        return start, end
    
    def advance_offset(self, count: int):
        """Advance the current offset"""
        self.current_offset += count
    
    def has_more_data(self) -> bool:
        """Check if there's more data to scroll"""
        if self.filtered_point_ids:
            return self.current_offset < len(self.filtered_point_ids)
        return True  # Unknown total, assume more data exists
    
    def estimate_memory_usage(self) -> int:
        """Estimate memory usage of this session"""
        base_size = 1024  # Base session overhead
        
        if self.filtered_point_ids:
            # Estimate 50 bytes per point ID (including overhead)
            point_ids_size = len(self.filtered_point_ids) * 50
        else:
            point_ids_size = 0
        
        # Filter condition size
        filter_size = len(str(self.filter_condition)) if self.filter_condition else 0
        
        return base_size + point_ids_size + filter_size


class ScrollSessionManager:
    """
    Manager for scroll sessions with automatic cleanup and memory management.
    
    Provides efficient session management with configurable memory limits
    and automatic cleanup of expired sessions.
    """
    
    def __init__(self, vexfs_client: VexFSClient, 
                 max_memory_mb: int = 100,
                 session_timeout_seconds: int = 3600,
                 cleanup_interval_seconds: int = 300):
        """
        Initialize scroll session manager.
        
        Args:
            vexfs_client: VexFS client for data operations
            max_memory_mb: Maximum memory usage for all sessions
            session_timeout_seconds: Session timeout in seconds
            cleanup_interval_seconds: Cleanup interval in seconds
        """
        self.vexfs_client = vexfs_client
        self.filter_engine = FilterEngine(vexfs_client)
        self.max_memory_bytes = max_memory_mb * 1024 * 1024
        self.session_timeout = session_timeout_seconds
        self.cleanup_interval = cleanup_interval_seconds
        
        self._sessions: Dict[str, ScrollSession] = {}
        self._lock = threading.RLock()
        self._last_cleanup = time.time()
        
        # Performance tracking
        self._stats = {
            'sessions_created': 0,
            'sessions_expired': 0,
            'sessions_cleaned': 0,
            'total_memory_usage': 0,
            'max_concurrent_sessions': 0
        }
    
    def create_session(self, collection: str,
                      filter_condition: Optional[Dict[str, Any]] = None,
                      with_payload: bool = True,
                      with_vector: bool = False,
                      batch_size: int = 100) -> str:
        """
        Create a new scroll session.
        
        Args:
            collection: Collection name
            filter_condition: Optional filter to apply
            with_payload: Include payload in results
            with_vector: Include vector in results
            batch_size: Default batch size for this session
            
        Returns:
            Session ID
            
        Raises:
            RuntimeError: If memory limit would be exceeded
        """
        with self._lock:
            # Cleanup expired sessions first
            self._cleanup_expired_sessions()
            
            # Generate unique session ID
            session_id = str(uuid.uuid4())
            
            # Pre-filter points if filter is specified
            filtered_point_ids = None
            if filter_condition:
                try:
                    filtered_point_ids = self.filter_engine.apply_filter(
                        collection, filter_condition
                    )
                except Exception as e:
                    logger.error(f"Filter application failed during session creation: {e}")
                    # Continue without pre-filtering
                    filtered_point_ids = None
            
            # Create session
            session = ScrollSession(
                session_id=session_id,
                collection=collection,
                filter_condition=filter_condition,
                with_payload=with_payload,
                with_vector=with_vector,
                created_at=time.time(),
                last_accessed=time.time(),
                current_offset=0,
                total_points=len(filtered_point_ids) if filtered_point_ids else None,
                filtered_point_ids=filtered_point_ids,
                batch_size=batch_size,
                memory_usage=0
            )
            
            # Calculate memory usage
            session.memory_usage = session.estimate_memory_usage()
            
            # Check memory limit
            total_memory = self._calculate_total_memory_usage() + session.memory_usage
            if total_memory > self.max_memory_bytes:
                # Try to free memory by cleaning old sessions
                self._cleanup_old_sessions()
                total_memory = self._calculate_total_memory_usage() + session.memory_usage
                
                if total_memory > self.max_memory_bytes:
                    raise RuntimeError(f"Memory limit exceeded. Current: {total_memory}, Limit: {self.max_memory_bytes}")
            
            # Store session
            self._sessions[session_id] = session
            self._stats['sessions_created'] += 1
            self._stats['max_concurrent_sessions'] = max(
                self._stats['max_concurrent_sessions'],
                len(self._sessions)
            )
            
            logger.info(
                "Scroll session created",
                session_id=session_id,
                collection=collection,
                has_filter=filter_condition is not None,
                filtered_points=len(filtered_point_ids) if filtered_point_ids else "unknown",
                memory_usage=session.memory_usage
            )
            
            return session_id
    
    def get_session(self, session_id: str) -> Optional[ScrollSession]:
        """
        Get a scroll session by ID.
        
        Args:
            session_id: Session ID
            
        Returns:
            ScrollSession if found, None otherwise
        """
        with self._lock:
            session = self._sessions.get(session_id)
            if session:
                if session.is_expired(self.session_timeout):
                    self._remove_session(session_id)
                    return None
                session.update_access_time()
            return session
    
    def continue_scroll(self, session_id: str, limit: int = 100) -> Dict[str, Any]:
        """
        Continue scrolling in an existing session.
        
        Args:
            session_id: Session ID
            limit: Number of points to return
            
        Returns:
            Scroll result with points and next cursor
            
        Raises:
            ValueError: If session not found or invalid
        """
        with self._lock:
            session = self.get_session(session_id)
            if not session:
                raise ValueError(f"Session {session_id} not found or expired")
            
            # Check if there's more data
            if not session.has_more_data():
                return {
                    'points': [],
                    'next_page_offset': None,
                    'session_id': session_id,
                    'total_points': session.total_points,
                    'has_more': False
                }
            
            # Get batch range
            start_offset, end_offset = session.get_next_batch_range(limit)
            
            # Get points for this batch
            if session.filtered_point_ids:
                # Use pre-filtered point IDs
                batch_point_ids = session.filtered_point_ids[start_offset:end_offset]
            else:
                # Generate point IDs on the fly (less efficient)
                batch_point_ids = self._generate_point_ids_batch(
                    session.collection, start_offset, limit
                )
            
            # Retrieve point data
            points = self._get_points_data(
                session.collection,
                batch_point_ids,
                session.with_payload,
                session.with_vector
            )
            
            # Update session offset
            session.advance_offset(len(points))
            
            # Determine next cursor
            next_offset = session.current_offset if session.has_more_data() else None
            
            result = {
                'points': points,
                'next_page_offset': next_offset,
                'session_id': session_id,
                'total_points': session.total_points,
                'has_more': session.has_more_data()
            }
            
            logger.debug(
                "Scroll continued",
                session_id=session_id,
                points_returned=len(points),
                current_offset=session.current_offset,
                has_more=session.has_more_data()
            )
            
            return result
    
    def close_session(self, session_id: str) -> bool:
        """
        Close and remove a scroll session.
        
        Args:
            session_id: Session ID to close
            
        Returns:
            True if session was found and closed
        """
        with self._lock:
            return self._remove_session(session_id)
    
    def get_session_info(self, session_id: str) -> Optional[Dict[str, Any]]:
        """
        Get information about a scroll session.
        
        Args:
            session_id: Session ID
            
        Returns:
            Session information dictionary
        """
        with self._lock:
            session = self.get_session(session_id)
            if not session:
                return None
            
            return {
                'session_id': session.session_id,
                'collection': session.collection,
                'created_at': session.created_at,
                'last_accessed': session.last_accessed,
                'current_offset': session.current_offset,
                'total_points': session.total_points,
                'has_filter': session.filter_condition is not None,
                'with_payload': session.with_payload,
                'with_vector': session.with_vector,
                'memory_usage': session.memory_usage,
                'has_more': session.has_more_data()
            }
    
    def list_sessions(self) -> List[Dict[str, Any]]:
        """
        List all active sessions.
        
        Returns:
            List of session information dictionaries
        """
        with self._lock:
            sessions = []
            for session_id in list(self._sessions.keys()):
                session_info = self.get_session_info(session_id)
                if session_info:
                    sessions.append(session_info)
            return sessions
    
    def get_statistics(self) -> Dict[str, Any]:
        """
        Get scroll session manager statistics.
        
        Returns:
            Statistics dictionary
        """
        with self._lock:
            current_memory = self._calculate_total_memory_usage()
            
            return {
                'active_sessions': len(self._sessions),
                'total_memory_usage_bytes': current_memory,
                'total_memory_usage_mb': current_memory / (1024 * 1024),
                'memory_limit_mb': self.max_memory_bytes / (1024 * 1024),
                'memory_utilization': current_memory / self.max_memory_bytes,
                'session_timeout_seconds': self.session_timeout,
                'stats': self._stats.copy()
            }
    
    def cleanup_all_sessions(self):
        """Clean up all sessions (for shutdown)"""
        with self._lock:
            session_count = len(self._sessions)
            self._sessions.clear()
            self._stats['sessions_cleaned'] += session_count
            logger.info(f"Cleaned up all {session_count} scroll sessions")
    
    def _remove_session(self, session_id: str) -> bool:
        """Remove a session (internal method)"""
        if session_id in self._sessions:
            del self._sessions[session_id]
            logger.debug(f"Scroll session {session_id} removed")
            return True
        return False
    
    def _cleanup_expired_sessions(self):
        """Clean up expired sessions"""
        current_time = time.time()
        
        # Only run cleanup if enough time has passed
        if current_time - self._last_cleanup < self.cleanup_interval:
            return
        
        expired_sessions = []
        for session_id, session in self._sessions.items():
            if session.is_expired(self.session_timeout):
                expired_sessions.append(session_id)
        
        for session_id in expired_sessions:
            self._remove_session(session_id)
            self._stats['sessions_expired'] += 1
        
        self._last_cleanup = current_time
        
        if expired_sessions:
            logger.info(f"Cleaned up {len(expired_sessions)} expired scroll sessions")
    
    def _cleanup_old_sessions(self):
        """Clean up oldest sessions to free memory"""
        if not self._sessions:
            return
        
        # Sort sessions by last accessed time
        sessions_by_age = sorted(
            self._sessions.items(),
            key=lambda x: x[1].last_accessed
        )
        
        # Remove oldest 25% of sessions
        sessions_to_remove = len(sessions_by_age) // 4
        if sessions_to_remove == 0 and sessions_by_age:
            sessions_to_remove = 1
        
        for i in range(sessions_to_remove):
            session_id, _ = sessions_by_age[i]
            self._remove_session(session_id)
            self._stats['sessions_cleaned'] += 1
        
        logger.info(f"Cleaned up {sessions_to_remove} old scroll sessions to free memory")
    
    def _calculate_total_memory_usage(self) -> int:
        """Calculate total memory usage of all sessions"""
        return sum(session.memory_usage for session in self._sessions.values())
    
    def _generate_point_ids_batch(self, collection: str, offset: int, limit: int) -> List[str]:
        """Generate point IDs for a batch (when not pre-filtered)"""
        # In a full implementation, this would efficiently query VexFS for point IDs
        # For now, generate sequential IDs
        return [str(i) for i in range(offset, offset + limit)]
    
    def _get_points_data(self, collection: str, point_ids: List[str],
                        with_payload: bool, with_vector: bool) -> List[Dict[str, Any]]:
        """Get point data for a list of point IDs"""
        try:
            # Use VexFS client to get point metadata
            points_data = self.vexfs_client.get_vector_metadata(collection, [int(pid) for pid in point_ids])
            
            # Format results
            formatted_points = []
            for point_data in points_data:
                point = {
                    'id': point_data['id']
                }
                
                if with_payload:
                    point['payload'] = point_data.get('payload', {})
                
                if with_vector:
                    point['vector'] = point_data.get('vector')
                
                formatted_points.append(point)
            
            return formatted_points
            
        except Exception as e:
            logger.error(f"Failed to get points data: {e}")
            return []