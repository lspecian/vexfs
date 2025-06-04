"""
Collections API Endpoints

This module implements Qdrant-compatible collection management endpoints
using VexFS v2 as the backend storage and search engine.
"""

from typing import Dict, Any, Optional
from fastapi import APIRouter, HTTPException, Depends, status
from fastapi.responses import JSONResponse
import time

from ..core.vexfs_client import VexFSClient, VexFSError
from ..models.qdrant_types import CreateCollection, CollectionInfo
from ..models.responses import (
    create_success_response,
    create_error_response,
    create_collection_info_response,
    create_collections_list_response
)
from ..utils.logging import get_logger, vexfs_operation_monitor
from ..utils.config import get_config

logger = get_logger(__name__)
router = APIRouter(prefix="/collections", tags=["collections"])

# Global VexFS client instance
_vexfs_client: Optional[VexFSClient] = None


def get_vexfs_client() -> VexFSClient:
    """Get or create VexFS client instance"""
    global _vexfs_client
    if _vexfs_client is None:
        config = get_config()
        try:
            _vexfs_client = VexFSClient(config.vexfs.device_path)
            logger.info("VexFS client initialized", device=config.vexfs.device_path)
        except VexFSError as e:
            logger.error("Failed to initialize VexFS client", error=str(e))
            raise HTTPException(
                status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
                detail=f"VexFS service unavailable: {e}"
            )
    return _vexfs_client


@router.put("/{collection_name}")
@vexfs_operation_monitor("create_collection")
async def create_collection(
    collection_name: str,
    collection_config: CreateCollection,
    vexfs_client: VexFSClient = Depends(get_vexfs_client)
) -> JSONResponse:
    """
    Create a new collection with specified configuration.
    
    This endpoint creates a new vector collection using VexFS v2's
    high-performance storage backend.
    """
    start_time = time.time()
    
    try:
        # Extract vector configuration
        if isinstance(collection_config.vectors, dict):
            # Named vectors - use the first one for now
            vector_config = next(iter(collection_config.vectors.values()))
        else:
            vector_config = collection_config.vectors
        
        # Create collection in VexFS
        result = vexfs_client.create_collection(
            name=collection_name,
            dimensions=vector_config.size,
            distance=vector_config.distance.value
        )
        
        processing_time = time.time() - start_time
        
        logger.info(
            "Collection created successfully",
            collection=collection_name,
            dimensions=vector_config.size,
            distance=vector_config.distance.value,
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_success_response(result, processing_time),
            status_code=status.HTTP_200_OK
        )
        
    except VexFSError as e:
        processing_time = time.time() - start_time
        logger.error(
            "Collection creation failed",
            collection=collection_name,
            error=str(e),
            processing_time=processing_time
        )
        
        if "already exists" in str(e):
            status_code = status.HTTP_409_CONFLICT
        else:
            status_code = status.HTTP_400_BAD_REQUEST
            
        return JSONResponse(
            content=create_error_response(str(e)),
            status_code=status_code
        )
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error(
            "Unexpected error creating collection",
            collection=collection_name,
            error=str(e),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_error_response("Internal server error"),
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@router.get("/{collection_name}")
@vexfs_operation_monitor("get_collection_info")
async def get_collection_info(
    collection_name: str,
    vexfs_client: VexFSClient = Depends(get_vexfs_client)
) -> JSONResponse:
    """
    Get information about a specific collection.
    
    Returns collection status, configuration, and statistics.
    """
    start_time = time.time()
    
    try:
        info = vexfs_client.get_collection_info(collection_name)
        processing_time = time.time() - start_time
        
        logger.debug(
            "Collection info retrieved",
            collection=collection_name,
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_success_response(info, processing_time),
            status_code=status.HTTP_200_OK
        )
        
    except VexFSError as e:
        processing_time = time.time() - start_time
        logger.error(
            "Failed to get collection info",
            collection=collection_name,
            error=str(e),
            processing_time=processing_time
        )
        
        if "not found" in str(e):
            status_code = status.HTTP_404_NOT_FOUND
        else:
            status_code = status.HTTP_400_BAD_REQUEST
            
        return JSONResponse(
            content=create_error_response(str(e)),
            status_code=status_code
        )
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error(
            "Unexpected error getting collection info",
            collection=collection_name,
            error=str(e),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_error_response("Internal server error"),
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@router.delete("/{collection_name}")
@vexfs_operation_monitor("delete_collection")
async def delete_collection(
    collection_name: str,
    vexfs_client: VexFSClient = Depends(get_vexfs_client)
) -> JSONResponse:
    """
    Delete a collection and all its data.
    
    This operation is irreversible and will remove all vectors
    and metadata associated with the collection.
    """
    start_time = time.time()
    
    try:
        success = vexfs_client.delete_collection(collection_name)
        processing_time = time.time() - start_time
        
        if success:
            logger.info(
                "Collection deleted successfully",
                collection=collection_name,
                processing_time=processing_time
            )
            
            return JSONResponse(
                content=create_success_response(True, processing_time),
                status_code=status.HTTP_200_OK
            )
        else:
            return JSONResponse(
                content=create_error_response("Failed to delete collection"),
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
            )
            
    except VexFSError as e:
        processing_time = time.time() - start_time
        logger.error(
            "Collection deletion failed",
            collection=collection_name,
            error=str(e),
            processing_time=processing_time
        )
        
        if "not found" in str(e):
            status_code = status.HTTP_404_NOT_FOUND
        else:
            status_code = status.HTTP_400_BAD_REQUEST
            
        return JSONResponse(
            content=create_error_response(str(e)),
            status_code=status_code
        )
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error(
            "Unexpected error deleting collection",
            collection=collection_name,
            error=str(e),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_error_response("Internal server error"),
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@router.get("")
@vexfs_operation_monitor("list_collections")
async def list_collections(
    vexfs_client: VexFSClient = Depends(get_vexfs_client)
) -> JSONResponse:
    """
    List all collections.
    
    Returns a list of all available collections with basic information.
    """
    start_time = time.time()
    
    try:
        collections = vexfs_client.list_collections()
        processing_time = time.time() - start_time
        
        logger.debug(
            "Collections listed",
            count=len(collections.get("collections", {})),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_collections_list_response(collections),
            status_code=status.HTTP_200_OK
        )
        
    except VexFSError as e:
        processing_time = time.time() - start_time
        logger.error(
            "Failed to list collections",
            error=str(e),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_error_response(str(e)),
            status_code=status.HTTP_400_BAD_REQUEST
        )
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error(
            "Unexpected error listing collections",
            error=str(e),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_error_response("Internal server error"),
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@router.patch("/{collection_name}")
async def update_collection(
    collection_name: str,
    update_config: Dict[str, Any],
    vexfs_client: VexFSClient = Depends(get_vexfs_client)
) -> JSONResponse:
    """
    Update collection configuration.
    
    Note: VexFS v2 has limited support for configuration updates.
    This endpoint provides compatibility but may not support all operations.
    """
    start_time = time.time()
    
    try:
        # For now, just verify the collection exists
        info = vexfs_client.get_collection_info(collection_name)
        processing_time = time.time() - start_time
        
        logger.info(
            "Collection update requested (limited support)",
            collection=collection_name,
            update_config=update_config,
            processing_time=processing_time
        )
        
        # Return current info as update result
        return JSONResponse(
            content=create_success_response(info, processing_time),
            status_code=status.HTTP_200_OK
        )
        
    except VexFSError as e:
        processing_time = time.time() - start_time
        logger.error(
            "Collection update failed",
            collection=collection_name,
            error=str(e),
            processing_time=processing_time
        )
        
        if "not found" in str(e):
            status_code = status.HTTP_404_NOT_FOUND
        else:
            status_code = status.HTTP_400_BAD_REQUEST
            
        return JSONResponse(
            content=create_error_response(str(e)),
            status_code=status_code
        )
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error(
            "Unexpected error updating collection",
            collection=collection_name,
            error=str(e),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_error_response("Internal server error"),
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )