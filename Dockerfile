FROM node:22-alpine AS frontend-builder

WORKDIR /app/admin-ui
COPY admin-ui/package.json ./
RUN npm install -g pnpm && pnpm install --ignore-scripts
COPY admin-ui ./
RUN pnpm build

WORKDIR /app/user-ui
COPY user-ui/package.json ./
RUN npm install
COPY user-ui ./
RUN npm run build

FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static

WORKDIR /app
# 依赖缓存层：先用桩 main 把全部 crates 编译进 target，源码/前端变更不再触发重新下载
# （网络抖动只影响本层首次构建；命中缓存后离线也能完成最终编译）
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

COPY src ./src
COPY assets ./assets
COPY --from=frontend-builder /app/admin-ui/dist /app/admin-ui/dist
COPY --from=frontend-builder /app/user-ui/dist /app/user-ui/dist

RUN touch src/main.rs && cargo build --release

FROM alpine:3.21

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /app/target/release/kiro2cc-proxy /app/kiro2cc-proxy

EXPOSE 5678

CMD sh -c 'mkdir -p /app/config && \
  if [ ! -f /app/config/config.json ]; then \
    echo "{\"host\":\"${HOST:-0.0.0.0}\",\"port\":${PORT:-5678},\"adminPsw\":\"${ADMIN_PSW:-$ADMIN_API_KEY}\"}" > /app/config/config.json; \
  fi && \
  if [ ! -f /app/config/credentials.json ]; then \
    echo "[]" > /app/config/credentials.json; \
  fi && \
  ./kiro2cc-proxy --config /app/config/config.json --credentials /app/config/credentials.json'
