//! Ecosystem Constants for TMCTOL Parachain
//!
//! This module centralizes all system-level constants, including dedicated account IDs for
//! token-driven coordination, pallet IDs, and fundamental economic parameters.
//!
//! These constants are the single source of truth for system architecture and are re-used
//! across all runtime configurations via the primitives crate.

/// Balance type alias for consistency across ecosystem
pub type Balance = u128;

/// Pallet identifiers for deriving pallet-owned accounts.
///
/// These IDs are used by Polkadot SDK's `PalletId::into_account_truncating()`
/// to deterministically generate accounts for pallet-specific operations.
///
/// Using `modl` prefix (Substrate standard) ensures type safety and prevents collision with user accounts.
pub mod pallet_ids {
  /// AAA (Account Abstraction Actor) pallet ID
  pub const AAA_PALLET_ID: &[u8; 8] = b"py/aaa00";

  /// Axial Router pallet ID (multi-token routing engine)
  pub const AXIAL_ROUTER_PALLET_ID: &[u8; 8] = b"axialrt0";

  /// Token Minting Curve pallet ID (unidirectional token ceiling)
  pub const TOKEN_MINTING_CURVE_PALLET_ID: &[u8; 8] = b"tmcurve0";

  /// Burning Manager pallet ID (deflationary fee sink)
  pub const BURNING_MANAGER_PALLET_ID: &[u8; 8] = b"burnmgr0";

  /// Zap Manager pallet ID (liquidity provisioning coordinator)
  pub const ZAP_MANAGER_PALLET_ID: &[u8; 8] = b"zapmgr00";

  /// Treasury-Owned Liquidity pallet ID (volatility floor management)
  pub const TOL_PALLET_ID: &[u8; 8] = b"tolpalle";

  /// Asset conversion pallet (Uniswap V2-like DEX)
  pub const ASSET_CONVERSION_PALLET_ID: &[u8; 8] = b"py/ascon";

  /// Asset Registry pallet ID
  pub const ASSET_REGISTRY_PALLET_ID: &[u8; 8] = b"assetreg";

  /// Bucket A (Anchor) identifier
  pub const BUCKET_A_ID: &[u8; 8] = b"bucket-a";

  /// Bucket B (Building) identifier
  pub const BUCKET_B_ID: &[u8; 8] = b"bucket-b";

  /// Bucket C (Capital) identifier
  pub const BUCKET_C_ID: &[u8; 8] = b"bucket-c";

  /// Bucket D (Dormant) identifier
  pub const BUCKET_D_ID: &[u8; 8] = b"bucket-d";
}

/// Ecosystem parameters defining mathematical constants and thresholds.
///
/// These parameters are global across all pallets and coordinate the
/// economic properties of the system.
pub mod params {
  use super::Balance;
  use sp_arithmetic::Permill;

  /// Precision scalar for all mathematical calculations (10^12).
  ///
  /// All price curves, fee calculations, and economic metrics use this precision
  /// to maintain consistency and prevent rounding errors.
  pub const PRECISION: Balance = 1_000_000_000_000;

  /// EMA oracle half-life in blocks (~10 minutes at 6s/block).
  ///
  /// Controls the responsiveness of the price oracle to market changes.
  /// Higher values create more stable (but lagged) prices; lower values react faster.
  pub const EMA_HALF_LIFE_BLOCKS: u32 = 100;

  /// Maximum allowed price deviation from EMA price (20%).
  ///
  /// Circuit breaker threshold: if market price deviates from the oracle price
  /// by more than this percentage, the router rejects the trade to prevent
  /// manipulation or anomalies.
  pub const MAX_PRICE_DEVIATION: Permill = Permill::from_percent(20);

  /// Maximum hops in multi-asset routing paths (3).
  ///
  /// Limits routing graph complexity and prevents excessive gas consumption
  /// on complex asset paths (e.g., ASSET_A -> Native -> ASSET_B -> ASSET_C).
  pub const MAX_HOPS: u32 = 3;

  /// TMC user allocation ratio (33.3% of minted tokens).
  ///
  /// When tokens are minted via TMC, 33.3% go directly to the user,
  /// and 66.6% go to the Zap Manager for liquidity provisioning.
  pub const TMC_USER_ALLOCATION: Permill = Permill::from_parts(333_333);

  /// TMC zap manager allocation ratio (66.6% of minted tokens).
  pub const TMC_ZAP_ALLOCATION: Permill = Permill::from_parts(666_667);

  /// Axial Router fee (0.5%).
  ///
  /// Protocol captures 0.5% on all swaps routed through the Axial Router.
  /// XYK pool fee is 0.0% — all fee revenue flows through the Router to the Burning Manager.
  pub const AXIAL_ROUTER_FEE: Permill = Permill::from_parts(5_000); // 50 bps = 5000 ppm

  /// TMC curve slope parameter (0.000001 per token).
  ///
  /// Controls the rate at which the price increases as more tokens are minted.
  /// Steeper slopes create more aggressive price escalation.
  pub const TMC_SLOPE_PARAMETER: Balance = PRECISION / 1_000_000; // 0.000001 in PRECISION units

  /// TOL bucket allocation target - Bucket A (50%)
  pub const TOL_BUCKET_A_ALLOCATION: Permill = Permill::from_parts(500_000);

  /// TOL bucket allocation target - Bucket B (16.67%)
  pub const TOL_BUCKET_B_ALLOCATION: Permill = Permill::from_parts(166_667);

  /// TOL bucket allocation target - Bucket C (16.67%)
  pub const TOL_BUCKET_C_ALLOCATION: Permill = Permill::from_parts(166_667);

  /// TOL bucket allocation target - Bucket D (16.66%)
  pub const TOL_BUCKET_D_ALLOCATION: Permill = Permill::from_parts(166_666);

  /// Minimum swap amount for foreign assets (1.0 in base units).
  ///
  /// Prevents spam and dust attacks on the router by enforcing a minimum
  /// transaction size.
  pub const MIN_SWAP_FOREIGN: Balance = PRECISION; // 1.0

  /// Burning Manager dust threshold (0.1 in reference units).
  pub const BURNING_MANAGER_DUST_THRESHOLD: Balance = PRECISION / 10; // 0.1

  /// Burning Manager minimum native token burn amount (10 tokens).
  pub const BURNING_MANAGER_MIN_BURN_NATIVE: Balance = 10 * PRECISION; // 10.0

  /// Burning Manager slippage tolerance (2%).
  pub const BURNING_MANAGER_SLIPPAGE_TOLERANCE: Permill = Permill::from_percent(2);

  /// TOL maximum price deviation (20%).
  pub const TOL_MAX_PRICE_DEVIATION: Permill = Permill::from_percent(20);

  /// TOL minimum swap foreign amount (1.0).
  pub const TOL_MIN_SWAP_FOREIGN: Balance = MIN_SWAP_FOREIGN; // 1.0

  /// Zap Manager minimum swap foreign amount (1.0).
  pub const ZAP_MANAGER_MIN_SWAP_FOREIGN: Balance = MIN_SWAP_FOREIGN; // 1.0

  /// Zap Manager dust threshold for surplus handling (1.0 in base units).
  ///
  /// Amounts below this threshold are considered dust and not processed.
  /// This prevents micro-transactions and optimizes gas usage.
  pub const ZAP_MANAGER_DUST_THRESHOLD: Balance = MIN_SWAP_FOREIGN; // 1.0

  /// Zap Manager retry cooldown in blocks (10 blocks = ~1 minute).
  ///
  /// If a zap operation fails (e.g. due to price deviation), the asset is locked
  /// for this duration to prevent resource waste on repeated failures.
  pub const ZAP_MANAGER_RETRY_COOLDOWN: u32 = 10;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn pallet_ids_are_correct_length() {
    assert_eq!(pallet_ids::AXIAL_ROUTER_PALLET_ID.len(), 8);
    assert_eq!(pallet_ids::TOKEN_MINTING_CURVE_PALLET_ID.len(), 8);
    assert_eq!(pallet_ids::BURNING_MANAGER_PALLET_ID.len(), 8);
    assert_eq!(pallet_ids::ZAP_MANAGER_PALLET_ID.len(), 8);
    assert_eq!(pallet_ids::TOL_PALLET_ID.len(), 8);
    assert_eq!(pallet_ids::ASSET_CONVERSION_PALLET_ID.len(), 8);
    assert_eq!(pallet_ids::ASSET_REGISTRY_PALLET_ID.len(), 8);
  }

  #[test]
  fn parameter_allocations_sum_to_one_million() {
    let user_zap_sum =
      params::TMC_USER_ALLOCATION.deconstruct() + params::TMC_ZAP_ALLOCATION.deconstruct();
    assert_eq!(user_zap_sum, 1_000_000, "TMC allocations must sum to 100%");

    let bucket_sum = params::TOL_BUCKET_A_ALLOCATION.deconstruct()
      + params::TOL_BUCKET_B_ALLOCATION.deconstruct()
      + params::TOL_BUCKET_C_ALLOCATION.deconstruct()
      + params::TOL_BUCKET_D_ALLOCATION.deconstruct();
    assert_eq!(
      bucket_sum, 1_000_000,
      "TOL bucket allocations must sum to 100%"
    );
  }

  #[test]
  fn precision_is_standard() {
    assert_eq!(params::PRECISION, 1_000_000_000_000);
  }
}
