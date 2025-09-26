#!/bin/bash

TOKEN="YOUR_GITHUB_TOKEN_HERE"
USERNAME="TaoSeekAI"
REPO="barter-rs"
BRANCH="vk/1103-"

echo "Testing GitHub Token: YOUR_GITHUB_TOKEN_HERE..."
echo "================================"

# Test API access
echo -e "\n1. Testing API access..."
API_USER=$(curl -s -H "Authorization: token $TOKEN" https://api.github.com/user | grep -o '"login":"[^"]*"' | cut -d'"' -f4)
if [ "$API_USER" == "$USERNAME" ]; then
    echo "✅ API access works for user: $API_USER"
else
    echo "❌ API access failed"
fi

# Method 1: Direct token
echo -e "\n2. Method 1: Direct token URL..."
git push https://${TOKEN}@github.com/${USERNAME}/${REPO}.git ${BRANCH} 2>&1 | head -3

# Method 2: x-access-token
echo -e "\n3. Method 2: x-access-token format..."
git push https://x-access-token:${TOKEN}@github.com/${USERNAME}/${REPO}.git ${BRANCH} 2>&1 | head -3

# Method 3: username:token
echo -e "\n4. Method 3: username:token format..."
git push https://${USERNAME}:${TOKEN}@github.com/${USERNAME}/${REPO}.git ${BRANCH} 2>&1 | head -3

# Method 4: oauth2
echo -e "\n5. Method 4: oauth2 format..."
git push https://oauth2:${TOKEN}@github.com/${USERNAME}/${REPO}.git ${BRANCH} 2>&1 | head -3

# Method 5: Using git credential helper
echo -e "\n6. Method 5: Git credential helper..."
git config --local credential.helper store
echo "https://${USERNAME}:${TOKEN}@github.com" > ~/.git-credentials
git push origin ${BRANCH} 2>&1 | head -3

# Clean up
rm -f ~/.git-credentials
git config --local --unset credential.helper 2>/dev/null

echo -e "\n================================"
echo "If all methods failed, the token likely lacks 'repo' scope permissions."
echo "Create a new token at: https://github.com/settings/tokens/new"
echo "Required scopes: [x] repo (Full control of private repositories)"