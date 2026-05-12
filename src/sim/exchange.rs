use ordered_float::NotNan;
use std::collections::{BTreeMap, VecDeque};

use crate::shared::types::{AgentId, Order, OrderId, OrderType, Side, SimTime, Trade};

pub struct LimitOrderBook {
    bids: BTreeMap<NotNan<f64>, VecDeque<Order>>, // descending (best bid = max key)
    asks: BTreeMap<NotNan<f64>, VecDeque<Order>>, // ascending (best ask = min key)
    // Buy stops (Side::Bid): trigger when price rises to or above stop_price
    stop_asks: BTreeMap<NotNan<f64>, Vec<Order>>,
    // Sell stops (Side::Ask): trigger when price falls to or below stop_price
    stop_bids: BTreeMap<NotNan<f64>, Vec<Order>>,
}

const MAX_STOP_DEPTH: usize = 64;

fn cancel_from_deque(map: &mut BTreeMap<NotNan<f64>, VecDeque<Order>>, id: OrderId) -> bool {
    let found = map
        .iter_mut()
        .find_map(|(k, q)| q.iter().position(|o| o.id == id).map(|pos| (*k, pos)));
    if let Some((key, pos)) = found {
        map.get_mut(&key).unwrap().remove(pos);
        if map[&key].is_empty() {
            map.remove(&key);
        }
        return true;
    }
    false
}

fn cancel_from_vec(map: &mut BTreeMap<NotNan<f64>, Vec<Order>>, id: OrderId) -> bool {
    let found = map
        .iter_mut()
        .find_map(|(k, v)| v.iter().position(|o| o.id == id).map(|pos| (*k, pos)));
    if let Some((key, pos)) = found {
        map.get_mut(&key).unwrap().remove(pos);
        if map[&key].is_empty() {
            map.remove(&key);
        }
        return true;
    }
    false
}

impl LimitOrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            stop_asks: BTreeMap::new(),
            stop_bids: BTreeMap::new(),
        }
    }

    pub fn best_bid(&self) -> Option<f64> {
        self.bids.keys().next_back().map(|k| **k)
    }

    pub fn best_ask(&self) -> Option<f64> {
        self.asks.keys().next().map(|k| **k)
    }

    pub fn insert_limit(&mut self, order: Order) {
        let price = NotNan::new(order.price).expect("order price is NaN");
        match order.side {
            Side::Bid => self.bids.entry(price).or_default().push_back(order),
            Side::Ask => self.asks.entry(price).or_default().push_back(order),
        }
    }

    pub fn insert_stop(&mut self, order: Order) {
        let stop_price = match &order.order_type {
            OrderType::Stop { stop_price } | OrderType::StopLimit { stop_price } => *stop_price,
            _ => panic!("insert_stop called with non-stop order"),
        };
        let key = NotNan::new(stop_price).expect("stop price is NaN");
        match order.side {
            Side::Bid => self.stop_asks.entry(key).or_default().push(order),
            Side::Ask => self.stop_bids.entry(key).or_default().push(order),
        }
    }

    pub fn cancel(&mut self, id: OrderId) -> bool {
        if cancel_from_deque(&mut self.bids, id) {
            return true;
        }
        if cancel_from_deque(&mut self.asks, id) {
            return true;
        }
        if cancel_from_vec(&mut self.stop_bids, id) {
            return true;
        }
        if cancel_from_vec(&mut self.stop_asks, id) {
            return true;
        }
        false
    }

    pub fn submit(&mut self, order: Order, now: SimTime) -> Vec<Trade> {
        match &order.order_type {
            OrderType::Stop { .. } | OrderType::StopLimit { .. } => {
                self.insert_stop(order);
                vec![]
            }
            _ => {
                let mut all_trades = Vec::new();
                self.match_order(order, now, 0, &mut all_trades);
                all_trades
            }
        }
    }

    fn match_order(
        &mut self,
        order: Order,
        now: SimTime,
        depth: usize,
        all_trades: &mut Vec<Trade>,
    ) {
        if depth > MAX_STOP_DEPTH {
            return;
        }

        if matches!(order.order_type, OrderType::Fok) && !self.can_fill_fully(&order) {
            return;
        }

        let aggressor_side = order.side;
        let is_ioc = matches!(order.order_type, OrderType::Ioc);
        let is_restable = matches!(
            order.order_type,
            OrderType::Limit | OrderType::Iceberg { .. }
        );

        let mut order = order;
        let trades = self.sweep(&mut order, now);
        let sweep_produced = !trades.is_empty();
        let last_price = trades.last().map(|t| t.price);
        all_trades.extend(trades);

        if !is_ioc && is_restable && order.remaining() > 0 {
            self.insert_limit(order);
        }

        if sweep_produced {
            if let Some(price) = last_price {
                let triggered = self.collect_triggered_stops(price, &aggressor_side);
                for stop_order in triggered {
                    let converted = self.convert_stop(stop_order);
                    self.match_order(converted, now, depth + 1, all_trades);
                }
            }
        }
    }

    fn sweep(&mut self, order: &mut Order, now: SimTime) -> Vec<Trade> {
        let mut trades = Vec::new();

        loop {
            if order.remaining() == 0 {
                break;
            }

            let best_key = match order.side {
                Side::Bid => self.asks.keys().next().copied(),
                Side::Ask => self.bids.keys().next_back().copied(),
            };

            let best_key = match best_key {
                Some(k) => k,
                None => break,
            };

            if !price_crosses(order, *best_key) {
                break;
            }

            // Drain this price level
            loop {
                if order.remaining() == 0 {
                    break;
                }

                // All borrows of self.asks/bids are scoped here so they drop before
                // any iceberg replenishment re-borrows the same map entry.
                let (fill_qty, resting_id, resting_agent, done_order) = {
                    let queue = match order.side {
                        Side::Bid => match self.asks.get_mut(&best_key) {
                            Some(q) if !q.is_empty() => q,
                            _ => break,
                        },
                        Side::Ask => match self.bids.get_mut(&best_key) {
                            Some(q) if !q.is_empty() => q,
                            _ => break,
                        },
                    };

                    // Nested block so `resting` borrow drops before pop_front.
                    let (fq, rid, ragent, exhausted) = {
                        let resting = queue.front_mut().unwrap();
                        let fq = order.remaining().min(resting.remaining());
                        order.filled += fq;
                        resting.filled += fq;
                        (fq, resting.id, resting.agent_id, resting.remaining() == 0)
                    };

                    let done = if exhausted {
                        Some(queue.pop_front().unwrap())
                    } else {
                        None
                    };
                    (fq, rid, ragent, done)
                    // queue borrow drops here
                };

                trades.push(Trade {
                    aggressor_order_id: order.id,
                    resting_order_id: resting_id,
                    aggressor_agent: order.agent_id,
                    resting_agent,
                    price: *best_key,
                    size: fill_qty,
                    aggressor_side: order.side,
                    time: now,
                });

                // Iceberg replenishment: queue borrow already dropped above.
                if let Some(done) = done_order {
                    if let OrderType::Iceberg {
                        display_qty,
                        hidden_qty,
                    } = done.order_type
                    {
                        if hidden_qty > 0 {
                            let replenish_qty = display_qty.min(hidden_qty);
                            let new_slice = Order {
                                quantity: replenish_qty,
                                filled: 0,
                                order_type: OrderType::Iceberg {
                                    display_qty,
                                    hidden_qty: hidden_qty - replenish_qty,
                                },
                                ..done
                            };
                            let queue2 = match order.side {
                                Side::Bid => self.asks.get_mut(&best_key).unwrap(),
                                Side::Ask => self.bids.get_mut(&best_key).unwrap(),
                            };
                            queue2.push_back(new_slice);
                        }
                    }
                }
            }

            // Remove empty price level
            let empty = match order.side {
                Side::Bid => self.asks.get(&best_key).is_none_or(|q| q.is_empty()),
                Side::Ask => self.bids.get(&best_key).is_none_or(|q| q.is_empty()),
            };
            if empty {
                match order.side {
                    Side::Bid => self.asks.remove(&best_key),
                    Side::Ask => self.bids.remove(&best_key),
                };
            }
        }

        trades
    }

    fn can_fill_fully(&self, order: &Order) -> bool {
        let mut needed = order.remaining();
        let limit_price = if order.price > 0.0 {
            Some(order.price)
        } else {
            None
        };

        match order.side {
            Side::Bid => {
                for (price_key, queue) in &self.asks {
                    if let Some(lp) = limit_price {
                        if **price_key > lp {
                            break;
                        }
                    }
                    for resting in queue {
                        let avail = effective_remaining(resting);
                        let fill = needed.min(avail);
                        needed -= fill;
                        if needed == 0 {
                            return true;
                        }
                    }
                }
            }
            Side::Ask => {
                for (price_key, queue) in self.bids.iter().rev() {
                    if let Some(lp) = limit_price {
                        if **price_key < lp {
                            break;
                        }
                    }
                    for resting in queue {
                        let avail = effective_remaining(resting);
                        let fill = needed.min(avail);
                        needed -= fill;
                        if needed == 0 {
                            return true;
                        }
                    }
                }
            }
        }

        needed == 0
    }

    fn collect_triggered_stops(&mut self, trade_price: f64, aggressor_side: &Side) -> Vec<Order> {
        let mut triggered = Vec::new();
        match aggressor_side {
            // Bid swept ask upward → trigger buy stops (in stop_asks) at or below trade_price
            Side::Bid => {
                let keys: Vec<NotNan<f64>> = self
                    .stop_asks
                    .keys()
                    .copied()
                    .filter(|k| **k <= trade_price)
                    .collect();
                for k in keys {
                    if let Some(orders) = self.stop_asks.remove(&k) {
                        triggered.extend(orders);
                    }
                }
            }
            // Ask swept bid downward → trigger sell stops (in stop_bids) at or above trade_price
            Side::Ask => {
                let keys: Vec<NotNan<f64>> = self
                    .stop_bids
                    .keys()
                    .copied()
                    .filter(|k| **k >= trade_price)
                    .collect();
                for k in keys {
                    if let Some(orders) = self.stop_bids.remove(&k) {
                        triggered.extend(orders);
                    }
                }
            }
        }
        triggered
    }

    fn convert_stop(&self, mut order: Order) -> Order {
        order.order_type = match order.order_type {
            OrderType::Stop { .. } => OrderType::Market,
            OrderType::StopLimit { .. } => OrderType::Limit,
            other => other,
        };
        order
    }
}

impl Default for LimitOrderBook {
    fn default() -> Self {
        Self::new()
    }
}

fn price_crosses(order: &Order, best_price: f64) -> bool {
    match order.order_type {
        OrderType::Market | OrderType::Ioc | OrderType::Fok => true,
        OrderType::Limit | OrderType::Iceberg { .. } => match order.side {
            Side::Bid => order.price >= best_price,
            Side::Ask => order.price <= best_price,
        },
        _ => false,
    }
}

fn effective_remaining(order: &Order) -> u32 {
    let base = order.remaining();
    match &order.order_type {
        OrderType::Iceberg { hidden_qty, .. } => base + hidden_qty,
        _ => base,
    }
}

pub fn make_order(
    id: OrderId,
    agent_id: AgentId,
    side: Side,
    order_type: OrderType,
    price: f64,
    quantity: u32,
) -> Order {
    Order {
        id,
        agent_id,
        side,
        order_type,
        price,
        quantity,
        filled: 0,
        submitted_at: 0,
        gtc: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit_bid(id: u64, price: f64, qty: u32) -> Order {
        make_order(id, id as u32, Side::Bid, OrderType::Limit, price, qty)
    }

    fn limit_ask(id: u64, price: f64, qty: u32) -> Order {
        make_order(id, id as u32, Side::Ask, OrderType::Limit, price, qty)
    }

    fn market_bid(id: u64, qty: u32) -> Order {
        make_order(id, id as u32, Side::Bid, OrderType::Market, 0.0, qty)
    }

    fn market_ask(id: u64, qty: u32) -> Order {
        make_order(id, id as u32, Side::Ask, OrderType::Market, 0.0, qty)
    }

    #[test]
    fn limit_order_rests_in_book() {
        let mut lob = LimitOrderBook::new();
        let trades = lob.submit(limit_bid(1, 10.00, 100), 0);
        assert!(trades.is_empty());
        assert_eq!(lob.best_bid(), Some(10.00));
        assert!(lob.best_ask().is_none());
    }

    #[test]
    fn market_order_fills_against_resting_limit() {
        let mut lob = LimitOrderBook::new();
        lob.submit(limit_ask(1, 10.00, 100), 0);
        let trades = lob.submit(market_bid(2, 100), 1);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, 10.00);
        assert_eq!(trades[0].size, 100);
        assert!(lob.best_ask().is_none());
    }

    #[test]
    fn multi_level_sweep() {
        let mut lob = LimitOrderBook::new();
        lob.submit(limit_ask(1, 10.01, 100), 0);
        lob.submit(limit_ask(2, 10.02, 100), 0);
        lob.submit(limit_ask(3, 10.03, 100), 0);
        let trades = lob.submit(market_bid(10, 250), 1);
        assert_eq!(trades.len(), 3);
        assert_eq!(trades[0].size, 100);
        assert_eq!(trades[0].price, 10.01);
        assert_eq!(trades[1].size, 100);
        assert_eq!(trades[1].price, 10.02);
        assert_eq!(trades[2].size, 50);
        assert_eq!(trades[2].price, 10.03);
    }

    #[test]
    fn partial_fill_remainder_rests() {
        let mut lob = LimitOrderBook::new();
        lob.submit(limit_ask(1, 10.00, 50), 0);
        let trades = lob.submit(limit_bid(2, 10.00, 100), 1);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].size, 50);
        assert_eq!(lob.best_bid(), Some(10.00));
        assert!(lob.best_ask().is_none());
    }

    #[test]
    fn ioc_cancels_remainder() {
        let mut lob = LimitOrderBook::new();
        lob.submit(limit_ask(1, 10.00, 50), 0);
        let ioc = make_order(2, 2, Side::Bid, OrderType::Ioc, 10.00, 100);
        let trades = lob.submit(ioc, 1);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].size, 50);
        assert!(lob.best_bid().is_none());
    }

    #[test]
    fn ioc_no_fill_fully_cancelled() {
        let mut lob = LimitOrderBook::new();
        let ioc = make_order(1, 1, Side::Bid, OrderType::Ioc, 10.00, 100);
        let trades = lob.submit(ioc, 0);
        assert!(trades.is_empty());
        assert!(lob.best_bid().is_none());
    }

    #[test]
    fn fok_fills_entirely() {
        let mut lob = LimitOrderBook::new();
        lob.submit(limit_ask(1, 10.00, 100), 0);
        let fok = make_order(2, 2, Side::Bid, OrderType::Fok, 0.0, 100);
        let trades = lob.submit(fok, 1);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].size, 100);
    }

    #[test]
    fn fok_insufficient_liquidity_no_fills() {
        let mut lob = LimitOrderBook::new();
        lob.submit(limit_ask(1, 10.00, 50), 0);
        let fok = make_order(2, 2, Side::Bid, OrderType::Fok, 0.0, 100);
        let trades = lob.submit(fok, 1);
        assert!(trades.is_empty());
        assert_eq!(lob.best_ask(), Some(10.00));
    }

    #[test]
    fn stop_triggers_and_fills() {
        let mut lob = LimitOrderBook::new();
        lob.submit(limit_ask(1, 10.00, 100), 0);
        lob.submit(limit_ask(3, 10.00, 50), 0);
        let stop = make_order(
            2,
            2,
            Side::Bid,
            OrderType::Stop { stop_price: 10.00 },
            0.0,
            50,
        );
        lob.submit(stop, 0);

        // Market bid fills 100 from ask1 → triggers buy stop → fills 50 from ask3
        let trades = lob.submit(market_bid(10, 100), 1);
        assert!(trades.len() >= 2);
        let total: u32 = trades.iter().map(|t| t.size).sum();
        assert_eq!(total, 150);
    }

    #[test]
    fn stop_cascade() {
        let mut lob = LimitOrderBook::new();
        lob.submit(limit_ask(1, 10.00, 100), 0);
        lob.submit(limit_ask(2, 10.01, 100), 0);
        lob.submit(limit_ask(3, 10.02, 100), 0);

        let stop1 = make_order(
            10,
            10,
            Side::Bid,
            OrderType::Stop { stop_price: 10.00 },
            0.0,
            100,
        );
        let stop2 = make_order(
            11,
            11,
            Side::Bid,
            OrderType::Stop { stop_price: 10.01 },
            0.0,
            100,
        );
        lob.submit(stop1, 0);
        lob.submit(stop2, 0);

        let trades = lob.submit(market_bid(20, 100), 1);
        println!("Stop cascade trades:");
        for t in &trades {
            println!(
                "  price={} size={} aggressor_id={}",
                t.price, t.size, t.aggressor_order_id
            );
        }
        assert_eq!(trades.len(), 3);
        assert_eq!(trades[0].price, 10.00);
        assert_eq!(trades[1].price, 10.01);
        assert_eq!(trades[2].price, 10.02);
        let total: u32 = trades.iter().map(|t| t.size).sum();
        assert_eq!(total, 300);
    }

    #[test]
    fn iceberg_replenishment() {
        let mut lob = LimitOrderBook::new();
        let iceberg = make_order(
            1,
            1,
            Side::Ask,
            OrderType::Iceberg {
                display_qty: 100,
                hidden_qty: 400,
            },
            10.00,
            100,
        );
        lob.submit(iceberg, 0);

        let trades = lob.submit(market_bid(10, 250), 1);
        println!("Iceberg trades:");
        for t in &trades {
            println!("  price={} size={}", t.price, t.size);
        }
        assert_eq!(trades.len(), 3);
        let sizes: Vec<u32> = trades.iter().map(|t| t.size).collect();
        assert_eq!(sizes, vec![100, 100, 50]);
    }

    #[test]
    fn cancel_removes_order() {
        let mut lob = LimitOrderBook::new();
        lob.submit(limit_bid(1, 10.00, 100), 0);
        assert!(lob.cancel(1));
        assert!(lob.best_bid().is_none());
    }

    #[test]
    fn manual_five_bids_market_ask_350() {
        // 5 bids at 9.95..9.99, market ask for 350
        let mut lob = LimitOrderBook::new();
        lob.submit(limit_bid(1, 9.95, 100), 0);
        lob.submit(limit_bid(2, 9.96, 100), 0);
        lob.submit(limit_bid(3, 9.97, 100), 0);
        lob.submit(limit_bid(4, 9.98, 100), 0);
        lob.submit(limit_bid(5, 9.99, 100), 0);
        let trades = lob.submit(market_ask(10, 350), 1);
        let total: u32 = trades.iter().map(|t| t.size).sum();
        assert_eq!(total, 350);
        assert_eq!(trades[0].price, 9.99);
        assert_eq!(trades[0].size, 100);
        assert_eq!(trades[1].price, 9.98);
        assert_eq!(trades[1].size, 100);
        assert_eq!(trades[2].price, 9.97);
        assert_eq!(trades[2].size, 100);
        assert_eq!(trades[3].price, 9.96);
        assert_eq!(trades[3].size, 50);
    }

    // --- Five bid limit order tests ---

    fn five_bids() -> LimitOrderBook {
        let mut lob = LimitOrderBook::new();
        lob.submit(limit_bid(1, 9.95, 100), 0);
        lob.submit(limit_bid(2, 9.96, 100), 0);
        lob.submit(limit_bid(3, 9.97, 100), 0);
        lob.submit(limit_bid(4, 9.98, 100), 0);
        lob.submit(limit_bid(5, 9.99, 100), 0);
        lob
    }

    #[test]
    fn five_bids_best_bid_is_highest_price() {
        let lob = five_bids();
        assert_eq!(lob.best_bid(), Some(9.99));
    }

    #[test]
    fn five_bids_no_asks() {
        let lob = five_bids();
        assert!(lob.best_ask().is_none());
    }

    #[test]
    fn five_bids_no_fills_on_insert() {
        let mut lob = LimitOrderBook::new();
        for (id, price) in [(1u64, 9.95f64), (2, 9.96), (3, 9.97), (4, 9.98), (5, 9.99)] {
            let trades = lob.submit(limit_bid(id, price, 100), 0);
            assert!(trades.is_empty(), "limit bid should rest, not fill");
        }
    }

    #[test]
    fn five_bids_fifo_at_same_price() {
        // Two bids at the same price: first submitted fills first
        let mut lob = LimitOrderBook::new();
        lob.submit(limit_bid(1, 10.00, 60), 0);
        lob.submit(limit_bid(2, 10.00, 60), 0);
        let trades = lob.submit(market_ask(10, 70), 1);
        assert_eq!(trades.len(), 2);
        // Order 1 fully filled (60), order 2 partially filled (10)
        assert_eq!(trades[0].resting_order_id, 1);
        assert_eq!(trades[0].size, 60);
        assert_eq!(trades[1].resting_order_id, 2);
        assert_eq!(trades[1].size, 10);
    }

    #[test]
    fn five_bids_cancel_best_then_check_new_best() {
        let mut lob = five_bids();
        assert!(lob.cancel(5)); // cancel the 9.99 bid
        assert_eq!(lob.best_bid(), Some(9.98));
    }

    #[test]
    fn five_bids_market_ask_exhausts_all_levels() {
        let mut lob = five_bids();
        let trades = lob.submit(market_ask(10, 500), 1);
        assert_eq!(trades.len(), 5);
        let total: u32 = trades.iter().map(|t| t.size).sum();
        assert_eq!(total, 500);
        assert!(lob.best_bid().is_none());
    }

    #[test]
    fn five_bids_limit_ask_only_crosses_eligible_levels() {
        // Limit ask at 9.97 should only fill against bids >= 9.97
        let mut lob = five_bids();
        let ask = make_order(10, 10, Side::Ask, OrderType::Limit, 9.97, 500);
        let trades = lob.submit(ask, 1);
        // Only 9.99, 9.98, 9.97 are at or above 9.97 — 300 shares total
        let total: u32 = trades.iter().map(|t| t.size).sum();
        assert_eq!(total, 300);
        // Bids at 9.96 and 9.95 still resting
        assert_eq!(lob.best_bid(), Some(9.96));
    }

    #[test]
    fn fok_600_against_500_no_fills() {
        let mut lob = LimitOrderBook::new();
        for (id, price) in [(1u64, 9.95f64), (2, 9.96), (3, 9.97), (4, 9.98), (5, 9.99)] {
            lob.submit(limit_bid(id, price, 100), 0);
        }
        let fok = make_order(10, 10, Side::Ask, OrderType::Fok, 0.0, 600);
        let trades = lob.submit(fok, 1);
        assert!(trades.is_empty());
        assert_eq!(lob.best_bid(), Some(9.99));
    }
}
