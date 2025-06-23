#!/usr/bin/env python3
"""
Simple example demonstrating indexing access to BQ files with pybq
"""

import sys
import os
import time
import pybq

def main():
    if len(sys.argv) != 2:
        print("Usage: python indexing_example.py <file.bq>", file=sys.stderr)
        sys.exit(1)

    file_path = sys.argv[1]

    print("pybq Indexing Example")
    print("====================")

    # Choose the appropriate reader class
    reader_class = pybq.open_vbq if file_path.endswith(".vbq") else pybq.open_bq

    with reader_class(file_path, 32) as reader:
        print(f"file: {os.path.basename(file_path)}")
        total_records = len(reader)
        print(f"records: {total_records:,}")
        print()

        # Demonstrate basic indexing access
        print("Basic Indexing:")
        print("--------------")
        
        if total_records > 0:
            # Access first record
            print(f"Accessing first record (index 0)...")
            first_record = reader[0]
            print(f"  Sequence: {first_record.get_sequence()[:50]}...")
            
            # Access last record (negative indexing)
            print(f"Accessing last record (index -1)...")
            last_record = reader[-1]
            print(f"  Sequence: {last_record.get_sequence()[:50]}...")
            
            # Access middle record if file has enough records
            if total_records > 4:
                print(f"Accessing record at index 4...")
                middle_record = reader[4]
                print(f"  Sequence: {middle_record.get_sequence()[:50]}...")
            
            print(f"\nRandom access demo - accessing indices: 0, 2, 1")
            indices = [0, 2, 1] if total_records >= 3 else [0]
            for i in indices:
                if i < total_records:
                    record = reader[i]
                    seq_preview = record.get_sequence()[:30]
                    print(f"  Record {i}: {seq_preview}...")

    print("\n✓ Indexing allows random access to any record!")
    print("✓ No need to iterate through all previous records!")

if __name__ == "__main__":
    main()