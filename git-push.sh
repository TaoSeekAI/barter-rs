#!/bin/bash

# GitHub PAT token
TOKEN="YOUR_GITHUB_PAT_HERE"

# Configure git
git config --local credential.helper ""
git config --local user.name "TaoSeekAI"
git config --local user.email "taoseekai@github.com"

# Push using the token
echo "Pushing to GitHub..."
git push https://x-access-token:${TOKEN}@github.com/TaoSeekAI/barter-rs.git vk/1103-

echo "Push completed!"