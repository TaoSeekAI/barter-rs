#!/bin/bash

# GitHub token (provided by user)
TOKEN="YOUR_GITHUB_TOKEN_HERE"

# Configure git to use the token
git config --local credential.helper ""
git config --local http.https://github.com/.extraheader "AUTHORIZATION: bearer ${TOKEN}"

# Push the changes
git push https://x-access-token:${TOKEN}@github.com/TaoSeekAI/barter-rs.git vk/1103-

echo "Push completed"