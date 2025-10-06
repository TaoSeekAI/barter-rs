use crate::{
    engine::state::{
        EngineState,
        instrument::{data::InstrumentDataState, filter::InstrumentFilter},
    },
    strategy::{
        algo::AlgoStrategy,
        close_positions::{ClosePositionsStrategy, close_open_positions_with_market_orders},
        on_disconnect::OnDisconnectStrategy,
        on_trading_disabled::OnTradingDisabled,
    },
    Engine,
};
use barter_data::event::{DataKind, MarketEvent};
use barter_execution::{
    order::{
        id::{ClientOrderId, StrategyId},
        request::{OrderRequestCancel, OrderRequestOpen},
        Order, OrderKind, RequestCancel, RequestOpen, Side,
    },
    AccountEvent,
};
use barter_instrument::{
    asset::AssetIndex, exchange::{ExchangeId, ExchangeIndex}, instrument::InstrumentIndex,
};
use derive_more::Constructor;
use rust_decimal::{Decimal, prelude::FromPrimitive};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fmt::Debug, marker::PhantomData};
use crate::Processor;

/// 交易量突破策略配置
///
/// 该策略监控交易量的异常变化，当交易量突然激增时产生交易信号。
///
/// # 参数说明
/// - `lookback_period`: 回顾周期，用于计算基准交易量（默认30个周期）
/// - `volume_surge_multiplier`: 交易量激增倍数，超过此倍数触发信号（默认3.0倍）
/// - `min_baseline_volume`: 最小基准交易量，避免在低流动性时误触发（默认1000）
/// - `entry_percentage`: 入场仓位百分比（默认5%）
/// - `stop_loss_percentage`: 止损百分比（默认2%）
/// - `take_profit_percentage`: 止盈百分比（默认5%）
/// - `cooldown_periods`: 冷却期，避免频繁交易（默认10个周期）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VolumeBreakoutConfig {
    /// 回顾周期长度（用于计算平均交易量）
    pub lookback_period: usize,
    /// 交易量激增倍数阈值
    pub volume_surge_multiplier: f64,
    /// 最小基准交易量（避免低流动性误触发）
    pub min_baseline_volume: f64,
    /// 入场仓位百分比
    pub entry_percentage: Decimal,
    /// 止损百分比
    pub stop_loss_percentage: Decimal,
    /// 止盈百分比
    pub take_profit_percentage: Decimal,
    /// 冷却期（避免频繁交易）
    pub cooldown_periods: usize,
}

impl Default for VolumeBreakoutConfig {
    fn default() -> Self {
        Self {
            lookback_period: 30,
            volume_surge_multiplier: 3.0,
            min_baseline_volume: 1000.0,
            entry_percentage: Decimal::from_f64(0.05).unwrap(), // 5%
            stop_loss_percentage: Decimal::from_f64(0.02).unwrap(), // 2%
            take_profit_percentage: Decimal::from_f64(0.05).unwrap(), // 5%
            cooldown_periods: 10,
        }
    }
}

/// 交易量数据点
#[derive(Debug, Clone, Copy, Constructor)]
pub struct VolumeDataPoint {
    /// 交易量
    pub volume: f64,
    /// 价格
    pub price: f64,
    /// 时间戳
    pub timestamp: i64,
}

/// 仓位信息
#[derive(Debug, Clone, Constructor)]
pub struct PositionInfo {
    /// 入场价格
    pub entry_price: Decimal,
    /// 止损价格
    pub stop_loss: Decimal,
    /// 止盈价格
    pub take_profit: Decimal,
    /// 仓位方向
    pub side: Side,
    /// 入场时间
    pub entry_time: i64,
}

/// 交易量监控数据
#[derive(Debug, Clone, Constructor)]
pub struct VolumeMonitor {
    /// 历史交易量数据
    pub volume_history: VecDeque<VolumeDataPoint>,
    /// 当前仓位
    pub position: Option<PositionInfo>,
    /// 冷却期计数器
    pub cooldown_counter: usize,
    /// 最后交易信号时间
    pub last_signal_time: i64,
}

impl VolumeMonitor {
    pub fn new() -> Self {
        Self {
            volume_history: VecDeque::new(),
            position: None,
            cooldown_counter: 0,
            last_signal_time: 0,
        }
    }

    /// 添加新的交易量数据点
    pub fn add_volume_data(&mut self, data_point: VolumeDataPoint, lookback_period: usize) {
        self.volume_history.push_back(data_point);

        // 保持固定长度的历史数据
        while self.volume_history.len() > lookback_period {
            self.volume_history.pop_front();
        }
    }

    /// 计算平均交易量
    pub fn calculate_average_volume(&self) -> f64 {
        if self.volume_history.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.volume_history.iter().map(|d| d.volume).sum();
        sum / self.volume_history.len() as f64
    }

    /// 检测交易量突破
    pub fn detect_volume_surge(&self, config: &VolumeBreakoutConfig) -> Option<VolumeSurgeSignal> {
        // 需要足够的历史数据
        if self.volume_history.len() < config.lookback_period {
            return None;
        }

        // 在冷却期内不产生信号
        if self.cooldown_counter > 0 {
            return None;
        }

        // 已有仓位时不产生新信号
        if self.position.is_some() {
            return None;
        }

        // 获取最新交易量
        let latest_volume = self.volume_history.back()?.volume;

        // 计算基准交易量（排除最新的数据点）
        let baseline_volumes: Vec<f64> = self.volume_history
            .iter()
            .rev()
            .skip(1)
            .take(config.lookback_period - 1)
            .map(|d| d.volume)
            .collect();

        if baseline_volumes.is_empty() {
            return None;
        }

        let avg_baseline_volume = baseline_volumes.iter().sum::<f64>() / baseline_volumes.len() as f64;

        // 检查基准交易量是否满足最小要求
        if avg_baseline_volume < config.min_baseline_volume {
            return None;
        }

        // 检测交易量激增
        let volume_ratio = latest_volume / avg_baseline_volume;
        if volume_ratio >= config.volume_surge_multiplier {
            // 判断方向：看价格变化
            let latest_price = self.volume_history.back()?.price;
            let prev_price = self.volume_history.iter().rev().nth(1)?.price;

            let side = if latest_price > prev_price {
                Side::Buy
            } else {
                Side::Sell
            };

            return Some(VolumeSurgeSignal {
                volume_ratio,
                avg_baseline_volume,
                current_volume: latest_volume,
                current_price: latest_price,
                side,
                timestamp: self.volume_history.back()?.timestamp,
            });
        }

        None
    }

    /// 更新冷却期
    pub fn update_cooldown(&mut self) {
        if self.cooldown_counter > 0 {
            self.cooldown_counter -= 1;
        }
    }

    /// 开始冷却期
    pub fn start_cooldown(&mut self, cooldown_periods: usize) {
        self.cooldown_counter = cooldown_periods;
    }
}

/// 交易量激增信号
#[derive(Debug, Clone, Constructor)]
pub struct VolumeSurgeSignal {
    /// 交易量比率
    pub volume_ratio: f64,
    /// 平均基准交易量
    pub avg_baseline_volume: f64,
    /// 当前交易量
    pub current_volume: f64,
    /// 当前价格
    pub current_price: f64,
    /// 建议方向
    pub side: Side,
    /// 信号时间
    pub timestamp: i64,
}

/// 扩展的仪器市场数据，包含交易量监控
#[derive(Debug, Clone, Deserialize, Serialize, Constructor)]
pub struct VolumeBreakoutInstrumentData {
    /// 订单簿L1数据
    pub l1: barter_data::subscription::book::OrderBookL1,
    /// 最后交易价格
    pub last_traded_price: Option<crate::Timed<Decimal>>,
    /// 交易量监控器
    #[serde(skip)]
    pub volume_monitor: VolumeMonitor,
}

impl Default for VolumeBreakoutInstrumentData {
    fn default() -> Self {
        Self {
            l1: Default::default(),
            last_traded_price: None,
            volume_monitor: VolumeMonitor::new(),
        }
    }
}

impl InstrumentDataState for VolumeBreakoutInstrumentData {
    type MarketEventKind = DataKind;

    fn price(&self) -> Option<Decimal> {
        self.l1
            .volume_weighed_mid_price()
            .or(self.last_traded_price.as_ref().map(|timed| timed.value))
    }
}

impl<InstrumentKey> Processor<&MarketEvent<InstrumentKey, DataKind>>
    for VolumeBreakoutInstrumentData
{
    type Audit = ();

    fn process(&mut self, event: &MarketEvent<InstrumentKey, DataKind>) -> Self::Audit {
        match &event.kind {
            DataKind::Trade(trade) => {
                // 更新最后交易价格
                if self
                    .last_traded_price
                    .as_ref()
                    .is_none_or(|price| price.time < event.time_exchange)
                    && let Some(price) = Decimal::from_f64(trade.price)
                {
                    self.last_traded_price
                        .replace(crate::Timed::new(price, event.time_exchange));
                }

                // 添加交易量数据
                let volume_point = VolumeDataPoint::new(
                    trade.quantity,
                    trade.price,
                    event.time_exchange.timestamp(),
                );
                self.volume_monitor.add_volume_data(volume_point, 30);
            }
            DataKind::OrderBookL1(l1) => {
                if self.l1.last_update_time < event.time_exchange {
                    self.l1 = l1.clone();
                }
            }
            _ => {}
        }
    }
}

impl<ExchangeKey, AssetKey, InstrumentKey>
    Processor<&AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>
    for VolumeBreakoutInstrumentData
{
    type Audit = ();
    fn process(&mut self, _: &AccountEvent<ExchangeKey, AssetKey, InstrumentKey>) -> Self::Audit {}
}

impl<ExchangeKey, InstrumentKey>
    crate::engine::state::order::in_flight_recorder::InFlightRequestRecorder<
        ExchangeKey,
        InstrumentKey,
    > for VolumeBreakoutInstrumentData
{
    fn record_cancel(&mut self, _: &OrderRequestCancel<ExchangeKey, InstrumentKey>) {}
    fn record_open(&mut self, _: &OrderRequestOpen<ExchangeKey, InstrumentKey>) {}
}

/// 交易量突破策略
///
/// 该策略通过监控交易量的异常变化来识别潜在的交易机会。
/// 当交易量突然激增到平均水平的数倍时，通常意味着市场情绪的重大变化。
#[derive(Debug, Clone)]
pub struct VolumeBreakoutStrategy {
    pub id: StrategyId,
    pub config: VolumeBreakoutConfig,
}

impl VolumeBreakoutStrategy {
    pub fn new(config: VolumeBreakoutConfig) -> Self {
        Self {
            id: StrategyId::new("volume_breakout"),
            config,
        }
    }

    pub fn with_id(id: StrategyId, config: VolumeBreakoutConfig) -> Self {
        Self { id, config }
    }
}

impl Default for VolumeBreakoutStrategy {
    fn default() -> Self {
        Self::new(VolumeBreakoutConfig::default())
    }
}

impl<GlobalData> AlgoStrategy<ExchangeIndex, InstrumentIndex>
    for VolumeBreakoutStrategy
{
    type State = EngineState<GlobalData, VolumeBreakoutInstrumentData>;

    fn generate_algo_orders(
        &self,
        state: &Self::State,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>>,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>>,
    ) {
        let mut open_orders = Vec::new();
        let mut cancel_orders = Vec::new();

        // 遍历所有仪器
        for instrument in state.instruments.as_ref() {
            let instrument_data = state.instruments_data.get(instrument.index);

            // 更新冷却期
            instrument_data.volume_monitor.update_cooldown();

            // 检查是否需要平仓（止损或止盈）
            if let Some(position) = &instrument_data.volume_monitor.position {
                if let Some(current_price) = instrument_data.price() {
                    let should_close = match position.side {
                        Side::Buy => {
                            current_price <= position.stop_loss
                                || current_price >= position.take_profit
                        }
                        Side::Sell => {
                            current_price >= position.stop_loss
                                || current_price <= position.take_profit
                        }
                    };

                    if should_close {
                        // 平仓
                        let close_side = match position.side {
                            Side::Buy => Side::Sell,
                            Side::Sell => Side::Buy,
                        };

                        open_orders.push(OrderRequestOpen {
                            exchange: instrument.exchange,
                            instrument: instrument.index,
                            strategy_id: self.id.clone(),
                            cid: ClientOrderId::random(),
                            order: RequestOpen {
                                side: close_side,
                                order: OrderKind::Market,
                            },
                        });

                        // 清除仓位并开始冷却期
                        instrument_data.volume_monitor.position = None;
                        instrument_data.volume_monitor.start_cooldown(self.config.cooldown_periods);
                        continue;
                    }
                }
            }

            // 检测交易量突破信号
            if let Some(signal) = instrument_data.volume_monitor.detect_volume_surge(&self.config) {
                if let Some(current_price) = instrument_data.price() {
                    // 计算止损和止盈价格
                    let (stop_loss, take_profit) = match signal.side {
                        Side::Buy => {
                            let stop_loss = current_price * (Decimal::ONE - self.config.stop_loss_percentage);
                            let take_profit = current_price * (Decimal::ONE + self.config.take_profit_percentage);
                            (stop_loss, take_profit)
                        }
                        Side::Sell => {
                            let stop_loss = current_price * (Decimal::ONE + self.config.stop_loss_percentage);
                            let take_profit = current_price * (Decimal::ONE - self.config.take_profit_percentage);
                            (stop_loss, take_profit)
                        }
                    };

                    // 生成开仓订单
                    open_orders.push(OrderRequestOpen {
                        exchange: instrument.exchange,
                        instrument: instrument.index,
                        strategy_id: self.id.clone(),
                        cid: ClientOrderId::random(),
                        order: RequestOpen {
                            side: signal.side,
                            order: OrderKind::Market,
                        },
                    });

                    // 记录仓位信息
                    instrument_data.volume_monitor.position = Some(PositionInfo {
                        entry_price: current_price,
                        stop_loss,
                        take_profit,
                        side: signal.side,
                        entry_time: signal.timestamp,
                    });
                }
            }
        }

        (cancel_orders, open_orders)
    }
}

impl<GlobalData> ClosePositionsStrategy for VolumeBreakoutStrategy
where
    VolumeBreakoutInstrumentData: InstrumentDataState,
{
    type State = EngineState<GlobalData, VolumeBreakoutInstrumentData>;

    fn close_positions_requests<'a>(
        &'a self,
        state: &'a Self::State,
        filter: &'a InstrumentFilter,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>> + 'a,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>> + 'a,
    )
    where
        ExchangeIndex: 'a,
        AssetIndex: 'a,
        InstrumentIndex: 'a,
    {
        close_open_positions_with_market_orders(&self.id, state, filter, |_| {
            ClientOrderId::random()
        })
    }
}

impl<Clock, ExecutionTxs, Risk> OnDisconnectStrategy<Clock, EngineState<(), VolumeBreakoutInstrumentData>, ExecutionTxs, Risk>
    for VolumeBreakoutStrategy
{
    type OnDisconnect = ();

    fn on_disconnect(
        _: &mut Engine<Clock, EngineState<(), VolumeBreakoutInstrumentData>, ExecutionTxs, Self, Risk>,
        _: ExchangeId,
    ) -> Self::OnDisconnect {
    }
}

impl<Clock, ExecutionTxs, Risk> OnTradingDisabled<Clock, EngineState<(), VolumeBreakoutInstrumentData>, ExecutionTxs, Risk>
    for VolumeBreakoutStrategy
{
    type OnTradingDisabled = ();

    fn on_trading_disabled(
        _: &mut Engine<Clock, EngineState<(), VolumeBreakoutInstrumentData>, ExecutionTxs, Self, Risk>,
    ) -> Self::OnTradingDisabled {
    }
}
