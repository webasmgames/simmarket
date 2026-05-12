# TODO: Phase 6 — HTML Debug Panels

## Overview

Add the HTML panel layer alongside the WebGPU canvas: a live LOB depth view, a scrolling time-and-sales tape, time controls (pause/play/speed), and a basic stats bar. The worker sends a JSON snapshot to the main thread via `postMessage` on each tick; a thin JS layer updates the DOM at `requestAnimationFrame` rate. This completes the debug app's core observability surface.

Before this phase: candlestick chart only. After this phase: full debug view — chart + LOB depth + tape + controls.

---

## Requirements

- [ ] Layout: chart on left (dominant), panels stacked on right (LOB, tape, stats)
- [ ] LOB panel: shows top 10 bid and ask levels with price and size; bids green, asks red; updates live
- [ ] Tape panel: scrolling list of recent trades — time, price, size, side (B/S), dark pool flag; newest at top; capped at 200 entries
- [ ] Stats bar: last price, daily change %, bid, ask, spread %, volume today, sim clock
- [ ] Time controls: Pause / Play / 1× / 5× / 20× / Max buttons; sim responds to speed changes
- [ ] Sim clock display: shows current simulated time (e.g. "10:32:15") updating in real time
- [ ] postMessage protocol: worker sends a compact snapshot JSON to main thread; main thread JS updates DOM — snapshot is sent at most once per rendered frame (not every tick)

---

## Design

### Layout (HTML/CSS)

```
┌────────────────────────────┬──────────────────┐
│                            │  Stats bar       │
│   WebGPU Canvas            ├──────────────────┤
│   (candlestick chart)      │  LOB             │
│                            │  (bids / asks)   │
│                            ├──────────────────┤
│                            │  Tape            │
│                            │  (time & sales)  │
├────────────────────────────┴──────────────────┤
│  Time controls: [Pause] [1×] [5×] [20×] [Max] │
└────────────────────────────────────────────────┘
```

### postMessage Protocol

Worker sends to main thread at most once per rAF (throttled in worker):
```json
{
  "tick": 12345678,
  "sim_time_str": "10:32:15",
  "last_price": 42.17,
  "daily_change_pct": 1.23,
  "bid": 42.16, "bid_size": 300,
  "ask": 42.18, "ask_size": 150,
  "spread_pct": 0.047,
  "volume": 843200,
  "bids": [[42.16, 300], [42.15, 500], ...],  // top 10
  "asks": [[42.18, 150], [42.19, 400], ...],  // top 10
  "new_trades": [{"time":"10:32:14","price":42.17,"size":100,"side":"B","dark":false}, ...]
}
```

Main thread speed control: `postMessage({type:"set_speed", multiplier: 5})` to worker; worker adjusts tick rate.

### Snapshot throttling

Worker tracks wall-clock time of last `postMessage`; sends at most every 16ms (60 FPS cap). The SAB is still written every tick for the WebGPU canvas path.

---

## Files

| File | Action | Notes |
|---|---|---|
| `index.html` | Modify | Add panel HTML structure, CSS layout, JS DOM update logic |
| `worker/sim_worker.js` | Modify | Add throttled postMessage; handle `set_speed` message |
| `src/sim/engine.rs` | Modify | Expose speed multiplier setter callable from WASM binding |
| `src/lib.rs` | Modify | Add `set_speed` wasm-bindgen export; wire postMessage send from tick loop |

---

## Tasks

- [ ] **1.** Add panel HTML structure to `index.html`: stats bar, LOB div, tape div, time controls bar
- [ ] **2.** Add CSS: two-column layout (canvas left, panels right), panel scrolling, color coding (green/red for bids/asks and trade sides)
- [ ] **3.** Implement JS `handleSnapshot(data)`: updates stats bar text, rebuilds LOB table rows, prepends new tape entries (cap list at 200)
- [ ] **4.** Implement postMessage throttle in `sim_worker.js`: track `lastSentMs`; send snapshot JSON only if `Date.now() - lastSentMs > 16`
- [ ] **5.** Implement `set_speed` message handler in worker: calls `wasm.set_speed(multiplier)`
- [ ] **6.** Add `#[wasm_bindgen]` `set_speed(multiplier: f32)` export in `src/lib.rs`; store multiplier in `SimState`; tick loop sleeps or skips based on multiplier
- [ ] **7.** Add time control button event listeners in `index.html` JS: each button posts `{type:"set_speed", multiplier: N}` to worker
- [ ] **8.** Add sim clock display: format `sim_time` as HH:MM:SS in the snapshot JSON; update a `<span>` each frame
- [ ] **9.** Wire LOB snapshot data: in Rust, serialize top 10 bids and asks into the JSON snapshot before postMessage

---

## Out of Scope

- Participant list / agent panel (Phase 7, added when HFT agent lands)
- Order entry UI (Phase 9)
- Community feed panel (Phase 8)
- Charting controls (zoom, pan, indicators)

---

## Manual Testing

- [ ] Load page — confirm two-column layout renders with chart on left and panels on right
- [ ] Watch LOB panel — confirm bid/ask levels update live, green bids above separator, red asks below
- [ ] Watch tape panel — confirm trades scroll in with newest at top; B/S side clearly labeled
- [ ] Click Pause — confirm sim clock stops incrementing; chart stops updating
- [ ] Click Play — confirm sim resumes from where it paused
- [ ] Click 20× — confirm sim clock advances visibly faster; tape fills with trades more rapidly
- [ ] Click Max — confirm CPU spins up (fan noise / Task Manager) and tape floods; chart may drop below 60 FPS (acceptable)
- [ ] Confirm stats bar shows non-zero spread % and updating volume

---

## Green Light

- [ ] Approved
