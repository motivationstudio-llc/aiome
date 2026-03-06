#!/bin/bash

# 外部生成エンジンを隔離環境で起動するスクリプト

# プロジェクトのルートディレクトリを取得
PROJECT_ROOT=$(cd $(dirname $0)/..; pwd)
WORKSPACE_DIR="$PROJECT_ROOT/workspace/aiome"
ENGINE_OUT_DIR="$WORKSPACE_DIR/engine_out"

# ディレクトリの作成
mkdir -p "$ENGINE_OUT_DIR"

echo "🔒 Starting Generative Engine with Synchronized Sandbox..."
echo "📂 Jail Root: $WORKSPACE_DIR"
echo "📁 Output Dir: $ENGINE_OUT_DIR"

if [ -d "Engine" ]; then
    cd Engine
    python3 main.py --output-directory "$ENGINE_OUT_DIR" "$@"
else
    echo "⚠️  Engine directory not found in project root."
    echo "Please ensure the engine is installed at: $PROJECT_ROOT/Engine"
fi
