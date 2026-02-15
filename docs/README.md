# TMCTOL Documentation Hub

> `Comprehensive Knowledge Base` | From Mathematical Specifications to Production Deployment.

This directory serves as the central navigation hub for the TMCTOL ecosystem. It unifies the theoretical economic specifications with the concrete technical architecture of the Polkadot SDK Parachain implementation.

## Documentation Structure

### 1. Essential Foundation

`START HERE`: Before contributing code or designing features, understanding the underlying framework patterns is mandatory.

- `[Polkadot SDK 2512 Best Practices](./polkadot-sdk-2512-insights.md)`
  _! REQUIRED READING !_
  Modern architecture patterns for the Polkadot SDK 2512 standard. Covers unified dependency management, `frame::v2` macros, `Omni Node` utilization, and the "Runtime-as-Config" pattern.

### 2. Philosophy & Vision

The strategic context defining "Why" the system exists.

- `The Fractal-Cybernetic Manifesto`
  Defines the "Real DAO" philosophy: a transition from Subjective Policy (Politics) to Objective Mechanism (Cybernetics). Outlines the separation of `L1 Strategy` (Mathematical Sovereignty) and `L2 Tactics` (Fractal Federation).
  - [English](./manifesto.en.md) | [Russian](./manifesto.ru.md)

### 3. Core Specifications (The Math)

The theoretical "Source of Truth" defining the economic laws independent of the blockchain implementation.

#### TMCTOL Protocol

The main protocol combining minting curves with automatic liquidity generation.

- [English](./tmctol.en.md) | [Russian](./tmctol.ru.md)

#### L2 TOL Integration

Layer-2 governance system with declining voting power and proxy superiority mechanics.

- [English](./l2-tol.en.md) | [Russian](./l2-tol.ru.md)

### 4. Implementation Architecture (The Code)

Technical guides for the Rust/Substrate implementation found in the `/template` directory.

- `[AAA Specification](./aaa-specification.md)`
  Deterministic Account Abstraction Actor specification (runtime primitive). Covers scheduler model, pipeline/task semantics, trigger model, circuit breakers, rent/lifecycle, and safety invariants.

- `[Core Architecture](./core-architecture.md)`
  _! SYSTEM BACKBONE !_
  The token-driven design foundation. Covers system accounts structure, "Omnivorous" balance monitoring, Bitmask Asset Taxonomy, separation of Abstract Actors from Concrete Pallets, and the operational token lifecycle checkpoint runbook.

- `[Axial Router Architecture](./axial-router-architecture.md)`
  The economic coordination actor. Details mechanism-over-policy design, EMA oracle, fee burning flows, and integration with Asset Conversion.

- `[Token Minting Curve Architecture](./token-minting-curve-architecture.md)`
  The unidirectional minting engine. Covers the linear bonding curve P = P₀ + slope·s, quadratic cost integration, TotalIssuance-based supply, and conservation invariants.

- `[Treasury-Owned Liquidity Architecture](./treasury-owned-liquidity-architecture.md)`
  The `tol_id`-scoped multi-bucket accumulation engine. Covers 4-bucket strategy, token-domain LP ingress routing (`AssetId -> TolId`), domain-scoped bucket/queue/buffer state isolation, Request Queue → Zap Buffer → Distribution pipeline, LP-only bucket invariants (`on_idle` non-LP sweeps to Burning Manager), LP ingress validation (`is_lp`), and clean-slate LP namespace initialization.

- `[Burning Manager Architecture](./burning-manager-architecture.md)`
  The passive deflationary actor. Documents the two-phase `on_idle` engine: LP unwinding, foreign→native swap via Router, and unconditional native burn.

- `[Zap Manager Architecture](./zap-manager-architecture.md)`
  The "Omnivorous" liquidity provisioning actor. Explains the state machine, "Patriotic Accumulation" strategy, and `RetryCooldown` resilience patterns.

- `[Asset Registry Architecture](./asset-registry-architecture.md)`
  The foreign asset gateway. Documents the Hybrid Registry pattern: deterministic hashing at registration, persistent Location→AssetId mapping, and XCM version migration.

### 5. Production & Operations

Guides for running the system in the real world.

- `[Production Deployment Guide](./production-deployment-guide.md)`
  Detailed operational guide. Covers infrastructure requirements, security hardening, monitoring systems, and chain specification generation.

- `[Paseo Testnet Guide](./paseo-testnet-guide.ru.md)`
  _! HANDS-ON TESTING !_
  Step-by-step guide for manual testing on the Paseo testnet. Covers: obtaining PAS, opening HRMP channels with AssetHub, registering foreign assets, token-domain binding (`bindTokenToTol`), minting Native via TMC, checking token-resolved TOL ingress routing, swaps through the Router, and burn verification.

- `[Zombienet Manual Testing](./zombienet-manual-testing.ru.md)`
  Step-by-step instructions for manual TMCTOL testing in local Zombienet. Covers: registration of synthetic foreign assets, the full economic cycle TMC→Zap→TOL, Router routing, the BM burn cycle, governance bucket reallocation, and edge cases across all security fixes.

- `[Governance Operations](./governance-operations.md)`
  Operational checklist for governance actions: parameter inventory, tuning guidance, pre-checks, and post-change validation across all pallets.

- `[Foreign Asset Migration](./foreign-asset-migration.md)`
  Guide for migrating XCM Location→AssetId keys, validating the 0xF… foreign namespace, and executing E2E XCM validation steps.

- `[Global Benchmarking Protocol](./global-benchmarking.md)`
  Task protocol for achieving 100% benchmarking coverage. Covers `frame_benchmarking::v2` patterns, `frame-omni-bencher` integration, and weight normalization.
