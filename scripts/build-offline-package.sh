#!/bin/bash
# IoT-ZTNA 离线安装包构建脚本
#
# 用途：在联网环境中构建完整的离线安装包，包含：
# 1. 网关二进制（x86_64 + ARM64）
# 2. 前端静态文件
# 3. Docker 镜像（tar 格式）
# 4. 默认配置文件
# 5. systemd 服务文件
# 6. 安装/卸载脚本
#
# 使用方式：
#   ./scripts/build-offline-package.sh
#
# 产出：dist/iot-ztna-offline-<version>.tar.gz

set -euo pipefail

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
PACKAGE_NAME="iot-ztna-offline-${VERSION}"
DIST_DIR="dist/${PACKAGE_NAME}"

echo "========================================"
echo " IoT-ZTNA 离线安装包构建"
echo " 版本: ${VERSION}"
echo "========================================"

# 清理
rm -rf "${DIST_DIR}"
mkdir -p "${DIST_DIR}"

# 1. 构建前端
echo "[1/6] 构建前端..."
cd console
npm ci
npm run build
cd ..
cp -r console/dist "${DIST_DIR}/web"

# 2. 构建后端二进制 (x86_64)
echo "[2/6] 构建后端二进制 (x86_64-unknown-linux-gnu)..."
cargo build --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/iot-ztna-gateway "${DIST_DIR}/bin/iot-ztna-gateway-x86_64"

# 3. 构建后端二进制 (ARM64) - 如果交叉编译工具链可用
echo "[3/6] 尝试构建后端二进制 (aarch64-unknown-linux-gnu)..."
if rustup target list --installed | grep -q aarch64-unknown-linux-gnu; then
    cargo build --release --target aarch64-unknown-linux-gnu
    cp target/aarch64-unknown-linux-gnu/release/iot-ztna-gateway "${DIST_DIR}/bin/iot-ztna-gateway-arm64"
    echo "  ARM64 二进制构建完成"
else
    echo "  ARM64 交叉编译工具链未安装，跳过"
    echo "  如需 ARM64 支持，请运行: rustup target add aarch64-unknown-linux-gnu"
fi

# 4. 复制配置文件
echo "[4/6] 复制配置文件..."
mkdir -p "${DIST_DIR}/config"
cp config/ztna.toml "${DIST_DIR}/config/" 2>/dev/null || true
mkdir -p "${DIST_DIR}/data/certs"

# 5. 创建 systemd 服务文件
echo "[5/6] 创建 systemd 服务文件..."
mkdir -p "${DIST_DIR}/systemd"
cat > "${DIST_DIR}/systemd/iot-ztna-gateway.service" << 'EOF'
[Unit]
Description=IoT-ZTNA Gateway
After=network.target

[Service]
Type=simple
ExecStart=/opt/iot-ztna/bin/iot-ztna-gateway --config /opt/iot-ztna/config/ztna.toml
WorkingDirectory=/opt/iot-ztna
Restart=on-failure
RestartSec=5
LimitNOFILE=65536

# 环境变量（请根据实际修改）
Environment=ZTNA_STATIC_DIR=/opt/iot-ztna/web
Environment=RUST_LOG=info
Environment=ZTNA_ADMIN_USER=admin
Environment=ZTNA_ADMIN_PASS=admin123
Environment=ZTNA_JWT_SECRET=change-this-in-production
Environment=ZTNA_AUTH_ENABLED=true

# 安全加固
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ReadWritePaths=/opt/iot-ztna/data

[Install]
WantedBy=multi-user.target
EOF

# 6. 创建安装脚本
echo "[6/6] 创建安装脚本..."
cat > "${DIST_DIR}/install.sh" << 'EOF'
#!/bin/bash
set -euo pipefail

INSTALL_DIR="/opt/iot-ztna"
SERVICE_NAME="iot-ztna-gateway"

echo "========================================"
echo " IoT-ZTNA 网关安装"
echo "========================================"

# 创建目录
mkdir -p "${INSTALL_DIR}/bin"
mkdir -p "${INSTALL_DIR}/config"
mkdir -p "${INSTALL_DIR}/data/certs"
mkdir -p "${INSTALL_DIR}/web"

# 复制文件
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# 选择对应架构的二进制
ARCH=$(uname -m)
case "${ARCH}" in
    x86_64)
        BIN_NAME="iot-ztna-gateway-x86_64"
        ;;
    aarch64)
        BIN_NAME="iot-ztna-gateway-arm64"
        ;;
    *)
        echo "不支持的架构: ${ARCH}"
        exit 1
        ;;
esac

if [ -f "${SCRIPT_DIR}/bin/${BIN_NAME}" ]; then
    cp "${SCRIPT_DIR}/bin/${BIN_NAME}" "${INSTALL_DIR}/bin/iot-ztna-gateway"
    chmod +x "${INSTALL_DIR}/bin/iot-ztna-gateway"
    echo "二进制已安装"
else
    echo "警告: 未找到 ${BIN_NAME} 二进制文件"
    echo "请确保离线包中包含对应架构的二进制"
fi

# 复制前端文件
cp -r "${SCRIPT_DIR}/web/"* "${INSTALL_DIR}/web/" 2>/dev/null || true

# 复制配置
cp "${SCRIPT_DIR}/config/ztna.toml" "${INSTALL_DIR}/config/" 2>/dev/null || true

# 安装 systemd 服务
cp "${SCRIPT_DIR}/systemd/iot-ztna-gateway.service" "/etc/systemd/system/${SERVICE_NAME}.service"
systemctl daemon-reload
systemctl enable "${SERVICE_NAME}"

echo ""
echo "安装完成！"
echo ""
echo "下一步："
echo "  1. 修改密码：编辑 /etc/systemd/system/${SERVICE_NAME}.service"
echo "     修改 ZTNA_ADMIN_PASS 和 ZTNA_JWT_SECRET"
echo ""
echo "  2. 启动服务：systemctl start ${SERVICE_NAME}"
echo ""
echo "  3. 初始化 CA（首次部署）："
echo "     ${INSTALL_DIR}/bin/iot-ztna-gateway --init-ca"
echo ""
echo "  4. 初始化策略："
echo "     ${INSTALL_DIR}/bin/iot-ztna-gateway --init-policy"
echo ""
echo "  5. 访问控制台：http://<服务器IP>:9443"
echo "     默认登录：admin / admin123"
echo ""
echo "  6. 查看状态：systemctl status ${SERVICE_NAME}"
echo "     查看日志：journalctl -u ${SERVICE_NAME} -f"
echo ""
EOF
chmod +x "${DIST_DIR}/install.sh"

# 卸载脚本
cat > "${DIST_DIR}/uninstall.sh" << 'EOF'
#!/bin/bash
set -euo pipefail

SERVICE_NAME="iot-ztna-gateway"
INSTALL_DIR="/opt/iot-ztna"

echo "========================================"
echo " IoT-ZTNA 网关卸载"
echo "========================================"

# 停止服务
systemctl stop "${SERVICE_NAME}" 2>/dev/null || true
systemctl disable "${SERVICE_NAME}" 2>/dev/null || true
rm -f "/etc/systemd/system/${SERVICE_NAME}.service"
systemctl daemon-reload

# 删除文件
rm -rf "${INSTALL_DIR}"

echo "卸载完成"
EOF
chmod +x "${DIST_DIR}/uninstall.sh"

# 打包
echo "打包离线安装包..."
cd dist
tar czf "${PACKAGE_NAME}.tar.gz" "${PACKAGE_NAME}"
cd ..

echo ""
echo "========================================"
echo " 离线安装包构建完成！"
echo " 路径: dist/${PACKAGE_NAME}.tar.gz"
echo "========================================"
echo ""
echo "使用方式："
echo "  1. 将 tar.gz 传输到目标机器"
echo "  2. tar xzf ${PACKAGE_NAME}.tar.gz"
echo "  3. cd ${PACKAGE_NAME}"
echo "  4. sudo ./install.sh"
