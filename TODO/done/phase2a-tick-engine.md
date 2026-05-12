# TODO: Phase 2a — Tick Engine

## Overview

Build the simulation runtime: simulated clock, intra-tick event queue, tick loop, and OHLCV candle accumulation. No agents yet — the tick loop simply drains whatever `OrderEvent`s are pushed onto the queue each tick. The engine is tested by injecting synthetic events directly.

Before this phase: a tested LOB with no runtime. After this phase: a tick loop that advances simulated time, drains ordered events into the LOB, and accumulates candles — ready for agents to plug into.

---

## Requirements

- [x] Simulated clock advances by 1ms per tick; configurable tick size
- [x] Intra-tick event queue holds `OrderEvent`s tagged with microsecond offsets; drains in ascending offset order
- [x] Tick loop: advance clock → drain event queue into LOB → accumulate trades into candles
- [x] OHLCV candles accumulate at 1-minute resolution
- [x] `SimState` owns the clock, the LOB map (keyed by `StockId`), the event queue, candles, and tape
- [x] Unit tests verify: events drain in offset order, partial-tick events don't bleed into next tick, candles accumulate open/high/low/close/volume correctly

---

## Design

### Data Structures

```rust
// src/shared/types.rs additions
type StockId = u32;

struct Candle {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: u64,
    sim_time: SimTime,
}

// src/sim/event_queue.rs
struct OrderEvent {
    intra_tick_offset_us: u32,  // determines drain order within a tick
    agent_id: AgentId,
    action: OrderAction,
}

enum OrderAction {
    Submit(Order),
    Cancel(OrderId),
}

// BinaryHeap<OrderEvent> ordered by intra_tick_offset_us ascending (min-heap)

// src/sim/engine.rs
struct SimState {
    clock: SimTime,
    tick_size_us: u64,
    books: HashMap<StockId, LimitOrderBook>,
    event_queue: EventQueue,
    candles: HashMap<StockId, Vec<Candle>>,
    tape: Vec<Trade>,
}
```

### Key Logic

**Tick loop** (`SimState::tick()`):
1. Drain `event_queue` in `intra_tick_offset_us` order, submitting each action to the relevant LOB
2. Collect all trades produced; append to tape
3. Update open/high/low/close/volume for the current minute's candle
4. Advance `clock` by `tick_size_us`

**Candle accumulation**: keyed by `(stock_id, minute_index)` where `minute_index = clock / 60_000_000`. On first trade of a minute, open = trade price. Each trade updates high/low. Close = last trade price. Volume accumulates.

**Event queue**: wrap `BinaryHeap` with a newtype; `push(event)` and `drain() -> Vec<OrderAction>` sorted ascending by offset. Queue is emptied each tick.

---

## Files

| File | Action | Notes |
|---|---|---|
| `src/shared/types.rs` | Modify | Add `StockId`, `Candle` |
| `src/sim/event_queue.rs` | Create | `OrderEvent`, `OrderAction`, `EventQueue` |
| `src/sim/engine.rs` | Create | `SimState`, `tick()` |
| `src/sim/mod.rs` | Modify | Add `pub mod engine; pub mod event_queue;` |

---

## Tasks

- [x] **1.** Add `StockId` (type alias `u32`) and `Candle` struct to `src/shared/types.rs`
- [x] **2.** Implement `OrderEvent`, `OrderAction`, and `EventQueue` in `src/sim/event_queue.rs`; `EventQueue::push()` and `EventQueue::drain_sorted() -> Vec<(u32, AgentId, OrderAction)>`
- [x] **3.** Implement `SimState` in `src/sim/engine.rs` with clock, tick_size_us, books, event_queue, candles, tape
- [x] **4.** Implement `SimState::tick()`: drain queue in offset order → submit to LOB → collect trades → update candles → advance clock
- [x] **5.** Implement candle accumulation: open/high/low/close/volume per stock per minute
- [x] **6.** Update `src/sim/mod.rs` to pub mod the new modules
- [x] **7.** Write unit tests: events drain in offset order regardless of insertion order; two events at the same offset drain in insertion order; candle open/high/low/close/volume are correct after a sequence of trades; clock advances by tick_size_us each tick

---

## Out of Scope

- Agent trait or any agent implementations
- Headless binary or CSV output
- Multiple stocks (SimState can support the map but tests use one stock)
- WASM or browser
- PRNG / seeding

---

## Manual Testing

- [x] In a unit test or scratch binary, push 5 `OrderEvent`s with offsets `[300, 100, 500, 200, 400]` and confirm they drain as `[100, 200, 300, 400, 500]`
- [x] Run a tick with two crossing orders and confirm the resulting trade appears in the tape and updates the candle

---

## Green Light

- [x] Approved
