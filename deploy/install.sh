#!/bin/bash
# IoT-ZTNA 网关安装脚本（源码编译后部署）
set -e

INSTALL_DIR="/opt/iot-ztna"
SERVICE_FILE="iot-ztna-gateway.service"
USER="ztna"

echo ""
echo "  ════════════════════════════════════════════"
echo "  IoT-ZTNA 网关安装"
echo "  ════════════════════════════════════════════"
echo ""

# 创建系统用户
if ! id "$USER" &>/dev/null; then
    echo "✅ 创建系统用户: $USER"
    useradd --system --no-create-home --shell /sbin/nologin "$USER"
fi

# 创建目录
echo "✅ 创建安装目录: $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"/{config,data,data/certs,console/dist}

# 复制二进制
echo "✅ 复制二进制..."
cp target/release/iot-ztna-gateway "$INSTALL_DIR/"

# 复制配置文件
echo "✅ 复制配置文件..."
cp config/ztna.toml "$INSTALL_DIR/config/"

# 复制前端文件
echo "✅ 复制前端文件..."
if [ -d console/dist ]; then
    cp -r console/dist/* "$INSTALL_DIR/console/dist/"
fi

# 安装 systemd 服务
echo "✅ 安装 systemd 服务..."
cp deploy/$SERVICE_FILE /etc/systemd/system/

# 设置权限
chown -R "$USER":"$USER" "$INSTALL_DIR"

# 启用服务
systemctl daemon-reload
systemctl enable iot-ztna-gateway

echo ""
echo "  ════════════════════════════════════════════"
echo "  ✅ 安装完成！"
echo "  ════════════════════════════════════════════"
echo ""
echo "  下一步："
echo "    1. 修改密码：编辑 /etc/systemd/system/$SERVICE_FILE"
echo "       修改 ZTNA_ADMIN_PASS 和 ZTNA_JWT_SECRET"
echo ""
echo "    2. 初始化 CA："
echo "       $INSTALL_DIR/iot-ztna-gateway --init-ca"
echo ""
echo "    3. 初始化策略："
echo "       $INSTALL_DIR/iot-ztna-gateway --init-policy"
echo ""
echo "    4. 启动服务："
echo "       systemctl start iot-ztna-gateway"
echo ""
echo "    5. 访问控制台：http://<服务器IP>:9443"
echo "       默认登录：admin / admin123"
echo ""
echo "    6. 查看状态：systemctl status iot-ztna-gateway"
echo "       查看日志：journalctl -u iot-ztna-gateway -f"
echo ""
