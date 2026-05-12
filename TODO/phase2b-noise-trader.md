# TODO: Phase 2b — Noise Trader Agent

## Overview

Add the `Agent` trait, `AgentPool`, and the first agent archetype: the noise trader. Wire agents into the tick loop from Phase 2a, add stale order cancellation, and produce the headless binary that writes `ohlcv.csv` after one simulated trading day. The goal is to confirm that a spread forms and price diffuses from noise alone before any strategic agents are added.

Before this phase: a tick engine with no agents. After this phase: `cargo run --bin headless -- --agents 1000 --seed 42` simulates a full day and writes `ohlcv.csv`.

---

## Requirements

- [ ] `Agent` trait with `schedule_interval_ticks()`, `latency_offset_us()`, `observe()`, and `decide()`
- [ ] `AgentPool` holds `Vec<Box<dyn Agent>>`; tick loop iterates it and collects `OrderEvent`s
- [ ] `MarketSnapshot` passed to agents: last price, best bid, best ask, bid size, ask size (read-only)
- [ ] `NoiseTrader`: 50/50 buy/sell; price = midpoint ± random(1–3) ticks; cancels its pending order before each new submission
- [ ] Stale order cancellation: cancel any noise trader order older than `patience_ticks`
- [ ] Seeded PRNG per agent — same seed produces identical output; `SmallRng` from `rand`
- [ ] Headless binary runs 9:30–16:00 (390 minutes × 60,000 ticks at 1ms/tick)
- [ ] `ohlcv.csv` has 390 rows, columns: `timestamp, open, high, low, close, volume, best_bid, best_ask, spread`

---

## Design

### Data Structures

```rust
// src/shared/types.rs additions
struct Account {
    agent_id: AgentId,
    cash: f64,
    position: i32,  // shares held; negative = short
}

// src/sim/agents/mod.rs
struct MarketSnapshot {
    stock_id: StockId,
    last_price: f64,
    best_bid: Option<f64>,
    best_ask: Option<f64>,
    bid_size: u32,
    ask_size: u32,
    clock: SimTime,
}

struct Observation {
    snapshot: MarketSnapshot,
}

trait Agent {
    fn agent_id(&self) -> AgentId;
    fn schedule_interval_ticks(&self) -> u64;
    fn latency_offset_us(&self) -> u32;
    fn observe(&self, market: &MarketSnapshot) -> Observation;
    fn decide(&mut self, obs: &Observation, rng: &mut SmallRng) -> Vec<OrderAction>;
}

// src/sim/agents/noise.rs
struct NoiseTrader {
    id: AgentId,
    stock_id: StockId,
    account: Account,
    patience_ticks: u64,
    order_size: u32,
    pending_order: Option<(OrderId, SimTime)>,  // (id, tick submitted)
    next_order_id: OrderId,
}
```

### Key Logic

**Tick loop extension** (in `SimState::tick()`):
1. For each agent due this tick (`clock % agent.schedule_interval_ticks() == 0`): build `MarketSnapshot` → call `observe()` → `decide()` → push returned `OrderAction`s as `OrderEvent`s with the agent's `latency_offset_us`
2. Before `decide()`, check stale cancellation: if `pending_order` age > `patience_ticks`, inject a `Cancel` event

**NoiseTrader decide()**:
- If pending order exists, emit `Cancel(pending_order_id)` first
- 50% chance buy, 50% sell (from RNG)
- Price = midpoint ± `rng.gen_range(1..=3)` ticks (one tick = $0.01)
- Emit `Submit(order)` and record new order ID + current tick as pending

**MarketSnapshot** is built from the LOB state at the start of the tick (before events drain).

**Headless binary** args: `--agents <N>` (default 500), `--seed <u64>` (default 0). Agents are assigned sequential IDs and seeds derived from the base seed. Session: 9:30:00–16:00:00 simulated time.

---

## Files

| File | Action | Notes |
|---|---|---|
| `src/shared/types.rs` | Modify | Add `Account` |
| `src/sim/agents/mod.rs` | Create | `Agent` trait, `AgentPool`, `MarketSnapshot`, `Observation` |
| `src/sim/agents/noise.rs` | Create | `NoiseTrader` implementation |
| `src/sim/engine.rs` | Modify | Wire `AgentPool` into tick loop; add `MarketSnapshot` construction |
| `src/sim/mod.rs` | Modify | Add `pub mod agents;` |
| `Cargo.toml` | Modify | Add `rand` dep with `small_rng` feature |
| `src/bin/headless.rs` | Create | CLI entry point; spawns agents; runs session; writes CSV |

---

## Tasks

- [ ] **1.** Add `rand` to `Cargo.toml` with features `["small_rng"]`
- [ ] **2.** Add `Account` to `src/shared/types.rs`
- [ ] **3.** Define `Agent` trait, `AgentPool`, `MarketSnapshot`, `Observation` in `src/sim/agents/mod.rs`
- [ ] **4.** Implement `NoiseTrader` in `src/sim/agents/noise.rs`
- [ ] **5.** Add stale cancellation logic: in tick loop, check each agent's pending order age before calling `decide()`
- [ ] **6.** Wire `AgentPool` into `SimState`; build `MarketSnapshot` at tick start; collect agent `OrderEvent`s and push to queue
- [ ] **7.** Create `src/bin/headless.rs`: parse `--agents` and `--seed`; spawn noise traders; run 9:30–16:00; write `ohlcv.csv`
- [ ] **8.** CSV columns: `timestamp, open, high, low, close, volume, best_bid, best_ask, spread`
- [ ] **9.** Write unit tests: `NoiseTrader::decide()` always emits a cancel before a new submit when a pending order exists; price is within midpoint ± 3 ticks; same seed produces identical action sequence

---

## Out of Scope

- Any strategic agents (HFT, retail, hedge fund)
- WASM or browser
- Community feed / SIR model
- Options
- Multiple stocks
- Reg T account enforcement (account struct exists but balances are not enforced)

---

## Manual Testing

- [ ] Run `cargo run --bin headless -- --agents 1000 --seed 42` and confirm `ohlcv.csv` is written with 390 rows
- [ ] Confirm spread column is non-zero throughout the day
- [ ] Run with `--seed 42` twice; confirm the two CSV files are byte-identical
- [ ] Run with `--seed 43`; confirm CSV differs from seed 42
- [ ] Run with `--agents 100` vs `--agents 5000`; confirm spread is wider with fewer agents
- [ ] Scan high/low columns; confirm price moves throughout the day

---

## Green Light

- [ ] Approved
