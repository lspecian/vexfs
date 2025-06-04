"""
Cluster API Endpoints

This module implements Qdrant-compatible cluster information endpoints.
Since VexFS v2 operates as a single-node system, this provides compatibility
responses for cluster-aware Qdrant clients.
"""

from typing import Dict, Any
from fastapi import APIRouter, Depends
from fastapi.responses import JSONResponse
import time

from ..models.responses import (
    create_success_response,
    create_cluster_info_response,
    create_health_response
)
from ..utils.logging import get_logger
from ..utils.config import get_config
from ..core.vexfs_client import VexFSClient

logger = get_logger(__name__)
router = APIRouter(tags=["cluster"])

# Global VexFS client instance
_vexfs_client = None


def get_vexfs_client() -> VexFSClient:
    """Get or create VexFS client instance"""
    global _vexfs_client
    if _vexfs_client is None:
        config = get_config()
        _vexfs_client = VexFSClient(config.vexfs.device_path)
    return _vexfs_client


@router.get("/cluster")
async def get_cluster_info() -> JSONResponse:
    """
    Get cluster information.
    
    Returns single-node cluster information for VexFS v2 compatibility.
    """
    start_time = time.time()
    
    try:
        processing_time = time.time() - start_time
        
        logger.debug(
            "Cluster info requested",
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_cluster_info_response(),
            status_code=200
        )
        
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error(
            "Error getting cluster info",
            error=str(e),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content={"detail": "Internal server error"},
            status_code=500
        )


@router.get("/")
async def health_check() -> JSONResponse:
    """
    Health check endpoint.
    
    Returns service health and version information.
    """
    start_time = time.time()
    
    try:
        processing_time = time.time() - start_time
        
        logger.debug(
            "Health check requested",
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_health_response(),
            status_code=200
        )
        
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error(
            "Error in health check",
            error=str(e),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content={"detail": "Service unhealthy"},
            status_code=503
        )


@router.get("/metrics")
async def get_metrics(
    vexfs_client: VexFSClient = Depends(get_vexfs_client)
) -> JSONResponse:
    """
    Get VexFS performance metrics.
    
    Returns detailed performance statistics from VexFS v2.
    """
    start_time = time.time()
    
    try:
        stats = vexfs_client.get_collection_stats()
        processing_time = time.time() - start_time
        
        logger.debug(
            "Metrics requested",
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_success_response(stats, processing_time),
            status_code=200
        )
        
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error(
            "Error getting metrics",
            error=str(e),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content={"detail": "Failed to get metrics"},
            status_code=500
        )


@router.get("/telemetry")
async def get_telemetry() -> JSONResponse:
    """
    Get telemetry information.
    
    Returns VexFS v2 telemetry data for monitoring.
    """
    start_time = time.time()
    
    try:
        config = get_config()
        processing_time = time.time() - start_time
        
        telemetry = {
            "id": "vexfs-qdrant-adapter",
            "version": "1.0.0",
            "system": "VexFS v2 Phase 3",
            "performance_targets": {
                "metadata_ops_per_sec": config.performance.target_metadata_ops,
                "search_ops_per_sec": config.performance.target_search_ops,
                "insert_ops_per_sec": config.performance.target_insert_ops
            },
            "device_path": config.vexfs.device_path,
            "api_endpoint": config.get_api_url()
        }
        
        logger.debug(
            "Telemetry requested",
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_success_response(telemetry, processing_time),
            status_code=200
        )
        
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error(
            "Error getting telemetry",
            error=str(e),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content={"detail": "Failed to get telemetry"},
            status_code=500
        )