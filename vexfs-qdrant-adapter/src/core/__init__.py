"""
Core modules for VexFS v2 integration
"""

from .vexfs_client import VexFSClient, VexFSError
from .ieee754 import (
    float_to_bits,
    bits_to_float,
    float_array_to_bits,
    bits_array_to_float,
    prepare_vector_for_kernel,
    prepare_batch_vectors_for_kernel
)

__all__ = [
    "VexFSClient",
    "VexFSError", 
    "float_to_bits",
    "bits_to_float",
    "float_array_to_bits",
    "bits_array_to_float",
    "prepare_vector_for_kernel",
    "prepare_batch_vectors_for_kernel"
]