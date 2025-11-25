# Polkadot SDK: Architecture Patterns and Best Practices [SDK 2512]

> `Status`: Active Standard (December 2025)
> `Version`: Polkadot SDK 2512.0.0 (v1.21.0)
> `Rust`: 1.88.0+

## Overview

This document defines the architectural standard for Polkadot SDK development, specifically for version 2512 (v1.21.0). It integrates critical optimizations in block processing (`LazyBlock`), XCM v5 integration, modernized runtime migration patterns, and Omni Node compatibility.

`Target Audience`: AI Agents & Core Developers. Use this context to avoid deprecated patterns (like `Executive::OnRuntimeUpgrade`) and navigate breaking changes in core pallets.

---

## 1. Critical Breaking Changes in SDK 2512

These are the immediate technical blockers encountered when upgrading from previous versions.

### 1.1 `LazyBlock` in Runtime APIs

`Impact`: `High`|`\*\*cope`: `runtime/src/apis.rs`

The `Core`, `BlockBuilder`, and `TryRuntime` APIs now utilize `LazyBlock` for performance optimization. This requires explicit type conversion using `.into()`.

`Pattern`:

```rust
impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        // ...
        fn execute_block(block: Block) {
            // MUST convert Block -> LazyBlock
            Executive::execute_block(block.into())
        }
        // ...
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        // ...
        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            // MUST convert Block -> LazyBlock
            data.check_extrinsics(&block.into())
        }
    }
}
```

### 1.2 `pallet-assets` Reserves

`Impact`: `Medium`|`\*\*cope`: Runtime Config & Mocks

`pallet-assets` now supports asset reservation natively.

1.  `Config`: Requires `type ReserveData`.
2.  `GenesisConfig`: Requires `reserves` field.

`Runtime Config`:

```rust
impl pallet_assets::Config for Runtime {
    // ...
    type ReserveData = (); // Use unit type if custom data not needed
}
```

`Mock Genesis`:

```rust
polkadot_sdk::pallet_assets::GenesisConfig::<Test> {
    assets: vec![],
    metadata: vec![],
    accounts: vec![],
    reserves: vec![], // NEW FIELD - MUST BE INITIALIZED
    next_asset_id: None,
}
```

### 1.3 Migration from `Executive` to `frame_system`

`Impact`: `Medium`|`\*\*cope`: `runtime/src/lib.rs`

The `OnRuntimeUpgrade` generic parameter in `Executive` is deprecated. Migrations must now be defined in `frame_system::Config`.

`Old Pattern (Deprecated)`:

```rust
pub type Executive = frame_executive::Executive<..., Migrations>;
```

`SDK 2512 Pattern`:

```rust
// runtime/src/lib.rs
pub type Executive = frame_executive::Executive<
  Runtime,
  Block,
  frame_system::ChainContext<Runtime>,
  Runtime,
  AllPalletsWithSystem,
  // No Migrations here!
>;

// runtime/src/configs/mod.rs
impl frame_system::Config for Runtime {
    // ...
    type SingleBlockMigrations = crate::Migrations; // Define here
}
```

### 1.4 XCM v5 & XCMP Queue

`Impact`: `High`|`\*\*cope`: Runtime Config

`cumulus-pallet-xcmp-queue` requires V5 configuration for migration.

```rust
// Implement the V5Config trait
impl cumulus_pallet_xcmp_queue::migration::v5::V5Config for Runtime {
    type ChannelList = ParachainSystem; // or XcmpQueue, depending on trait impl
    type MessageProcessor = <Self as pallet_message_queue::Config>::MessageProcessor;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    // ...
    type ChannelList = ParachainSystem; // New requirement
}
```

---

## 2. Core Architecture Patterns

### 2.1 Unified Dependency Management

The SDK architecture emphasizes workspace-level dependency management to prevent version conflicts. Use `psvm` (Polkadot SDK Version Manager) to align versions.

```toml
[workspace.dependencies]
polkadot-sdk = { version = "2512.0.0", default-features = false }
# All system pallets use version from sdk
cumulus-pallet-parachain-system = { version = "0.25.0", default-features = false }
sp-arithmetic = { version = "28.0.0", default-features = false }
```

`Critical Note`: Hardcoded `dev-dependencies`in individual pallets (e.g.,`sp-io`, `sp-core`) are deprecated due to duplicate lang item conflicts. Use workspace dependencies exclusively.

### 2.2 Modern Runtime Construction

The "Runtime-as-Config" pattern continues to be the standard. Keep `lib.rs` clean; move pallet configurations to `configs/` modules.

```rust
// runtime/src/lib.rs
#[frame_support::runtime]
mod runtime {
    #[runtime::runtime]
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        RuntimeFreezeReason,
        RuntimeHoldReason,
        RuntimeSlashReason,
        RuntimeLockId,
        RuntimeTask,
        RuntimeViewFunction // Essential for Omni Node
    )]
    pub struct Runtime;

    #[runtime::pallet_index(0)]
    pub type System = frame_system;
    // ... additional pallets
}
```

---

## 3. Omni Node & Production Readiness

### 3.1 Runtime View Functions

SDK 2512 reinforces the Omni Node architecture. `RuntimeViewFunction` is critical for exposing runtime internals without state modification.

```rust
impl frame_support::view_functions::runtime_api::RuntimeViewFunction<Block> for Runtime {
    fn execute_view_function(
        id: frame_support::view_functions::ViewFunctionId,
        input: Vec<u8>
    ) -> Result<Vec<u8>, frame_support::view_functions::ViewFunctionDispatchError> {
        Runtime::execute_view_function(id, input)
    }
}
```

### 3.2 Required Runtime APIs

Missing runtime APIs don't cause compilation errors but prevent block production (node starts but stuck at genesis).

`Required APIs for Production`:

- `GetParachainInfo` - Parachain identification
- `RelayParentOffsetApi` - Relay chain communication
- `AuraApi` - Consensus authorities
- `AuraUnincludedSegmentApi` - Async backing validation
- `CollectCollationInfo` - Candidate packaging

```rust
// RelayParentOffsetApi - Critical for relay chain communication
impl cumulus_primitives_core::RelayParentOffsetApi<Block> for Runtime {
    fn relay_parent_offset() -> u32 {
        0  // Standard offset
    }
}
```

### 3.3 Async Backing Configuration

Modern parachains must use `FixedVelocityConsensusHook` matching the official template.

```rust
// FixedVelocityConsensusHook for async backing (REQUIRED for template compliance)
pub type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
    Runtime,
    RELAY_CHAIN_SLOT_DURATION_MILLIS,
    BLOCK_PROCESSING_VELOCITY,
    UNINCLUDED_SEGMENT_CAPACITY,
>;

// Critical Macro: Handles set_validation_data
cumulus_pallet_parachain_system::register_validate_block! {
    Runtime = Runtime,
    BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
```

---

## 4. Advanced Integration Patterns

### 4.1 XCM Hybrid Registry Pattern

For robust foreign asset handling, use the Hybrid Registry pattern which persists asset mappings to handle XCM version changes.

```rust
impl pallet_asset_registry::Config for Runtime {
    type RegistryOrigin = AssetsForceOrigin;
    type AssetIdGenerator = LocationToAssetId; // Deterministic hashing
    type AssetOwner = AssetRegistryAccount;
}
```

### 4.2 Trait-Based Communication

Modern pallets communicate through well-defined traits rather than direct dependencies.

```rust
pub trait CrossPalletInterface<AccountId, AssetKind, Balance> {
    fn execute_operation(origin: AccountId, asset: AssetKind, amount: Balance) -> DispatchResult;
}

impl<T: Config> CrossPalletInterface<T::AccountId, T::AssetKind, T::Balance> for Pallet<T> {
    fn execute_operation(origin: T::AccountId, asset: T::AssetKind, amount: T::Balance) -> DispatchResult {
        // Implementation
        Ok(())
    }
}
```

---

## 5. Development Workflow

### 5.1 Testing Hygiene

- `Mock Update`: Always update `pallet_assets::GenesisConfig` in mocks when upgrading SDK (add `reserves`).
- `Type Safety`: Ensure `ReserveData` is defined in all `pallet_assets::Config` implementations (Runtime + Mocks).
- `Workspace Tests`: Run `cargo test --workspace` to validate all pallets and runtime integrations simultaneously.

### 5.2 Production Verification

Before deployment checklist:

1.  `Check Weights`: Run benchmarks with SDK 2512 tooling (`frame-omni-bencher`).
2.  `Verify Migrations`: Ensure `SingleBlockMigrations` includes `MigrateToLatestXcmVersion`.
3.  `Validate APIs`: Confirm `LazyBlock` usage does not introduce overhead.
4.  `Template Compliance`: Verify `register_validate_block!` macro is present and `ConsensusHook` is `FixedVelocity`.

---

## 6. Summary of SDK 2509 vs 2512

| Feature             | SDK 2509                     | SDK 2512                        | Action Required                           |
| :------------------ | :--------------------------- | :------------------------------ | :---------------------------------------- |
| `**`Runtime API`**` | `Block`                      | `LazyBlock`                     | Update `apis.rs` signatures & conversions |
| `**`Assets`**`      | Standard Config              | + `ReserveData`                 | Update Config & Mock Genesis              |
| `**`Migrations`**`  | `Executive<..., Migrations>` | `System::SingleBlockMigrations` | Move migration tuple to `frame_system`    |
| `**`XCM`**`         | v4/v5 Hybrid                 | v5 Focus                        | Implement `V5Config` for Queue            |
| `**`Toolchain`**`   | Rust 1.81+                   | Rust 1.88                       | Update `rust-toolchain.toml`              |

se this guide as the authoritative reference for all code generation tasks targeting the current Polkadot SDK.
