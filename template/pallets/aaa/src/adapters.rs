//! Adapter traits for AAA pallet (§2)
//!
//! Two traits abstract all runtime-specific operations, keeping pallet-aaa
//! fully generic over asset types and independent of any runtime implementation.

use frame::prelude::*;

/// Asset mutations and queries (§2.1).
///
/// Covers Transfer, SplitTransfer, Burn, Mint, and balance queries.
/// `mint` is privileged — pallet rejects Mint tasks for User AAA at creation.
pub trait AssetOps<AccountId, AssetId, Balance> {
  fn transfer(
    from: &AccountId,
    to: &AccountId,
    asset: AssetId,
    amount: Balance,
  ) -> Result<(), DispatchError>;

  fn burn(who: &AccountId, asset: AssetId, amount: Balance) -> Result<(), DispatchError>;

  fn mint(to: &AccountId, asset: AssetId, amount: Balance) -> Result<(), DispatchError>;

  fn balance(who: &AccountId, asset: AssetId) -> Balance;
}

/// DEX operations — swap and liquidity (§2.2).
///
/// Optional: required only when SwapExactIn/Out, AddLiquidity, or
/// RemoveLiquidity tasks are present in a pipeline.
pub trait DexOps<AccountId, AssetId, Balance> {
  fn swap_exact_in(
    who: &AccountId,
    asset_in: AssetId,
    asset_out: AssetId,
    amount_in: Balance,
    min_out: Balance,
  ) -> Result<Balance, DispatchError>;

  fn swap_exact_out(
    who: &AccountId,
    asset_in: AssetId,
    asset_out: AssetId,
    amount_out: Balance,
    max_in: Balance,
  ) -> Result<Balance, DispatchError>;

  fn get_quote(asset_in: AssetId, asset_out: AssetId, amount_in: Balance) -> Option<Balance>;

  fn add_liquidity(
    who: &AccountId,
    asset_a: AssetId,
    asset_b: AssetId,
    amount_a: Balance,
    amount_b: Balance,
  ) -> Result<(Balance, Balance, Balance), DispatchError>;

  fn remove_liquidity(
    who: &AccountId,
    lp_asset: AssetId,
    lp_amount: Balance,
  ) -> Result<(Balance, Balance), DispatchError>;

  fn get_pool_reserves(asset_a: AssetId, asset_b: AssetId) -> Option<(Balance, Balance)>;
}

/// No-op `AssetOps` for use in configurations where asset ops are not needed.
impl<AccountId, AssetId, Balance: Default> AssetOps<AccountId, AssetId, Balance> for () {
  fn transfer(_: &AccountId, _: &AccountId, _: AssetId, _: Balance) -> Result<(), DispatchError> {
    Ok(())
  }

  fn burn(_: &AccountId, _: AssetId, _: Balance) -> Result<(), DispatchError> {
    Ok(())
  }

  fn mint(_: &AccountId, _: AssetId, _: Balance) -> Result<(), DispatchError> {
    Ok(())
  }

  fn balance(_: &AccountId, _: AssetId) -> Balance {
    Balance::default()
  }
}

/// No-op `DexOps` for configurations where DEX is not used.
impl<AccountId, AssetId, Balance: Default> DexOps<AccountId, AssetId, Balance> for () {
  fn swap_exact_in(
    _: &AccountId,
    _: AssetId,
    _: AssetId,
    _: Balance,
    _: Balance,
  ) -> Result<Balance, DispatchError> {
    Err(DispatchError::Other("DexOps not configured"))
  }

  fn swap_exact_out(
    _: &AccountId,
    _: AssetId,
    _: AssetId,
    _: Balance,
    _: Balance,
  ) -> Result<Balance, DispatchError> {
    Err(DispatchError::Other("DexOps not configured"))
  }

  fn get_quote(_: AssetId, _: AssetId, _: Balance) -> Option<Balance> {
    None
  }

  fn add_liquidity(
    _: &AccountId,
    _: AssetId,
    _: AssetId,
    _: Balance,
    _: Balance,
  ) -> Result<(Balance, Balance, Balance), DispatchError> {
    Err(DispatchError::Other("DexOps not configured"))
  }

  fn remove_liquidity(
    _: &AccountId,
    _: AssetId,
    _: Balance,
  ) -> Result<(Balance, Balance), DispatchError> {
    Err(DispatchError::Other("DexOps not configured"))
  }

  fn get_pool_reserves(_: AssetId, _: AssetId) -> Option<(Balance, Balance)> {
    None
  }
}
