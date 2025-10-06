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
    Engine, Processor,
};
use barter_data::event::{DataKind, MarketEvent};
use barter_execution::{
    order::{
        id::{ClientOrderId, StrategyId},
        request::{OrderRequestCancel, OrderRequestOpen},
        OrderKind, RequestOpen, Side,
    },
    AccountEvent,
};
use barter_instrument::{
    asset::AssetIndex, exchange::{ExchangeId, ExchangeIndex}, instrument::InstrumentIndex,
};
use derive_more::Constructor;
use rust_decimal::{Decimal, prelude::FromPrimitive};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fmt::Debug};

/// Volume Breakout Strategy Configuration
///
/// Monitors trading volume for sudden surges that may indicate market opportunities.
///
/// # Parameters
/// - `lookback_period`: Historical periods to calculate baseline volume (default: 30)
/// - `volume_surge_multiplier`: Threshold multiplier for volume surge detection (default: 3.0x)
/// - `min_baseline_volume`: Minimum baseline volume to avoid low liquidity triggers (default: 1000)
/// - `stop_loss_percentage`: Stop loss percentage (default: 2%)
/// - `take_profit_percentage`: Take profit percentage (default: 5%)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VolumeBreakoutConfig {
    /// Lookback period length for calculating average volume
    pub lookback_period: usize,
    /// Volume surge multiplier threshold
    pub volume_surge_multiplier: f64,
    /// Minimum baseline volume (avoids low liquidity false triggers)
    pub min_baseline_volume: f64,
    /// Stop loss percentage
    pub stop_loss_percentage: Decimal,
    /// Take profit percentage
    pub take_profit_percentage: Decimal,
}

impl Default for VolumeBreakoutConfig {
    fn default() -> Self {
        Self {
            lookback_period: 30,
            volume_surge_multiplier: 3.0,
            min_baseline_volume: 1000.0,
            stop_loss_percentage: Decimal::from_f64(0.02).unwrap(), // 2%
            take_profit_percentage: Decimal::from_f64(0.05).unwrap(), // 5%
        }
    }
}

/// Volume data point
#[derive(Debug, Clone, Copy, Constructor)]
pub struct VolumeDataPoint {
    /// Trading volume
    pub volume: f64,
    /// Price
    pub price: f64,
    /// Timestamp
    pub timestamp: i64,
}

/// Volume monitoring data
#[derive(Debug, Clone, Constructor)]
pub struct VolumeMonitor {
    /// Historical volume data
    pub volume_history: VecDeque<VolumeDataPoint>,
}

impl VolumeMonitor {
    pub fn new() -> Self {
        Self {
            volume_history: VecDeque::new(),
        }
    }

    /// Add new volume data point
    pub fn add_volume_data(&mut self, data_point: VolumeDataPoint, lookback_period: usize) {
        self.volume_history.push_back(data_point);

        // Maintain fixed length history
        while self.volume_history.len() > lookback_period {
            self.volume_history.pop_front();
        }
    }

    /// Calculate average volume
    pub fn calculate_average_volume(&self) -> f64 {
        if self.volume_history.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.volume_history.iter().map(|d| d.volume).sum();
        sum / self.volume_history.len() as f64
    }

    /// Detect volume surge
    pub fn detect_volume_surge(&self, config: &VolumeBreakoutConfig) -> Option<VolumeSurgeSignal> {
        // Need enough historical data
        if self.volume_history.len() < config.lookback_period {
            return None;
        }

        // Get latest volume
        let latest_volume = self.volume_history.back()?.volume;

        // Calculate baseline volume (excluding latest data point)
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

        // Check if baseline volume meets minimum requirement
        if avg_baseline_volume < config.min_baseline_volume {
            return None;
        }

        // Detect volume surge
        let volume_ratio = latest_volume / avg_baseline_volume;
        if volume_ratio >= config.volume_surge_multiplier {
            // Determine direction based on price change
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
}

impl Default for VolumeMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Volume surge signal
#[derive(Debug, Clone, Constructor)]
pub struct VolumeSurgeSignal {
    /// Volume ratio
    pub volume_ratio: f64,
    /// Average baseline volume
    pub avg_baseline_volume: f64,
    /// Current volume
    pub current_volume: f64,
    /// Current price
    pub current_price: f64,
    /// Suggested direction
    pub side: Side,
    /// Signal timestamp
    pub timestamp: i64,
}

/// Extended instrument market data with volume monitoring
#[derive(Debug, Clone, Deserialize, Serialize, Constructor)]
pub struct VolumeBreakoutInstrumentData {
    /// L1 order book data
    pub l1: barter_data::subscription::book::OrderBookL1,
    /// Last traded price
    pub last_traded_price: Option<crate::Timed<Decimal>>,
    /// Volume monitor
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
                // Update last traded price
                if self
                    .last_traded_price
                    .as_ref()
                    .is_none_or(|price| price.time < event.time_exchange)
                    && let Some(price) = Decimal::from_f64(trade.price)
                {
                    self.last_traded_price
                        .replace(crate::Timed::new(price, event.time_exchange));
                }

                // Add volume data
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
    fn record_in_flight_cancel(&mut self, _: &OrderRequestCancel<ExchangeKey, InstrumentKey>) {}
    fn record_in_flight_open(&mut self, _: &OrderRequestOpen<ExchangeKey, InstrumentKey>) {}
}

/// Volume Breakout Strategy
///
/// This strategy identifies trading opportunities by monitoring abnormal volume changes.
/// When volume suddenly surges to several times the average level, it typically indicates
/// significant changes in market sentiment.
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
        let cancel_orders = Vec::new();

        // Iterate through all instruments
        for instrument in state.instruments.as_ref() {
            let instrument_data = state.instruments_data.get(instrument.index);

            // Detect volume breakthrough signal
            if let Some(_signal) = instrument_data.volume_monitor.detect_volume_surge(&self.config) {
                if let Some(_current_price) = instrument_data.price() {
                    // Generate opening order (simplified - real implementation would include
                    // position management, stop loss, take profit, etc.)
                    open_orders.push(OrderRequestOpen {
                        exchange: instrument.exchange,
                        instrument: instrument.index,
                        strategy_id: self.id.clone(),
                        cid: ClientOrderId::random(),
                        order: RequestOpen {
                            side: _signal.side,
                            order: OrderKind::Market,
                        },
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

impl<Clock, GlobalData, ExecutionTxs, Risk> OnDisconnectStrategy<Clock, EngineState<GlobalData, VolumeBreakoutInstrumentData>, ExecutionTxs, Risk>
    for VolumeBreakoutStrategy
{
    type OnDisconnect = ();

    fn on_disconnect(
        _: &mut Engine<Clock, EngineState<GlobalData, VolumeBreakoutInstrumentData>, ExecutionTxs, Self, Risk>,
        _: ExchangeId,
    ) -> Self::OnDisconnect {
    }
}

impl<Clock, GlobalData, ExecutionTxs, Risk> OnTradingDisabled<Clock, EngineState<GlobalData, VolumeBreakoutInstrumentData>, ExecutionTxs, Risk>
    for VolumeBreakoutStrategy
{
    type OnTradingDisabled = ();

    fn on_trading_disabled(
        _: &mut Engine<Clock, EngineState<GlobalData, VolumeBreakoutInstrumentData>, ExecutionTxs, Self, Risk>,
    ) -> Self::OnTradingDisabled {
    }
}
