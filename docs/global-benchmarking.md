# TASK PROTOCOL: Global Benchmarking Implementation (Substrate V2)

`Target`: AI Agent / Core Developer
`Context`: Polkadot SDK 2512 Standards
`Objective`: Achieve 100% benchmarking coverage for all custom pallets, ensuring accurate Weight (RefTime + ProofSize) calculation for the Runtime.

---

## 1. Executive Summary

We have successfully implemented the Testing Framework. The next critical phase is `Benchmarking`.
Unlike unit tests, which verify logic, benchmarks verify `computational complexity` and `storage intensity`.

`Mandatory Requirement`: All benchmarks must adhere to the `Frame Benchmarking V2` syntax (Rust procedural macros). Legacy `benchmarks!` macros are strictly forbidden.

---

## 2. Technical Standards (The "2025 Spec")

### 2.1. Syntax & Structure

- `Macro`: Use `frame_benchmarking::v2::*`.
- `Typing`: Code must be strictly typed (Rust-native).
- `Granularity`: Every `#[pallet::call]` (dispatchable) must have a corresponding benchmark.
- `Complexity analysis`:
  - Use `Linear<Min, Max>` for quantifiable inputs (vector lengths, iteration counts).
  - Use `constant` for fixed-cost operations.

### 2.2. Weight Dimensions

In 2025, a "Weight" consists of two dimensions. Your benchmarks must capture both:

1.  `RefTime`: Execution time on reference hardware (picoseconds).
2.  `ProofSize`: The amount of storage data read (Merkle proof size in bytes).

### 2.3. Isolation

- Use `whitelisted_caller()`: Do not benchmark the signature verification logic (handled by the System pallet).
- `State Setup`: Pre-fill storage with worst-case scenarios before the `#[extrinsic_call]`.

---

## 3. Implementation Roadmap

### Phase A: Infrastructure Setup (Scaffolding) [COMPLETED]

`Action`: For every pallet in `/pallets/*`:

1.  Ensure `Cargo.toml` includes `frame-benchmarking` with the `optional = true` flag.
2.  Ensure the `runtime-benchmarks` feature is correctly propagated from the workspace root down to the pallet.
3.  Create `src/benchmarking.rs`.

### Phase B: Writing the Benchmarks (Logic) [COMPLETED]

`Action`: Implement the logic. Follow this strictly typed pattern:

```rust
// src/benchmarking.rs
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_std::vec;

#[benchmarks]
mod benches {
    use super::*;

    // 1. Define complexity parameters (e.g., 'x' items in a list)
    #[benchmark]
    fn process_list(x: Linear<1, 1000>) {
        // SETUP: Prepare the state to represent the worst case for 'x'
        let caller: T::AccountId = whitelisted_caller();
        let data = vec![0u8; x as usize];

        // Pre-seed storage if necessary to force DB reads
        Something::<T>::put(&data);

        // EXECUTION: The target extrinsic
        #[extrinsic_call]
        process_list(RawOrigin::Signed(caller), data);

        // VERIFICATION: Ensure it actually worked
        assert_eq!(Something::<T>::get().len(), x as usize);
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
```

### Phase C: The Weight Generation Pipeline [NEXT STEP]

`Action`: Execute the generation script on reference hardware. We are not guessing weights; we are measuring them.

1.  `Verification`: `cargo test --features runtime-benchmarks` must pass to ensure benchmark logic is sound (Completed).
2.  `CLI Generation`: Use the standard command to generate `weights.rs` for each pallet.
    ```bash
    ./target/release/polkadot-omni-node benchmark pallet \
      --chain dev \
      --pallet "pallet_name" \
      --extrinsic "*" \
      --steps 50 \
      --repeat 20 \
      --output pallets/pallet-name/src/weights.rs \
      --template .maintain/frame-weight-template.hbs
    ```

### Phase D: Runtime Integration

`Action`: Wire the results into `runtime/src/lib.rs`.

1.  Import the `WeightInfo` trait from the pallet.
2.  Inject the generated weights into the Pallet Config:
    ```rust
    impl pallet_template::Config for Runtime {
        // ...
        type WeightInfo = pallet_template::weights::SubstrateWeight<Runtime>;
    }
    ```

---

## 4. Critical Guidelines for the Agent

1.  `Worst-Case Assumption`: Always benchmark the path of highest resistance. If a function iterates over a map, the map must be full. If it resizes a vector, the vector must be at maximum capacity.
2.  `No Logic Branching`: Benchmarks must be deterministic. Avoid `if/else` inside the `#[benchmark]` body based on random seeds.
3.  `Database Hygiene`: The benchmark timer starts when `#[extrinsic_call]` begins. Ensure all setup (creating accounts, seeding balances) happens _before_ that line.
4.  `Clean Traits`: Ensure `src/weights.rs` defines a `pub trait WeightInfo`. Do not hardcode weights in the pallet call; always use `T::WeightInfo::function_name()`.

## 5. Deliverables Checklist

- [x] `Source Code`: `benchmarking.rs` created and validated for all custom pallets (`axial-router`, `zap-manager`, `treasury-owned-liquidity`, `burning-manager`, `token-minting-curve`).
- [ ] `Artifacts`: `weights.rs` generated via CLI on reference hardware.
- [x] `Configuration`: `Pallet::Config` prepared to receive generated weights (currently using default `()`).
- [x] `Validation`: `cargo test --features runtime-benchmarks` verifies benchmark logic execution.
- [x] `Documentation`: Code follows Frame Benchmarking V2 standards with worst-case assumptions.

---

`Status`: Phase A/B Completed (Logic Implemented). Phase C (Generation) Pending Hardware Access.
`Priority`: High.
`Deadline`: December 2025
