#!/bin/bash

TOKEN="YOUR_GITHUB_TOKEN_HERE"
USERNAME="TaoSeekAI"

echo "Configuring git with new token..."

# Clear any existing credentials
git config --global --unset credential.helper 2>/dev/null
git config --local --unset credential.helper 2>/dev/null

# Configure git
git config --global user.name "$USERNAME"
git config --global user.email "taoseekai@users.noreply.github.com"

# Method 1: Direct push with all environment variables set
echo "Method 1: Direct push with environment variables..."
export GIT_TERMINAL_PROMPT=0
export GIT_ASKPASS=echo
export GIT_USERNAME=$USERNAME
export GIT_PASSWORD=$TOKEN

git push https://${TOKEN}@github.com/${USERNAME}/barter-rs.git vk/1103- 2>&1

if [ $? -eq 0 ]; then
    echo "✅ Push successful!"
    exit 0
fi

# Method 2: Using credential.helper with token
echo -e "\nMethod 2: Using credential.helper..."
git config credential.helper '!f() { echo "username='$USERNAME'"; echo "password='$TOKEN'"; }; f'
git push https://github.com/${USERNAME}/barter-rs.git vk/1103- 2>&1

if [ $? -eq 0 ]; then
    echo "✅ Push successful!"
    exit 0
fi

# Method 3: Force push with no terminal prompt
echo -e "\nMethod 3: Force push..."
git -c core.askPass=echo -c credential.helper= push https://${TOKEN}@github.com/${USERNAME}/barter-rs.git vk/1103- 2>&1

if [ $? -eq 0 ]; then
    echo "✅ Push successful!"
    exit 0
fi

echo -e "\n❌ All methods failed"
echo "The token may be invalid or the repository may have additional security settings."