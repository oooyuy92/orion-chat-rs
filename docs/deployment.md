# Orion Chat 部署指南

本文档介绍如何使用 Docker 部署 Orion Chat 应用。

## 概述

Orion Chat 提供基于 Docker 的部署方案，支持：
- 一键部署，无需手动配置环境
- 数据持久化，升级不丢失数据
- PWA 支持，可安装到移动设备
- 轻量级运行，资源占用低

适用场景：
- 个人服务器部署
- 团队内部使用
- 移动设备访问

## 前置要求

### 系统要求
- 操作系统：Linux、macOS 或 Windows (with WSL2)
- 内存：至少 512MB 可用内存
- 磁盘：至少 1GB 可用空间

### 软件依赖
- Docker 20.10 或更高版本
- Docker Compose 2.0 或更高版本

安装 Docker：
```bash
# Ubuntu/Debian
curl -fsSL https://get.docker.com | sh

# macOS
brew install docker docker-compose

# 验证安装
docker --version
docker-compose --version
```

## 快速开始

### 1. 克隆仓库
```bash
git clone https://github.com/yourusername/orion-chat-rs.git
cd orion-chat-rs
```

### 2. 启动服务
```bash
docker-compose up -d
```

### 3. 访问应用
打开浏览器访问：`http://localhost:28080`

应用启动后，您可以：
- 在浏览器中直接使用
- 在移动设备上安装为 PWA（见下文）

### 4. 查看日志
```bash
docker-compose logs -f
```

### 5. 停止服务
```bash
docker-compose down
```

## 环境变量配置

可以通过修改 `docker-compose.yml` 或创建 `.env` 文件来配置环境变量。

### 可用环境变量

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `DATABASE_PATH` | `/data/orion.db` | SQLite 数据库文件路径 |
| `DATA_DIR` | `/data` | 数据目录路径 |
| `STATIC_DIR` | `/app/static` | 静态文件目录（前端构建产物） |
| `PORT` | `28080` | 服务监听端口 |

### 配置示例

修改 `docker-compose.yml`：
```yaml
services:
  orion-chat:
    environment:
      - DATABASE_PATH=/data/orion.db
      - DATA_DIR=/data
      - PORT=8080  # 修改端口
    ports:
      - "8080:8080"  # 同时修改端口映射
```

或创建 `.env` 文件：
```bash
DATABASE_PATH=/data/orion.db
DATA_DIR=/data
PORT=28080
```

然后在 `docker-compose.yml` 中引用：
```yaml
services:
  orion-chat:
    env_file: .env
```

## 数据持久化

### Named Volume

默认配置使用 Docker Named Volume 来持久化数据：

```yaml
volumes:
  orion-data:/data
```

这种方式的优点：
- 数据独立于容器生命周期
- 升级应用不会丢失数据
- Docker 自动管理存储位置

### 查看数据位置

```bash
# 查看 volume 详情
docker volume inspect orion-chat_orion-data

# 输出示例
[
    {
        "Name": "orion-chat_orion-data",
        "Mountpoint": "/var/lib/docker/volumes/orion-chat_orion-data/_data"
    }
]
```

### 数据备份

```bash
# 备份数据库
docker-compose exec orion-chat cp /data/orion.db /data/orion.db.backup

# 或者从宿主机备份
docker cp orion-chat_orion-chat_1:/data/orion.db ./backup/orion.db.$(date +%Y%m%d)
```

### 数据恢复

```bash
# 停止服务
docker-compose down

# 恢复数据
docker cp ./backup/orion.db.20260322 orion-chat_orion-chat_1:/data/orion.db

# 重启服务
docker-compose up -d
```

### 使用 Bind Mount（可选）

如果需要直接访问数据文件，可以使用 bind mount：

```yaml
services:
  orion-chat:
    volumes:
      - ./data:/data  # 使用本地目录
```

## 端口配置

### 默认端口

应用默认监听 `28080` 端口。

### 修改端口

方法 1：修改 `docker-compose.yml`
```yaml
services:
  orion-chat:
    ports:
      - "8080:28080"  # 宿主机:容器
    environment:
      - PORT=28080  # 容器内端口保持不变
```

方法 2：同时修改容器内端口
```yaml
services:
  orion-chat:
    ports:
      - "8080:8080"
    environment:
      - PORT=8080
```

### 使用反向代理

推荐使用 Nginx 或 Caddy 作为反向代理：

Nginx 配置示例：
```nginx
server {
    listen 80;
    server_name chat.example.com;

    location / {
        proxy_pass http://localhost:28080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}
```

## PWA 安装

### 移动设备安装

1. 在移动设备浏览器中访问应用
2. 点击浏览器菜单中的"添加到主屏幕"或"安装"
3. 应用将作为独立 App 安装到设备

### 支持的浏览器

- iOS Safari 11.3+
- Android Chrome 70+
- Android Firefox 58+
- Edge 17+

### 离线功能

PWA 支持离线访问：
- 静态资源缓存
- 离线时可查看历史对话
- 网络恢复后自动同步

## 故障排查

### 容器无法启动

检查日志：
```bash
docker-compose logs orion-chat
```

常见问题：
1. 端口被占用
   ```bash
   # 检查端口占用
   lsof -i :28080

   # 修改端口
   # 编辑 docker-compose.yml，修改 ports 配置
   ```

2. 权限问题
   ```bash
   # 确保数据目录有写权限
   sudo chown -R 1000:1000 ./data
   ```

3. 磁盘空间不足
   ```bash
   # 检查磁盘空间
   df -h

   # 清理 Docker 资源
   docker system prune -a
   ```

### 数据库错误

如果遇到数据库损坏：
```bash
# 停止服务
docker-compose down

# 备份当前数据库
docker cp orion-chat_orion-chat_1:/data/orion.db ./orion.db.broken

# 尝试修复
sqlite3 ./orion.db.broken "PRAGMA integrity_check;"

# 如果无法修复，从备份恢复
docker cp ./backup/orion.db.latest orion-chat_orion-chat_1:/data/orion.db

# 重启服务
docker-compose up -d
```

### 网络问题

如果无法访问应用：
```bash
# 检查容器状态
docker-compose ps

# 检查容器网络
docker inspect orion-chat_orion-chat_1 | grep IPAddress

# 测试容器内服务
docker-compose exec orion-chat wget -O- http://localhost:28080
```

### 查看详细日志

```bash
# 实时查看日志
docker-compose logs -f --tail=100

# 查看特定时间段日志
docker-compose logs --since 30m

# 导出日志到文件
docker-compose logs > orion-chat.log
```

## 升级和维护

### 升级到新版本

```bash
# 1. 备份数据
docker-compose exec orion-chat cp /data/orion.db /data/orion.db.backup

# 2. 停止服务
docker-compose down

# 3. 拉取最新代码
git pull origin master

# 4. 重新构建镜像
docker-compose build --no-cache

# 5. 启动新版本
docker-compose up -d

# 6. 验证服务
docker-compose logs -f
```

### 数据迁移

从旧版本迁移数据：
```bash
# 1. 导出旧版本数据
docker cp old-container:/data/orion.db ./orion.db.old

# 2. 停止新容器
docker-compose down

# 3. 复制数据到新容器
docker cp ./orion.db.old orion-chat_orion-chat_1:/data/orion.db

# 4. 重启服务
docker-compose up -d
```

### 定期维护

建议定期执行以下维护任务：

1. 数据备份（每周）
   ```bash
   docker-compose exec orion-chat cp /data/orion.db /data/orion.db.$(date +%Y%m%d)
   ```

2. 清理旧日志（每月）
   ```bash
   docker-compose logs --tail=0 > /dev/null
   ```

3. 更新镜像（按需）
   ```bash
   docker-compose pull
   docker-compose up -d
   ```

4. 检查磁盘空间
   ```bash
   docker system df
   ```

## 生产环境建议

### 安全配置

1. 使用 HTTPS
   - 配置 SSL 证书
   - 使用 Let's Encrypt 自动续期

2. 限制访问
   - 配置防火墙规则
   - 使用 VPN 或内网访问

3. 定期备份
   - 自动化备份脚本
   - 异地备份存储

### 性能优化

1. 资源限制
   ```yaml
   services:
     orion-chat:
       deploy:
         resources:
           limits:
             cpus: '1'
             memory: 512M
           reservations:
             memory: 256M
   ```

2. 日志轮转
   ```yaml
   services:
     orion-chat:
       logging:
         driver: "json-file"
         options:
           max-size: "10m"
           max-file: "3"
   ```

### 监控

使用 Docker 健康检查：
```yaml
services:
  orion-chat:
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:28080"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

## 常见问题

**Q: 如何修改数据库位置？**
A: 修改 `docker-compose.yml` 中的 `DATABASE_PATH` 环境变量和 volume 映射。

**Q: 可以在同一台服务器上运行多个实例吗？**
A: 可以，但需要修改端口和 volume 名称以避免冲突。

**Q: 如何重置所有数据？**
A: 执行 `docker-compose down -v` 会删除所有数据，然后重新启动。

**Q: 支持集群部署吗？**
A: 当前版本使用 SQLite，不支持多实例共享数据库。如需集群部署，需要迁移到 PostgreSQL 等支持并发的数据库。

## 获取帮助

- GitHub Issues: https://github.com/yourusername/orion-chat-rs/issues
- 文档: https://github.com/yourusername/orion-chat-rs/docs
- 社区讨论: https://github.com/yourusername/orion-chat-rs/discussions
