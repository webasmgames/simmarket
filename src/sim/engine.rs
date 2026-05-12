use std::collections::HashMap;

use crate::shared::types::{Candle, SimTime, StockId, Trade};
use crate::sim::event_queue::{EventQueue, OrderAction};
use crate::sim::exchange::LimitOrderBook;

const MICROS_PER_MINUTE: u64 = 60_000_000;

pub struct SimState {
    pub clock: SimTime,
    pub tick_size_us: u64,
    pub books: HashMap<StockId, LimitOrderBook>,
    pub event_queue: EventQueue,
    pub candles: HashMap<StockId, Vec<Candle>>,
    pub tape: Vec<Trade>,
}

impl SimState {
    pub fn new(tick_size_us: u64) -> Self {
        Self {
            clock: 0,
            tick_size_us,
            books: HashMap::new(),
            event_queue: EventQueue::new(),
            candles: HashMap::new(),
            tape: Vec::new(),
        }
    }

    pub fn tick(&mut self) {
        let events = self.event_queue.drain_sorted();

        for event in events {
            let book = self.books.entry(event.stock_id).or_default();

            let trades = match event.action {
                OrderAction::Submit(order) => book.submit(order, self.clock),
                OrderAction::Cancel(id) => {
                    book.cancel(id);
                    vec![]
                }
            };

            if !trades.is_empty() {
                let stock_candles = self.candles.entry(event.stock_id).or_default();
                let minute_start = (self.clock / MICROS_PER_MINUTE) * MICROS_PER_MINUTE;

                for trade in &trades {
                    let on_current_candle = stock_candles
                        .last()
                        .is_some_and(|c| c.sim_time == minute_start);

                    if on_current_candle {
                        let c = stock_candles.last_mut().unwrap();
                        if trade.price > c.high {
                            c.high = trade.price;
                        }
                        if trade.price < c.low {
                            c.low = trade.price;
                        }
                        c.close = trade.price;
                        c.volume += trade.size as u64;
                    } else {
                        stock_candles.push(Candle {
                            open: trade.price,
                            high: trade.price,
                            low: trade.price,
                            close: trade.price,
                            volume: trade.size as u64,
                            sim_time: minute_start,
                        });
                    }
                }

                self.tape.extend(trades);
            }
        }

        self.clock += self.tick_size_us;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::{Order, OrderType, Side};
    use crate::sim::event_queue::OrderEvent;

    const STOCK: StockId = 1;
    const TICK_MS: u64 = 1_000; // 1 ms in microseconds

    fn limit_order(id: u64, side: Side, price: f64, qty: u32) -> Order {
        Order {
            id,
            agent_id: id as u32,
            side,
            order_type: OrderType::Limit,
            price,
            quantity: qty,
            filled: 0,
            submitted_at: 0,
            gtc: false,
        }
    }

    fn market_order(id: u64, side: Side, qty: u32) -> Order {
        Order {
            id,
            agent_id: id as u32,
            side,
            order_type: OrderType::Market,
            price: 0.0,
            quantity: qty,
            filled: 0,
            submitted_at: 0,
            gtc: false,
        }
    }

    fn push(state: &mut SimState, offset: u32, order: Order) {
        state.event_queue.push(OrderEvent {
            intra_tick_offset_us: offset,
            agent_id: order.agent_id,
            stock_id: STOCK,
            action: OrderAction::Submit(order),
        });
    }

    #[test]
    fn clock_advances_by_tick_size() {
        let mut state = SimState::new(TICK_MS);
        assert_eq!(state.clock, 0);
        state.tick();
        assert_eq!(state.clock, TICK_MS);
        state.tick();
        assert_eq!(state.clock, 2 * TICK_MS);
    }

    #[test]
    fn events_drain_in_offset_order_within_tick() {
        let mut state = SimState::new(TICK_MS);
        // Ask rests first (lower offset), then market bid sweeps
        push(&mut state, 100, limit_order(1, Side::Ask, 10.00, 100));
        push(&mut state, 200, market_order(2, Side::Bid, 100));
        state.tick();
        assert_eq!(state.tape.len(), 1);
        assert_eq!(state.tape[0].price, 10.00);
        assert_eq!(state.tape[0].size, 100);
    }

    #[test]
    fn wrong_offset_order_no_trade() {
        // Market bid arrives before ask — no resting liquidity at match time → no trade
        let mut state = SimState::new(TICK_MS);
        push(&mut state, 100, market_order(2, Side::Bid, 100));
        push(&mut state, 200, limit_order(1, Side::Ask, 10.00, 100));
        state.tick();
        assert!(state.tape.is_empty());
    }

    #[test]
    fn trade_ends_up_in_tape() {
        let mut state = SimState::new(TICK_MS);
        push(&mut state, 100, limit_order(1, Side::Ask, 10.00, 100));
        push(&mut state, 200, market_order(2, Side::Bid, 100));
        state.tick();
        assert_eq!(state.tape.len(), 1);
        assert_eq!(state.tape[0].aggressor_order_id, 2);
        assert_eq!(state.tape[0].resting_order_id, 1);
    }

    #[test]
    fn candle_single_trade() {
        let mut state = SimState::new(TICK_MS);
        push(&mut state, 100, limit_order(1, Side::Ask, 10.00, 100));
        push(&mut state, 200, market_order(2, Side::Bid, 100));
        state.tick();
        let candles = state.candles.get(&STOCK).unwrap();
        assert_eq!(candles.len(), 1);
        let c = &candles[0];
        assert_eq!(c.open, 10.00);
        assert_eq!(c.high, 10.00);
        assert_eq!(c.low, 10.00);
        assert_eq!(c.close, 10.00);
        assert_eq!(c.volume, 100);
        assert_eq!(c.sim_time, 0); // minute 0 starts at t=0
    }

    #[test]
    fn candle_ohlcv_multiple_trades() {
        let mut state = SimState::new(TICK_MS);
        // Three resting asks at different prices; one market bid sweeps all
        push(&mut state, 100, limit_order(1, Side::Ask, 9.99, 100));
        push(&mut state, 200, limit_order(2, Side::Ask, 10.00, 100));
        push(&mut state, 300, limit_order(3, Side::Ask, 10.01, 100));
        push(&mut state, 400, market_order(4, Side::Bid, 300));
        state.tick();
        let candles = state.candles.get(&STOCK).unwrap();
        assert_eq!(candles.len(), 1);
        let c = &candles[0];
        assert_eq!(c.open, 9.99); // first trade price
        assert_eq!(c.high, 10.01);
        assert_eq!(c.low, 9.99);
        assert_eq!(c.close, 10.01); // last trade price
        assert_eq!(c.volume, 300);
    }

    #[test]
    fn candle_new_minute_starts_new_candle() {
        // 1ms ticks; a minute is 60_000_000 µs = 60_000 ticks
        let mut state = SimState::new(TICK_MS);

        // Trade in minute 0
        push(&mut state, 100, limit_order(1, Side::Ask, 10.00, 100));
        push(&mut state, 200, market_order(2, Side::Bid, 100));
        state.tick();

        // Advance clock past the 1-minute mark (60_000 ticks of 1ms each)
        for _ in 0..60_000 {
            state.tick();
        }
        // clock is now at (60_001 * 1_000) = 60_001_000 µs → minute index 1
        assert!(state.clock >= MICROS_PER_MINUTE);

        // Trade in minute 1
        push(&mut state, 100, limit_order(3, Side::Ask, 11.00, 50));
        push(&mut state, 200, market_order(4, Side::Bid, 50));
        state.tick();

        let candles = state.candles.get(&STOCK).unwrap();
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].sim_time, 0);
        assert_eq!(candles[0].close, 10.00);
        assert_eq!(candles[1].sim_time, MICROS_PER_MINUTE);
        assert_eq!(candles[1].open, 11.00);
        assert_eq!(candles[1].volume, 50);
    }

    #[test]
    fn cancel_removes_resting_order() {
        let mut state = SimState::new(TICK_MS);
        // Submit ask, then cancel it, then try to fill — no trade expected
        push(&mut state, 100, limit_order(1, Side::Ask, 10.00, 100));
        state.event_queue.push(OrderEvent {
            intra_tick_offset_us: 200,
            agent_id: 1,
            stock_id: STOCK,
            action: OrderAction::Cancel(1),
        });
        push(&mut state, 300, market_order(2, Side::Bid, 100));
        state.tick();
        assert!(state.tape.is_empty());
    }

    #[test]
    fn partial_tick_events_do_not_bleed_into_next_tick() {
        let mut state = SimState::new(TICK_MS);
        // Only submit ask in tick 0; market bid arrives in tick 1
        push(&mut state, 100, limit_order(1, Side::Ask, 10.00, 100));
        state.tick(); // ask rests, no trade

        push(&mut state, 100, market_order(2, Side::Bid, 100));
        state.tick(); // bid matches resting ask

        assert_eq!(state.tape.len(), 1);
        // Trade time should be clock at start of tick 1 = TICK_MS
        assert_eq!(state.tape[0].time, TICK_MS);
    }
}
