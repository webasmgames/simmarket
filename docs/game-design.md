# Game Design Document — SimMarket

## Vision

A tick-accurate stock market simulation where the player is a retail trader with a small account, following a WSB-style synthetic community. The fantasy is the absurd, meme-fueled, occasionally-genuine world of internet retail trading: reading "DD" posts at 2am, YOLOing into a meme stock, watching the ticker gap down 30% at open, posting loss porn. The simulation underneath is rigorous — real market microstructure, real agent archetypes, real instrument mechanics — but the experience is darkly comedic and human.

The player is always outgunned. HFTs see your order before you do. Hedge funds have prime brokerage leverage you don't. Market makers print money on your spread. The community is irrational and occasionally right. Winning is the exception; understanding why you lost is the game.

---

## Core Loop

```
Read community feed / DD posts
  → Identify a play (stock, options chain, thesis)
    → Size the position within account constraints
      → Enter / manage the trade
        → Tick forward — price moves, agents react
          → Exit (voluntarily or via margin call)
            → Post outcome to community feed
              → Scenario objective check
```

Time flows at a player-controlled rate. The player can pause to think, slow down during volatile moments, or fast-forward through quiet sessions. The simulation never waits — agents are always trading.

---

## Player Account

### Account Type: Reg T Margin Account
- **Starting cash**: varies by scenario (e.g. $10,000)
- **Buying power**: 2× equity for marginable securities (50% initial margin requirement)
- **Maintenance margin**: 25% equity; falling below triggers a margin call
- **Margin call resolution**: player must deposit cash, close positions, or the broker force-liquidates at the worst price in the next tick
- **Short selling**: allowed; requires borrowing shares (borrow fee varies by short interest and availability); buy-in risk if lender recalls shares

### Pattern Day Trader Rule
- Accounts under $25,000 are limited to **3 round-trip day trades in any rolling 5-day window**
- Exceeding the limit flags the account; broker may restrict to closing-only for 90 days
- This is a real constraint and a meaningful design pressure — the player must choose when to use day trades

### Options Approval Levels
- **Level 1** — covered calls, cash-secured puts (available from start)
- **Level 2** — long calls/puts, debit spreads (unlocked after first scenario)
- **Level 3** — credit spreads, naked puts (unlocked later in campaign)
- Naked calls are never available to retail (Level 4 / portfolio margin territory)

### Order Types Available to Player
- Market order
- Limit order
- Stop-loss / stop-limit
- Good-till-cancel (GTC)
- Day order

---

## Instruments

### Stocks
- Standard equity shares on a simulated exchange
- Each stock has a sector, float size, short interest %, and institutional ownership %
- These parameters shape agent behavior and community interest

### Options
- American-style options (exercisable any time before expiry)
- Player sees: strike, expiry, bid/ask, last price, volume, open interest, IV, delta, gamma, theta, vega
- Options expire worthless, get exercised, or can be closed before expiry
- IV crush after earnings events is simulated
- 0DTE (zero days to expiry) options are available on major indices/ETFs — extremely high gamma, WSB staple

### Short Selling
- Player borrows shares through broker; availability and borrow rate update each market session
- Hard-to-borrow stocks (high short interest) have elevated rates and risk of buy-in
- Short squeeze mechanic: if a heavily shorted stock gaps up sharply, short sellers cover, which drives price higher

### Margin / Leverage
- Margin interest accrues daily on borrowed amounts
- Reg T 50% initial / 25% maintenance
- Margin calls are dramatic — broker message appears, countdown timer, forced liquidation at bid if unresolved

---

## The Community Feed (WSB-Style NPC Simulation)

The feed is the game's information layer. It is noisy, emotional, and occasionally contains real signal buried in noise.

### Feed Post Types

| Type | Description | Signal Quality |
|---|---|---|
| **DD Post** | Long-form analysis of a stock (thesis, financials, options play) | Variable — can be genius or bag-holding cope |
| **Position Post** | NPC shows their position: "YOLO'd my life savings into $XYZ calls" | Sentiment signal; watch for crowd pile-in |
| **Gain Porn** | Screenshot of a green P&L | Confirms a play worked; usually posted at the top |
| **Loss Porn** | Screenshot of a devastating loss | Often marks a bottom; community eulogizes |
| **News Reaction** | Community reacts to a market event (earnings beat/miss, FDA approval, etc.) | Real-time sentiment |
| **Meme / Shitpost** | Noise | Noise, except when volume spikes — extreme meme volume = incoming pump signal |
| **Mod Post** | Trending tickers list, daily discussion thread | Aggregated sentiment data |

### Community Sentiment Meter
- Each stock has a sentiment score (−100 to +100) derived from feed activity
- High positive sentiment → NPC retail buying pressure → price momentum
- Sentiment can be "manufactured" by coordinated whale posting (a scenario mechanic)
- The player can see the sentiment meter but cannot directly control it

### WSB Vocabulary (flavor text, authenticity)
- "Rocket" / "🚀🚀🚀" — going to the moon
- "Tendies" — profits
- "Retard / regarded" — self-deprecating
- "Diamond hands" — holding through pain
- "Paper hands" — selling early
- "Apes" — the community
- "YOLO" — all-in single position
- "DD" — due diligence
- "Theta gang" — option premium sellers
- "Bag holding" — stuck in a losing position
- "Short squeeze" — forced covering driving price up
- "Gamma squeeze" — options market makers forced to buy as delta hedging

---

## Scenario Campaign

Scenarios are self-contained episodes with defined starting conditions, objectives, and time limits. Each teaches a mechanic and references a real-world market event archetype.

### Scenario Structure
Each scenario provides:
- Starting capital and account type
- One or more "primary tickers" (the scenario focuses on these)
- Market background (current conditions, relevant news)
- Objective(s) with grading (bronze/silver/gold)
- Unlocks (new instruments, approval levels, community intel)

### Scenario List (Draft)

#### 1. "First Blood" — Tutorial
- $5,000 cash account, no margin
- Single stock, no options
- Objective: make any profit in 5 simulated trading days
- Teaches: order types, bid/ask spread, order book, basic agent behavior

#### 2. "The Squeeze" — Short Squeeze Archetype (GME/AMC-style)
- $10,000 margin account
- $CORP is 140% short interest, float is small, institutions are short
- Community starts posting squeeze DD
- Objective: profit from the squeeze without getting caught holding at the top
- Teaches: short interest mechanics, borrow rates, gamma squeeze interaction, exit timing
- Twist: the squeeze is real, but the top comes fast and the community keeps posting "🚀" as it crashes

#### 3. "Theta Gang" — Premium Selling
- $25,000 account (unlocks day trading)
- Sell covered calls on a stable stock, collect premium
- Market stays range-bound… until it doesn't
- Objective: survive an unexpected spike that blows through your short calls
- Teaches: options selling, assignment risk, IV expansion

#### 4. "Fed Day" — Macro Volatility
- $15,000 account
- FOMC decision in 2 simulated hours; IV is elevated
- Community is split: "pivot 🚀" vs "hawkish 🌈🐻"
- Objective: position correctly for the announcement OR profit from IV crush via straddle/strangle
- Teaches: event-driven trading, IV crush, news interpretation

#### 5. "Earnings YOLO" — 0DTE Options
- $8,000 account
- $TECH reports earnings after close; community consensus is a beat
- 0DTE options available; spreads are wide
- Objective: turn $8k into $50k (or lose it all trying)
- Teaches: 0DTE mechanics, gamma, IV crush on earnings, the house always wins on spreads

#### 6. "Dark Money" — Institutional Manipulation
- $20,000 account
- Player notices unusual options activity in $PHARM before an FDA ruling
- "Whale" institutional agent is accumulating quietly; community hasn't noticed yet
- Objective: identify the signal before the community does and position ahead of the move
- Teaches: unusual options activity, dark pool flow, order book reading

#### 7. "The Crash" — Bear Market Survival
- $30,000 account at the peak of a simulated bull run
- Fed hikes rates; tech sector starts bleeding; community is in denial
- Objective: preserve 80% of capital while the market drops 40%
- Teaches: hedging, put buying, cash is a position, stop-losses

#### 8. "Market Maker" — Understanding the Other Side (Perspective Shift)
- Player controls a simulated market-making desk for one session
- Must maintain two-sided quotes, manage inventory, hedge delta
- Objective: earn the spread without getting run over by informed flow
- Teaches: how spreads work, why liquidity disappears in volatile markets, the dealer's perspective

---

## UI/UX Concepts

### Main Views
- **Chart view** — candlestick chart with volume, indicators (VWAP, BBANDS, RSI), selectable timeframe
- **Order book** — live L2 depth, bid/ask walls, tape (time & sales)
- **Options chain** — full chain with Greeks, OI, IV per strike/expiry
- **Portfolio** — positions, P&L, buying power, margin usage
- **Community feed** — WSB-style scrolling feed, pinned DD posts, trending tickers sidebar
- **News ticker** — macro and company-specific headlines

### Time Controls
- Pause / play / 2× / 5× / 10× / max speed
- "Skip to open" / "Skip to close" buttons
- Slow-motion mode auto-triggers during high-volatility events (dramatic effect)

### Feedback and Feel
- Market tape sound (optional) — rapid clicking during volume spikes
- Visual effects on extreme price moves — screen flash, vignette
- Margin call UI — hostile red overlay, countdown, force-liquidation animation
- "Loss porn" generated for the player's own devastating losses, shareable to the feed
