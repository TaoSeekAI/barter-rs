use barter::{
    backtest::clock::LiveClock,
    engine::{
        builder::{EngineFeedMode, SystemArgs, SystemBuilder},
        state::{data::DefaultGlobalData, EngineEvent},
        AuditMode, TradingState,
    },
    risk::DefaultRiskManager,
    strategy::volume_breakout::{VolumeBreakoutConfig, VolumeBreakoutInstrumentData, VolumeBreakoutStrategy},
};
use barter_data::{
    exchange::ExchangeId,
    streams::{consumer::MarketStreamEvent, init_multi_exchange_market_stream},
    subscription::{SubKind, Subscription},
};
use barter_execution::client::{ExecutionClient, MockExecutionBuilder};
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex},
    exchange::ExchangeIndex,
    index::IndexedInstruments,
    instrument::{kind::InstrumentKind, name::InstrumentNameExchange, Instrument},
};
use rust_decimal_macros::dec;
use std::{collections::HashMap, time::Duration};
use tokio_stream::StreamExt;
use tracing::{debug, info};

/// 交易量突破策略示例
///
/// 该示例展示如何使用 VolumeBreakoutStrategy 来监控交易量异常并产生交易信号。
///
/// 策略逻辑：
/// 1. 监控过去30个周期的交易量
/// 2. 当交易量突然达到平均值的3倍时触发信号
/// 3. 根据价格方向（上涨/下跌）决定买入或卖出
/// 4. 设置2%止损，5%止盈
/// 5. 交易后进入10个周期的冷却期
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    init_logging();

    info!("=== Volume Breakout Strategy Example ===");
    info!("This strategy detects sudden volume surges and generates trading signals");

    // 配置策略参数
    let config = VolumeBreakoutConfig {
        lookback_period: 30,           // 回顾30个周期
        volume_surge_multiplier: 3.0,  // 交易量激增3倍
        min_baseline_volume: 1000.0,   // 最小基准交易量1000
        entry_percentage: dec!(0.05),  // 5%仓位
        stop_loss_percentage: dec!(0.02), // 2%止损
        take_profit_percentage: dec!(0.05), // 5%止盈
        cooldown_periods: 10,          // 10个周期冷却
    };

    info!("Strategy Configuration:");
    info!("  - Lookback Period: {} periods", config.lookback_period);
    info!("  - Volume Surge Multiplier: {}x", config.volume_surge_multiplier);
    info!("  - Min Baseline Volume: {}", config.min_baseline_volume);
    info!("  - Entry Position: {}%", config.entry_percentage * dec!(100));
    info!("  - Stop Loss: {}%", config.stop_loss_percentage * dec!(100));
    info!("  - Take Profit: {}%", config.take_profit_percentage * dec!(100));
    info!("  - Cooldown: {} periods", config.cooldown_periods);

    // 定义交易工具
    let instruments = vec![
        Instrument::new(
            InstrumentNameExchange {
                exchange: ExchangeId::BinanceFuturesUsd,
                name: "btc_usdt_perp".to_string(),
            },
            InstrumentKind::Perpetual,
            AssetNameExchange {
                exchange: ExchangeId::BinanceFuturesUsd,
                name: "btc".to_string(),
            },
            AssetNameExchange {
                exchange: ExchangeId::BinanceFuturesUsd,
                name: "usdt".to_string(),
            },
        ),
        Instrument::new(
            InstrumentNameExchange {
                exchange: ExchangeId::BinanceFuturesUsd,
                name: "eth_usdt_perp".to_string(),
            },
            InstrumentKind::Perpetual,
            AssetNameExchange {
                exchange: ExchangeId::BinanceFuturesUsd,
                name: "eth".to_string(),
            },
            AssetNameExchange {
                exchange: ExchangeId::BinanceFuturesUsd,
                name: "usdt".to_string(),
            },
        ),
    ];

    // 构建索引化工具
    let instruments = IndexedInstruments::new(instruments);
    info!("Monitoring instruments: {:?}", instruments.as_ref().iter().map(|i| &i.name_exchange.name).collect::<Vec<_>>());

    // 初始化市场数据流
    let subscriptions = instruments
        .as_ref()
        .iter()
        .flat_map(|instrument| {
            vec![
                Subscription::new(
                    instrument.exchange,
                    (instrument.name_exchange.clone(), SubKind::PublicTrades),
                ),
                Subscription::new(
                    instrument.exchange,
                    (instrument.name_exchange.clone(), SubKind::OrderBooksL1),
                ),
            ]
        })
        .collect();

    let market_stream = init_multi_exchange_market_stream(&instruments, subscriptions)
        .await?
        .map(|event| match event {
            MarketStreamEvent::Event(event) => EngineEvent::Market(event),
            MarketStreamEvent::Unhealthy(unhealthy) => EngineEvent::MarketUnhealthy(unhealthy),
        });

    // 配置模拟执行客户端
    let mut execution_clients = HashMap::new();

    // Binance Futures 模拟客户端
    execution_clients.insert(
        ExchangeIndex::from(ExchangeId::BinanceFuturesUsd),
        MockExecutionBuilder::default()
            .latency(Duration::from_millis(10))
            .balances({
                let mut balances = HashMap::new();
                balances.insert(
                    AssetIndex::from(AssetNameExchange {
                        exchange: ExchangeId::BinanceFuturesUsd,
                        name: "usdt".to_string(),
                    }),
                    dec!(10000.0), // 初始资金 10000 USDT
                );
                balances
            })
            .build_client()?,
    );

    info!("Initialized mock execution clients with 10000 USDT");

    // 构建系统参数
    let args = SystemArgs::new(
        &instruments,
        execution_clients,
        LiveClock,
        VolumeBreakoutStrategy::new(config),
        DefaultRiskManager::default(),
        market_stream,
    );

    // 构建并初始化交易系统
    let mut system = SystemBuilder::new(args)
        .engine_feed_mode(EngineFeedMode::Iterator)
        .audit_mode(AuditMode::Enabled)
        .trading_state(TradingState::Disabled)
        .build::<EngineEvent, DefaultGlobalData, VolumeBreakoutInstrumentData>()?
        .init_with_runtime(tokio::runtime::Handle::current())
        .await?;

    info!("Trading system initialized");

    // 获取审计流
    let audit_rx = system.audit_rx.take().unwrap();
    let audit_task = tokio::spawn(async move {
        let mut audit_stream = audit_rx.into_stream();
        while let Some(audit) = audit_stream.next().await {
            match &audit.event {
                barter::engine::audit::EngineAudit::Order(order_audit) => {
                    info!("📋 Order Event: {:?}", order_audit);
                }
                barter::engine::audit::EngineAudit::Fill(fill_audit) => {
                    info!("✅ Fill Event: {:?}", fill_audit);
                }
                barter::engine::audit::EngineAudit::Position(position_audit) => {
                    info!("📊 Position Event: {:?}", position_audit);
                }
                barter::engine::audit::EngineAudit::Balance(balance_audit) => {
                    debug!("💰 Balance Event: {:?}", balance_audit);
                }
                barter::engine::audit::EngineAudit::Shutdown(_) => {
                    info!("System shutting down...");
                    break;
                }
                _ => {}
            }
        }
        audit_stream
    });

    // 启用交易
    info!("Enabling trading...");
    system.trading_state(TradingState::Enabled);

    // 运行策略
    info!("Strategy is now running. Monitoring for volume surges...");
    info!("Will run for 60 seconds...");

    tokio::time::sleep(Duration::from_secs(60)).await;

    // 关闭系统前先平仓
    info!("Closing all positions...");
    system.cancel_orders(barter::engine::state::instrument::filter::InstrumentFilter::None);
    system.close_positions(barter::engine::state::instrument::filter::InstrumentFilter::None);

    // 等待订单执行
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 关闭系统
    info!("Shutting down system...");
    let (engine, _shutdown_audit) = system.shutdown().await?;
    let _audit_stream = audit_task.await?;

    // 生成交易总结
    info!("Generating trading summary...");
    let trading_summary = engine
        .trading_summary_generator(dec!(0.05))
        .generate(barter::statistic::summary::pnl::unreal::PnlReturnPeriod::Daily);

    // 打印交易总结
    println!("\n{}", "=".repeat(80));
    println!("TRADING SUMMARY");
    println!("{}", "=".repeat(80));
    trading_summary.print_summary();
    println!("{}", "=".repeat(80));

    Ok(())
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_max_level(tracing::Level::INFO)
        .init();
}
