"""
IEEE 754 Float ↔ uint32_t Conversion Utilities

This module provides utilities for converting between Python float values
and their IEEE 754 bit representation as uint32_t integers. This is required
for VexFS v2 kernel module compatibility, which operates entirely in integer
space to avoid floating-point operations in kernel space.

The conversion functions here mirror the C utilities in vexfs_v2_uapi.h.
"""

import struct
import numpy as np
from typing import List, Union


def float_to_bits(f: float) -> int:
    """
    Convert a Python float to its IEEE 754 bit representation as uint32_t.
    
    This mirrors the C function vexfs_float_to_bits() in vexfs_v2_uapi.h.
    
    Args:
        f: Float value to convert
        
    Returns:
        IEEE 754 bit representation as uint32_t integer
        
    Example:
        >>> float_to_bits(1.0)
        1065353216
        >>> float_to_bits(-1.0) 
        3212836864
    """
    # Use struct to get the exact bit representation
    return struct.unpack('<I', struct.pack('<f', f))[0]


def bits_to_float(bits: int) -> float:
    """
    Convert IEEE 754 bit representation (uint32_t) back to Python float.
    
    This mirrors the C function vexfs_bits_to_float() in vexfs_v2_uapi.h.
    
    Args:
        bits: IEEE 754 bit representation as uint32_t integer
        
    Returns:
        Float value
        
    Example:
        >>> bits_to_float(1065353216)
        1.0
        >>> bits_to_float(3212836864)
        -1.0
    """
    # Use struct to convert bits back to float
    return struct.unpack('<f', struct.pack('<I', bits))[0]


def float_array_to_bits(floats: List[float]) -> List[int]:
    """
    Convert an array of floats to their IEEE 754 bit representations.
    
    This mirrors the C function vexfs_float_array_to_bits() in vexfs_v2_uapi.h.
    
    Args:
        floats: List of float values to convert
        
    Returns:
        List of IEEE 754 bit representations as uint32_t integers
        
    Example:
        >>> float_array_to_bits([1.0, -1.0, 0.5])
        [1065353216, 3212836864, 1056964608]
    """
    return [float_to_bits(f) for f in floats]


def bits_array_to_float(bits: List[int]) -> List[float]:
    """
    Convert an array of IEEE 754 bit representations back to floats.
    
    This mirrors the C function vexfs_bits_array_to_float() in vexfs_v2_uapi.h.
    
    Args:
        bits: List of IEEE 754 bit representations as uint32_t integers
        
    Returns:
        List of float values
        
    Example:
        >>> bits_array_to_float([1065353216, 3212836864, 1056964608])
        [1.0, -1.0, 0.5]
    """
    return [bits_to_float(b) for b in bits]


def numpy_array_to_bits(arr: np.ndarray) -> np.ndarray:
    """
    Convert a NumPy float array to IEEE 754 bit representation.
    
    Optimized version using NumPy's view functionality for better performance
    with large vectors.
    
    Args:
        arr: NumPy array of float32 values
        
    Returns:
        NumPy array of uint32 bit representations
        
    Example:
        >>> import numpy as np
        >>> arr = np.array([1.0, -1.0, 0.5], dtype=np.float32)
        >>> numpy_array_to_bits(arr)
        array([1065353216, 3212836864, 1056964608], dtype=uint32)
    """
    # Ensure input is float32
    if arr.dtype != np.float32:
        arr = arr.astype(np.float32)
    
    # Use NumPy's view to reinterpret bytes as uint32
    return arr.view(np.uint32)


def bits_array_to_numpy(bits: np.ndarray) -> np.ndarray:
    """
    Convert IEEE 754 bit representations back to NumPy float array.
    
    Optimized version using NumPy's view functionality for better performance
    with large vectors.
    
    Args:
        bits: NumPy array of uint32 bit representations
        
    Returns:
        NumPy array of float32 values
        
    Example:
        >>> import numpy as np
        >>> bits = np.array([1065353216, 3212836864, 1056964608], dtype=np.uint32)
        >>> bits_array_to_numpy(bits)
        array([ 1. , -1. ,  0.5], dtype=float32)
    """
    # Ensure input is uint32
    if bits.dtype != np.uint32:
        bits = bits.astype(np.uint32)
    
    # Use NumPy's view to reinterpret bytes as float32
    return bits.view(np.float32)


def validate_vector_dimensions(vector: Union[List[float], np.ndarray]) -> int:
    """
    Validate vector dimensions according to VexFS v2 constraints.
    
    Args:
        vector: Vector to validate
        
    Returns:
        Number of dimensions
        
    Raises:
        ValueError: If dimensions are invalid
    """
    if isinstance(vector, np.ndarray):
        dimensions = vector.shape[0] if vector.ndim == 1 else vector.shape[1]
    else:
        dimensions = len(vector)
    
    # VexFS v2 dimension constraints from UAPI
    if dimensions <= 0 or dimensions > 65536:
        raise ValueError(f"Invalid vector dimensions: {dimensions}. Must be 1-65536.")
    
    return dimensions


def prepare_vector_for_kernel(vector: Union[List[float], np.ndarray]) -> List[int]:
    """
    Prepare a vector for VexFS v2 kernel module consumption.
    
    This function:
    1. Validates vector dimensions
    2. Converts to IEEE 754 bit representation
    3. Returns as list of uint32_t integers
    
    Args:
        vector: Input vector as list of floats or NumPy array
        
    Returns:
        List of IEEE 754 bit representations ready for kernel IOCTL
        
    Raises:
        ValueError: If vector is invalid
    """
    # Validate dimensions
    validate_vector_dimensions(vector)
    
    # Convert to list if NumPy array
    if isinstance(vector, np.ndarray):
        vector = vector.tolist()
    
    # Convert to IEEE 754 bits
    return float_array_to_bits(vector)


def prepare_batch_vectors_for_kernel(vectors: List[Union[List[float], np.ndarray]]) -> List[int]:
    """
    Prepare a batch of vectors for VexFS v2 kernel module consumption.
    
    This function flattens multiple vectors into a single array of IEEE 754
    bit representations suitable for batch insert operations.
    
    Args:
        vectors: List of vectors, each as list of floats or NumPy array
        
    Returns:
        Flattened list of IEEE 754 bit representations
        
    Raises:
        ValueError: If vectors have inconsistent dimensions or are invalid
    """
    if not vectors:
        raise ValueError("Empty vector batch")
    
    # Validate all vectors have same dimensions
    first_dims = validate_vector_dimensions(vectors[0])
    for i, vector in enumerate(vectors[1:], 1):
        dims = validate_vector_dimensions(vector)
        if dims != first_dims:
            raise ValueError(f"Vector {i} has {dims} dimensions, expected {first_dims}")
    
    # Convert all vectors and flatten
    all_bits = []
    for vector in vectors:
        vector_bits = prepare_vector_for_kernel(vector)
        all_bits.extend(vector_bits)
    
    return all_bits


# Constants matching VexFS v2 UAPI
VEXFS_VECTOR_FLOAT32 = 0x01
VEXFS_SEARCH_EUCLIDEAN = 0x00
VEXFS_SEARCH_COSINE = 0x01
VEXFS_SEARCH_DOT_PRODUCT = 0x02

# Distance metric mapping for Qdrant compatibility
QDRANT_TO_VEXFS_DISTANCE = {
    "Euclidean": VEXFS_SEARCH_EUCLIDEAN,
    "Cosine": VEXFS_SEARCH_COSINE,
    "Dot": VEXFS_SEARCH_DOT_PRODUCT,
}

VEXFS_TO_QDRANT_DISTANCE = {
    VEXFS_SEARCH_EUCLIDEAN: "Euclidean",
    VEXFS_SEARCH_COSINE: "Cosine", 
    VEXFS_SEARCH_DOT_PRODUCT: "Dot",
}


def get_vexfs_distance_type(qdrant_distance: str) -> int:
    """
    Convert Qdrant distance metric to VexFS v2 distance type.
    
    Args:
        qdrant_distance: Qdrant distance metric name
        
    Returns:
        VexFS v2 distance type constant
        
    Raises:
        ValueError: If distance metric is not supported
    """
    if qdrant_distance not in QDRANT_TO_VEXFS_DISTANCE:
        supported = list(QDRANT_TO_VEXFS_DISTANCE.keys())
        raise ValueError(f"Unsupported distance metric: {qdrant_distance}. Supported: {supported}")
    
    return QDRANT_TO_VEXFS_DISTANCE[qdrant_distance]


def get_qdrant_distance_name(vexfs_distance: int) -> str:
    """
    Convert VexFS v2 distance type to Qdrant distance metric name.
    
    Args:
        vexfs_distance: VexFS v2 distance type constant
        
    Returns:
        Qdrant distance metric name
        
    Raises:
        ValueError: If distance type is not supported
    """
    if vexfs_distance not in VEXFS_TO_QDRANT_DISTANCE:
        supported = list(VEXFS_TO_QDRANT_DISTANCE.keys())
        raise ValueError(f"Unsupported VexFS distance type: {vexfs_distance}. Supported: {supported}")
    
    return VEXFS_TO_QDRANT_DISTANCE[vexfs_distance]