# TODO: Phase 7 — HFT Market Maker + Participants Panel

## Overview

Add the market-making HFT agent. These agents post two-sided quotes with a 95–99% cancel rate, compress the spread via competition, and withdraw when adverse selection pressure rises. Also add the participants panel to the debug app so agent positions and P&L are visible in real time.

Before this phase: noise trader market with wide, uncompressed spreads and no strategic behavior. After this phase: a market with HFT market makers visibly competing, tight spreads during calm periods, withdrawal and spread widening during volatile ones, and a panel showing every agent's book.

---

## Requirements

- [ ] `MarketMakingHft` agent posts two-sided limit orders 1 tick inside current best bid/ask
- [ ] Cancel-to-trade ratio is 95–99%: agent cancels and reposts quotes every tick; only a small fraction of submitted orders actually fill
- [ ] Intra-tick latency offset for HFT is 1–50 µs (runs before all other agent types in the event queue)
- [ ] Spread compresses noticeably vs. noise-trader-only baseline when 20+ HFT market makers are active
- [ ] Adverse selection signal: agent tracks rolling net signed volume (last 20 trades); widens spread when imbalance is high
- [ ] Inventory limit: ±500 shares; agent switches to one-sided quoting when limit is hit
- [ ] Agent P&L tracked per tick (unrealized + realized)
- [ ] Participants panel in debug app: lists all agents by type, current position (shares), cash, P&L; updates live

---

## Design

```rust
// src/sim/agents/hft.rs
struct MarketMakingHft {
    account: Account,
    stock_id: StockId,
    inventory: i32,          // positive = long, negative = short
    pending_bid_id: Option<OrderId>,
    pending_ask_id: Option<OrderId>,
    signed_volume_window: VecDeque<i32>,  // +size for buy, -size for sell; last 20 trades
    rng: SmallRng,
}

impl Agent for MarketMakingHft {
    fn latency_offset_us(&self) -> u32 { 1 + rng.gen_range(0..50) }
    fn schedule_interval_ticks(&self) -> u64 { 1 }  // runs every tick

    fn decide(&mut self, obs: &Observation, rng: &mut SmallRng) -> Vec<OrderAction> {
        // 1. Cancel pending quotes
        // 2. Compute spread width = base_spread + inventory_skew + adverse_selection_skew
        // 3. If inventory within limits: submit new bid and ask
        // 4. If inventory at limit: submit only the side that reduces inventory
    }
}
```

**Spread formula**:
```
half_spread = base_ticks                           // e.g. 1 tick minimum
            + abs(inventory) * inventory_skew_k   // wider when inventory skewed
            + signed_vol_imbalance * adv_sel_k    // wider when flow is toxic
```

**Participants snapshot**: extend `SimSnapshot` with a `participants` array summarizing each agent's type, position, cash, and P&L. Serialized into the postMessage JSON and rendered as a table.

---

## Files

| File | Action | Notes |
|---|---|---|
| `src/sim/agents/hft.rs` | Create | `MarketMakingHft` implementation |
| `src/shared/snapshot.rs` | Modify | Add `ParticipantSnapshot` array to `SimSnapshot` |
| `src/sim/engine.rs` | Modify | Update signed volume window from tape each tick; populate participant snapshots |
| `index.html` | Modify | Add participants panel (agent list table) |
| `worker/sim_worker.js` | Modify | Include participants in postMessage JSON |

---

## Tasks

- [ ] **1.** Implement `MarketMakingHft` struct and `Agent` impl in `src/sim/agents/hft.rs`
- [ ] **2.** Implement cancel-and-reprice logic: cancel `pending_bid_id` and `pending_ask_id` each tick before reposting
- [ ] **3.** Implement spread formula with inventory skew and adverse selection components
- [ ] **4.** Implement one-sided quoting when `abs(inventory) >= INVENTORY_LIMIT`
- [ ] **5.** Track signed volume window in `SimState`: on each `Trade`, push `+size` (buyer-initiated) or `-size` (seller-initiated) to a per-stock `VecDeque<i32>` capped at 20 entries
- [ ] **6.** Track per-agent P&L: realized P&L from fills + unrealized mark-to-market from last price
- [ ] **7.** Add `ParticipantSnapshot { agent_type: u8, position: i32, cash: f32, pnl: f32 }` array to `SimSnapshot`
- [ ] **8.** Populate participant snapshots in `write_shared_buffer()` / postMessage JSON
- [ ] **9.** Add participants table to `index.html`: columns Agent Type | Position | Cash | P&L; rows update live
- [ ] **10.** Spawn 20 `MarketMakingHft` instances in the sim alongside noise traders; confirm spread compresses

---

## Out of Scope

- Traditional (designated) market maker
- Directional HFT (spoofing, momentum ignition) — Phase 11
- Options delta hedging
- Inter-stock stat arb

---

## Manual Testing

- [ ] Load debug app with 20 HFT market makers + 5,000 noise traders; confirm spread in stats bar is visibly tighter than noise-trader-only baseline (run Phase 2 first to record baseline spread, compare)
- [ ] Watch participants panel — confirm HFT agents show fluctuating position (never stuck at 0 or at inventory limit continuously)
- [ ] Watch LOB panel during a price move — confirm HFT bids/asks reprice rapidly (book "shimmers" as quotes cancel and repost)
- [ ] Manually trigger high signed-volume imbalance by watching tape for a run of buyer-initiated trades; confirm spread in stats bar widens during that period
- [ ] Check an HFT agent's P&L after 10 simulated minutes — confirm it is positive on average (market making is profitable against noise traders)
- [ ] Confirm no agent's position exceeds ±500 shares (inventory limit enforced)

---

## Green Light

- [ ] Approved
