#!/bin/bash

echo "🚀 启动本地部署环境..."

# 检查Docker是否运行
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker未运行，请先启动Docker"
    exit 1
fi

# 检查镜像是否存在
if ! docker images | grep -q "barter-rs.*local"; then
    echo "📦 镜像不存在，开始构建..."
    docker build -t barter-rs:local .

    if [ $? -ne 0 ]; then
        echo "❌ 镜像构建失败"
        exit 1
    fi
fi

echo "✅ 镜像准备就绪"

# 启动服务
echo "🔄 启动Docker Compose服务..."
docker compose -f docker-compose.local.yml up -d

# 检查服务状态
echo "⏳ 等待服务启动..."
sleep 5

# 显示服务状态
echo "📊 服务状态："
docker compose -f docker-compose.local.yml ps

# 显示日志
echo ""
echo "📝 查看日志命令："
echo "  docker compose -f docker-compose.local.yml logs -f barter-strategy"

echo ""
echo "🛑 停止服务命令："
echo "  docker compose -f docker-compose.local.yml down"

echo ""
echo "✨ 服务已启动！"
echo "  - API: http://localhost:8080"
echo "  - Metrics: http://localhost:9090"
echo "  - Grafana: http://localhost:3000 (admin/admin)"