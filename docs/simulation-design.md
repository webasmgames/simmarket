# Simulation Design Document — SimMarket

## Overview

The simulation is an agent-based model (ABM) of a single simulated exchange. Multiple stocks trade simultaneously. Time advances in 1ms increments of simulated market time, but the engine is **event-driven within each tick**: agents produce timestamped order events with per-agent latency offsets, and the matching engine processes them in strict timestamp order — not in a single batch. This preserves intra-tick causality (an HFT reacting to a retail order queues its response after that order, not simultaneously with it).

The goal is not to model every real-world institution precisely but to produce emergent market behavior that feels authentic: momentum, mean-reversion, volatility clustering, fat-tail returns, flash crashes, short squeezes, and the irrational exuberance of a retail-driven meme stock.

---

## Simulated Exchange

### Market Hours
- Pre-market: 4:00 AM–9:30 AM ET — continuous limit-order-only market (no market orders, no stops); opening auction book accumulates separately via MOO/LOO orders
- Regular session: 9:30 AM–4:00 PM ET — full continuous double auction, all agents active
- Closing auction: 4:00 PM — volume-maximizing uncross of MOC/LOC orders; primary venue for institutional rebalancing and index NAV pricing
- After-hours: 4:00 PM–8:00 PM ET — continuous limit-order-only, lower volume, wider spreads
- Overnight: skip forward (apply overnight events: news, gaps, earnings)

### Order Types Processed by Exchange
- **Market order** — fill at best available price immediately
- **Limit order** — rest in book at specified price; fill when matched
- **Stop order** — becomes market order when trigger price is touched
- **Stop-limit** — becomes limit order when trigger price is touched
- **IOC (Immediate-or-Cancel)** — fill what's available, cancel remainder
- **FOK (Fill-or-Kill)** — fill entire order or cancel entirely
- **MOO / MOC** — market-on-open / market-on-close; route to opening or closing auction only
- **LOO / LOC** — limit-on-open / limit-on-close; limit-priced auction participation
- **Iceberg / Reserve** — display only N shares; replenish from hidden reserve automatically; used by institutions to conceal order size
- **Pegged** — automatically reprices to track NBBO midpoint; used by institutional algos and dark pool crossing engines
- **Midpoint IOC** — fills at current NBBO midpoint or cancels; primary dark pool execution type

### Limit Order Book (LOB)
- Per-stock, separate bid and ask side
- Price-time priority: best price first, FIFO within same price level
- Tick size: $0.01 for stocks above $1; varies per scenario
- Structure: price level map → FIFO queue of orders at each level
- Bids sorted descending, asks sorted ascending
- Spread = best ask − best bid

### Opening Auction (9:30 AM)
- Pre-market orders accumulate in an indicative book
- At open, exchange finds the price that maximizes matched volume
- Results in opening gap if overnight news shifts the equilibrium

### Circuit Breakers
- Level 1: 7% decline from prior close → 15-minute halt
- Level 2: 13% decline → 15-minute halt
- Level 3: 20% decline → remainder of day halted
- Individual stock: 10% move in 5 minutes → 5-minute trading pause
- These create scenario-level dramatic moments

### Short Sale Mechanics
- Each stock has a **borrow pool** — shares available to short; pool depletes as short interest grows
- **Locate requirement** (Reg SHO Rule 203): before executing a short sale, broker must confirm a locate is available from the pool; when locates are exhausted, short selling stops regardless of borrow fee — this is the forced-covering trigger in a squeeze
- Borrow fee (annualized rate): rises non-linearly with pool utilization — 0.3% at 0–50% utilization, 5–30% at 50–80%, 50–150%+ above 80%
- If a lender recalls shares, the borrower receives a **buy-in notice** and must cover
- Reg SHO: fails-to-deliver above threshold trigger mandatory close-out
- Short interest % of float is published daily (with 2-week lag, as in reality)

---

## Agent Archetypes

### Design Philosophy
Agents have simple rules but collectively produce complex emergent behavior. No agent has perfect information. Each has a "world model" — an estimate of true value, trend, and risk — that they update from market data and signals. Agents are never omniscient; they make mistakes.

All agents carry a **fear state** — a scalar [0, 1] derived from a trailing window of realized volatility. Fear state scales order submission frequency and size down across all agent types. This is the primary mechanism for volatility clustering (GARCH-like dynamics): when volatility is high, agents pull back, liquidity drops, the next shock is amplified.

---

### 1. Noise / Liquidity Trader
**Population**: 2,000–20,000 per stock (most numerous archetype)
**Account**: small, no leverage
**Rules**:
- Submits random buy or sell limit orders near the midprice, driven by exogenous liquidity needs (rebalancing, cash management) — no directional thesis
- Order size: small, fixed (1–100 shares)
- Price: midpoint ± random offset drawn from a tight distribution (usually 1–3 ticks away)
- Hold time: random; cancels stale orders after a patience threshold
- No fear state, no momentum, no sentiment response — purely exogenous

**Why this archetype is required**: without uninformed liquidity flow, every order in the book is directional, market makers detect informed flow in every trade, spreads go to maximum, and the market degenerates. Noise traders are the counterparty that makes the market function. They should account for ~30–50% of order count. They are cheap to compute and can run at a low scheduling tier.

---

### 2. Retail Trader (NPC)
**Population**: 500–5,000 per stock (varies by scenario)
**Account**: Reg T, $1,000–$50,000
**Rules**:
- Primarily momentum-driven: buy if price rising AND community sentiment positive
- FOMO trigger: if price moves >3% without them, probability of chasing increases sharply
- Fear trigger: if position down >10%, panic-sell probability increases
- Attention-limited: only tracks 1–3 stocks at a time; switches based on community feed
- Options usage: mostly buy calls/puts (rarely sell); bias toward OTM (lottery mentality)
- PDT-constrained if under $25k
- Hold time: hours to days (rarely weeks)
- Entry signal weight: community sentiment 60%, price momentum 30%, "DD quality" 10%

**Collective behavior**: retail herding produces momentum and overshoot. Retail selling produces capitulation. Retail options buying inflates IV and causes dealer hedging flows.

---

### 3. High Frequency Trader (HFT)
**Population**: 20–50 market-making HFTs + 3–10 directional HFTs per exchange
**Account**: proprietary trading firm, no capital constraint for simulation purposes
**Intra-tick latency offset**: 1–50 microseconds (acts first within each tick)
**Rules**:

- **Market making HFT** (population: 20–50): posts quotes at or near NBBO; cancel-to-trade ratio 95–99% (for every executed trade, 20–200 orders are submitted and canceled). This is what creates the "shimmer" in real order book depth. Quote updates happen every tick; each HFT independently competes to capture spread, which drives spread toward minimum tick via Bertrand competition.

- **Statistical arbitrage HFT** (population: 3–5): detects price discrepancy between correlated stocks (index constituent vs ETF basket); arbs the spread instantly; transmits information between correlated stocks.

- **Spoofing** (adversarial, population: 1–2): a two-step strategy — (1) agent already holds a resting order on side A (e.g., ask at $10.05), (2) agent places a large visible bid at $10.00 to create artificial demand signal, (3) price ticks up, the resting ask fills, (4) agent immediately cancels the spoof bid. The contra position must exist before the spoof layer is placed. Illegal in reality; simulated here as an adversarial agent the player can learn to recognize.

- **Momentum ignition** (adversarial, population: 1–2): distinct from spoofing — agent submits aggressive market orders that actually execute and move the price, then immediately flips to trade against the momentum created. Requires real fills; some orders must execute to push price.

- **Order flow detection**: HFTs observe the intra-tick event queue ahead of slower agents (representing co-location / direct feed access). When a large aggressive order is detected in the queue, HFT can insert a response event before the order reaches the book.

- **PFOF routing**: a configurable fraction of retail market orders are routed to a designated market-making HFT (payment for order flow) rather than the lit exchange. The HFT fills at NBBO or better internally, capturing spread while providing price improvement.

**Collective behavior**: compresses spreads in normal conditions via competition; widens and withdraws when adverse selection signal (order flow imbalance) spikes; "vanishing liquidity" in crashes is a mechanical consequence of HFT inventory limits being hit.

---

### 4. Market Maker (Traditional)
**Population**: 1–3 per stock
**Account**: designated market maker, assigned by exchange; portfolio margin
**Intra-tick latency offset**: 50–500 microseconds
**Rules**:
- Obligated to maintain two-sided quotes within exchange rules (max spread, min size)
- **Inventory limits**: hard cap of ±5,000 shares net. When inventory hits the limit, MM switches to one-sided quoting (only the side that reduces inventory) until inventory normalizes. If losses exceed a daily risk limit, MM withdraws entirely — this is the mechanical cause of "vanishing liquidity."
- **Spread = inventory component + adverse selection component**: spread widens both when inventory is skewed (inventory-based, reactive) and when order flow imbalance is high (adverse selection, predictive). Adverse selection is measured as a rolling net signed volume signal: if the last N trades are predominantly buys, the MM widens ask and tightens bid preemptively.
- **Delta hedging is threshold-based, not continuous**: MM hedges aggregate book delta when |net delta| exceeds a configurable band (e.g., ±200 delta units). Hedging is lumpy — it happens in discrete rebalances, not every tick. This produces the "staircase" price pattern during gamma squeezes.
- Vanna (∂delta/∂IV) and charm (∂delta/∂time) are included in the hedging calculation, especially important near expiry and during IV spikes
- **Pin risk at expiry**: within 30 minutes of expiry, options near the strike have delta that swings between 0 and 1 rapidly; MM must hedge more frequently during this window
- Goes "risk-off" (widens dramatically, reduces size) near major binary events

**Collective behavior**: provides baseline liquidity; adverse selection detection means spreads widen before volatility spikes (predictive, not just reactive); options gamma hedging creates self-reinforcing price moves (gamma squeeze); inventory limits cause liquidity withdrawal in crashes.

---

### 5. Hedge Fund
**Population**: 2–5 per stock (may be long or short)
**Account**: prime brokerage, portfolio margin (effective 6–10× leverage on vol-adjusted basis)
**Rules**:
- Fundamental value estimate updated on earnings, macro events, sector rotation
- Builds large positions slowly (VWAP/TWAP execution to minimize market impact)
- Short thesis: identifies overvalued stocks, builds short over days/weeks
- Long/short pairs: simultaneously long one stock, short correlated stock
- Risk management: cuts position if loss exceeds 15% drawdown
- Communicates via analyst notes that surface in the news feed
- May take activist positions (public disclosure triggers 13D filing)

**Collective behavior**: price discovery toward fundamental value over medium term; short squeezes occur when hedge fund short thesis fails and they must cover.

---

### 6. Institutional Investor (Mutual Fund / Pension)
**Population**: 1–3 per stock
**Account**: long-only, no leverage
**Rules**:
- Quarterly rebalancing: buy/sell to maintain target portfolio weight
- Earnings-driven updates: revise position after quarterly earnings
- Executes via algorithms (VWAP/TWAP over hours to days)
- Rebalancing timing has a stochastic offset (± a few days) to avoid perfectly predictable front-running
- Uses iceberg orders to conceal order size; VWAP execution is randomized to resist pattern detection
- Low urgency, high size

**Collective behavior**: creates large, slow flows that sophisticated agents partially front-run; "window dressing" at quarter-end; rebalancing flows are the primary driver of the U-shaped intraday volume profile (concentrated at open and close).

---

### 7. Dark Pool / Block Trader
**Population**: implicit (not a distinct agent, but a mechanism)
**Behavior**:
- Large institutional orders (>$1M notional) have a configurable probability of routing to a simulated dark pool
- Execution price: NBBO midpoint at time of submission (standard ATS crossing price)
- Dark pool orders do not appear in the lit book; no agent has pre-trade visibility into dark pool activity — by definition
- Trade prints to the consolidated tape after execution, visible to all agents simultaneously as a large off-exchange (TRF) print
- Lit market price impact from the print is proportional to print size relative to average daily volume, with a decay function (partial permanent impact + partial temporary)
- HFTs may attempt to detect dark pool activity by submitting small probe IOC orders to see if they get filled by a crossing engine — this is a strategy, not a visibility privilege

---

## Information Architecture

### True Value
Each stock has a **true value** — the price the stock "should" be trading at given fundamentals. True value evolves according to:
- Base: geometric Brownian motion with scenario-calibrated annual volatility `σ_v` (e.g., 30% for a mid-cap stock)
- Events: Poisson-distributed jumps of random sign and scenario-calibrated magnitude on earnings/news/macro events
- **Leverage effect**: when stock price falls, `σ_v` increases proportionally (equity vol rises as the firm becomes more leveraged) — produces asymmetric vol response to up vs down moves
- True value is never visible to agents; they estimate it from signals
- Hedge fund fundamental estimate = true value + noise term; position aggressiveness scales with `|(estimate − price) / estimate|`, not a binary buy/sell signal

### Agent Information Sets
| Agent | Price history | L2 book | Options flow | News | Dark pool prints | Signed tape | True value |
|---|---|---|---|---|---|---|---|
| Noise Trader | No | No | No | No | No | No | No |
| Retail NPC | Yes | Delayed | No | Sentiment only | No (tape only) | No | No |
| HFT | Yes (real-time) | Yes (real-time) | Partial | No | Yes (tape, post-trade) | Yes | No |
| Market Maker | Yes | Yes | Yes | No | Yes (tape, post-trade) | Yes | No |
| Hedge Fund | Yes | Delayed | Yes | Yes (full text) | Yes (tape, post-trade) | Delayed | Estimate |
| Institution | Yes | Delayed | No | Yes | Yes (tape, post-trade) | No | Estimate |
| Player | Yes | Yes | Yes | Feed-filtered | Yes (tape, post-trade) | Yes | No |

Dark pool prints are **post-trade tape events only** — no agent has pre-trade or privileged access to dark pool order flow by definition. "Signed tape" = time-and-sales feed with buyer/seller initiation side, used by HFT and MM for adverse selection / order flow toxicity signals.

### News Event System
- Events are scheduled per scenario; some are random
- Event types: earnings (beat/miss/in-line), FDA ruling, macro (rate decision, CPI print), management change, analyst upgrade/downgrade, secondary offering, share buyback announcement
- Each event has: true impact magnitude, initial headline ambiguity, and community sentiment delta
- The gap between the true impact and the community's initial interpretation creates trading opportunity
- Example: "FDA APPROVES $PHARM DRUG" headlines before the market opens, but the actual approval is for a smaller indication than expected → stock gaps up, then fades as details emerge

---

## Options Simulation

### Options Chain Generation
- For each optionable stock: generate full chain with weekly and monthly expirations
- Strikes: ATM ± N strikes (N determined by scenario/volatility)
- Pricing: Black-Scholes as baseline; IV surface uses a skew model (puts are more expensive due to crash risk)
- Greeks computed per tick: delta, gamma, theta, vega, rho

### IV Surface Dynamics
- Base IV derived from realized volatility with a premium
- **IV skew**: puts > calls (negative skew) for most stocks
- **IV crush**: after binary events (earnings, FDA), IV collapses by 40–70%
- **Fear premium**: IV rises as market falls (VIX-like behavior)
- **Gamma expansion**: IV spikes on short squeeze (demand for calls → market maker hedging → feedback loop)

### Options Agent Behavior
- Retail buys OTM calls (speculative) and OTM puts (protective, sometimes speculative)
- Hedge funds buy puts as hedges; sell calls for yield
- Market maker is on the other side of most retail options flow; must delta-hedge
- Delta hedging by market makers creates feedback: market maker buying stock as call delta rises (gamma squeeze mechanism)

### Options Market Maker Delta Hedging
This is the gamma squeeze engine. Delta hedging is **threshold-based on the aggregate book**, not per-option or continuous:

1. Retail buys large OTM calls on $CORP; MM sells them
2. MM's aggregate net book delta = −2,000 (short delta exposure); within tolerance band → no hedge
3. More retail buying → net delta = −3,500; exceeds hedge band → MM buys shares in a discrete rebalance
4. Price rises → delta increases for existing positions → net delta drifts again → next rebalance
5. The price pattern is a **staircase**, not a smooth ramp: flat periods (within band) punctuated by sharp steps (rebalance events)
6. As price approaches the maximum-gamma strike cluster, gamma is highest and rebalances are largest — the squeeze accelerates non-linearly
7. **Gamma cliff**: if price crosses the max-gamma strike, gamma drops sharply and the self-reinforcing buying stops abruptly; support vanishes; price can reverse violently
8. **Vanna and charm effects**: if IV is also rising during the squeeze (common), vanna (∂delta/∂IV) adds additional buying pressure; as expiry approaches, charm (∂delta/∂time) shifts delta without price movement, forcing additional hedges

MM hedges the aggregate net book — not option-by-option. Hedging frequency increases near expiry and when net delta exceeds the hard band.

---

## Community Feed Simulation

### NPC Post Generation
- Posts are generated procedurally from templates combined with actual simulation data
- Position posts reference actual prices and P&L from the NPC's account
- DD posts are generated from a stock's parameters (short interest, IV, news) + templates
- Loss porn uses actual screenshots of NPC P&L (simulated)
- Post timing correlates with volatility: more posts when price is moving fast

### Sentiment → Price Feedback Loop

The sentiment loop uses an **SIR epidemic model** to prevent runaway dynamics:

- **S (Susceptible)**: retail NPCs not yet invested in the stock — the pool that can still be pulled in
- **I (Infected)**: retail NPCs currently holding a position driven by sentiment
- **R (Recovered/Exhausted)**: retail NPCs who exited and are immune to re-entry for a cooldown period

```
Price rises
  → Infected NPCs post "🚀" → sentiment score rises
    → Susceptible NPCs convert to Infected → demand rises → price rises more
      → S pool depletes → infection rate slows (fewer susceptibles to recruit)
        → First Infected wave takes profit → converts to Recovered
          → Sentiment decays exponentially toward neutral (no new infections to sustain it)
            → Recovered NPCs eventually return to Susceptible after cooldown
              → Crash / consolidation phase
```

**Damping mechanisms** (required to prevent degenerate dynamics):
1. **Susceptible pool depletion**: as more NPCs pile in, the marginal new entrant rate falls — there are fewer uninvested agents to recruit
2. **Sentiment mean-reversion**: sentiment decays at rate proportional to current value when no new posts arrive; a stock cannot stay at peak sentiment without continuous buying
3. **Capital constraint**: Infected NPCs who have deployed their buying power cannot increase demand further; they can only provide supply when they exit
4. **Signal quality damping**: high-quality DD (rating 4–5) sustains sentiment longer; low-quality meme posts (1–2) decay faster

This produces the characteristic meme-stock shape: days-to-weeks of buildup with declining acceleration, a sharp peak, then a rapid crash as Recovered NPCs cannot re-enter immediately.

### Signal Quality Model
DD posts have a hidden quality rating (1–5):
- 1 = pure meme cope, bag holder thesis, wrong
- 3 = real analysis, directionally correct, timing wrong
- 5 = genuinely correct thesis with catalyst
The player cannot see quality directly; must evaluate content. Quality correlates with writer's NPC profile (some NPCs are consistently better or worse).

---

## Market Microstructure Emergent Behaviors

The simulation should produce these naturally from agent interactions. Likelihood assessment based on review:

| Behavior | Primary Mechanism | Likelihood |
|---|---|---|
| Bid-ask spread widens in volatility | MM adverse selection signal + inventory limits | Likely — requires VPIN-style trigger, not raw vol threshold |
| Momentum (trending) | Retail herding, dealer hedging | Very likely |
| Mean reversion | HFT stat arb, value buyers | Likely — requires calibration to cancel momentum at daily frequency |
| Volatility clustering | Fear state scaling across all agents; spread-vol feedback | Likely with fear state implemented; unlikely without it |
| Fat-tailed returns | Correlated herding events; endogenous imitation | Partially likely — emerges during events; requires imitation for quiet periods |
| Short squeeze | Locate exhaustion → forced covering + dealer hedging | Likely with borrow pool depletion model |
| Gamma squeeze | Threshold-based aggregate delta hedging cascade | Likely with staircase hedging model |
| Opening gap | Overnight event repricing via auction | Very likely |
| U-shaped intraday volume | Institutional execution clustering at open/close | Likely |
| Quarter-end effects | Institutional rebalancing with stochastic timing | Likely but less sharp than deterministic scheduling |
| IV smile / skew | Structural skew seed + per-strike demand pressure | Likely — partly baked in, not fully emergent |
| Absence of return autocorrelation | Momentum (retail) cancelled by mean-reversion (HFT/value) | Requires careful calibration — most fragile target |

**Removed claim**: "Afternoon drift" was in the original table but is not a realistic emergent behavior from the agent set described. Institutional VWAP execution produces volume but not systematic directional drift at intraday resolution. Removed.

---

## Calibration Targets

Phase 2 (headless simulation) should validate against these empirical targets before adding UI. Run 252 simulated trading days and compute:

| Target | Empirical Value | Test |
|---|---|---|
| Return kurtosis (daily) | 4–8 (Gaussian = 3) | Fat tails present |
| Raw return autocorrelation | Near zero at lag 1–5 (p > 0.05, Ljung-Box) | Weak-form efficiency |
| Absolute return autocorrelation | Significantly positive to lag 20–50 | Volatility clustering |
| Bid-ask spread (normal conditions) | 0.02–0.05% of price for liquid stock | Spread realism |
| Spread in volatility spike | 0.1–0.5% of price | Liquidity withdrawal |
| Volume-volatility correlation | 0.3–0.5 (contemporaneous) | Activity coupling |
| Intraday volume profile | U-shaped (high open/close, low midday) | Institutional flow |

Parameter sweeps over retail population size, retail herding strength, and HFT aggression should find the region where all seven targets are simultaneously satisfied (moment-matching calibration).
