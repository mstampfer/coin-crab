#!/usr/bin/env python3

import subprocess
import sys

def test_timeframe(timeframe, expected_days):
    """Test a specific timeframe endpoint"""
    import os
    from dotenv import load_dotenv
    
    load_dotenv()
    api_key = os.getenv('CMC_API_KEY')
    if not api_key:
        raise ValueError("CMC_API_KEY environment variable is required")
    
    endpoint = f"cmc://historical/btc?timeframe={timeframe}&api_key={api_key}"
    
    print(f"🧪 Testing timeframe: {timeframe} (expecting ~{expected_days} days of data)")
    print(f"   Endpoint: {endpoint}")
    
    # Create a simple test program that calls our function
    test_program = f"""
use std::ffi::{{CStr, CString}};

extern "C" {{
    fn get_historical_crypto_data(endpoint: *const std::os::raw::c_char) -> *mut std::os::raw::c_char;
}}

fn main() {{
    let endpoint = "{endpoint}";
    let endpoint_cstring = CString::new(endpoint).unwrap();
    let result_ptr = unsafe {{ get_historical_crypto_data(endpoint_cstring.as_ptr()) }};
    
    if result_ptr.is_null() {{
        println!("   ❌ Function returned null");
        return;
    }}
    
    let result_cstr = unsafe {{ CStr::from_ptr(result_ptr) }};
    let result_string = result_cstr.to_string_lossy();
    
    match serde_json::from_str::<serde_json::Value>(&result_string) {{
        Ok(json) => {{
            if let (Some(success), Some(data)) = (json.get("success"), json.get("data")) {{
                if success.as_bool() == Some(true) {{
                    if let Some(array) = data.as_array() {{
                        println!("   ✅ {{}} data points", array.len());
                    }}
                }} else {{
                    println!("   ❌ Success=false");
                    if let Some(error) = json.get("error") {{
                        println!("      Error: {{}}", error);
                    }}
                }}
            }}
        }}
        Err(e) => {{
            println!("   ❌ JSON error: {{}}", e);
        }}
    }}
}}
"""
    
    # Write and run the test
    with open('/tmp/timeframe_test.rs', 'w') as f:
        f.write(test_program)
    
    try:
        # Compile and run
        subprocess.run(['rustc', '/tmp/timeframe_test.rs', '-o', '/tmp/timeframe_test', '-L', '/Volumes/OWC Express 1M2/workspace/coin-crab-app/target/debug/deps'], check=True, capture_output=True)
        result = subprocess.run(['/tmp/timeframe_test'], capture_output=True, text=True, timeout=10)
        print(result.stdout.strip())
        if result.stderr:
            print(f"   Stderr: {result.stderr.strip()}")
    except Exception as e:
        print(f"   ❌ Test failed: {e}")

def main():
    print("🧪 Testing all timeframes that iOS sends...")
    
    timeframes = [
        ("1h", 1),    # 1H button
        ("24h", 1),   # 24H button - this works
        ("7d", 7),    # 7D button
        ("30d", 30),  # 30D button
        ("90d", 90),  # 90D button
        ("365d", 365), # 1Y button
        ("all", 1825)  # All button
    ]
    
    for timeframe, expected_days in timeframes:
        test_timeframe(timeframe, expected_days)
        print()

if __name__ == "__main__":
    main()