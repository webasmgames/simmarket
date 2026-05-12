pub type OrderId = u64;
pub type AgentId = u32;
pub type SimTime = u64; // microseconds of simulated time
pub type StockId = u32;

#[derive(Debug, Clone)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
    pub sim_time: SimTime, // start-of-minute timestamp (minute_index * 60_000_000)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug, Clone)]
pub enum OrderType {
    Market,
    Limit,
    Stop { stop_price: f64 },
    StopLimit { stop_price: f64 },
    Ioc,
    Fok,
    Iceberg { display_qty: u32, hidden_qty: u32 },
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: OrderId,
    pub agent_id: AgentId,
    pub side: Side,
    pub order_type: OrderType,
    pub price: f64, // limit price; 0.0 for market
    pub quantity: u32,
    pub filled: u32,
    pub submitted_at: SimTime,
    pub gtc: bool,
}

impl Order {
    pub fn remaining(&self) -> u32 {
        self.quantity - self.filled
    }
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub aggressor_order_id: OrderId,
    pub resting_order_id: OrderId,
    pub aggressor_agent: AgentId,
    pub resting_agent: AgentId,
    pub price: f64,
    pub size: u32,
    pub aggressor_side: Side,
    pub time: SimTime,
}
