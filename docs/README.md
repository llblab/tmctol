# TMCTOL Documentation Hub

> `Comprehensive Knowledge Base` | From Mathematical Specifications to Production Deployment.

This directory serves as the central navigation hub for the TMCTOL ecosystem. It unifies the theoretical economic specifications with the concrete technical architecture of the Polkadot SDK Parachain implementation.

## 📚 Documentation Structure

### 1. Essential Foundation

`START HERE`: Before contributing code or designing features, understanding the underlying framework patterns is mandatory.

- `[Polkadot SDK 2512 Best Practices](./polkadot-sdk-2512-insights.md)`
  _! REQUIRED READING !_
  Modern architecture patterns for the Polkadot SDK 2512 standard. Covers unified dependency management, `frame::v2` macros, `Omni Node` utilization, and the "Runtime-as-Config" pattern.

### 2. Philosophy & Vision

The strategic context defining "Why" the system exists.

- `[The Fractal-Cybernetic Manifesto](./real-dao-manifesto.md)`
  Defines the "Real DAO" philosophy: a transition from Subjective Policy (Politics) to Objective Mechanism (Cybernetics). Outlines the separation of `L1 Strategy` (Mathematical Sovereignty) and `L2 Tactics` (Fractal Federation).

### 3. Core Specifications (The Math)

The theoretical "Source of Truth" defining the economic laws independent of the blockchain implementation.

#### TMCTOL Protocol

The main protocol combining minting curves with automatic liquidity generation.

- [English](./tmctol.en.md) | [Русский](./tmctol.ru.md)

#### L2 TOL Integration

Layer-2 governance system with declining voting power and proxy superiority mechanics.

- [English](./l2-tol.en.md) | [Русский](./l2-tol.ru.md)

#### Axial Router Specification

The abstract routing logic and pathfinding requirements.

- [English](./axial-router.en.md) | [Русский](./axial-router.ru.md)

### 4. Implementation Architecture (The Code)

Technical guides for the Rust/Substrate implementation found in the `/template` directory.

- `[Core Architecture](./core-architecture.md)`
  _! SYSTEM BACKBONE !_
  The token-driven design foundation. Covers system accounts structure, "Omnivorous" balance monitoring, Bitmask Asset Taxonomy, and the separation of Abstract Actors from Concrete Pallets.

- `[TOL Implementation Guide](./tol-implementation-guide.md)`
  Deep dive into the Treasury-Owned Liquidity system. Covers the 4-bucket architecture (Floor, Operations, Overflow), the buffer-based Zap algorithm, and mathematical floor price calculations.

- `[Axial Router Architecture](./axial-router-architecture.md)`
  The concrete implementation of the router as an economic coordination actor. Details the mechanism-over-policy design, fee burning flows, and integration with Asset Conversion.

- `[Zap Manager Architecture](./zap-manager-architecture.md)`
  Documentation for the "Omnivorous" liquidity provisioning actor. Explains the state machine, "Patriotic Accumulation" strategy, and `RetryCooldown` resilience patterns.

### 5. Production & Operations

Guides for running the system in the real world.

- `[Polkadot SDK 2509 Insights](./polkadot-sdk-2509-insights.md)`
  Modern architecture patterns, best practices, and integration guide for Polkadot SDK stable2509.

- `[Production Deployment Guide](./production-deployment-guide.md)`
  Detailed operational guide. Covers infrastructure requirements, security hardening, monitoring systems, and chain specification generation.
