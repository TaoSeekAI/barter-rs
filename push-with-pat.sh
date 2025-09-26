#!/bin/bash

# GitHub Personal Access Token
PAT="YOUR_GITHUB_PAT_HERE"
USERNAME="TaoSeekAI"
REPO_URL="https://github.com/TaoSeekAI/barter-rs.git"

echo "Attempting to push to GitHub using Personal Access Token..."

# Method 1: Using git credential store
echo "Method 1: Using credential store..."
git config --global credential.helper store
echo "https://${USERNAME}:${PAT}@github.com" > ~/.git-credentials
git push origin vk/1103- 2>&1

if [ $? -eq 0 ]; then
    echo "✅ Push successful!"
    exit 0
fi

# Method 2: Direct URL with token
echo "Method 2: Direct URL with token..."
git push https://${PAT}@github.com/TaoSeekAI/barter-rs.git vk/1103- 2>&1

if [ $? -eq 0 ]; then
    echo "✅ Push successful!"
    exit 0
fi

# Method 3: Using username and token
echo "Method 3: Using username and token..."
git push https://${USERNAME}:${PAT}@github.com/TaoSeekAI/barter-rs.git vk/1103- 2>&1

if [ $? -eq 0 ]; then
    echo "✅ Push successful!"
    exit 0
fi

echo "❌ All methods failed. Please check your token permissions."