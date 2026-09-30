#!/bin/bash
# IoT-ZTNA 一键启动脚本
# 用法：./quickstart.sh
# 会自动构建并启动 Docker 容器，然后初始化 CA 和策略

set -e

echo ""
echo "  ╔══════════════════════════════════════════════╗"
echo "  ║         IoT-ZTNA 网关一键部署                ║"
echo "  ║         零信任 · 物联网 · 安全网关           ║"
echo "  ╚══════════════════════════════════════════════╝"
echo ""

# 检查 Docker
if ! command -v docker &>/dev/null; then
    echo "❌ 未检测到 Docker，请先安装 Docker。"
    echo "   安装指南：https://docs.docker.com/engine/install/"
    exit 1
fi

if ! docker compose version &>/dev/null; then
    echo "❌ 未检测到 Docker Compose，请先安装。"
    echo "   安装指南：https://docs.docker.com/compose/install/"
    exit 1
fi

echo "✅ Docker 环境正常"
echo ""

# 构建并启动
echo "📦 正在构建镜像（首次运行需要 2~3 分钟）..."
docker compose up -d --build

echo ""
echo "⏳ 等待服务启动..."
sleep 5

# 检查健康状态
echo "🔍 检查服务健康状态..."
for i in $(seq 1 10); do
    if curl -sf http://localhost:8443/api/v1/health >/dev/null 2>&1; then
        echo "✅ 网关服务已就绪"
        break
    fi
    if [ $i -eq 10 ]; then
        echo "⚠️  服务启动超时，请检查日志：docker compose logs"
        exit 1
    fi
    echo "   等待中... ($i/10)"
    sleep 3
done

# 初始化 CA
echo ""
echo "🔐 初始化根 CA..."
if docker exec iot-ztna-gateway /app/iot-ztna-gateway --init-ca 2>/dev/null; then
    echo "✅ 根 CA 初始化完成"
else
    echo "ℹ️  CA 可能已存在，跳过"
fi

# 初始化策略
echo "📋 初始化默认策略..."
if docker exec iot-ztna-gateway /app/iot-ztna-gateway --init-policy 2>/dev/null; then
    echo "✅ 默认策略初始化完成"
else
    echo "ℹ️  策略可能已存在，跳过"
fi

# 完成
echo ""
echo "  ╔══════════════════════════════════════════════╗"
echo "  ║         🎉 部署成功！                         ║"
echo "  ╚══════════════════════════════════════════════╝"
echo ""
echo "  📌 访问地址："
echo "     管理控制台：http://localhost:8443"
echo "     健康检查：  http://localhost:8443/api/v1/health"
echo ""
echo "  🔑 默认登录："
echo "     用户名：admin"
echo "     密码：  admin123"
echo ""
echo "  📖 常用命令："
echo "     查看日志：  docker compose logs -f"
echo "     停止服务：  docker compose down"
echo "     重启服务：  docker compose restart"
echo ""
echo "  ⚠️  生产环境请修改默认密码！"
echo "     设置环境变量后重新启动："
echo "     export ZTNA_ADMIN_PASS=your-secure-password"
echo "     export ZTNA_JWT_SECRET=your-jwt-secret"
echo "     docker compose up -d"
echo ""
