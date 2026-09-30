# 前端控制台 Dockerfile
FROM node:20-bookworm AS builder

WORKDIR /app

COPY package.json package-lock.json* ./
RUN npm install

COPY . .
RUN npm run build

# === 运行镜像 ===
FROM nginx:alpine

COPY --from=builder /app/dist /usr/share/nginx/html

# Nginx 配置 - 代理 API 到后端
RUN echo 'server { \
    listen 80; \
    location / { \
        root /usr/share/nginx/html; \
        try_files $uri $uri/ /index.html; \
    } \
    location /api/ { \
        proxy_pass http://ztna-gateway:8443; \
    } \
    location /ws { \
        proxy_pass http://ztna-gateway:8443; \
        proxy_http_version 1.1; \
        proxy_set_header Upgrade $http_upgrade; \
        proxy_set_header Connection "upgrade"; \
    } \
}' > /etc/nginx/conf.d/default.conf

EXPOSE 80
