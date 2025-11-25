# Pallets Directory

This directory contains the custom pallets that implement the core DeFi functionality of the parachain. Each pallet is designed with modern FRAME patterns and follows the project's architectural principles for production-ready blockchain applications.

## 🏗️ Available Pallets

### [Axial Router](./axial-router/)

Multi-AMM trading infrastructure providing intelligent routing across different automated market makers. Implements trait-based architecture for extensible AMM support with optimal price discovery and execution.

## 🎯 Pallet Architecture Philosophy

Our pallets implement several key architectural patterns:

- **Trait-Based Extensibility**: Clean interfaces for cross-pallet communication
- **KISS Principle**: Simple, maintainable implementations that scale
- **Production-Ready Design**: Economic security, error handling, and operational excellence
- **Automated Execution**: Sophisticated scheduling and batch processing capabilities

## 📚 Technical Implementation Guides

For detailed technical implementation, architectural decisions, and production deployment patterns, see the comprehensive guides in the [documentation directory](../../docs/):

- **[Axial Router Architecture Guide](../../docs/axial-router-architecture.md)** - Modern multi-token routing system optimized for TMC ecosystems

## 🚀 Quick Start

Each pallet directory contains:

- **Source code** (`src/`) with comprehensive implementation
- **Local README** with pallet-specific orientation and quick reference
- **Tests** demonstrating functionality and integration patterns

Navigate to individual pallet directories for component-specific orientation and development guidance.

## 🔧 Development Integration

These pallets are designed for seamless integration with the runtime configuration located in [`../runtime/src/configs/`](../runtime/src/configs/mod.rs). The modular design enables flexible deployment scenarios while maintaining architectural consistency.

For development workflow and contribution guidelines, refer to the [Documentation index](../../docs/README.md) for comprehensive technical guides and architectural patterns.
