# SimMarket — Phase Roadmap

Each phase is one focused implementation session. Spec lives in `TODO/<phase>.md`. Implement only after the spec is curated and Open Questions are resolved.

| Status | Phase | File | Description |
|---|---|---|---|
| ✅ | 1 | [phase1-lob.md](TODO/done/phase1-lob.md) | Limit Order Book + matching engine (pure Rust, no agents, no WASM) |
| 🔴 | 2a | [phase2a-tick-engine.md](TODO/phase2a-tick-engine.md) | Tick engine: simulated clock, intra-tick event queue, candle accumulation |
| 🔴 | 2b | [phase2b-noise-trader.md](TODO/phase2b-noise-trader.md) | Agent trait, noise trader, headless binary, ohlcv.csv output |
| 👀 | 3 | [phase3-calibration.md](TODO/phase3-calibration.md) | Calibration harness — validate sim produces realistic microstructure |
| 👀 | 4 | [phase4-wasm-worker.md](TODO/phase4-wasm-worker.md) | Compile to WASM, run sim in Web Worker, SharedArrayBuffer setup |
| 👀 | 5 | [phase5-webgpu-canvas.md](TODO/phase5-webgpu-canvas.md) | WebGPU device + candlestick chart rendering |
| 👀 | 6 | [phase6-debug-panels.md](TODO/phase6-debug-panels.md) | HTML debug panels: LOB depth, tape, time controls, stats |
| 👀 | 7 | [phase7-hft-market-maker.md](TODO/phase7-hft-market-maker.md) | HFT market maker agent + participants panel |
| 👀 | 8 | [phase8-retail-npc.md](TODO/phase8-retail-npc.md) | Retail NPC agent, SIR sentiment model, community feed |
| 👀 | 9 | [phase9-player-account.md](TODO/phase9-player-account.md) | Player Reg T account, broker routing layer, order entry UI, routing visualization |
| 👀 | 10 | [phase10-options.md](TODO/phase10-options.md) | Options engine: chain generation, Greeks, IV surface, MM delta hedging |
| 👀 | 11 | [phase11-remaining-agents.md](TODO/phase11-remaining-agents.md) | Hedge fund, institution, directional HFT, dark pool mechanism |
