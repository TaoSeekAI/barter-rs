-- Barter Trading Database Schema

-- Create schema
CREATE SCHEMA IF NOT EXISTS barter;

-- Create trades table
CREATE TABLE IF NOT EXISTS barter.trades (
    id BIGSERIAL PRIMARY KEY,
    trade_id VARCHAR(100) UNIQUE NOT NULL,
    exchange VARCHAR(50) NOT NULL,
    instrument VARCHAR(50) NOT NULL,
    side VARCHAR(10) NOT NULL,
    price DECIMAL(20, 8) NOT NULL,
    quantity DECIMAL(20, 8) NOT NULL,
    fee DECIMAL(20, 8),
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create orders table
CREATE TABLE IF NOT EXISTS barter.orders (
    id BIGSERIAL PRIMARY KEY,
    order_id VARCHAR(100) UNIQUE NOT NULL,
    exchange VARCHAR(50) NOT NULL,
    instrument VARCHAR(50) NOT NULL,
    order_type VARCHAR(20) NOT NULL,
    side VARCHAR(10) NOT NULL,
    price DECIMAL(20, 8),
    quantity DECIMAL(20, 8) NOT NULL,
    filled_quantity DECIMAL(20, 8) DEFAULT 0,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create positions table
CREATE TABLE IF NOT EXISTS barter.positions (
    id BIGSERIAL PRIMARY KEY,
    exchange VARCHAR(50) NOT NULL,
    instrument VARCHAR(50) NOT NULL,
    quantity DECIMAL(20, 8) NOT NULL,
    entry_price DECIMAL(20, 8) NOT NULL,
    current_price DECIMAL(20, 8),
    unrealized_pnl DECIMAL(20, 8),
    realized_pnl DECIMAL(20, 8) DEFAULT 0,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(exchange, instrument)
);

-- Create performance metrics table
CREATE TABLE IF NOT EXISTS barter.performance_metrics (
    id BIGSERIAL PRIMARY KEY,
    metric_date DATE NOT NULL,
    total_pnl DECIMAL(20, 8),
    win_rate DECIMAL(5, 2),
    sharpe_ratio DECIMAL(10, 4),
    max_drawdown DECIMAL(10, 4),
    total_trades INTEGER,
    winning_trades INTEGER,
    losing_trades INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(metric_date)
);

-- Create indexes
CREATE INDEX idx_trades_timestamp ON barter.trades(timestamp);
CREATE INDEX idx_trades_exchange_instrument ON barter.trades(exchange, instrument);
CREATE INDEX idx_orders_status ON barter.orders(status);
CREATE INDEX idx_orders_exchange_instrument ON barter.orders(exchange, instrument);
CREATE INDEX idx_positions_exchange_instrument ON barter.positions(exchange, instrument);

-- Create update trigger for orders
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_orders_updated_at BEFORE UPDATE
    ON barter.orders FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();