# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

SimMarket is a browser-based stock market simulation game. The player is a retail trader with a small account competing against realistic agent archetypes (HFTs, market makers, hedge funds). The core is a tick-accurate agent-based model with real market microstructure dynamics.

## Rules (Binding in Every Session)

**Never run any of the following without explicit instruction:**
- `cargo` (any subcommand: build, test, check, fmt, clippy, run)
- `git` (any subcommand)
- Shell scripts, `make`, `trunk`, `wasm-pack`

After making code changes, tell Josh what to run to verify — don't run it yourself.

**Never stage or commit.** Josh reviews diffs, stages what he wants, and tells Claude to commit in a later turn.

**Scope discipline:** Implement exactly what the active `TODO/<phase>.md` spec says — no more, no less. No refactoring outside scope, no "while I'm here" cleanups, no speculative abstractions. If you notice a problem outside scope, flag it and move on.

**Design decisions not covered by the spec:** Stop and ask. Do not make silent judgment calls.

**Docs are source of truth.** If implementation must diverge from `docs/`, note it explicitly and update the doc.

## Workflow

Work is organized as phases. Each phase has a spec in `TODO/<phase>.md`. The process:
1. Josh and Claude curate the spec together until all Open Questions are resolved
2. In a **fresh session**, Claude oneshots the implementation
3. Josh tests, reviews, and stages/commits
4. Repeat for the next phase

`TODO.md` has the full phase roadmap and current status. `TODO/_template.md` is the spec template.

## Architecture

```
Browser Main Thread
  └── egui Application (WASM)
        ├── reads sim state via SharedArrayBuffer
        └── sends player orders via MessageChannel

Web Worker — Simulation Thread (WASM)
  └── tick loop: agents → event queue → LOB → options → community feed → snapshot
```

**Stack:**
- Core simulation: Rust → WASM (via wasm-pack)
- UI: egui (immediate-mode, compiled to WASM, renders via WebGPU backend)
- Rendering: WebGPU (egui_wgpu); custom instanced passes for charts
- Parallelism: Web Worker + SharedArrayBuffer (double-buffered snapshot)
- Build: Trunk (or Vite) + wasm-pack

**Thread communication:**
- Sim → Renderer: SharedArrayBuffer double-buffer; sim writes back buffer, atomically swaps; renderer reads front buffer with no lock
- Renderer → Sim: MessageChannel postMessage (low-frequency player orders)

**COOP/COEP headers** are required for SharedArrayBuffer: `Cross-Origin-Opener-Policy: same-origin` + `Cross-Origin-Embedder-Policy: require-corp`.

## Simulation Engine

**Tick structure** — intra-tick event queue, not batch-then-match:
1. Advance clock; process scheduled events (earnings, news, halts)
2. Each active agent produces `OrderEvent`s with per-agent latency offsets (HFTs: 1–50 µs; retail: 500–999 µs; noise: 800–1000 µs)
3. Event queue sorted by intra-tick timestamp → drained into LOB in strict order
4. Update options chains, agent accounts, community feed, OHLCV candles
5. Write snapshot to SharedArrayBuffer

**Limit Order Book:** `BTreeMap<NotNan<f64>, VecDeque<Order>>` for bids (descending) and asks (ascending). Matching sweeps from best price; stop cascades are resolved recursively within the same event (not deferred).

**Agent scheduling tiers:** HFT and MM run every tick; noise traders every 100–1,000 ticks; retail NPCs every 1,000–60,000; hedge funds/institutions much less frequently. Agents produce events tagged with their latency offset; the event queue enforces causal ordering.

**Community feed:** SIR epidemic model (Susceptible/Infected/Recovered) drives the sentiment → price feedback loop. Prevents runaway dynamics via susceptible pool depletion, sentiment mean-reversion, and capital constraints.

## Planned Project Structure

```
simmarket/
├── Cargo.toml
├── Trunk.toml
├── index.html
├── src/
│   ├── main.rs             # WASM entry; spawns sim worker, starts render loop
│   ├── app.rs              # egui application root
│   ├── sim/
│   │   ├── engine.rs       # Tick loop, SimState
│   │   ├── exchange.rs     # LOB, matching engine
│   │   ├── agents/         # noise, retail, hft, market_maker, hedge_fund, institution
│   │   ├── options.rs      # Black-Scholes, Greeks, IV surface
│   │   ├── events.rs       # News, earnings, macro events
│   │   ├── community.rs    # Feed generation, SIR sentiment model
│   │   └── scenarios/      # tutorial, squeeze, ...
│   ├── render/             # chart, orderbook, portfolio
│   ├── ui/                 # feed, options_chain, controls
│   └── shared/
│       ├── types.rs        # OrderId, AgentId, SimTime, Side, OrderType, Order, Trade
│       └── snapshot.rs     # SimSnapshot (repr(C), lives in SharedArrayBuffer)
└── worker/
    └── sim_worker.js       # Web Worker bootstrap
```

## Design Documents

- `docs/game-design.md` — player account mechanics (Reg T, PDT rule, options levels), instruments, scenario list, UI concepts
- `docs/simulation-design.md` — agent archetypes, order types, microstructure behaviors, calibration targets (7 empirical targets to hit by end of Phase 2)
- `docs/technical-design.md` — architecture, LOB data structures, options engine, SharedArrayBuffer layout, performance targets
