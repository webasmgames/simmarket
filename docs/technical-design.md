# Technical Design Document — SimMarket

## Stack Summary

| Layer | Technology | Rationale |
|---|---|---|
| Core simulation | Rust | Performance, memory safety, deterministic behavior |
| Runtime | WASM (via wasm-pack) | Runs in browser at near-native speed |
| Rendering | WebGPU | GPU-accelerated charts, order book heatmap, particle FX |
| UI | egui (compiled to WASM) | Pure Rust, renders via WebGPU, no JS framework dependency |
| Parallelism | Web Workers + SharedArrayBuffer | Simulation thread separate from render thread |
| Build | wasm-pack + Vite (or Trunk) | Standard WASM toolchain; Trunk is Rust-native |

### Why Rust/WASM over native?
- Runs everywhere without install — distributable as a URL
- Rust's ownership model eliminates entire classes of simulation bugs (data races, use-after-free)
- WASM performance is within 1.5× native for CPU-bound workloads; simulation is CPU-bound
- WebGPU is the future of GPU compute in browser; available in Chrome/Edge/Safari

### Why egui?
- Entire stack in one language (Rust)
- egui renders immediate-mode UI via its own renderer, which we target to WebGPU
- No React, no JS state management, no npm hell
- The immediate-mode model fits a simulation UI: state lives in the sim, UI just reads it

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Browser Main Thread                                     │
│  ┌──────────────────────────────────────────────────┐   │
│  │  egui Application (WASM)                         │   │
│  │  - Chart view, Order book, Options chain         │   │
│  │  - Portfolio panel, Community feed               │   │
│  │  - Player input → order submission               │   │
│  └──────────────┬──────────────────────┬────────────┘   │
│                 │ SharedArrayBuffer    │ MessageChannel  │
│                 │ (read sim state)     │ (send orders)   │
│  ┌──────────────▼──────────────────────▼────────────┐   │
│  │  Web Worker — Simulation Thread (WASM)            │   │
│  │  - Tick engine                                    │   │
│  │  - Agent pool                                     │   │
│  │  - Limit Order Book (per stock)                   │   │
│  │  - Options chain updater                          │   │
│  │  - Community feed generator                       │   │
│  │  - Scenario event scheduler                       │   │
│  └───────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Communication Between Threads

**Simulation → Renderer (hot path)**:
- SharedArrayBuffer holds a double-buffered snapshot of simulation state
- Sim writes to back buffer each tick; atomically swaps when complete
- Renderer reads from front buffer; no locking needed for read

**Renderer → Simulation (player orders)**:
- MessageChannel (postMessage) for order submission
- Orders are low-frequency relative to ticks; async messaging is fine
- Player orders are enqueued with a simulated execution latency offset (default: 100–200ms of simulated time, representing retail broker routing delay) — this puts them behind HFTs in the intra-tick event queue and makes front-running mechanically grounded

**Why not just run everything on the main thread?**
- Simulation at full speed (thousands of ticks/second) would starve the render loop
- Separating them allows each to run at its own rate

---

## Simulation Engine

### Tick Structure

The tick uses an **intra-tick event queue** rather than batch-then-match. Each agent decision produces a timestamped `OrderEvent` with a per-agent latency offset. The matching engine drains the queue in strict timestamp order, so causality is preserved within the tick: an HFT reacting to a retail order sees it first, then inserts its response after it in the queue.

```
fn tick(state: &mut SimState, dt: SimDuration) {
    advance_clock(state, dt);
    process_scheduled_events(state);         // earnings, news, halts
    build_event_queue(state);                // all agents produce OrderEvents with latency offsets
    // event queue is sorted by intra-tick timestamp before draining
    drain_event_queue_into_lob(state);       // process events in timestamp order; matching is continuous
    update_options_chains(state);            // recompute Greeks, IV surface
    update_agent_accounts(state);            // P&L, margin checks, liquidations
    generate_community_posts(state);         // probabilistic post generation
    snapshot_ohlcv(state);                   // update candle data for renderer
    write_shared_buffer(state);              // publish state to renderer
}

// Intra-tick latency offsets (microseconds within the 1ms tick window):
// Noise trader:    800–1000 µs  (slowest — exogenous, not latency-sensitive)
// Retail NPC:      500–999 µs   (retail broker routing delay)
// Institution:     400–800 µs   (algo execution, not latency-competitive)
// Hedge Fund:      200–500 µs   (prime brokerage, DMA but not co-located)
// Market Maker:    50–500 µs    (DMA, near-exchange)
// HFT:             1–50 µs      (co-located, direct feed)
// Player:          configurable, default 100–200 µs (represents retail broker; see below)
```

### Variable Tick Rate
- Wall-clock time is decoupled from simulated time
- Player sets a speed multiplier (1×, 5×, 50×, max)
- At 1×: 1 simulated ms per 1 real ms (1000 ticks/second for 1ms resolution)
- At max: simulate as fast as CPU allows
- Pause: tick loop suspends; renderer continues reading last snapshot

### Determinism and Replay
- All random number generation uses a seeded PRNG per-agent
- Given the same seed and same player actions, a scenario is reproducible
- Enables: scenario replay, bug reproduction, seed-sharing (friends play same scenario)

---

## Limit Order Book (LOB)

### Data Structure
```rust
struct LimitOrderBook {
    bids: BTreeMap<OrderedFloat<f64>, VecDeque<Order>>,  // price → FIFO queue
    asks: BTreeMap<OrderedFloat<f64>, VecDeque<Order>>,
    // BTreeMap gives O(log n) insert/remove, O(1) best price
}

struct Order {
    id: OrderId,
    agent_id: AgentId,
    side: Side,            // Bid | Ask
    order_type: OrderType, // Limit | Market | Stop | StopLimit
    price: f64,            // limit price (0 for market)
    stop_price: f64,       // trigger price for stop orders
    quantity: u32,
    filled: u32,
    submitted_at: SimTime,
    gtc: bool,
}
```

### Matching Algorithm
```
For each incoming market or aggressively-priced limit order:
  1. Check if it crosses the spread
  2. Walk opposite side of book, filling from best price
  3. For each matched order: generate Trade record, update signed tape, notify both agents
  4. If partially filled: rest remainder in book (limit) or cancel (IOC)
  5. Collect all stop orders whose stop_price was crossed during the sweep
  6. Convert triggered stops to market/limit orders and re-enter matching — RECURSIVE
     (repeat steps 1–6 until no new stops are triggered; this is the flash crash cascade)
```

Stop cascade must be resolved recursively within the same event, not deferred to the next tick. Deferring would suppress flash crash dynamics by inserting an artificial 1ms damper between cascade steps.

### Performance Considerations
- BTreeMap is cache-hostile; for ultra-high frequency, consider a price-indexed array with a freelist
- At simulated HFT volumes, the LOB is the hot path — profile early
- Alternative: segment the book into price "buckets" (1% wide); use array of buckets, BTreeMap only for cross-bucket queries
- Start with BTreeMap; optimize only if profiling shows it's the bottleneck

---

## Agent Pool

### Memory Layout
Agents are stored in flat Vec<Agent> for cache locality. Each agent type is a separate pool.

```rust
enum AgentVariant {
    NoiseTrader(NoiseTraderAgent),   // most numerous; random liquidity-motivated orders
    RetailNpc(RetailNpcAgent),
    Hft(HftAgent),
    MarketMaker(MarketMakerAgent),
    HedgeFund(HedgeFundAgent),
    Institution(InstitutionAgent),
    Player(PlayerAgent),
}

struct AgentPool {
    agents: Vec<AgentVariant>,
    // Per-agent accounts stored separately for cache reasons
    accounts: Vec<Account>,
}
```

### Agent Decision Interface
```rust
trait Agent {
    fn observe(&self, market: &MarketSnapshot, account: &Account) -> Observation;
    fn decide(&mut self, obs: &Observation, rng: &mut AgentRng) -> Vec<OrderAction>;
}

enum OrderAction {
    Submit(Order),
    Cancel(OrderId),
    Modify(OrderId, Order),
}
```

### Agent Scheduling
Not all agents run every tick. Scheduling tiers:
- **HFT**: every tick; intra-tick latency offset 1–50 µs
- **Market Maker**: every tick; intra-tick latency offset 50–500 µs
- **Noise Trader**: every 100–1,000 ticks; intra-tick latency offset 800–1000 µs (latency-insensitive)
- **Retail NPC**: every 1,000–60,000 ticks; intra-tick latency offset 500–999 µs
- **Hedge Fund**: every 60,000–600,000 ticks; intra-tick latency offset 200–500 µs
- **Institution**: every 600,000+ ticks; intra-tick latency offset 400–800 µs

Within a tick, agents at each scheduling tier produce events tagged with their latency offset. The event queue sorts all events from all active agents by offset before passing to the matching engine. This dramatically reduces compute (slow agents run infrequently) while preserving causal ordering.

---

## Options Engine

### Greeks Computation
Black-Scholes closed form per-option per-tick (vectorizable):

```rust
struct OptionGreeks {
    price: f64,
    delta: f64,   // ∂price/∂spot
    gamma: f64,   // ∂²price/∂spot²
    theta: f64,   // ∂price/∂time (per day)
    vega: f64,    // ∂price/∂IV (per 1% IV move)
    rho: f64,     // ∂price/∂rate
}

fn black_scholes(spot: f64, strike: f64, t: f64, r: f64, iv: f64, is_call: bool) -> OptionGreeks
```

### IV Surface
- Store IV as a 2D array: [expiry][strike] → f64
- Update IV surface each tick based on:
  - Realized volatility of underlying (trailing window)
  - Supply/demand from options order flow (if retail buys OTM calls, those IVs rise)
  - Event proximity premium (IV rises as earnings approach)
  - Post-event crush (IV drops sharply after the event)

---

## Rendering (WebGPU)

### Render Architecture
egui renders its UI via a WebGPU backend (egui_wgpu). Custom chart drawing is done via:
- **Option A**: egui's `Painter` API (CPU-side path building, GPU upload per frame) — simpler
- **Option B**: custom WebGPU render pass (instanced quads for candles, compute shader for order book heatmap) — more performant for large datasets
- Start with Option A; migrate hot paths to Option B as needed

### Chart Views

#### Candlestick Chart
- Each candle: one instanced quad with OHLC encoded in vertex data
- Color: green if close > open, red otherwise
- Volume bars: secondary row of instanced quads, scaled by volume
- GPU instance buffer updated once per frame (only changed candles are dirty)

#### Order Book Depth Visualization
- L2 heatmap: price on Y axis, time on X axis, intensity = volume at that price level
- Recent trades shown as dots on the price axis
- Implemented as a texture updated each frame (or compute shader for large depth)

#### Options Chain
- Table view with color coding (IV rank, OI change)
- Updated once per tick, low-frequency relative to charts

### Performance Targets
- 60 FPS during normal simulation
- 30 FPS at max sim speed (rendering takes a back seat to simulation throughput)
- Render thread never blocks on simulation

---

## State Sharing (Shared Memory Layout)

```rust
// Written by sim thread, read by render thread
// Stored in SharedArrayBuffer
#[repr(C)]
struct SimSnapshot {
    tick: u64,
    sim_time: SimTime,          // current simulated timestamp
    stocks: [StockSnapshot; MAX_STOCKS],
    player_account: AccountSnapshot,
    feed_ring: [FeedPost; FEED_RING_SIZE],  // ring buffer of recent posts
    feed_head: AtomicU32,
    tape_ring: [TapePrint; TAPE_RING_SIZE], // time-and-sales ring buffer
    tape_head: AtomicU32,
}

#[repr(C)]
struct StockSnapshot {
    symbol: [u8; 8],
    last_price: f32,
    last_print_size: u32,
    last_print_side: u8,        // 0 = buyer-initiated, 1 = seller-initiated
    bid: f32,
    bid_size: u32,
    ask: f32,
    ask_size: u32,
    volume_today: u32,
    candles_1m: [Candle; 390],  // one per minute of regular session
    candles_5m: [Candle; 78],
    candles_1d: [Candle; 252],  // one year of daily candles
    sentiment: i16,             // −100 to 100
    short_interest_pct: f32,
    borrow_utilization_pct: f32, // borrow pool utilization; drives fee rate and locate availability
}

#[repr(C)]
struct TapePrint {
    sim_time: SimTime,
    price: f32,
    size: u32,
    side: u8,   // buyer or seller initiated
    is_dark_pool: u8,
}
```

---

## Project Structure

```
simmarket/
├── Cargo.toml
├── Trunk.toml              # Trunk build config (WASM + static assets)
├── index.html              # Entry point
├── src/
│   ├── main.rs             # WASM entry point; spawns sim worker, starts render loop
│   ├── app.rs              # egui application root
│   ├── sim/
│   │   ├── mod.rs
│   │   ├── engine.rs       # Tick loop, SimState
│   │   ├── exchange.rs     # LOB, matching engine, order types
│   │   ├── agents/
│   │   │   ├── mod.rs
│   │   │   ├── noise.rs         # Noise/liquidity traders
│   │   │   ├── retail.rs
│   │   │   ├── hft.rs
│   │   │   ├── market_maker.rs
│   │   │   ├── hedge_fund.rs
│   │   │   └── institution.rs
│   │   ├── options.rs      # Black-Scholes, Greeks, IV surface
│   │   ├── events.rs       # News, earnings, macro events
│   │   ├── community.rs    # Feed generation, sentiment model
│   │   └── scenarios/
│   │       ├── mod.rs
│   │       ├── tutorial.rs
│   │       ├── squeeze.rs
│   │       └── ...
│   ├── render/
│   │   ├── mod.rs
│   │   ├── chart.rs        # Candlestick, volume
│   │   ├── orderbook.rs    # L2 depth view
│   │   └── portfolio.rs    # P&L, positions view
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── feed.rs         # WSB-style community feed panel
│   │   ├── options_chain.rs
│   │   └── controls.rs     # Time controls, order entry
│   └── shared/
│       ├── mod.rs
│       ├── types.rs        # SimTime, OrderId, AgentId, etc.
│       └── snapshot.rs     # SimSnapshot layout (repr(C))
├── worker/
│   └── sim_worker.js       # Web Worker bootstrap (loads WASM, runs tick loop)
└── docs/
    ├── game-design.md
    ├── simulation-design.md
    └── technical-design.md
```

---

## Development Phases

### Phase 1 — Core Exchange (no agents, no UI)
- Implement LOB with matching engine
- Unit tests: market orders, limit orders, partial fills, cancels, price-time priority
- Benchmark: target 1M order submissions/second before moving on

### Phase 2 — Basic Agents + Headless Simulation
- Implement noise trader, retail NPC, and market-making HFT agents
- Run 1 stock, 10,000 noise traders, 1,000 retail NPCs, 20 HFT market makers for 252 simulated days
- No UI yet; output CSV of OHLCV + signed tape for analysis
- **Calibration harness**: compute all 7 targets from simulation-design.md calibration section; report pass/fail against empirical ranges
- Do not proceed to Phase 3 until spread, kurtosis, and volume-volatility correlation targets are met

### Phase 3 — WASM + Basic Renderer
- Compile to WASM, run in browser via Trunk
- Candlestick chart renders from snapshot
- Single stock, no player interaction yet

### Phase 4 — Player Account + Order Entry
- Player agent wired up to order entry UI
- Reg T margin enforced
- P&L panel

### Phase 5 — Options
- Options chain generation and display
- Player can buy/sell options
- Market maker delta hedging (rudimentary)
- Greeks panel

### Phase 6 — Full Agent Set + Scenarios
- Hedge fund, institution agents
- Community feed generation
- Scenario 1 (tutorial) and Scenario 2 (squeeze) playable end-to-end

### Phase 7 — Polish + Additional Scenarios
- WebGPU optimizations (instanced candles, order book heatmap)
- Sound, visual effects
- Remaining scenarios
- Save/load, scenario replay

---

## Key Risks and Mitigations

| Risk | Mitigation |
|---|---|
| WASM multi-threading (SharedArrayBuffer) requires COOP/COEP headers | Set `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` on dev server; document for deployment |
| egui's WebGPU backend is not yet 1.0 stable | Pin egui version; isolate renderer behind trait so it can be swapped |
| Black-Scholes is slow for large option chains | Vectorize with SIMD (Rust's `std::simd` or `packed_simd`); only recompute dirty strikes |
| Agent count explosion at high tick rates | Agent scheduling tiers (slow agents run infrequently); benchmark and cap population |
| LOB performance at HFT order volumes | Profile Phase 1; consider price-bucket optimization before agents are added |
| Intra-tick event queue sort cost at high agent counts | Pre-sort by agent type (HFTs always first); only sort within tier. BinaryHeap insertion is O(log n). Profile in Phase 1. |
| Recursive stop cascade runaway | Cap recursion depth; log if cascade exceeds N levels (indicates potential infinite loop from adversarial order patterns) |
