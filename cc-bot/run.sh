#!/bin/bash
# Discord Bot起動スクリプト

set -e

# スクリプトのディレクトリに移動
cd "$(dirname "$0")"

# .envファイルの存在確認
if [ ! -f ".env" ]; then
    echo "Error: .env file not found"
    exit 1
fi

# 環境変数を読み込み
export $(cat .env | grep -v '^#' | xargs)

# デバッグログを有効化
export RUST_LOG=debug

# ボット起動（既にcc-botディレクトリ内）
cargo run
