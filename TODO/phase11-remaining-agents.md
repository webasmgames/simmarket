# TODO: Phase 11 — Remaining Agents

## Overview

Add the remaining agent archetypes: hedge fund, institutional investor, directional HFT (spoofing and momentum ignition), and the full dark pool mechanism with borrow pool depletion. This completes the agent population described in simulation-design.md and makes the full range of emergent behaviors possible — short squeezes, index arbitrage, institutional rebalancing flows, and adversarial HFT strategies the player can learn to recognize.

Before this phase: noise traders, HFT market makers, retail NPCs, player account, options engine. After this phase: full agent ecosystem; all emergent behaviors from the simulation design doc are mechanically present.

---

## Requirements

- [ ] `HedgeFundAgent` holds a fundamental value estimate (GBM-based); positions aggressively in proportion to gap between estimate and market price; executes via VWAP over simulated hours
- [ ] `InstitutionAgent` rebalances quarterly with stochastic timing offset; executes via VWAP with iceberg orders; creates U-shaped intraday volume when combined with other institutions
- [ ] `SpooferHft` holds a contra resting order on side A, places a large visible order on side B to move price, cancels the spoof when the contra fills
- [ ] `MomentumIgnitionHft` submits aggressive market orders to move price, then immediately flips and trades against the momentum created
- [ ] Dark pool mechanism: midpoint IOC execution, tape print after fill, price impact proportional to print size vs ADV, borrow pool depletion as shorts accumulate
- [ ] Borrow pool utilization in snapshot; locate exhaustion halts new short sales
- [ ] Correlated true value processes across stocks (for hedge fund long/short pairs and HFT stat arb)
- [ ] Participants panel shows all new agent types with positions and P&L

---

## Design

```rust
// src/sim/agents/hedge_fund.rs
struct HedgeFundAgent {
    account: Account,
    stock_id: StockId,
    fundamental_estimate: f64,    // GBM: updated on events + noise
    estimate_noise: f64,          // agent-specific error term
    target_position: i32,         // sign(estimate - price) * position_scale
    vwap_state: VwapExecutor,     // spreads execution over N ticks
}

// src/sim/agents/institution.rs
struct InstitutionAgent {
    account: Account,
    stock_id: StockId,
    target_weight: f32,           // fraction of portfolio in this stock
    rebalance_schedule: SimTime,  // next rebalance time (stochastic offset ±2 days)
    vwap_executor: VwapExecutor,
    iceberg_display_qty: u32,
}

// src/sim/agents/hft.rs (extend existing file)
struct SpooferHft {
    account: Account,
    stock_id: StockId,
    contra_order_id: Option<OrderId>,   // resting order on side A
    spoof_order_id: Option<OrderId>,    // large visible order on side B
    state: SpooferState,
}

enum SpooferState { Idle, ContraPlaced, SpoofPlaced, Exiting }

struct MomentumIgnitionHft {
    account: Account,
    stock_id: StockId,
    ignition_position: i32,      // shares accumulated from aggressive orders
    state: MiState,
}

enum MiState { Idle, Igniting, Flipping }
```

**Correlated true value**: each stock has a `true_value` GBM. Add a shared market factor: `dV_i = β_i * dM + ε_i` where `dM` is a common factor shock and `ε_i` is idiosyncratic. Hedge fund stat arb watches the spread between long and short legs.

**Borrow pool depletion**: track `short_interest_shares` per stock. When `short_interest_shares / float > 0.5`, locate availability drops; above 0.8, locate becomes scarce (probabilistic rejection); above 0.95, new short sales are blocked.

---

## Files

| File | Action | Notes |
|---|---|---|
| `src/sim/agents/hedge_fund.rs` | Create | `HedgeFundAgent`, `VwapExecutor` |
| `src/sim/agents/institution.rs` | Create | `InstitutionAgent` |
| `src/sim/agents/hft.rs` | Modify | Add `SpooferHft`, `MomentumIgnitionHft` |
| `src/sim/dark_pool.rs` | Create | Dark pool crossing engine, borrow pool, locate logic |
| `src/sim/true_value.rs` | Create | Correlated GBM true value processes with common factor |
| `src/shared/snapshot.rs` | Modify | Add `borrow_utilization_pct` to `StockSnapshot` |
| `src/sim/engine.rs` | Modify | Tick true value process; tick borrow pool; wire new agents |

---

## Tasks

- [ ] **1.** Implement correlated GBM true value in `true_value.rs`: common market factor + per-stock idiosyncratic; updated each simulated minute
- [ ] **2.** Implement `VwapExecutor`: slices a target position into small orders spread over N ticks, scaled to current volume; used by hedge fund and institution
- [ ] **3.** Implement `HedgeFundAgent`: compute fundamental estimate as true value + noise term; set target position proportional to `|(estimate - price) / estimate|`; use VwapExecutor for execution; cut if 15% drawdown
- [ ] **4.** Implement `InstitutionAgent`: rebalance trigger with stochastic ±2-day offset; iceberg order submission via VwapExecutor
- [ ] **5.** Implement `SpooferHft` state machine: Idle → ContraPlaced (place resting order side A) → SpoofPlaced (place large visible order side B) → wait for contra fill → Exiting (cancel spoof) → Idle
- [ ] **6.** Implement `MomentumIgnitionHft` state machine: Idle → Igniting (submit aggressive market orders until position hits threshold) → Flipping (submit opposing market orders to exit against created momentum) → Idle
- [ ] **7.** Implement borrow pool in `dark_pool.rs`: `short_interest_shares` counter; locate request logic with utilization-based rejection probability; fee rate schedule
- [ ] **8.** Implement dark pool crossing engine: attempt midpoint IOC against contra resting orders; emit `TapePrint` with `is_dark_pool: true`; compute price impact on lit market proportional to print size / ADV
- [ ] **9.** Add `borrow_utilization_pct` to `StockSnapshot`; display in debug stats bar
- [ ] **10.** Wire all new agents into `SimState` initialization; expose counts as config params
- [ ] **11.** Update participants panel to show hedge fund and institution agent types

---

## Out of Scope

- Game scenarios (separate layer on top of this simulation)
- Player UI for short selling (can be added to Phase 9 order form later)
- Cross-venue arbitrage (only one simulated exchange)

---

## Manual Testing

- [ ] Confirm hedge fund agent takes a position: watch participants panel — HF position should grow slowly over simulated hours, not instantly
- [ ] Confirm VWAP execution: watch tape during a period when HF is building a position — should see small regular prints of similar size rather than one large block
- [ ] Confirm institution rebalancing: at simulated quarter-end, institution should produce a burst of volume (visible in volume bars on chart)
- [ ] Watch for a spoofer event: large order appears suddenly on one side of the LOB (LOB panel shows a wall), then disappears within seconds — retail NPC sentiment or price should visibly react briefly
- [ ] Watch for momentum ignition: a burst of aggressive same-side trades in the tape (all B or all S), followed by rapid price reversal and a burst of opposite-side trades from the same agent
- [ ] Confirm borrow pool: check `borrow_utilization_pct` in stats bar when a short squeeze scenario has high short interest — should approach 100% and new short orders should fail (player can verify by trying to short sell and having the order rejected)
- [ ] Confirm dark pool prints: watch tape with `is_dark_pool` indicator — dark prints should appear, especially on large institutional orders

---

## Green Light

- [ ] Approved
