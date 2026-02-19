#!/bin/bash

# .env ファイルから環境変数を読み込む（安全な方法）
if [ -f .env ]; then
    set -a
    source .env
    set +a
else
    echo "Error: .env file not found"
    echo "Please copy .env.example to .env and fill in your credentials"
    exit 1
fi

echo "Starting bot..."
echo "GLM API Key: ${GLM_API_KEY:0:10}..."
echo "Discord Token: ${DISCORD_BOT_TOKEN:0:10}..."

cd cc-bot && cargo run
