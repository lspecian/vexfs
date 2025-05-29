#!/usr/bin/env python3
"""
Basic VexFS Python SDK usage example
"""

import vexfs

def main():
    # Add a document
    doc_id = vexfs.add("Hello world", {"type": "greeting", "lang": "en"})
    print(f"Added document with ID: {doc_id}")
    
    # Query (placeholder - will need actual vector)
    results = vexfs.query([0.1, 0.2, 0.3], top_k=5)
    print(f"Query results: {results}")
    
    # Delete document
    vexfs.delete(doc_id)
    print("Document deleted")

if __name__ == "__main__":
    main()