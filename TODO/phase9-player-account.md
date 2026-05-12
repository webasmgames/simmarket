# TODO: Phase 9 — Player Account + Order Routing Visualization

## Overview

Add the player as a Reg T retail account with a broker routing layer. The player submits orders via an order entry form in the debug app; orders route through a simulated broker agent that decides lit exchange vs dark pool; the routing path is visualized in real time. This is the "build.exe" moment — you can feel how your order interacts with the market.

Before this phase: fully observable market, no player participation. After this phase: player can place orders, watch them route, see fills in their portfolio panel, and observe how the market reacts to their presence.

---

## Requirements

- [ ] Player has a Reg T margin account with configurable starting cash (default $25,000)
- [ ] Buying power = 2× equity for marginable securities; maintenance margin 25%
- [ ] Player can submit: market, limit, stop, stop-limit orders; buy or sell side
- [ ] Player orders route through a `BrokerAgent` that decides routing: lit exchange or dark pool (midpoint IOC)
- [ ] Dark pool routing probability is configurable; scales with order size (larger orders more likely dark)
- [ ] Order routing is visualized: a brief highlight/annotation on the debug UI shows the path (Player → Broker → Exchange or Dark Pool) and the fill result
- [ ] Player's open orders are listed with cancel button
- [ ] Player portfolio panel: position, average cost, unrealized P&L, realized P&L, cash, buying power
- [ ] Margin call warning displayed if equity falls below 25% maintenance threshold
- [ ] Player order has a simulated execution latency of 100–200ms (behind HFTs in the event queue)
- [ ] Player fills appear in the tape with a distinct marker

---

## Design

```rust
// src/sim/agents/player.rs
struct PlayerAgent {
    account: Account,           // cash, margin, positions
    pending_orders: Vec<OrderId>,
    avg_cost: HashMap<StockId, f32>,
    realized_pnl: f32,
}

// src/sim/broker.rs
struct BrokerAgent {
    dark_pool_base_prob: f32,   // base probability of dark routing
    dark_pool_size_threshold: u32,  // orders above this size are more likely dark
}

impl BrokerAgent {
    fn route(&self, order: &Order, nbbo: (f64, f64), rng: &mut SmallRng) -> Route
}

enum Route { LitExchange, DarkPool }

// Dark pool execution:
// If Route::DarkPool: attempt midpoint IOC fill; if no contra, fall back to lit exchange
// Print to tape with is_dark_pool = true
```

**Player order lifecycle**:
1. Player submits order via UI → `postMessage({type:"order", ...})` to worker
2. Worker receives message; creates `Order` with player's agent ID and 100–200ms latency offset
3. On the tick when the order's timestamp is reached: `BrokerAgent.route()` decides exchange vs dark
4. If dark: attempt midpoint IOC against dark pool; if unfilled, route to lit exchange
5. If lit: insert into LOB normally
6. Fill result posted back to main thread in next snapshot; routing path included

**Routing visualization**: snapshot includes a `last_player_route` field describing the most recent routing event (order ID, route taken, fill price, fill size, dark pool flag). JS renders a brief overlay annotation on the debug UI.

---

## Files

| File | Action | Notes |
|---|---|---|
| `src/sim/agents/player.rs` | Create | `PlayerAgent`, Reg T account logic, margin calculation |
| `src/sim/broker.rs` | Create | `BrokerAgent`, routing decision, dark pool execution |
| `src/shared/snapshot.rs` | Modify | Add `player_account: PlayerSnapshot`, `last_player_route: RouteEvent` |
| `src/lib.rs` | Modify | Handle `{type:"order"}` postMessage from main thread; enqueue into sim |
| `index.html` | Modify | Order entry form, open orders panel, portfolio panel, routing visualization overlay |

---

## Tasks

- [ ] **1.** Implement `PlayerAgent` with Reg T account: cash, margin, positions, buying power calculation
- [ ] **2.** Implement margin check on order submission: reject if order would exceed buying power
- [ ] **3.** Implement maintenance margin check per tick: if equity < 25% of position value, set `margin_call: true` in snapshot
- [ ] **4.** Implement `BrokerAgent` routing decision: `dark_prob = base_prob + size_bonus * (size / size_threshold)`; roll against it
- [ ] **5.** Implement dark pool execution: midpoint IOC at `(bid + ask) / 2`; if no contra available, fall back to lit; emit `TapePrint` with `is_dark_pool: true`
- [ ] **6.** Handle `{type:"order"}` postMessage in worker: parse order fields, create `Order` with player agent ID and latency offset, enqueue for the appropriate tick
- [ ] **7.** Add `PlayerSnapshot { cash: f32, buying_power: f32, position: i32, avg_cost: f32, unrealized_pnl: f32, realized_pnl: f32, open_orders: [OrderSummary; 16], margin_call: bool }` to snapshot
- [ ] **8.** Add `RouteEvent { order_id: u64, route: u8, fill_price: f32, fill_size: u32, dark: bool }` to snapshot
- [ ] **9.** Build order entry form in `index.html`: symbol selector, side (Buy/Sell), order type, quantity, price (optional); Submit button
- [ ] **10.** Build portfolio panel: position, avg cost, unrealized P&L, realized P&L, cash, buying power; red margin call warning when triggered
- [ ] **11.** Build open orders panel: list of resting orders with Cancel button (sends `{type:"cancel", order_id}` to worker)
- [ ] **12.** Build routing visualization: when `last_player_route` changes, show a 2-second overlay "Order → Broker → [Exchange / Dark Pool] → Filled at $X.XX"

---

## Out of Scope

- PDT rule enforcement (later)
- Options order entry (Phase 10)
- Short selling order entry (later)
- Multiple stocks in the order form

---

## Manual Testing

- [ ] Submit a market buy order for 100 shares — confirm it fills immediately at ask price and appears in portfolio panel
- [ ] Submit a limit buy order 5 ticks below current bid — confirm it rests in the LOB (visible in LOB panel) and eventually fills as price drifts down
- [ ] Submit a large market order (1,000+ shares) — confirm some fills route to dark pool (routing visualization shows "Dark Pool") and appear in tape with dark pool marker
- [ ] Watch routing visualization overlay — confirm it appears for 2 seconds after each fill and correctly describes the route taken
- [ ] Deplete buying power by entering a large position; attempt another order — confirm it is rejected with a buying power message
- [ ] Let a position go against you until equity nears 25% of position value — confirm margin call warning appears in portfolio panel
- [ ] Cancel a resting limit order — confirm it disappears from the LOB panel and open orders list

---

## Green Light

- [ ] Approved
