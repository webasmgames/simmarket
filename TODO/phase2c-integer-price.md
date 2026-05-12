# TODO: Phase 2c — Integer Price Representation

## Overview

Replace the LOB's `NotNan<f64>` price keys with a fixed-point integer type (`Price = i64`, 1 unit = $0.0001). This matches how real exchanges represent prices: NASDAQ's ITCH 5.0 feed encodes prices as 4-decimal fixed-point integers, and SEC Rule 612 sets $0.0001 as the minimum price increment for sub-dollar stocks and $0.01 for stocks ≥ $1. Using i64 keys eliminates floating-point key comparison bugs in the BTreeMap and makes the LOB's price arithmetic exact.

Before this phase: the LOB uses `BTreeMap<NotNan<f64>, ...>` and accepts/emits `f64` prices throughout. After this phase: the LOB uses `BTreeMap<Price, ...>` internally; `Order.price` and `Trade.price` remain `f64` for the agent-facing API, with conversion at the LOB boundary. The `ordered_float` crate dependency is no longer needed by the LOB.

---

## Requirements

- [ ] `Price` type alias (`i64`, 1 unit = $0.0001) and conversion functions in `src/shared/types.rs`
- [ ] LOB bid, ask, stop_bids, and stop_asks maps all keyed by `Price`; `ordered_float` import removed from `exchange.rs`
- [ ] `Order.price` and `Trade.price` remain `f64`; conversion happens at LOB submit boundary only
- [ ] `best_bid()` and `best_ask()` return `Option<f64>` (unchanged signature)
- [ ] All existing LOB tests continue to pass unchanged

---

## Design

### Data Structures

```rust
// src/shared/types.rs additions
pub type Price = i64; // 1 unit = $0.0001; $10.01 → 100_100

pub const TICKS_PER_DOLLAR: i64 = 10_000;

pub fn to_price(dollars: f64) -> Price {
    (dollars * TICKS_PER_DOLLAR as f64).round() as i64
}

pub fn from_price(ticks: Price) -> f64 {
    ticks as f64 / TICKS_PER_DOLLAR as f64
}
```

```rust
// src/sim/exchange.rs — changed map key types only
struct LimitOrderBook {
    bids:      BTreeMap<Price, VecDeque<Order>>, // descending: best bid = max key
    asks:      BTreeMap<Price, VecDeque<Order>>, // ascending:  best ask = min key
    stop_asks: BTreeMap<Price, Vec<Order>>,       // buy stops: trigger when price rises to/above
    stop_bids: BTreeMap<Price, Vec<Order>>,       // sell stops: trigger when price falls to/below
}
```

### Key Logic

**Conversion boundary** — `submit()` is the only entry point for external prices. Convert once there:

```
insert_limit(order):  key = to_price(order.price)
insert_stop(order):   key = to_price(stop_price extracted from order.order_type)
sweep emit trade:     trade.price = from_price(key)   // back to f64 for Trade
price_crosses:        to_price(order.price) vs key (both Price)
collect_triggered_stops(trade_price_f64): convert to Price once, compare as integers
```

**No change to** `Order`, `Trade`, `best_bid()`, `best_ask()`, or any test helper — only the internal map key type and the handful of places that construct or compare keys.

---

## Files

| File | Action | Notes |
|---|---|---|
| `src/shared/types.rs` | Modify | Add `Price`, `TICKS_PER_DOLLAR`, `to_price()`, `from_price()` |
| `src/sim/exchange.rs` | Modify | Swap map keys to `Price`; remove `ordered_float` import; update `insert_limit`, `insert_stop`, `sweep`, `price_crosses`, `collect_triggered_stops`, `cancel_from_deque`, `cancel_from_vec` |

---

## Tasks

- [ ] **1.** Add `Price`, `TICKS_PER_DOLLAR`, `to_price()`, `from_price()` to `src/shared/types.rs`
- [ ] **2.** Change all four BTreeMap key types in `LimitOrderBook` from `NotNan<f64>` to `Price`
- [ ] **3.** Update `insert_limit` and `insert_stop` to convert `f64` price fields to `Price` via `to_price()`
- [ ] **4.** Update `sweep`: `best_key` is now `Price`; emit `trade.price = from_price(best_key)`; `price_crosses` compares `to_price(order.price)` against `Price` key
- [ ] **5.** Update `collect_triggered_stops`: convert `trade_price: f64` to `Price` once at entry, compare as integers
- [ ] **6.** Update `cancel_from_deque` and `cancel_from_vec` signatures to use `Price` key (or remove the key type annotation if inferred)
- [ ] **7.** Remove `use ordered_float::NotNan;` from `exchange.rs`
- [ ] **8.** Confirm all existing tests in `exchange.rs` and `engine.rs` pass without modification

---

## Out of Scope

- Changing `Order.price` or `Trade.price` to `Price` — the f64 agent-facing API is intentional
- Exposing `Price` to agents or `MarketSnapshot` — agents think in dollars, not ticks
- Sub-penny order rejection (SEC Rule 612 enforcement) — not needed for a simulation
- Removing `ordered_float` from `Cargo.toml` — it may be used by future code; leave the dependency

---

## Manual Testing

- [ ] Submit a limit bid at $10.01 and a limit ask at $10.01; confirm a trade fires (key equality is exact)
- [ ] Submit a limit ask at $10.00 + $0.01 (computed) and a market bid; confirm it matches the same level as a literal $10.01 ask
- [ ] Run `./preflight.sh` clean

---

## Green Light

- [ ] Approved
