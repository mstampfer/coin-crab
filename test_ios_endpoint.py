#!/usr/bin/env python3

import subprocess
import sys

def test_ios_endpoint():
    """Test the exact endpoint format that iOS would send"""
    
    # Test the exact format that iOS sends
    import os
    from dotenv import load_dotenv
    
    load_dotenv()
    api_key = os.getenv('CMC_API_KEY')
    if not api_key:
        raise ValueError("CMC_API_KEY environment variable is required")
    
    endpoint = f"cmc://historical/sol?timeframe=24h&interval=1h&api_key={api_key}"
    
    print("🧪 Testing iOS endpoint format for SOL:")
    print(f"   Endpoint: {endpoint}")
    
    # Call our rust test function via cargo
    try:
        result = subprocess.run([
            'cargo', 'test', 'test_historical_data_api', '--', '--nocapture'
        ], cwd='/Volumes/OWC Express 1M2/workspace/coin-crab-app', 
           capture_output=True, text=True, timeout=30)
        
        print(f"   Exit code: {result.returncode}")
        print(f"   STDOUT: {result.stdout}")
        if result.stderr:
            print(f"   STDERR: {result.stderr}")
            
    except subprocess.TimeoutExpired:
        print("   ❌ Test timed out")
    except Exception as e:
        print(f"   ❌ Test failed: {e}")

if __name__ == "__main__":
    test_ios_endpoint()