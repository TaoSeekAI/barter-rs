#!/usr/bin/env python3

import subprocess
import sys
import os

# GitHub token
token = "YOUR_GITHUB_PAT_HERE"
username = "TaoSeekAI"
repo = "barter-rs"
branch = "vk/1103-"

# Set up credentials
os.environ["GIT_ASKPASS"] = "echo"

# Try different URL formats
urls = [
    f"https://{token}@github.com/{username}/{repo}.git",
    f"https://{username}:{token}@github.com/{username}/{repo}.git",
    f"https://x-access-token:{token}@github.com/{username}/{repo}.git",
    f"https://oauth2:{token}@github.com/{username}/{repo}.git"
]

print(f"Attempting to push branch {branch} to GitHub...")

for i, url in enumerate(urls, 1):
    print(f"\nMethod {i}: Testing URL format...")
    try:
        # Try to push
        result = subprocess.run(
            ["git", "push", url, branch],
            capture_output=True,
            text=True,
            timeout=30
        )

        if result.returncode == 0:
            print(f"✅ Push successful using method {i}!")
            print(result.stdout)
            sys.exit(0)
        else:
            print(f"Method {i} failed: {result.stderr[:200]}")
    except subprocess.TimeoutExpired:
        print(f"Method {i} timed out")
    except Exception as e:
        print(f"Method {i} error: {str(e)}")

print("\n❌ All methods failed. The token may be invalid or lack required permissions.")
print("\nTo create a new token:")
print("1. Visit: https://github.com/settings/tokens/new")
print("2. Select scopes: repo (full control)")
print("3. Generate token and try again")