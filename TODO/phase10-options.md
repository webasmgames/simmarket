# TODO: Phase 10 — Options Engine

## Overview

Add the options layer: chain generation, Black-Scholes pricing with Greeks, a parametric IV surface, and market maker delta hedging (threshold-based, aggregate book). Add an options chain panel to the debug app. This phase makes the gamma squeeze mechanism live — you can watch the market maker's hedging flows push price as the chain accumulates delta.

Before this phase: equity-only market. After this phase: full options chain visible in debug app, Greeks updating live, market maker delta hedging visible in their position panel.

---

## Requirements

- [ ] Options chain generated at session open: weekly and monthly expirations, ATM ± 10 strikes
- [ ] Black-Scholes pricing computes bid/ask mid, delta, gamma, theta, vega per contract per tick
- [ ] IV surface stored as `[expiry_idx][strike_idx] → f32`; initialized with structural negative skew (puts > calls)
- [ ] IV updates per tick: per-strike net signed options flow moves that strike's IV; mean-reversion toward structural skew
- [ ] IV crush: after a binary event tick, IV collapses 40–70% across the chain
- [ ] Market maker delta hedges aggregate net book delta on a threshold basis (`|net_delta| > hedge_band`)
- [ ] Hedging buys/sells equity shares; trades appear in the tape and move the stock price
- [ ] Options chain panel in debug app: table of strikes with bid, ask, IV, delta, gamma, theta; toggle call/put; selectable expiry
- [ ] Player can buy and sell options via the order entry form (extend Phase 9 UI)

---

## Design

```rust
// src/sim/options.rs
struct OptionsChain {
    stock_id: StockId,
    expirations: Vec<ExpiryDate>,    // sorted ascending
    strikes: Vec<f64>,               // ATM ± 10, sorted ascending
    iv_surface: Vec<Vec<f32>>,       // [expiry_idx][strike_idx]
    contracts: Vec<Vec<OptionsContract>>,  // [expiry_idx][strike_idx] × 2 (call+put)
}

struct OptionsContract {
    is_call: bool,
    expiry: ExpiryDate,
    strike: f64,
    open_interest: u32,
    // Greeks recomputed each tick:
    mid_price: f32,
    delta: f32,
    gamma: f32,
    theta: f32,
    vega: f32,
    iv: f32,
}

// Market maker book delta:
struct MmOptionsBook {
    net_delta: f32,              // aggregate delta exposure from all options positions
    hedge_band: f32,             // hedge when |net_delta| exceeds this
    last_hedge_delta: f32,       // delta at last hedge; rebalance when drift exceeds band
}
```

**Black-Scholes** (per contract per tick):
- Standard closed-form for call/put price, delta, gamma, theta, vega
- IV input from `iv_surface[expiry][strike]`
- Greeks vectorizable — compute all contracts in a tight loop (no GPU needed at this scale)

**IV update per tick**:
```
iv_surface[e][s] += net_signed_options_flow[e][s] * flow_impact_k
iv_surface[e][s] += (structural_skew[e][s] - iv_surface[e][s]) * mean_reversion_k
```

**Threshold hedging**:
```
if abs(mm_book.net_delta - mm_book.last_hedge_delta) > hedge_band:
    shares_to_trade = round(mm_book.net_delta - mm_book.last_hedge_delta)
    submit market order for shares_to_trade (buy if positive, sell if negative)
    mm_book.last_hedge_delta = mm_book.net_delta
```

---

## Files

| File | Action | Notes |
|---|---|---|
| `src/sim/options.rs` | Create | `OptionsChain`, `OptionsContract`, Black-Scholes, IV surface, `MmOptionsBook` |
| `src/sim/agents/market_maker.rs` | Create | `TraditionalMarketMaker` with options book and threshold delta hedging |
| `src/shared/snapshot.rs` | Modify | Add `OptionsChainSnapshot` (condensed chain for display) |
| `src/sim/engine.rs` | Modify | Tick options chain update; compute Greeks; trigger MM hedge if threshold exceeded |
| `index.html` | Modify | Options chain panel; extend order entry form for options |

---

## Tasks

- [ ] **1.** Implement Black-Scholes in `options.rs`: `bs_price(spot, strike, t, r, iv, is_call) -> f32` and `bs_greeks(...) -> (delta, gamma, theta, vega)`
- [ ] **2.** Implement normal CDF approximation (Abramowitz & Stegun or similar) needed by Black-Scholes
- [ ] **3.** Implement `OptionsChain::generate(spot, expirations, n_strikes)`: build strike grid centered at ATM; initialize IV surface with structural skew (puts 5–15% higher IV than equidistant calls)
- [ ] **4.** Implement per-tick Greeks recomputation loop across all contracts
- [ ] **5.** Implement IV update: per-strike net signed options flow tracking; mean-reversion toward structural skew; IV crush on event tick
- [ ] **6.** Implement `MmOptionsBook` delta tracking: sum delta × open_interest across all MM short positions each tick
- [ ] **7.** Implement threshold hedging: check `|net_delta - last_hedge_delta| > hedge_band`; if true, submit equity market order for the difference; update `last_hedge_delta`
- [ ] **8.** Implement `TraditionalMarketMaker` agent: quotes options contracts (bid = BS_mid - half_spread, ask = BS_mid + half_spread); accumulates book; hedges
- [ ] **9.** Add condensed `OptionsChainSnapshot` to `SimSnapshot`: top 5 strikes × 2 expirations, call and put, with Greeks
- [ ] **10.** Add options chain panel to `index.html`: table with Strike | Call IV | Call Delta | Call Bid/Ask | Put Bid/Ask | Put Delta | Put IV; expiry selector; call/put toggle
- [ ] **11.** Extend order entry form: add option type (Call/Put), strike, expiry selectors when "Options" mode toggled
- [ ] **12.** Wire player options fills to portfolio panel: show options positions with contract details and Greeks

---

## Out of Scope

- 0DTE options (can be added as a scenario parameter later)
- GPU compute for Greeks (CPU loop is fast enough for this chain size)
- Full options LOB (parametric IV surface is sufficient)
- Vanna/charm in hedging (listed in sim design for accuracy but deferred from this phase)

---

## Manual Testing

- [ ] Open options chain panel — confirm strikes are listed ATM ± 10, Greeks are non-zero and plausible (delta between 0 and 1 for calls, −1 and 0 for puts)
- [ ] Confirm IV skew: compare put IV at −5 strikes vs call IV at +5 strikes — puts should have higher IV
- [ ] Buy 10 call contracts OTM; watch market maker's position in participants panel — confirm their net delta changes in the negative direction (they're short delta)
- [ ] Watch for a delta hedge event: with 10+ contracts accumulated, the MM's net delta should eventually exceed the hedge band and trigger an equity buy (visible in tape as a market order)
- [ ] Trigger an IV crush: wait for a simulated earnings event; confirm IV collapses visibly across the chain (all IVs drop ~50%)
- [ ] Confirm options P&L in portfolio panel updates correctly as underlying price moves (call value rises with price)

---

## Green Light

- [ ] Approved
