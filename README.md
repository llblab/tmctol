# TMCTOL: Token Minting Curve + Treasury-Owned Liquidity

> A tokenomics framework with mathematically guaranteed price boundaries, implemented as a production-ready Polkadot SDK Parachain.

This repository serves as a Specification & Reference Framework for a self-sustaining economic model where liquidity accumulation transforms unlimited downside risk into calculable bounded risk. It combines a rigorous economic specification with a modern, forkable blockchain template optimized for Polkadot's Omni Node architecture.

---

## 1. Key Features

### The Economics (Physics)

- Price Boundaries:
  - Ceiling: Driven by a linear minting curve ($P = P_0 + slope \cdot s$).
  - Floor: Backed by treasury-owned XYK liquidity.
  - Corridor: Worst-case downside limited to 11–25% of the ceiling, depending on bucket utilization.
- Self-Reinforcing System:
  - Price Ratchet: Growth raises both the floor and the ceiling.
  - Bidirectional Compression: Burning lowers the ceiling, while TOL accumulation raises the floor.
  - Automatic Liquidity: 66.6% of mints are automatically locked in Treasury-Owned Liquidity (TOL).

### The Technology (Engine)

- DEX Parachain: A comprehensive DeFi infrastructure featuring automated market making and asset management.
- Axial Router: A multi-AMM router that enforces fee burning and optimal trade execution.
- Omni Node Architecture: Fully optimized for the Polkadot SDK 2512 standard, eliminating node boilerplate.
- Universal Asset Support: Native, Local, and XCM-ready asset management.

---

## 2. Project Structure

This monorepo contains two distinct but synchronized layers:

- [`/simulator`](./simulator/) (The Specification):
  - _Language_: JavaScript / BigInt.
  - _Purpose_: The mathematical "Source of Truth". Validates the economic formulas using infinite precision before any code is written.
- [`/template`](./template/) (The Reference Implementation):
  - _Language_: Rust / Substrate.
  - _Purpose_: The production-ready Parachain Template. Implements the standard using `frame::v2` patterns.
- [`/docs`](./docs/) (The Knowledge Base):
  - Comprehensive technical guides and architectural insights.

---

## 3. Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Polkadot Omni Node](https://github.com/paritytech/polkadot-sdk)

### Development Setup

1. Validate the Economics (Simulator)
   Ensure the mathematical guarantees hold:

```bash
node ./simulator/tests.js
```

2. Build the Parachain (Template)
   Compile the Rust implementation:

```bash
cd template

# Build the runtime workspace
cargo build --release --workspace

# Run implementation tests
cargo test --workspace
```

3. Run the Node
   Start a local development chain:

```bash
polkadot-omni-node --dev --tmp
```

---

## 4. Documentation

📖 [Complete Documentation Index](./docs/README.md) - Start here for technical guides.
🤖 [Agent Conventions](./AGENTS.md) - Development protocols and project context.

### Key Guides

- [Economic Specification](./docs/tmctol.en.md) - Framework foundation and core concepts.
- [Axial Router Architecture](./docs/axial-router-architecture.md) - Multi-AMM trading infrastructure.
- [Production Deployment](./docs/production-deployment-guide.md) - Deployment and configuration.

### Mathematical Guarantees

```rust
// TMC Ceiling Calculation
let ceiling = initial_price + (slope × total_supply / PRECISION);

// XYK Floor Calculation
let k = TOL_native * TOL_foreign;
let floor_price = k / (TOL_native + tokens_sold)²;
```
