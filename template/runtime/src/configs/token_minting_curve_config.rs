//! Token Minting Curve pallet configuration for the parachain runtime.
//!
//! Configures the linear price ceiling bonding curve system for TMCTOL framework
//! with mathematical price boundaries and treasury-owned liquidity distribution.
//!
//! All account IDs and economic parameters are imported from `primitives::ecosystem`,
//! serving as the single source of truth.

use super::*;

use polkadot_sdk::frame_support::traits::{
  Get, fungible::Mutate as NativeMutate, fungibles::Mutate,
};
use polkadot_sdk::sp_runtime::{DispatchError, traits::AccountIdConversion};
use primitives::{AssetInspector, AssetKind, ecosystem};
use scale_info::prelude::marker::PhantomData;
use sp_runtime::Permill;

parameter_types! {
  /// Initial price for token minting (1:1 ratio for testing)
  pub const TmcInitialPrice: Balance = 1_000_000_000_000;

  /// Pallet ID for the token minting curve (from ecosystem constants)
  pub const TmcPalletId: PalletId = PalletId(*ecosystem::pallet_ids::TOKEN_MINTING_CURVE_PALLET_ID);

  /// Precision for mathematical calculations (ecosystem constant: 10^12)
  pub const TmcPrecision: Balance = ecosystem::params::PRECISION;

  /// Slope parameter for linear price ceiling (ecosystem constant: 0.0001 per token)
  pub const TmcSlopeParameter: Balance = ecosystem::params::TMC_SLOPE_PARAMETER;

  /// Distribution ratio for user allocation (ecosystem constant: 33.3%)
  pub const TmcUserAllocationRatio: Permill = ecosystem::params::TMC_USER_ALLOCATION;

  /// Distribution ratio for zap manager allocation (ecosystem constant: 66.6%)
  pub const TmcZapAllocationRatio: Permill = ecosystem::params::TMC_ZAP_ALLOCATION;
}

/// Treasury account for TOL distribution (derived from PalletId)
pub struct TolTreasuryAccount;
impl Get<AccountId> for TolTreasuryAccount {
  fn get() -> AccountId {
    PalletId(*ecosystem::pallet_ids::TOL_PALLET_ID).into_account_truncating()
  }
}

/// Zap manager account for token-driven liquidity provisioning (derived from PalletId)
pub struct ZapManagerAccount;
impl Get<AccountId> for ZapManagerAccount {
  fn get() -> AccountId {
    PalletId(*ecosystem::pallet_ids::ZAP_MANAGER_PALLET_ID).into_account_truncating()
  }
}

/// Adapter to integrate Token Minting Curve with Zap Manager for token-driven liquidity provisioning
pub struct TolZapAdapter<T: pallet_token_minting_curve::pallet::Config> {
  _phantom: PhantomData<T>,
}

impl<T: pallet_token_minting_curve::pallet::Config> Default for TolZapAdapter<T> {
  fn default() -> Self {
    Self {
      _phantom: Default::default(),
    }
  }
}

impl<T: pallet_token_minting_curve::pallet::Config<Balance = u128>> TolZapAdapter<T> {
  /// Create a new TolZapAdapter instance
  pub fn new() -> Self {
    Self::default()
  }

  /// Transfer tokens to Zap Manager account for liquidity provisioning
  fn transfer_to_zap_manager(
    token_asset: AssetKind,
    native_amount: T::Balance,
    _foreign_amount: T::Balance,
  ) -> Result<(), DispatchError> {
    let zap_manager_account = T::ZapManagerAccount::get();

    match token_asset {
      AssetKind::Native => {
        <T::Currency as NativeMutate<T::AccountId>>::transfer(
          &Self::account_id(),
          &zap_manager_account,
          native_amount,
          polkadot_sdk::frame_support::traits::tokens::Preservation::Expendable,
        )?;
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        T::Assets::transfer(
          id,
          &Self::account_id(),
          &zap_manager_account,
          native_amount,
          polkadot_sdk::frame_support::traits::tokens::Preservation::Expendable,
        )?;
      }
    }

    Ok(())
  }

  /// Get the TMC account ID for token transfers
  fn account_id() -> T::AccountId {
    use polkadot_sdk::sp_runtime::traits::AccountIdConversion;
    T::PalletId::get().into_account_truncating()
  }
}

impl<T: pallet_token_minting_curve::pallet::Config<Balance = u128>>
  pallet_token_minting_curve::TolZapInterface<T::Balance> for TolZapAdapter<T>
{
  fn execute_zap_after_minting(
    token_asset: AssetKind,
    total_tol: T::Balance,
    foreign_amount: T::Balance,
  ) -> Result<(T::Balance, T::Balance), DispatchError> {
    Self::transfer_to_zap_manager(token_asset, total_tol, foreign_amount)?;
    Ok((total_tol, foreign_amount))
  }

  fn add_to_zap_buffer(
    token_asset: AssetKind,
    total_native: T::Balance,
    total_foreign: T::Balance,
  ) -> Result<(), DispatchError> {
    Self::transfer_to_zap_manager(token_asset, total_native, total_foreign)
  }
}

pub struct TmctolDomainGlue;
impl pallet_token_minting_curve::DomainGlueHook for TmctolDomainGlue {
  fn on_curve_created(
    token_asset: AssetKind,
    foreign_asset: AssetKind,
  ) -> Result<(), DispatchError> {
    if matches!(token_asset, AssetKind::Native) || token_asset.is_lp() {
      return Ok(());
    }
    crate::TreasuryOwnedLiquidity::ensure_domain_for_token(token_asset, foreign_asset, 0)?;
    crate::ZapManager::enable_asset(RuntimeOrigin::root(), token_asset)?;
    Ok(())
  }
}

impl pallet_token_minting_curve::pallet::Config for Runtime {
  type AdminOrigin = frame_system::EnsureRoot<AccountId>;
  type Assets = pallet_assets::Pallet<Runtime>;
  type Balance = Balance;
  type Currency = Balances;
  type InitialPrice = TmcInitialPrice;
  type PalletId = TmcPalletId;
  type Precision = TmcPrecision;
  type SlopeParameter = TmcSlopeParameter;
  type TolZapAdapter = TolZapAdapter<Runtime>;
  type DomainGlueHook = TmctolDomainGlue;
  type TreasuryAccount = TolTreasuryAccount;
  type UserAllocationRatio = TmcUserAllocationRatio;
  type WeightInfo = crate::weights::pallet_token_minting_curve::SubstrateWeight<Runtime>;
  type ZapManagerAccount = ZapManagerAccount;
}
