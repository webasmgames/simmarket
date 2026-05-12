# TODO: Phase 2 — Tick Engine + Noise Trader

## Overview

Wire up the tick loop, intra-tick event queue, agent trait, and the first agent archetype: the noise trader. Run the simulation headless and output OHLCV CSV. The noise trader submits random limit orders near the midpoint with no directional thesis — pure exogenous liquidity flow. The goal is to confirm that a spread forms, price diffuses, and the tape looks like tape before any strategic agents are added.

Before this phase: a tested LOB with no runtime. After this phase: a binary you can run that simulates a market day and writes `ohlcv.csv`.

---

## Requirements

- [ ] Tick loop advances simulated clock by 1ms per tick; configurable speed multiplier
- [ ] Intra-tick event queue: agents produce `OrderEvent`s with microsecond latency offsets; queue drains in timestamp order into the LOB
- [ ] `Agent` trait with `observe()` and `decide()` methods
- [ ] `NoiseTrader` agent: submits random buy or sell limit orders 1–3 ticks from current midpoint; cancels stale orders after a patience threshold
- [ ] Agent pool supports spawning N noise traders with configurable parameters
- [ ] OHLCV candles accumulate at 1-minute resolution for the regular session
- [ ] Headless binary writes `ohlcv.csv` after 1 simulated trading day (9:30–16:00)
- [ ] Bid-ask spread is visible in output (mid, best bid, best ask logged per minute)
- [ ] Seeded PRNG per agent — same seed produces identical output

---

## Design

### Data Structures

```rust
// src/sim/engine.rs
struct SimState {
    clock: SimTime,           // current simulated microseconds since epoch
    books: HashMap<StockId, LimitOrderBook>,
    agent_pool: AgentPool,
    candles: HashMap<StockId, Vec<Candle>>,
    tape: Vec<Trade>,
}

struct Candle {
    open: f64, high: f64, low: f64, close: f64,
    volume: u64,
    sim_time: SimTime,
}

// src/sim/event_queue.rs
struct OrderEvent {
    intra_tick_offset_us: u32,  // microseconds within tick; determines ordering
    agent_id: AgentId,
    action: OrderAction,
}

enum OrderAction {
    Submit(Order),
    Cancel(OrderId),
}

// BinaryHeap<OrderEvent> ordered by intra_tick_offset_us ascending
```

```rust
// src/sim/agents/mod.rs
trait Agent {
    fn schedule_interval_ticks(&self) -> u64;      // how often this agent runs
    fn latency_offset_us(&self) -> u32;             // intra-tick position
    fn observe(&self, market: &MarketSnapshot) -> Observation;
    fn decide(&mut self, obs: &Observation, rng: &mut SmallRng) -> Vec<OrderAction>;
}

// src/sim/agents/noise.rs
struct NoiseTrader {
    account: Account,
    stock_id: StockId,
    patience_ticks: u64,       // cancel stale orders after this many ticks
    order_size: u32,
    rng_seed: u64,
    pending_order: Option<OrderId>,
}
```

### Key Logic

**Tick loop**:
1. Advance clock by `dt` (1ms default)
2. For each agent due to run this tick: call `observe()` → `decide()` → push `OrderEvent`s onto the queue with the agent's `latency_offset_us`
3. Sort queue by `intra_tick_offset_us`; drain in order, submitting each action to the LOB
4. Accumulate trades into OHLCV candles
5. Cancel stale noise trader orders past their patience threshold

**Noise trader decide()**:
- 50% chance buy, 50% sell
- Cancel any pending order first
- Price = midpoint ± random(1–3) ticks
- Submit limit order; record order ID as pending

**MarketSnapshot** passed to agents: last price, best bid, best ask, bid size, ask size (read-only view of book state at start of tick).

---

## Files

| File | Action | Notes |
|---|---|---|
| `src/sim/engine.rs` | Create | SimState, tick loop, candle accumulation |
| `src/sim/event_queue.rs` | Create | OrderEvent, BinaryHeap-based queue |
| `src/sim/agents/mod.rs` | Create | Agent trait, AgentPool, MarketSnapshot, Observation |
| `src/sim/agents/noise.rs` | Create | NoiseTrader implementation |
| `src/shared/types.rs` | Modify | Add StockId, Account, Candle |
| `src/bin/headless.rs` | Create | Spawns N noise traders, runs 1 day, writes ohlcv.csv |

---

## Tasks

- [ ] **1.** Add `SmallRng` (from `rand` crate) to `Cargo.toml`; add `rand` dependency
- [ ] **2.** Define `StockId`, `Account`, `Candle`, `MarketSnapshot`, `Observation` in `shared/types.rs`
- [ ] **3.** Implement `OrderEvent` and `BinaryHeap`-based intra-tick event queue in `event_queue.rs`
- [ ] **4.** Define `Agent` trait in `agents/mod.rs`; define `AgentPool` with `Vec<Box<dyn Agent>>`
- [ ] **5.** Implement `SimState` in `engine.rs` with clock, books map, agent pool, candles, tape
- [ ] **6.** Implement tick loop: advance clock → collect agent decisions → drain event queue → accumulate candles
- [ ] **7.** Implement stale order cancellation: track each agent's pending order IDs with submission tick; cancel if `current_tick - submitted_tick > patience_ticks`
- [ ] **8.** Implement `NoiseTrader` in `agents/noise.rs`
- [ ] **9.** Create `src/bin/headless.rs`: parse args for N (agent count) and seed; spawn N noise traders on one stock; run 9:30–16:00 (390 minutes × 60,000 ticks); write `ohlcv.csv`
- [ ] **10.** CSV columns: `timestamp, open, high, low, close, volume, best_bid, best_ask, spread`

---

## Out of Scope

- Any strategic agents (HFT, retail, etc.)
- WASM or browser
- Community feed
- Options
- Multiple stocks
- COOP/COEP headers

---

## Manual Testing

- [ ] Run `cargo run --bin headless -- --agents 1000 --seed 42` and confirm `ohlcv.csv` is written with 390 rows (one per minute of regular session)
- [ ] Open `ohlcv.csv` and confirm spread column is non-zero throughout the day (a spread forms from noise trader activity)
- [ ] Run with `--seed 42` twice and confirm the two CSV files are byte-identical (determinism)
- [ ] Run with `--seed 43` and confirm the CSV differs from seed 42 (different randomness)
- [ ] Run with `--agents 100` vs `--agents 5000` and confirm spread is wider with fewer agents (less liquidity)
- [ ] Scan the high/low columns and confirm price moves throughout the day (not stuck at initial value)

---

## Green Light

- [ ] Approved
