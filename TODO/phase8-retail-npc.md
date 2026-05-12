# TODO: Phase 8 — Retail NPC + Community Feed

## Overview

Add the retail NPC archetype and the community feed simulation. Retail NPCs are momentum-driven, sentiment-influenced, and fear-sensitive. The SIR epidemic model governs how sentiment propagates through the retail population. The community feed generates synthetic posts from NPC positions and market events. A feed panel in the debug app shows the live stream of posts.

Before this phase: noise traders + HFT market makers, no directional herding. After this phase: visible momentum dynamics, sentiment waves, and a scrolling WSB-style feed of synthetic NPC posts.

---

## Requirements

- [ ] `RetailNpcAgent` submits momentum-following orders weighted by sentiment and price trend
- [ ] Order type selection: limit order 80% / market order 20% in normal state; market order 70% on FOMO trigger
- [ ] Position size: 5–25% of account value per trade, scaled by conviction signal
- [ ] FOMO trigger fires when price moves >3% without the agent; increases market order probability
- [ ] Fear trigger fires when position is down >10%; increases panic-sell probability
- [ ] Fear state scales order frequency and size down as a function of trailing realized volatility
- [ ] SIR model per stock: S (uninvested), I (invested/sentiment-driven), R (recently exited, cooling down)
- [ ] Sentiment score (−100 to +100) derived from size of I pool and recent post volume
- [ ] Community feed generates posts: position posts, gain/loss porn, meme/rocket posts
- [ ] Feed panel in debug app: scrolling feed of posts, newest at top
- [ ] Sentiment score displayed in stats bar per stock

---

## Design

```rust
// src/sim/agents/retail.rs
struct RetailNpcAgent {
    account: Account,
    stock_id: StockId,
    sir_state: SirState,     // Susceptible | Infected | Recovered
    position: i32,
    fear_state: f32,         // 0.0–1.0; scales activity down
    cooldown_ticks: u64,     // ticks remaining before Recovered → Susceptible
    rng: SmallRng,
}

enum SirState { Susceptible, Infected, Recovered }

// src/sim/community.rs
struct SirModel {
    susceptible: u32,
    infected: u32,
    recovered: u32,
    infection_rate: f32,     // β: S→I per tick per infected neighbor
    recovery_rate: f32,      // γ: I→R per tick
    cooldown_ticks: u64,     // R→S after this many ticks
}

struct CommunityFeed {
    posts: VecDeque<FeedPost>,   // capped at 500
}

struct FeedPost {
    author_id: AgentId,
    post_type: PostType,
    content: String,        // procedurally generated from template + sim data
    sim_time: SimTime,
}

enum PostType { Position, GainPorn, LossPorn, Meme, Reaction }
```

**Sentiment derivation**:
```
sentiment = clamp((infected / total_retail) * 200 - 100, -100, 100)
          + recent_post_velocity_bonus
```

**Post generation**: each tick, each Infected NPC has a small probability of generating a post. Post type is weighted by their P&L: positive P&L → position/gain porn; negative → loss porn/cope. Meme posts fire at random regardless of P&L.

---

## Files

| File | Action | Notes |
|---|---|---|
| `src/sim/agents/retail.rs` | Create | `RetailNpcAgent` implementation |
| `src/sim/community.rs` | Create | `SirModel`, `CommunityFeed`, post generation, templates |
| `src/shared/snapshot.rs` | Modify | Add `feed_ring: [FeedPost; 64]`, `feed_head: u32`, `sentiment: i16` to snapshot |
| `src/sim/engine.rs` | Modify | Tick SIR model; generate posts; update sentiment in snapshot |
| `index.html` | Modify | Add feed panel; add sentiment display to stats bar |
| `worker/sim_worker.js` | Modify | Include new_posts in postMessage JSON |

---

## Tasks

- [ ] **1.** Implement `RetailNpcAgent` with `SirState`, account, fear state, and position tracking
- [ ] **2.** Implement momentum signal: `(last_price - price_N_ticks_ago) / price_N_ticks_ago`; N = 60,000 ticks (1 simulated minute)
- [ ] **3.** Implement order decision: combine sentiment (60%) + momentum (30%) + fear_state penalty; select order type by urgency state
- [ ] **4.** Implement FOMO trigger: if price moved >3% since last observe and agent has no position, switch to market order with high probability
- [ ] **5.** Implement fear state: trailing realized vol = std dev of last 20 per-minute returns; `fear_state = clamp(realized_vol / vol_threshold, 0, 1)`; scale order size and frequency by `(1 - fear_state)`
- [ ] **6.** Implement `SirModel` in `community.rs`: tick S→I transitions (function of sentiment × infection_rate × S), I→R transitions (function of recovery_rate), R→S after cooldown
- [ ] **7.** Implement sentiment score derivation from SIR model state
- [ ] **8.** Implement post template system: 5–10 templates per `PostType`; fill in price, P&L, ticker from sim data at generation time
- [ ] **9.** Implement probabilistic post generation per infected NPC per tick
- [ ] **10.** Add feed ring buffer to `SimSnapshot`; populate from `CommunityFeed` in `write_shared_buffer()`
- [ ] **11.** Add feed panel to `index.html`: scrolling div, newest post prepended; each post styled by type
- [ ] **12.** Add sentiment bar/number to stats display

---

## Out of Scope

- DD posts with signal quality model (later)
- Player interaction with the feed
- Multiple stocks / cross-stock sentiment

---

## Manual Testing

- [ ] Watch the feed panel for 2 simulated minutes — confirm posts appear and include price/P&L data (not blank templates)
- [ ] Watch sentiment score in stats bar — confirm it fluctuates, not stuck at 0
- [ ] Watch chart during a sentiment spike — confirm price momentum is visible (price trending in one direction) during high sentiment
- [ ] Watch fear state effect: set speed to Max, wait for a volatile period, then confirm post volume drops and NPC order frequency decreases (tape slows relative to price movement)
- [ ] Confirm SIR dynamics: sentiment should not stay pegged at max indefinitely — it should rise, peak, and fall as the S pool depletes and R agents cool down
- [ ] Check participants panel — confirm retail NPC positions are non-zero during sentiment events and return toward zero during recovery

---

## Green Light

- [ ] Approved
