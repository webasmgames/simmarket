# TODO: Phase 3 — Calibration Harness

## Overview

Before building any UI, validate that the simulation produces statistically realistic market behavior. Run 252 simulated trading days headless and compute the seven empirical targets from the simulation design doc. This is the gate between "the engine exists" and "the engine is trustworthy enough to build on." Do not proceed to Phase 4 until targets are met.

Before this phase: a headless sim that outputs one day of OHLCV. After this phase: a calibration binary that runs a full simulated year, computes statistics, and prints a pass/fail table.

---

## Requirements

- [ ] Simulation runs 252 consecutive trading days without panicking or producing NaN/Inf prices
- [ ] Daily returns are computed from close-to-close prices
- [ ] Kurtosis of daily returns is in range 4–8 (Gaussian = 3; fat tails expected)
- [ ] Ljung-Box Q-statistic on raw daily returns at lags 1–5 fails to reject zero autocorrelation (p > 0.05) — weak-form efficiency
- [ ] ACF of absolute daily returns is significantly positive at lag 1 (indicates volatility clustering)
- [ ] Average bid-ask spread across the year is in range 0.02–0.10% of price
- [ ] Volume-volatility correlation (contemporaneous) is positive and > 0.2
- [ ] Intraday volume profile is U-shaped: open and close buckets have higher average volume than midday bucket
- [ ] Output is a pass/fail table printed to stdout with actual values alongside targets

---

## Design

### Statistical Targets

```
Target 1 — Kurtosis:               actual > 3.0       (any excess kurtosis is a start)
Target 2 — Return autocorrelation: Ljung-Box p > 0.05 at lag 1-5
Target 3 — Vol clustering:         ACF(|r_t|, lag=1) > 0.05
Target 4 — Avg spread:             0.02% < spread < 0.10% of mid price
Target 5 — Vol spike spread:       spread widens during high-vol days
Target 6 — Vol-volume correlation: corr(|r_t|, volume_t) > 0.20
Target 7 — Intraday U-shape:       volume[open_bucket] > volume[mid_bucket]
                                   volume[close_bucket] > volume[mid_bucket]
```

### Key Logic

**Calibration run**: 252 × 390 minutes × tick loop. Store daily OHLCV (close price, volume, avg spread) and intraday volume profile (sum volume per 30-minute bucket across all days).

**Kurtosis**: `E[(r - μ)^4] / σ^4` where `r` is the vector of daily log returns.

**Ljung-Box Q**: `Q = n(n+2) Σ_{k=1}^{h} (ρ_k² / (n-k))` where `ρ_k` is ACF at lag k, n = sample size, h = number of lags. Compare to chi-squared critical value at h degrees of freedom.

**ACF**: standard autocorrelation function computed on absolute returns vector.

**Spread**: average of `(ask - bid) / mid` sampled once per minute across all days.

All stats implemented in pure Rust — no external stats library needed for these.

---

## Files

| File | Action | Notes |
|---|---|---|
| `src/bin/calibrate.rs` | Create | Main calibration binary |
| `src/calibration/mod.rs` | Create | Stats functions: kurtosis, ljung_box, acf, corr |
| `src/sim/engine.rs` | Modify | Expose per-day summary and intraday volume profile |

---

## Tasks

- [ ] **1.** Create `src/calibration/mod.rs` with pure-Rust implementations of: `kurtosis(returns: &[f64])`, `ljung_box(returns: &[f64], lags: usize) -> f64` (returns p-value approximation), `acf(series: &[f64], lag: usize) -> f64`, `pearson_corr(a: &[f64], b: &[f64]) -> f64`
- [ ] **2.** Extend `SimState` / `engine.rs` to accumulate: daily close prices, daily volume, daily avg spread, per-30min-bucket intraday volume
- [ ] **3.** Create `src/bin/calibrate.rs`: run 252 days, collect daily summaries, compute all 7 stats
- [ ] **4.** Print pass/fail table: stat name | target | actual | PASS/FAIL
- [ ] **5.** Accept CLI args: `--agents N`, `--seed S` to allow parameter sweeps

---

## Out of Scope

- Automated parameter sweep (manual for now — run with different args, observe output)
- Plotting or visualization
- Any agents beyond noise traders
- WASM

---

## Manual Testing

- [ ] Run `cargo run --bin calibrate -- --agents 5000 --seed 42` and confirm it completes without panic and prints a table with 7 rows
- [ ] Confirm kurtosis value is printed and is a real number (even if FAIL — just confirm it runs)
- [ ] Confirm spread target: if FAIL, reduce agent count and re-run to see spread widen (demonstrates the stat is sensitive to agent count)
- [ ] Run with `--agents 500` and `--agents 10000` and compare kurtosis — higher agent count should produce different distribution shape
- [ ] Confirm intraday U-shape check: manually scan the volume-by-bucket output and visually verify open/close buckets are larger than midday

---

## Notes

With only noise traders (no strategic agents), some targets will likely FAIL — particularly kurtosis (noise traders produce near-Gaussian returns) and vol clustering. That is expected and acceptable at this stage. The harness existing and running correctly is the deliverable. Strategic agents added in later phases will push stats toward targets.

---

## Green Light

- [ ] Approved
