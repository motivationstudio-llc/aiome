#!/bin/bash
# Aiome Secret Setup Utility

set -e

ENV_FILE=".env"

if [ ! -f "$ENV_FILE" ]; then
    echo "❌ .env file not found. Please create it first (e.g., from .env.example)."
    exit 1
fi

generate_secret() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        LC_ALL=C tr -dc 'A-Za-z0-9' < /dev/urandom | head -c 32
    else
        # Linux
        head /dev/urandom | tr -dc A-Za-z0-9 | head -c 32
    fi
}

update_env() {
    local key=$1
    local new_secret=$(generate_secret)
    
    if grep -q "^$key=" "$ENV_FILE"; then
        sed -i '' "s/^$key=.*/$key=$new_secret/" "$ENV_FILE" 2>/dev/null || \
        sed -i "s/^$key=.*/$key=$new_secret/" "$ENV_FILE"
        echo "✅ Updated $key in $ENV_FILE"
    else
        echo "$key=$new_secret" >> "$ENV_FILE"
        echo "✅ Added $key to $ENV_FILE"
    fi
}

echo "🛡️  Aiome Security Setup"
echo "-----------------------"

# Update API Server Secret
if grep -q "API_SERVER_SECRET=dev_secret" "$ENV_FILE"; then
    update_env "API_SERVER_SECRET"
else
    echo "ℹ️  API_SERVER_SECRET already seems to be set to a custom value."
fi

# Update Federation Secret
if grep -q "FEDERATION_SECRET=dev_secret" "$ENV_FILE"; then
    update_env "FEDERATION_SECRET"
else
    echo "ℹ️  FEDERATION_SECRET already seems to be set to a custom value."
fi

echo "-----------------------"
echo "🚀 Setup complete. Please restart your api-server to apply changes."
