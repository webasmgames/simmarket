# TODO: Phase 1 — Limit Order Book + Matching Engine

## Overview

Build the core exchange data structure and matching algorithm in pure Rust — no agents, no WASM, no UI. This is the foundation everything else runs on. The LOB holds resting buy (bid) and sell (ask) orders sorted by price-time priority. The matching engine drains incoming orders against the book, handles partial fills, and recursively resolves stop cascades.

Before this phase the project is empty. After this phase there is a tested, benchmarked matching engine that accepts orders and produces fills.

---

## Requirements

- [ ] Limit orders rest in the book at their specified price; FIFO within a price level
- [ ] Market orders fill immediately against the best available price, sweeping multiple levels if needed
- [ ] Partial fills are handled correctly: remainder either rests (limit) or cancels (IOC)
- [ ] Stop orders trigger when their stop price is touched during a sweep, becoming market orders
- [ ] Stop-limit orders trigger when their stop price is touched, becoming limit orders
- [ ] Stop cascades are resolved recursively within a single matching event (not deferred to next tick)
- [ ] IOC orders fill what's available and cancel the remainder immediately
- [ ] FOK orders fill entirely or cancel entirely — no partial fills
- [ ] Iceberg orders display only `display_qty` shares; hidden reserve replenishes automatically after each fill
- [ ] Each fill produces a `Trade` record with price, size, aggressor side, and both agent IDs
- [ ] Unit tests cover all order types, partial fills, multi-level sweeps, and the stop cascade

---

## Design

### Data Structures

```rust
// src/shared/types.rs
type OrderId = u64;
type AgentId = u32;
type SimTime = u64;  // microseconds of simulated time

enum Side { Bid, Ask }

enum OrderType {
    Market,
    Limit,
    Stop { stop_price: f64 },
    StopLimit { stop_price: f64 },
    Ioc,
    Fok,
    Iceberg { display_qty: u32, hidden_qty: u32 },
}

struct Order {
    id: OrderId,
    agent_id: AgentId,
    side: Side,
    order_type: OrderType,
    price: f64,        // limit price; 0.0 for market
    quantity: u32,
    filled: u32,
    submitted_at: SimTime,
    gtc: bool,
}

struct Trade {
    aggressor_order_id: OrderId,
    resting_order_id: OrderId,
    aggressor_agent: AgentId,
    resting_agent: AgentId,
    price: f64,
    size: u32,
    aggressor_side: Side,
    time: SimTime,
}
```

```rust
// src/sim/exchange.rs
struct LimitOrderBook {
    bids: BTreeMap<NotNan<f64>, VecDeque<Order>>,  // descending (best bid = max key)
    asks: BTreeMap<NotNan<f64>, VecDeque<Order>>,  // ascending (best ask = min key)
    stop_bids: BTreeMap<NotNan<f64>, Vec<Order>>,  // triggered when ask falls to or below stop
    stop_asks: BTreeMap<NotNan<f64>, Vec<Order>>,  // triggered when bid rises to or above stop
}
```

### Key Logic

**Matching** (called for each incoming aggressive order):
1. Determine opposite side of book
2. Walk from best price inward, filling until order is complete or book exhausted
3. For each matched resting order: emit `Trade`, update filled quantities, remove if fully filled
4. Collect all stop orders whose stop price was crossed by this sweep
5. Convert triggered stops to market/limit orders and recurse — repeat until no new stops trigger
6. Return vec of `Trade` records and any unfilled remainder

**Iceberg replenishment**: after each fill that depletes the displayed quantity, automatically move `min(hidden_qty, display_qty)` shares from hidden to displayed. Replenished shares go to the back of the FIFO queue at that price level (they lose time priority).

**FOK**: attempt a full dry-run match before committing. If the book cannot fill the entire quantity, cancel without executing any fills.

---

## Files

| File | Action | Notes |
|---|---|---|
| `Cargo.toml` | Create | Workspace root; single crate `simmarket`; deps: `ordered-float`, `thiserror` |
| `src/lib.rs` | Create | Crate root; `pub mod sim; pub mod shared;` |
| `src/shared/mod.rs` | Create | `pub mod types;` |
| `src/shared/types.rs` | Create | All shared types: OrderId, AgentId, SimTime, Side, OrderType, Order, Trade |
| `src/sim/mod.rs` | Create | `pub mod exchange;` |
| `src/sim/exchange.rs` | Create | LimitOrderBook, matching algorithm |
| `src/sim/exchange_tests.rs` | Create | Unit tests (inline or separate file via `#[cfg(test)]`) |

---

## Tasks

- [x] **1.** Create `Cargo.toml` with crate name `simmarket`, edition 2021, deps `ordered-float` and `thiserror`
- [x] **2.** Create `src/lib.rs`, `src/shared/mod.rs`, `src/sim/mod.rs` module stubs
- [x] **3.** Define all types in `src/shared/types.rs`: `OrderId`, `AgentId`, `SimTime`, `Side`, `OrderType`, `Order`, `Trade`
- [x] **4.** Implement `LimitOrderBook` struct with BTreeMap bid/ask sides and stop order buckets
- [x] **5.** Implement `insert_limit()` — add a resting limit order to the correct price level FIFO queue
- [x] **6.** Implement `cancel()` — remove an order by ID from the book
- [x] **7.** Implement `match_order()` — core sweep: walk opposite side, fill, collect trades, return remainder
- [x] **8.** Implement stop trigger collection during sweep: gather all stops whose price was crossed
- [x] **9.** Implement recursive stop cascade: convert triggered stops and re-enter `match_order()`; cap recursion at depth 64
- [x] **10.** Implement IOC semantics: after sweep, cancel any unfilled remainder
- [x] **11.** Implement FOK semantics: dry-run check before committing fills
- [x] **12.** Implement iceberg display/hidden split and auto-replenishment after each partial fill
- [x] **13.** Write unit tests: limit order resting, market order sweep, multi-level sweep, partial fill, IOC cancel, FOK all-or-nothing, stop trigger, stop cascade (chain of stops), iceberg replenishment

---

## Out of Scope

- MOO / MOC / LOO / LOC order types (need auction mechanism — later)
- Pegged orders (need live market context — later)
- Agents of any kind
- WASM compilation
- Maker-taker fee accounting
- Intra-tick event queue and latency offsets (Phase 2)
- Opening / closing auction (later)

---

## Manual Testing

- [ ] Run `cargo test` — all tests pass with no warnings
- [ ] Run `cargo test -- --nocapture` on the stop cascade test and confirm the trade log shows the full chain of triggered stops printing in order
- [ ] Add 5 bid limit orders at prices $9.95, $9.96, $9.97, $9.98, $9.99 (100 shares each) and one ask market order for 350 shares; confirm 3 full fills + 1 partial fill at $9.97, remainder 50 shares gone (market order exhausted)
- [ ] Submit a FOK for 600 shares against the same book (only 500 available); confirm zero fills and order cancelled
- [ ] Submit an iceberg ask (display 100, hidden 400) at $10.00; hit it with a market bid for 250 shares; confirm 3 fills of 100 (2 replenishments) and 150 hidden remaining

---

## Green Light

- [x] Approved
