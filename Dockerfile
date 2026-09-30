# IoT-ZTNA 网关 Dockerfile
# 多阶段构建：前端 → Rust 编译 → 最终运行镜像
#
# 构建命令：docker compose build
# 运行命令：docker compose up -d

# === 阶段 1: 构建前端控制台 ===
FROM node:20-bookworm AS frontend-builder

WORKDIR /build/console
COPY console/package.json console/package-lock.json* ./
RUN npm ci --silent || npm install --silent
COPY console/ ./
RUN npm run build

# === 阶段 2: 编译 Rust 后端 ===
FROM rust:1.85-bookworm AS backend-builder

WORKDIR /build

# 安装编译依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# 复制源码（利用 Docker 缓存层）
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# 编译
RUN cargo build --release

# === 阶段 3: 最终运行镜像 ===
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    nftables \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 复制后端二进制
COPY --from=backend-builder /build/target/release/iot-ztna-gateway /app/iot-ztna-gateway

# 复制前端构建产物（网关内置静态文件服务，无需 Nginx）
COPY --from=frontend-builder /build/console/dist /app/console/dist

# 创建数据目录
RUN mkdir -p /app/data /app/config /app/data/certs

# 环境变量默认值（可通过 docker-compose.yml 或 -e 覆盖）
ENV ZTNA_STATIC_DIR=/app/console/dist \
    RUST_LOG=info \
    ZTNA_ADMIN_USER=admin \
    ZTNA_ADMIN_PASS=admin123 \
    ZTNA_AUTH_ENABLED=true

EXPOSE 8443

ENTRYPOINT ["/app/iot-ztna-gateway"]
CMD ["-c", "/app/config/ztna.toml"]
