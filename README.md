# `TMCTOL`: Token Minting Curve + Treasury-Owned Liquidity

A tokenomics framework with mathematically guaranteed price boundaries through liquidity accumulation. Transforms unlimited downside risk into calculable bounded risk.

## Key Features

### Price Boundaries

- `Ceiling`: Driven by linear minting curve
- `Floor`: Backed by treasury-owned XYK liquidity
- `Worst-case`: 11–25% of ceiling, depending on bucket utilization

### Self-Reinforcing System

- `Price Ratchet`: Growth raises floor and ceiling
- `Bidirectional Compression`: Burning lowers ceiling, TOL raises floor
- `Automatic Liquidity`: 66.6% of mints locked in TOL

## How It Works

### Core Components

1. `TMC`: Minting with supply-dependent linear price growth
2. `TOL`: XYK liquidity from mints under community control
3. `Axial Router`: Routes to best price with fee burning

### Mathematical Guarantees

```rust
// TMC Ceiling
let ceiling = initial_price + (slope × total_supply / PRECISION);

// XYK floor
let k = TOL_native * TOL_foreign;
let floor_price = k / (TOL_native + tokens_sold)²;
```

## Getting Started

### For Researchers

- [TMCTOL Manifesto](./docs/manifesto.en.md)
- [Specification](./docs/tmctol.en.md) - Framework foundation and core concepts
- [L2 TOL](./docs/l2-tol.en.md) - Second-order DAOs with autonomous liquidity

### Simulator & Validation

- [Model](./simulator/model.js) - Tokenomics implementation
- [Tests](./simulator/tests.js) - 55 test cases validating system guarantees
- [Tests Mirror](./simulator/tests.md) - Live Test Documentation Suite

Run tests:

```bash
deno ./simulator/tests.js
```
