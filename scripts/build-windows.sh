#!/bin/bash

# AriaType Windows 构建脚本
# 自动检查并设置签名密钥

set -e

echo "🚀 AriaType Windows 构建脚本"
echo "============================"
echo ""

# 检查私钥文件
PRIVATE_KEY_PATH=~/.tauri/ariatype.key
if [ ! -f "$PRIVATE_KEY_PATH" ]; then
    echo "❌ 错误: 未找到私钥文件"
    echo "   路径: $PRIVATE_KEY_PATH"
    echo ""
    echo "请先生成密钥对:"
    echo "  pnpm tauri signer generate -w ~/.tauri/ariatype.key"
    exit 1
fi

echo "✅ 找到私钥文件: $PRIVATE_KEY_PATH"

# 导出私钥路径
export TAURI_SIGNING_PRIVATE_KEY="$PRIVATE_KEY_PATH"

# 检查是否设置了密码环境变量
if [ -z "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" ]; then
    echo ""
    echo "⚠️  未设置 TAURI_SIGNING_PRIVATE_KEY_PASSWORD 环境变量"
    echo ""
    read -s -p "请输入密钥密码（如果密钥无密码，直接回车）: " PASSWORD
    echo ""
    
    if [ -n "$PASSWORD" ]; then
        export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$PASSWORD"
        echo "✅ 密码已设置"
    else
        echo "ℹ️  继续构建（假设密钥无密码）"
    fi
fi

echo ""
echo "📦 开始构建..."
echo ""

# 执行构建
pnpm run tauri:build:win

echo ""
echo "✅ 构建完成！"
echo ""
echo "📁 构建产物:"
echo "   - NSIS: src-tauri/target/release/bundle/nsis/AriaType-setup.exe"
echo "   - MSI: src-tauri/target/release/bundle/msi/AriaType.msi"
echo "   - 签名: src-tauri/target/release/bundle/nsis/AriaType-setup.exe.sig"
echo "   - 清单: src-tauri/target/release/bundle/latest.json"
