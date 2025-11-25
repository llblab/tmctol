# TOL Implementation Guide

# Treasury-Owned Liquidity: Architecture and On-Chain Implementation

## Executive Summary

Treasury-Owned Liquidity (TOL) represents a sophisticated economic primitive that transforms treasury assets into perpetually managed liquidity pools. This guide provides a comprehensive overview of the on-chain implementation, focusing on the Zap algorithm, bucket system, and mathematical foundations that enable guaranteed floor prices.

## 1. System Architecture

### 1.1 Core Components

The TOL system consists of three primary components working in synergy:

1. **Four-Bucket Distribution System**: Segregates liquidity into strategically allocated buckets with controlled emission rates
2. **Zap Algorithm**: Automated liquidity provision mechanism that maintains pool balance
3. **Floor Price Guarantee**: Mathematical assurance that token value cannot fall below a calculable minimum

### 1.2 Data Structures

```rust
// TOL Configuration
pub struct TolConfig {
    pub token_asset: AssetId,
    pub foreign_asset: AssetId,
    pub native_asset: AssetId,
    pub total_tol_allocation: Balance,
    pub current_tol_supply: Balance,
}

// Bucket Distribution
pub struct BucketAllocation {
    pub target_allocation_ppm: u32,    // Parts per million
    pub native_reserves: Balance,
    pub foreign_reserves: Balance,
    pub lp_tokens: Balance,
}

// Zap Buffer for Accumulation
pub struct ZapBuffer {
    pub pending_native: Balance,
    pub pending_foreign: Balance,
}
```

## 2. Four-Bucket Liquidty Distribution

### 2.1 Bucket Architecture

The TOL system distributes liquidity across four distinct buckets, each with specific allocation targets:

- **Bucket A**: 50% allocation (500,000 PPM) - Primary liquidity
- **Bucket B**: 16.67% allocation (166,667 PPM) - Secondary support
- **Bucket C**: 16.67% allocation (166,667 PPM) - Tertiary support
- **Bucket D**: 16.66% allocation (166,666 PPM) - Reserve buffer

### 2.2 Allocation Mechanics

When tokens are minted, the system automatically distributes them according to these ratios:

```rust
// Simplified allocation calculation
let bucket_a_amount = total_tokens * 500_000 / 1_000_000;
let bucket_b_amount = total_tokens * 166_667 / 1_000_000;
let bucket_c_amount = total_tokens * 166_667 / 1_000_000;
let bucket_d_amount = total_tokens * 166_666 / 1_000_000;
```

Each bucket independently manages its native and foreign reserves, creating a multi-layered liquidity structure.

### 2.3 Storage Optimization

Rather than using complex enum-based storage, the implementation uses separate storage maps for each bucket:

```rust
pub type BucketA<T: Config> = StorageMap<_, Blake2_128Concat, AssetId, BucketAllocation>;
pub type BucketB<T: Config> = StorageMap<_, Blake2_128Concat, AssetId, BucketAllocation>;
pub type BucketC<T: Config> = StorageMap<_, Blake2_128Concat, AssetId, BucketAllocation>;
pub type BucketD<T: Config> = StorageMap<_, Blake2_128Concat, AssetId, BucketAllocation>;
```

This approach provides type safety and clear separation between bucket states.

## 3. Zap Algorithm Implementation

### 3.1 Core Principle

The Zap algorithm automatically balances and adds liquidity to Uniswap V2-style pools using accumulated tokens. It operates through a buffer mechanism that accumulates tokens until a threshold is reached, then executes a comprehensive Zap operation.

### 3.2 Buffer-Based Accumulation

```rust
pub fn receive_mint_allocation(
    token_asset: AssetId,
    total_native: Balance,
    total_foreign: Balance,
) -> DispatchResult {
    // Add to buffer - zap will be triggered in on_initialize hook
    Self::add_to_zap_buffer(token_asset, total_native, total_foreign)?;

    // Emit event for tracking
    Self::deposit_event(Event::ZapBufferUpdated {
        token_asset,
        pending_native: total_native,
        pending_foreign: total_foreign,
    });

    Ok(())
}
```

### 3.3 Threshold-Based Execution

At the beginning of each block, the `on_initialize` hook checks if the buffer exceeds the minimum threshold and executes specialized Zap functions based on pool state:

```rust
fn on_initialize(_block_number: BlockNumberFor<T>) -> Weight {
    for (token_asset, buffer) in ZapBuffers::<T>::iter() {
        // Check if foreign buffer exceeds threshold
        if buffer.pending_foreign >= T::MinSwapForeign::get() {
            // Get pool state to determine initialization needs
            let tol_config = TolConfigurations::<T>::get(token_asset);
            if let Some(config) = tol_config {
                let pool_id = T::AssetConversion::get_pool_id(config.native_asset, config.foreign_asset);
                if let Some(id) = pool_id {
                    let reserves = T::AssetConversion::get_pool_reserves(id);

                    match reserves {
                        Some((native_reserve, foreign_reserve)) => {
                            // Pool exists, determine initialization needs
                            if native_reserve == 0 || foreign_reserve == 0 {
                                // Pool exists but empty, use initialization
                                Self::pool_initialization_zap(token_asset)?;
                            } else {
                                // Normal zap with buffer management
                                Self::execute_zap_with_buffer(
                                    token_asset,
                                    buffer.pending_native,
                                    buffer.pending_foreign,
                                )?;
                            }
                        }
                        None => {
                            // Pool doesn't exist, initialize with buffer
                            Self::pool_initialization_zap(token_asset)?;
                        }
                    }
                } else {
                    // Pool ID not found, initialize
                    Self::pool_initialization_zap(token_asset)?;
                }
            }
        }
    }

    // Return minimal weight
    T::WeightInfo::create_tol()
}
```

### 3.4 Zap Execution Logic

The core Zap operation follows these steps:

1. **Pool Analysis**: Check if the XYK pool is initialized
2. **Spot Price Calculation**: Calculate current pool price: `spot_price = foreign_reserves * PRECISION / native_reserves`
3. **Imbalance Detection**: Determine if more native or foreign tokens are needed for balance
4. **Swap Execution**: If imbalance exists, swap excess tokens
5. **Liquidity Addition**: Add balanced tokens to the pool

```rust
fn execute_zap(
    token_asset: AssetId,
    total_native: Balance,
    total_foreign: Balance,
) -> Result<ZapResult, DispatchError> {
    let tol_config = TolConfigurations::<T>::get(token_asset)
        .ok_or(Error::<T>::NoTolExists)?;

    // Get XYK pool reserves
    let pool_id = T::AssetConversion::get_pool_id(
        tol_config.native_asset,
        tol_config.foreign_asset
    ).ok_or(Error::<T>::InsufficientLiquidity)?;

    let (native_reserve, foreign_reserve) = T::AssetConversion::get_pool_reserves(pool_id)
        .ok_or(Error::<T>::InsufficientLiquidity)?;

    // Handle uninitialized pool
    if native_reserve == 0 || foreign_reserve == 0 {
        return Self::try_initialize_pool(token_asset, total_native, total_foreign);
    }

    // Calculate imbalance
    let spot_price = foreign_reserve.saturating_mul(T::Precision::get()) / native_reserve.max(1);
    let foreign_needed_for_native = total_native.saturating_mul(spot_price) / T::Precision::get();

    // Execute swap if excess native
    let (mut native_to_pool, mut foreign_to_pool) = (total_native, total_foreign);

    if total_foreign < foreign_needed_for_native && total_native >= T::MinSwapForeign::get() {
        // Calculate minimum acceptable foreign out with slippage protection
        let one_hundred = Permill::from_percent(100);
        let deviation = T::MaxPriceDeviation::get();
        let max_slippage = if one_hundred > deviation {
            Permill::from_parts(one_hundred.deconstruct() - deviation.deconstruct())
        } else {
            Permill::zero()
        };

        let min_foreign_out = foreign_needed_for_native.saturating_sub(total_foreign);
        let min_acceptable = min_foreign_out.saturating_mul(max_slippage.deconstruct() as u128) / 1_000_000;

        if let Some(foreign_out) = T::AssetConversion::quote_price_exact_tokens_for_tokens(
            tol_config.native_asset,
            tol_config.foreign_asset,
            total_native,
            true, // include_fee
        ) {
            if foreign_out >= min_acceptable {
                // Perform swap
                let foreign_out = T::AssetConversion::swap_exact_tokens_for_tokens(
                    T::TreasuryAccount::get(),
                    vec![tol_config.native_asset, tol_config.foreign_asset],
                    total_native,
                    min_acceptable,
                    T::TreasuryAccount::get(),
                    false, // keep_alive
                )?;

                native_to_pool = 0;
                foreign_to_pool = total_foreign.saturating_add(foreign_out);
            }
        }
    }

    // Add liquidity to pool
    let (lp_minted, native_used, foreign_used) =
        Self::add_liquidity_to_pool(token_asset, native_to_pool, foreign_to_pool)?;

    // Calculate leftovers
    let leftover_native = native_to_pool.saturating_sub(native_used);
    let leftover_foreign = foreign_to_pool.saturating_sub(foreign_used);

    Ok(ZapResult {
        native_used,
        foreign_used,
        lp_minted,
        leftover_native,
        leftover_foreign,
    })
}
```

### 3.5 Specialized Zap Functions

The TOL system implements two specialized functions for different Zap scenarios:

#### Pool Initialization Zap

Used when the XYK pool doesn't exist or is empty:

```rust
fn pool_initialization_zap(token_asset: AssetId) -> Result<ZapResult, DispatchError> {
    let tol_config = TolConfigurations::<T>::get(token_asset)
        .ok_or(Error::<T>::NoTolExists)?;
    let buffer = ZapBuffers::<T>::get(token_asset).unwrap_or_default();

    // Minimum liquidity threshold for initial pool
    const MIN_INITIAL_LIQUIDITY: Balance = 1_000_000_000_000_000; // 1e18

    // Allocate minimum available tokens to initialize pool
    let native_for_pool = buffer.pending_native.min(MIN_INITIAL_LIQUIDITY);
    let foreign_for_pool = buffer.pending_foreign.min(MIN_INITIAL_LIQUIDITY);

    // Execute zap to initialize pool
    let zap_result = Self::execute_zap(token_asset, native_for_pool, foreign_for_pool)?;

    // Update buffer with remaining tokens
    let remaining_native = buffer.pending_native.saturating_sub(native_for_pool);
    let remaining_foreign = buffer.pending_foreign.saturating_sub(foreign_for_pool);

    if remaining_native > 0 || remaining_foreign > 0 {
        let updated_buffer = ZapBuffer {
            pending_native: remaining_native,
            pending_foreign: remaining_foreign,
        };
        ZapBuffers::<T>::insert(token_asset, updated_buffer);
    } else {
        // Clear buffer if fully consumed
        ZapBuffers::<T>::remove(token_asset);
    }

    Ok(zap_result)
}
```

#### Buffer-Based Zap

Used when the pool exists and has liquidity:

```rust
fn execute_zap_with_buffer(
    token_asset: AssetId,
    minted_native: Balance,
    minted_foreign: Balance,
) -> Result<ZapResult, DispatchError> {
    let tol_config = TolConfigurations::<T>::get(token_asset)
        .ok_or(Error::<T>::NoTolExists)?;

    // Get current buffer state
    let mut buffer = ZapBuffers::<T>::get(token_asset).unwrap_or_default();

    // Add newly minted tokens to buffer
    buffer.pending_native = buffer.pending_native.saturating_add(minted_native);
    buffer.pending_foreign = buffer.pending_foreign.saturating_add(minted_foreign);

    // Get pool reserves for spot price calculation
    let pool_id = T::AssetConversion::get_pool_id(
        tol_config.native_asset,
        tol_config.foreign_asset
    ).ok_or(Error::<T>::InsufficientLiquidity)?;

    let (native_reserve, foreign_reserve) = T::AssetConversion::get_pool_reserves(pool_id)
        .ok_or(Error::<T>::InsufficientLiquidity)?;

    // Calculate balanced amounts from buffer
    let spot_price = foreign_reserve.saturating_mul(T::Precision::get()) / native_reserve.max(1);
    let native_needed_for_foreign = buffer.pending_foreign.saturating_mul(T::Precision::get()) / spot_price.max(1);
    let min_swap = T::MinSwapForeign::get();

    // Execute zap with minimal leftover
    let (native_to_pool, foreign_to_pool) = if buffer.pending_native > native_needed_for_foreign {
        // Excess native, swap part of it
        if buffer.pending_native >= min_swap {
            let swap_amount = buffer.pending_native.saturating_sub(native_needed_for_foreign);
            let native_to_pool = buffer.pending_native.saturating_sub(swap_amount);
            (native_to_pool, buffer.pending_foreign)
        } else {
            (buffer.pending_native, buffer.pending_foreign)
        }
    } else {
        // Excess foreign, use all available
        (buffer.pending_native, buffer.pending_foreign)
    };

    // Execute zap operation
    let zap_result = Self::execute_zap(token_asset, native_to_pool, foreign_to_pool)?;

    // Calculate remaining tokens and update buffer
    let leftover_native = buffer.pending_native.saturating_sub(native_to_pool);
    let leftover_foreign = buffer.pending_foreign.saturating_sub(foreign_to_pool);

    if leftover_native > 0 || leftover_foreign > 0 {
        let updated_buffer = ZapBuffer {
            pending_native: leftover_native,
            pending_foreign: leftover_foreign,
        };
        ZapBuffers::<T>::insert(token_asset, updated_buffer);
    } else {
        // Clear buffer if no leftovers
        ZapBuffers::<T>::remove(token_asset);
    }

    Ok(zap_result)
}
```

### 3.7 Mathematical Foundations

After successful Zap execution, LP tokens are distributed across the four buckets:

```rust
fn distribute_lp_tokens(
    token_asset: AssetId,
    lp_minted: Balance,
    native_used: Balance,
    foreign_used: Balance,
) -> DispatchResult {
    let bucket_a = BucketA::<T>::get(token_asset).unwrap_or_default();
    let bucket_b = BucketB::<T>::get(token_asset).unwrap_or_default();
    let bucket_c = BucketC::<T>::get(token_asset).unwrap_or_default();
    let bucket_d = BucketD::<T>::get(token_asset).unwrap_or_default();

    // Calculate LP allocations based on bucket targets
    let bucket_a_lp = lp_minted.saturating_mul(bucket_a.target_allocation_ppm as u128) / 1_000_000;
    let bucket_b_lp = lp_minted.saturating_mul(bucket_b.target_allocation_ppm as u128) / 1_000_000;
    let bucket_c_lp = lp_minted.saturating_mul(bucket_c.target_allocation_ppm as u128) / 1_000_000;
    let bucket_d_lp = lp_minted.saturating_mul(bucket_d.target_allocation_ppm as u128) / 1_000_000;

    // Update bucket reserves and LP tokens
    // Implementation details for updating each bucket...

    // Emit events for transparency
    Self::deposit_event(Event::LiquidityAdded {
        token_asset,
        bucket_id: 0, // Bucket A
        native_amount: native_used / 4,
        foreign_amount: foreign_used / 4,
        lp_tokens_received: bucket_a_lp,
    });

    // Similar events for buckets B, C, D...

    Ok(())
}
```

## 4. Mathematical Foundations

### 3.7.1 Floor Price Calculation

The floor price is guaranteed by the relationship between total reserves:

```
P_floor = R_foreign * PRECISION / (R_native)²
```

Where:

- `R_foreign` = Total foreign reserves across all buckets
- `R_native` = Total native reserves across all buckets
- `PRECISION` = Fixed precision constant (10^12 in current implementation)

### 3.7.2 Bucket Contribution to Floor Price

Each bucket contributes to the overall floor price based on its reserves:

```rust
fn calculate_floor_price(token_asset: AssetId) -> Option<Balance> {
    // Get total TOL reserves
    let (total_native, total_foreign) = Self::get_total_tol_reserves(token_asset)?;

    if total_native == 0 {
        return Some(0);
    }

    // Calculate hyperbolic floor price
    let denominator = total_native
        .saturating_mul(total_native)
        .saturating_div(T::Precision::get());

    let floor_price = total_foreign.saturating_mul(T::Precision::get()) / denominator.max(1);

    Some(floor_price)
}
```

### 3.7.3 Spot Price Calculation

The current spot price in the XYK pool provides the basis for imbalance detection:

```
spot_price = foreign_reserves * PRECISION / native_reserves
```

### 3.7.4 Swap Amount Calculation

When excess native tokens are detected, the system calculates how much foreign is needed:

```
foreign_needed = native_amount * spot_price / PRECISION
```

## 4. Integration with Token Minting

### 5.1 Flow When Tokens Are Minted

1. User calls `mint_tokens` with foreign tokens
2. System calculates user tokens and TOL allocation
3. TOL allocation is sent to the buffer via `add_to_zap_buffer`
4. Buffer accumulates until threshold is met
5. Zap algorithm executes at beginning of next block

### 4.2 Adapter Pattern Implementation

```rust
// Adapter pattern for clean integration
pub struct TolZapAdapter;

impl pallet_token_minting_curve::TolZapInterface<AssetId, Balance> for TolZapAdapter {
    fn add_to_zap_buffer(
        token_asset: AssetId,
        total_native: Balance,
        total_foreign: Balance
    ) {
        let _ = pallet_treasury_owned_liquidity::Pallet::<Runtime>::add_to_zap_buffer(
            token_asset,
            total_native,
            total_foreign,
        );
    }
}
```

## 5. Configuration and Constants

### 5.1 Runtime Configuration

All key parameters are configurable through runtime configuration:

```rust
pub struct TolAssetConversionAdapter;

impl pallet_treasury_owned_liquidity::Config for Runtime {
    type Assets = pallet_assets::Pallet<Runtime>;
    type NativeAsset = TolNativeAsset;
    type TreasuryAccount = TolTreasuryAccount;
    type Precision = TolPrecision;
    type BucketAAllocation = TolBucketAAllocation;
    type BucketBAllocation = TolBucketBAllocation;
    type BucketCAllocation = TolBucketCAllocation;
    type BucketDAllocation = TolBucketDAllocation;
    type AssetConversion = TolAssetConversionAdapter;
    type MinSwapForeign = TolMinSwapForeign;  // Threshold for Zap execution
    type MaxPriceDeviation = TolMaxPriceDeviation;  // Slippage protection
    type WeightInfo = ();
}
```

```rust
/// Precision for mathematical calculations (10^12)
pub const TolPrecision: Balance = 1_000_000_000_000;

/// Minimum foreign amount for swap operations (1e18)
pub const TolMinSwapForeign: Balance = 1_000_000_000_000_000_000;

/// Maximum price deviation for swaps (20%)
pub const TolMaxPriceDeviation: Permill = Permill::from_percent(20);

/// Target allocation percentages
pub const TolBucketAAllocation: Permill = Permill::from_parts(500_000);  // 50%
pub const TolBucketBAllocation: Permill = Permill::from_parts(166_667);  // 16.67%
pub const TolBucketCAllocation: Permill = Permill::from_parts(166_667);  // 16.67%
pub const TolBucketDAllocation: Permill = Permill::from_parts(166_666);  // 16.66%
```

## 6. Event System

### 7.1 Key Events

The TOL system emits comprehensive events for transparency:

```rust
pub enum Event<T: Config> {
    // TOL creation
    TolCreated {
        token_asset: AssetId,
        foreign_asset: AssetId,
        total_allocation: Balance,
    },

    // Liquidity operations
    LiquidityAdded {
        token_asset: AssetId,
        bucket_id: u32,
        native_amount: Balance,
        foreign_amount: Balance,
        lp_tokens_received: Balance,
    },

    // Zap execution
    LiquidityZapped {
        token_asset: AssetId,
        native_used: Balance,
        foreign_used: Balance,
        lp_tokens_minted: Balance,
    },

    // Buffer tracking
    ZapBufferUpdated {
        token_asset: AssetId,
        pending_native: Balance,
        pending_foreign: Balance,
    },
}
```

## 7. Security Considerations

### 7.1 Access Control

- **TOL Creation**: Only treasury account can create TOL configurations
- **Parameter Updates**: Only treasury can update bucket allocations
- **Buffer Management**: Only system processes can execute Zap operations

### 7.2 Economic Attack Resistance

1. **Price Manipulation Protection**: 20% slippage protection during swaps
2. **Threshold-Based Execution**: Prevents gas-wasting micro-transactions
3. **Mathematical Floor Price**: Hyperbolic relationship prevents price collapse
4. **Four-Bucket Distribution**: Spreads risk across multiple liquidity pools

````

## 8. Monitoring and Public Interface

### 8.1 State Queries

```rust
// Get current buffer state
pub fn get_buffer_state(token_asset: AssetId) -> Option<ZapBuffer> {
    ZapBuffers::<T>::get(token_asset)
}

// Check if Zap should trigger
pub fn should_trigger_zap(token_asset: AssetId) -> bool {
    let buffer = ZapBuffers::<T>::get(token_asset).unwrap_or_default();
    buffer.pending_foreign >= T::MinSwapForeign::get()
}

// Calculate current floor price
pub fn calculate_floor_price(token_asset: AssetId) -> Option<Balance> {
    // Implementation as shown in section 4.2
}

// Get total reserves across all buckets
pub fn get_total_tol_reserves(token_asset: AssetId) -> Option<(Balance, Balance)> {
    // Sum all bucket reserves
}
```

## 9. Summary of Economic Properties

The TOL system provides the following guarantees:

1. **Floor Price**: Mathematical guarantee that token value cannot fall below calculable minimum
2. **Perpetual Liquidity**: Continuous rebalancing maintains healthy pool reserves
3. **Capital Efficiency**: Zap algorithm optimizes liquidity provision
4. **Transparency**: All operations emit events and can be queried on-chain
5. **Gas Optimization**: Threshold-based execution reduces transaction costs
6. **Governance Ready**: All parameters configurable through runtime governance

This implementation represents a production-ready economic primitive that transforms treasury assets into mathematically-guaranteed liquidity infrastructure with clear separation of concerns and comprehensive test coverage.
````
