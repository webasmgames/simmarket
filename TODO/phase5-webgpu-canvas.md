# TODO: Phase 5 — WebGPU Canvas + Candlestick Chart

## Overview

Establish the WebGPU rendering pipeline on the main thread and draw a live candlestick chart from the simulation snapshot. This phase validates the full data path: sim produces OHLCV in the worker → snapshot written to SharedArrayBuffer → main thread reads snapshot → GPU renders candles. No HTML panels yet, just the chart on a canvas.

Before this phase: sim running in worker, tick count visible in console. After this phase: a live candlestick chart updating in real time from the noise trader simulation.

---

## Requirements

- [ ] WebGPU device initializes without error on Chrome/Edge (primary targets)
- [ ] Canvas fills the browser window (or a defined area)
- [ ] Candlestick chart renders 1-minute OHLCV candles from the simulation snapshot
- [ ] Candles are green if close ≥ open, red if close < open
- [ ] Volume bars render below the candlesticks, scaled to fit a fixed height
- [ ] Chart updates smoothly at 60 FPS — new candles appear as the sim produces them
- [ ] Y-axis shows price labels at regular intervals
- [ ] X-axis shows time labels (simulated time, e.g. "09:45")
- [ ] Only dirty (changed) candles upload new data to GPU — not full buffer re-upload every frame

---

## Design

### Rendering Approach

Use egui's `Painter` API (Option A from technical-design.md) for this phase — CPU-side path building uploaded to GPU via egui_wgpu. This is simpler than custom WGSL and sufficient for the debug tool. Custom instanced rendering (Option B) is deferred unless profiling shows it's needed.

egui runs on the main thread. Each frame:
1. Read `SimSnapshot` from SharedArrayBuffer
2. Extract candle array for the primary stock
3. Call egui `Painter` to draw rectangles for each candle body, lines for wicks, rectangles for volume bars
4. egui submits draw calls to WebGPU

### Candle Layout

```
Canvas height split:
  - Top 70%: candlestick chart with wicks
  - Bottom 20%: volume bars
  - 10%: padding / axes

Candle body: rect from open to close price
Wick: vertical line from low to high
Volume bar: rect from 0 to volume, scaled so max volume = full bar height
```

### Data Path

```rust
// main thread, each frame:
let snapshot = read_snapshot_from_sab(&sab);  // atomic read
let candles = &snapshot.stocks[0].candles[..snapshot.stocks[0].candle_count as usize];
draw_candlesticks(&painter, candles, &price_range, &time_range);
draw_volume_bars(&painter, candles, &vol_range);
draw_axes(&painter, &price_range, &time_range);
```

---

## Files

| File | Action | Notes |
|---|---|---|
| `Cargo.toml` | Modify | Add `egui`, `eframe`, `egui-wgpu` (or `eframe` with wgpu feature) |
| `src/app.rs` | Create | eframe `App` impl; main thread loop; reads SAB, calls render |
| `src/render/mod.rs` | Create | `pub mod chart;` |
| `src/render/chart.rs` | Create | `draw_candlesticks()`, `draw_volume_bars()`, `draw_axes()` using egui Painter |
| `src/lib.rs` | Modify | Add eframe startup in main-thread init path |
| `index.html` | Modify | Add canvas element with id for eframe to attach to |

---

## Tasks

- [ ] **1.** Add `eframe` (with `wgpu` feature) to `Cargo.toml`; confirm it compiles to WASM
- [ ] **2.** Create `src/app.rs`: implement `eframe::App` trait; `update()` reads SAB snapshot and triggers chart redraw
- [ ] **3.** Implement `read_snapshot_from_sab()`: atomic load of swap flag, memcpy front buffer into a local `SimSnapshot`
- [ ] **4.** Implement `draw_candlesticks()` in `chart.rs`: map price range to canvas Y coords; draw body rect and wick line per candle using `Painter::rect` and `Painter::line_segment`
- [ ] **5.** Implement `draw_volume_bars()`: map volume range to bar height; draw below chart area
- [ ] **6.** Implement `draw_axes()`: Y-axis price labels every N price units; X-axis time labels every 30 minutes
- [ ] **7.** Implement dirty-candle tracking: compare new candle data to last-drawn; only update changed candles (avoids full redraw each frame)
- [ ] **8.** Wire eframe startup into `src/lib.rs` WASM entry point alongside the worker spawn
- [ ] **9.** Confirm chart renders and updates live as sim runs

---

## Out of Scope

- HTML panels (Phase 6)
- Zoom / pan / timeframe switching
- Indicators (VWAP, RSI, etc.)
- Multiple stocks
- Custom WGSL shaders (egui Painter is sufficient here)

---

## Manual Testing

- [ ] Load page — confirm canvas appears and candlestick chart renders within 2 seconds of load
- [ ] Watch chart for 30 seconds — confirm new candles appear as simulated time advances (candle count grows left to right)
- [ ] Confirm candle colors: zoom into a candle where open ≠ close and verify color matches direction
- [ ] Confirm volume bars render below chart and scale with candle volume (taller bars on higher-volume candles)
- [ ] Open DevTools Performance tab — confirm main thread is hitting 60 FPS and GPU usage is visible
- [ ] Resize browser window — confirm chart reflows to fill available space

---

## Green Light

- [ ] Approved
