# 本地部署指南 (Local Deployment Guide)

## 快速开始

### 1. 构建Docker镜像

由于GitHub Token暂时无法使用，我们先在本地构建镜像：

```bash
# 构建本地镜像
docker build -t barter-rs:local .
```

### 2. 使用Docker Compose启动服务

```bash
# 使用本地配置启动
docker-compose -f docker-compose.local.yml up -d

# 查看日志
docker-compose -f docker-compose.local.yml logs -f barter-strategy

# 停止服务
docker-compose -f docker-compose.local.yml down
```

## 配置说明

### Binance测试网配置

系统已配置为使用Binance测试网，API密钥已经集成：
- API Key: `Wt104kkmijNETENuP4hpJfnGLZxjcjhpH7cYVckIvGAeeI6vxd24Vf8zGKs4lznM`
- API Secret: `rqh7ifHSsAJOGdoyiRKNzSINqchHCMx5nJ2FtaI9OcMgqXIJOHhpHIpoGXaZRwOu`
- 测试网URL: https://testnet.binance.vision

### 交易配置

默认配置：
- 交易模式: Paper Trading (模拟交易)
- 交易对: BTCUSDT
- 时间间隔: 1分钟
- 最大持仓: 1000 USDT
- 每笔交易风险: 2%

## 监控

系统包含完整的监控栈：
- Prometheus: http://localhost:9091
- Grafana: http://localhost:3000 (admin/admin)

## 故障排除

### 如果Docker构建失败

1. 确保所有workspace成员的Cargo.toml存在
2. 如果缺少Cargo.lock，构建过程会自动生成

### 如果服务无法启动

1. 检查端口占用：
```bash
lsof -i:8080
lsof -i:9090
lsof -i:3000
```

2. 查看详细日志：
```bash
docker-compose -f docker-compose.local.yml logs --tail=100 barter-strategy
```

## CI/CD修复说明

GitHub Actions工作流已修复：
- 更新了所有过时的action版本
- 修复了deprecated的upload-artifact (v3 -> v4)
- 添加了简化的build-and-push工作流

待推送到GitHub后，CI/CD将自动：
1. 构建Docker镜像
2. 推送到GitHub Container Registry (ghcr.io)
3. 支持多架构 (linux/amd64, linux/arm64)

## 下一步

1. 获取有效的GitHub Token并推送代码
2. 等待CI/CD构建完成
3. 使用官方镜像替换本地构建：
   ```yaml
   image: ghcr.io/taoseekai/barter-rs:latest
   ```

## 项目结构

```
barter-rs/
├── barter-strategy/      # 核心交易策略实现
│   ├── src/
│   │   ├── signal.rs    # 信号采集层
│   │   ├── processor.rs # 信号处理层
│   │   ├── judgment.rs  # 信号判断层
│   │   ├── action.rs    # 策略动作层
│   │   └── execution.rs # 策略执行层
├── docker-compose.local.yml  # 本地部署配置
├── docker-compose.testnet.yml # 测试网配置
├── Dockerfile               # Docker构建文件
└── .github/workflows/       # CI/CD配置
    ├── docker.yml          # 完整的Docker构建流程
    └── build-and-push.yml  # 简化的构建推送流程
```