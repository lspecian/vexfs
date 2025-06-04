#!/usr/bin/env python3
"""
VexFS Python Bindings Setup
"""

from pybind11.setup_helpers import Pybind11Extension, build_ext
from setuptools import setup, Extension
import pybind11

# Define the extension module
ext_modules = [
    Pybind11Extension(
        "vexfs_python",
        [
            "src/vexfs_python.cpp",
            "src/vector_operations.cpp", 
            "src/filesystem_interface.cpp",
        ],
        include_dirs=[
            pybind11.get_cmake_dir(),
            "../../",  # VexFS root directory
            "../../rust/src",  # Rust source directory
        ],
        language='c++',
        cxx_std=17,
    ),
]

setup(
    name="vexfs-python",
    version="1.0.0",
    author="VexFS Team",
    author_email="dev@vexfs.org",
    description="Python bindings for VexFS vector filesystem",
    long_description="Python bindings for VexFS, a high-performance vector filesystem with advanced indexing capabilities.",
    ext_modules=ext_modules,
    cmdclass={"build_ext": build_ext},
    zip_safe=False,
    python_requires=">=3.8",
    install_requires=[
        "pybind11>=2.10.0",
        "numpy>=1.20.0",
    ],
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: Apache Software License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Programming Language :: C++",
        "Topic :: Software Development :: Libraries",
        "Topic :: System :: Filesystems",
    ],
)