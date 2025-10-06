# Barter 策略总览

## 现有策略

### 1. DefaultStrategy (默认策略)
- **文件位置**: `barter/src/strategy/mod.rs`
- **用途**: 演示用途，不进行任何实际交易
- **特点**:
  - 不生成算法订单
  - 使用市价单平仓
  - 断线和禁用交易时无操作

### 2. VolumeBreakoutStrategy (交易量突破策略) ⭐ 新增
- **文件位置**: `barter/src/strategy/volume_breakout.rs`
- **用途**: 监控交易量异常，捕捉市场情绪突变
- **核心逻辑**:
  ```
  交易量激增 = 当前交易量 / 平均交易量 ≥ 3倍
  ↓
  价格上涨 → 买入 | 价格下跌 → 卖出
  ↓
  设置止损(2%) 和 止盈(5%)
  ↓
  触发后进入冷却期(10个周期)
  ```

## 策略对比

| 特性 | DefaultStrategy | VolumeBreakoutStrategy |
|------|----------------|----------------------|
| 主动交易 | ❌ | ✅ |
| 风险管理 | 基础 | 完整(止损/止盈) |
| 信号生成 | 无 | 交易量突破检测 |
| 参数可配置 | ❌ | ✅ |
| 适用场景 | 演示 | 实际交易 |
| 冷却期机制 | ❌ | ✅ |

## 快速开始

### 使用 VolumeBreakoutStrategy

```rust
use barter::strategy::volume_breakout::{
    VolumeBreakoutConfig,
    VolumeBreakoutStrategy,
    VolumeBreakoutInstrumentData,
};

// 创建策略
let strategy = VolumeBreakoutStrategy::new(VolumeBreakoutConfig {
    lookback_period: 30,
    volume_surge_multiplier: 3.0,
    min_baseline_volume: 1000.0,
    entry_percentage: dec!(0.05),
    stop_loss_percentage: dec!(0.02),
    take_profit_percentage: dec!(0.05),
    cooldown_periods: 10,
});

// 在系统中使用
let system = SystemBuilder::new(args)
    .build::<EngineEvent, DefaultGlobalData, VolumeBreakoutInstrumentData>()?;
```

### 运行示例

```bash
# 运行交易量突破策略示例
cargo run --example volume_breakout_strategy

# 查看实时日志
cargo run --example volume_breakout_strategy 2>&1 | grep "Volume"
```

## 策略选择指南

### 何时使用 DefaultStrategy
- 学习 Barter 框架
- 开发自定义策略的起点
- 简单的回测验证

### 何时使用 VolumeBreakoutStrategy
- 需要监控交易量异常
- 捕捉市场突发行情
- 自动化交易执行
- 24/7 加密货币交易

## 开发自定义策略

基于现有策略模板，你可以：

1. **复制模板**
   ```bash
   cp barter/src/strategy/volume_breakout.rs barter/src/strategy/my_strategy.rs
   ```

2. **实现必要的 Trait**
   - `AlgoStrategy`: 生成交易订单
   - `ClosePositionsStrategy`: 平仓逻辑
   - `OnDisconnectStrategy`: 断线处理
   - `OnTradingDisabled`: 禁用交易处理

3. **定义自定义数据**
   ```rust
   #[derive(Debug, Clone)]
   pub struct MyInstrumentData {
       pub l1: OrderBookL1,
       pub my_custom_indicator: f64,
       // 添加你需要的数据
   }
   ```

4. **注册策略**
   在 `barter/src/strategy/mod.rs` 中添加：
   ```rust
   pub mod my_strategy;
   ```

## 更多资源

- 📖 [交易量突破策略详细文档](./volume-breakout-strategy.md)
- 📖 [项目整体分析](./project-analysis.md)
- 💻 [示例代码](../barter/examples/)
- 🔗 [Barter 官方文档](https://docs.rs/barter/)

---

**提示**: 所有策略在实盘使用前都应该经过充分的回测和小额测试。