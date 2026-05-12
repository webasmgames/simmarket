# TODO: Phase 4 — WASM + Web Worker

## Overview

Compile the simulation to WASM, move it into a Web Worker, and establish the SharedArrayBuffer communication channel with the main thread. No rendering yet — the deliverable is the sim running invisibly in the browser with its state being written to a shared buffer that the main thread can read. Verified via browser console.

Before this phase: a native Rust binary. After this phase: the same sim running in a browser Web Worker, writing tick count and last price to a SharedArrayBuffer readable from the console.

---

## Requirements

- [ ] `trunk serve` starts a dev server and loads the app in the browser without errors
- [ ] Simulation runs inside a Web Worker (not the main thread)
- [ ] SharedArrayBuffer is allocated and accessible from both worker and main thread
- [ ] Sim writes `SimSnapshot` (tick count + stock snapshot) to SharedArrayBuffer each tick
- [ ] Main thread reads tick count from SharedArrayBuffer and logs it to console at 1-second intervals — confirms sim is running
- [ ] COOP/COEP headers are set so SharedArrayBuffer is available (Trunk dev server config)
- [ ] Seeded determinism is preserved in WASM build

---

## Design

### Communication

```
Web Worker (WASM sim)                     Main Thread
─────────────────────────────────────────────────────
SimSnapshot written to SAB back buffer
AtomicU32 swap signals buffer ready  →   Main thread reads front buffer
                                          Logs snapshot.tick to console
Player order submitted               ←   postMessage({type:"order", ...})
Worker validates + enqueues order
```

### SharedArrayBuffer Layout (repr(C))

```rust
// src/shared/snapshot.rs
#[repr(C)]
struct SimSnapshot {
    tick: u64,
    sim_time: u64,                        // simulated microseconds
    stocks: [StockSnapshot; MAX_STOCKS],
    swap_flag: u32,                       // AtomicU32: which buffer is current
}

#[repr(C)]
struct StockSnapshot {
    last_price: f32,
    bid: f32,
    bid_size: u32,
    ask: f32,
    ask_size: u32,
    volume_today: u32,
    spread_pct: f32,
    candle_count: u32,
    candles: [Candle; 390],              // 1-minute candles, regular session
}

const MAX_STOCKS: usize = 8;
```

### Worker Bootstrap

```js
// worker/sim_worker.js
importScripts('./simmarket_bg.wasm');  // loaded by wasm-bindgen glue
// wasm init → call exported run_sim(sab_ptr, seed, agent_count)
```

Trunk handles WASM compilation and asset bundling. The worker JS file is a static asset copied to `dist/`.

---

## Files

| File | Action | Notes |
|---|---|---|
| `Cargo.toml` | Modify | Add `wasm-bindgen`, `js-sys`, `web-sys` (features: SharedArrayBuffer, Worker, Atomics) |
| `Trunk.toml` | Create | Dev server config with COOP/COEP headers |
| `index.html` | Create | Minimal HTML: canvas placeholder, script that spawns worker and reads SAB |
| `worker/sim_worker.js` | Create | Worker bootstrap: load WASM, call `run_sim()` |
| `src/lib.rs` | Modify | Add `#[wasm_bindgen]` exported `run_sim(sab: JsValue, seed: u64, agents: u32)` entry point |
| `src/shared/snapshot.rs` | Create | `SimSnapshot`, `StockSnapshot`, `Candle` as `repr(C)` |
| `src/sim/engine.rs` | Modify | Add `write_shared_buffer()` step at end of tick |

---

## Tasks

- [ ] **1.** Add to `Cargo.toml`: `wasm-bindgen`, `js-sys`, `web-sys` (features: `SharedArrayBuffer`, `Atomics`); add `[lib] crate-type = ["cdylib", "rlib"]`
- [ ] **2.** Create `Trunk.toml` with dev server headers: `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: require-corp`
- [ ] **3.** Define `SimSnapshot` and `StockSnapshot` as `#[repr(C)]` structs in `src/shared/snapshot.rs`
- [ ] **4.** Implement `write_shared_buffer(snapshot: &SimSnapshot, sab: &SharedArrayBuffer)` using `Atomics::store` for the swap flag
- [ ] **5.** Add `#[wasm_bindgen]` exported `run_sim(sab: JsValue, seed: u64, agent_count: u32)` in `src/lib.rs` — initializes sim, enters tick loop
- [ ] **6.** Create `index.html`: allocate SharedArrayBuffer, spawn `sim_worker.js` passing SAB, poll SAB every second and log `snapshot.tick` to console
- [ ] **7.** Create `worker/sim_worker.js`: import WASM, call `run_sim` with the SAB received via `postMessage`
- [ ] **8.** Verify `trunk serve` compiles without error and browser console shows incrementing tick count

---

## Out of Scope

- Any rendering (WebGPU, canvas, HTML panels)
- Player orders (postMessage from main thread not implemented yet)
- Multiple stocks
- Community feed or tape ring buffer in snapshot

---

## Manual Testing

- [ ] Run `trunk serve`, open browser, open DevTools console — confirm no errors on load
- [ ] Confirm SharedArrayBuffer is not `undefined` in console (COOP/COEP headers working)
- [ ] Confirm tick count logged to console increments every second (sim is running in worker)
- [ ] Confirm last_price in snapshot is a non-zero, non-NaN float (noise trader market has a price)
- [ ] Hard-reload the page and confirm sim restarts cleanly (no stale worker state)
- [ ] Open DevTools Performance tab, record 5 seconds — confirm worker thread is active and main thread is idle (sim not blocking render thread)

---

## Notes

`trunk serve` by default does not set COOP/COEP headers. These must be configured in `Trunk.toml` or via a local proxy. Without them, `SharedArrayBuffer` is undefined in the browser and `new SharedArrayBuffer()` throws. This is documented in the Key Risks section of technical-design.md.

---

## Green Light

- [ ] Approved
