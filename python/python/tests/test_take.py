#!/usr/bin/env python3
"""
Test script to validate the new take functionality with row indices.
"""
import lancedb
import pyarrow as pa
import numpy as np


def test_take_api():
    """Test the take API with row indices."""
    
    print("Testing take API...")
    
    # Connect to lancedb server
    db = lancedb.connect("db://my-db", api_key="sk_localtest", host_override="http://localhost:10024")
    
    # Test data - create a simple table first
    table_name = "test_take_table"
    
    # Create test data using pyarrow
    data = pa.table({
        "id": pa.array([0, 1, 2, 3, 4], type=pa.int32()),
        "value": pa.array([10, 20, 30, 40, 50], type=pa.int32()),
        "name": pa.array(["alice", "bob", "charlie", "diana", "eve"], type=pa.string()),
        "vector": pa.array([[1.0, 2.0, 3.0, 4.0], 
                           [5.0, 6.0, 7.0, 8.0],
                           [9.0, 10.0, 11.0, 12.0],
                           [13.0, 14.0, 15.0, 16.0],
                           [17.0, 18.0, 19.0, 20.0]], type=pa.list_(pa.float32()))
    })
    
    # Try to create table
    try:
        # Drop table if exists
        try:
            db.drop_table(table_name)
        except:
            pass
            
        table = db.create_table(table_name, data)
        print(f"✓ Created table {table_name}")
    except Exception as e:
        print(f"✗ Exception creating table: {e}")
        return
    
    # Test 1: Take specific rows by indices
    print("\n--- Test 1: Take rows by indices [0, 2, 4] ---")
    try:
        reader = table.take([0, 2, 4])
        print("✓ Take request successful")
        print(f"Reader type: {type(reader)}")
        print(f"Reader schema: {reader.schema}")
        result = reader.read_all()  # Convert RecordBatchReader to Table
        print(f"✓ Retrieved {len(result)} rows")
        print("Result data:")
        print(result.to_pandas())
        
        # Verify we got the expected rows
        expected_ids = [0, 2, 4]
        actual_ids = result.column('id').to_pylist()
        if actual_ids == expected_ids:
            print("✓ Retrieved correct row indices")
        else:
            print(f"✗ Expected IDs {expected_ids}, got {actual_ids}")
            
    except Exception as e:
        print(f"✗ Exception during take request: {e}")
    
    # Test 2: Take with column selection
    print("\n--- Test 2: Take rows with column selection ---")
    try:
        reader = table.take([1, 3], columns=["id", "name"])
        print("✓ Take with column selection successful")
        result = reader.read_all()  # Convert RecordBatchReader to Table
        print(f"✓ Retrieved {len(result)} rows")
        print(f"✓ Columns: {result.column_names}")
        print("Result data:")
        print(result.to_pandas())
        
        # Verify columns
        expected_columns = ["id", "name"]
        if result.column_names == expected_columns:
            print("✓ Retrieved correct columns")
        else:
            print(f"✗ Expected columns {expected_columns}, got {result.column_names}")
            
    except Exception as e:
        print(f"✗ Exception during column selection take: {e}")
    
    # Test 3: Empty indices (should return empty batch)
    print("\n--- Test 3: Empty indices ---")
    try:
        reader = table.take([])
        result = reader.read_all()  # Convert RecordBatchReader to Table
        print(f"✓ Empty indices returned {len(result)} rows")
        if len(result) == 0:
            print("✓ Empty indices correctly handled")
        else:
            print(f"✗ Expected 0 rows for empty indices, got {len(result)}")
            
    except Exception as e:
        print(f"✗ Exception during empty indices test: {e}")
    
    # Test 4: Large indices range
    print("\n--- Test 4: Larger row indices range ---")
    try:
        reader = table.take(list(range(0, 5, 2)), columns=["value"])
        print("✓ Large indices range successful")
        result = reader.read_all()  # Convert RecordBatchReader to Table
        print(f"✓ Retrieved {len(result)} rows")
        print("Result data:")
        print(result.to_pandas())
        
        expected_values = [10, 30, 50]  # values at indices 0, 2, 4
        actual_values = result.column('value').to_pylist()
        if actual_values == expected_values:
            print("✓ Retrieved correct values")
        else:
            print(f"✗ Expected values {expected_values}, got {actual_values}")
            
    except Exception as e:
        print(f"✗ Exception during large indices test: {e}")
    
    # Test 5: Test with larger dataset
    print("\n--- Test 5: Create larger dataset and test take ---")
    try:
        # Create a larger table
        large_table_name = "test_take_large"
        try:
            db.drop_table(large_table_name)
        except:
            pass
            
        # Create data with 10000 rows
        n_rows = 10000
        large_data = pa.table({
            "id": pa.array(range(n_rows), type=pa.int32()),
            "value": pa.array([i * 10 for i in range(n_rows)], type=pa.int32()),
            "text": pa.array([f"row_{i}" for i in range(n_rows)], type=pa.string()),
            "vector": pa.array([[float(i), float(i+1), float(i+2), float(i+3)] for i in range(n_rows)], 
                             type=pa.list_(pa.float32()))
        })
        
        large_table = db.create_table(large_table_name, large_data)
        print(f"✓ Created large table {large_table_name} with {n_rows} rows")
        
        # Test taking scattered indices
        scattered_indices = [0, 100, 500, 1000, 2500, 5000, 7500, 9999]
        reader = large_table.take(scattered_indices, columns=["id", "text"])
        
        print("✓ Large table take request successful")
        result = reader.read_all()  # Convert RecordBatchReader to Table
        print(f"✓ Retrieved {len(result)} rows from large table")
        print("Sample of results:")
        print(result.to_pandas())
        
        # Verify we got the correct IDs
        actual_ids = result.column('id').to_pylist()
        if actual_ids == scattered_indices:
            print("✓ Retrieved correct scattered indices from large table")
        else:
            print(f"✗ Expected IDs {scattered_indices}, got {actual_ids}")
            
    except Exception as e:
        print(f"✗ Exception during large table test: {e}")

if __name__ == "__main__":
    test_take_api()