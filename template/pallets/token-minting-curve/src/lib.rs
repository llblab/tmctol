//! Token Minting Curve Pallet
//!
//! Implements linear price ceiling with unidirectional minting for TMCTOL framework.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

pub mod types;
pub use types::AssetKind;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
pub mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

use frame::prelude::*;

use frame::deps::{
  frame_support::traits::{
    fungible::{Inspect as NativeInspect, Mutate as NativeMutate},
    fungibles::{Inspect, Mutate},
    tokens::Preservation,
  },
  sp_core::U256,
  sp_runtime::{
    Permill,
    traits::{AccountIdConversion, AtLeast32BitUnsigned, UniqueSaturatedInto, Zero},
  },
};

/// Hook trait for deterministic TMCTOL domain glue on curve activation
pub trait DomainGlueHook {
  fn on_curve_created(
    token_asset: AssetKind,
    foreign_asset: AssetKind,
  ) -> Result<(), frame::deps::sp_runtime::DispatchError>;
}

impl DomainGlueHook for () {
  fn on_curve_created(
    _token_asset: AssetKind,
    _foreign_asset: AssetKind,
  ) -> Result<(), frame::deps::sp_runtime::DispatchError> {
    Ok(())
  }
}

/// Interface for TOL Zap algorithm integration
pub trait TolZapInterface<Balance> {
  /// Execute Zap algorithm after minting for TOL distribution
  fn execute_zap_after_minting(
    token_asset: AssetKind,
    total_tol: Balance,
    foreign_amount: Balance,
  ) -> Result<(Balance, Balance), frame::deps::sp_runtime::DispatchError>;

  /// Add tokens to zap buffer for future processing
  fn add_to_zap_buffer(
    token_asset: AssetKind,
    total_native: Balance,
    total_foreign: Balance,
  ) -> Result<(), frame::deps::sp_runtime::DispatchError>;
}

#[frame::pallet]
pub mod pallet {
  use super::WeightInfo;
  use super::*;

  #[pallet::config]
  pub trait Config: frame_system::Config {
    /// Asset management interface for local assets
    type Assets: Inspect<Self::AccountId, AssetId = u32, Balance = Balance>
      + Mutate<Self::AccountId, AssetId = u32, Balance = Balance>;

    /// Currency interface for native asset
    type Currency: NativeMutate<Self::AccountId, Balance = Balance>
      + NativeInspect<Self::AccountId, Balance = Balance>;

    /// Origin that can perform governance operations
    type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

    /// Balance type
    type Balance: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen;

    /// Pallet ID for fee collection
    type PalletId: Get<frame::deps::frame_support::PalletId>;

    /// Treasury account for TOL distribution
    type TreasuryAccount: Get<Self::AccountId>;

    /// Initial price for token minting
    type InitialPrice: Get<Self::Balance>;

    /// Slope parameter for linear price ceiling
    type SlopeParameter: Get<Self::Balance>;

    /// Precision for mathematical calculations
    type Precision: Get<Self::Balance>;

    /// Zap manager account for liquidity provisioning
    #[pallet::constant]
    type ZapManagerAccount: Get<Self::AccountId>;

    /// Distribution ratio for user allocation (1/3)
    type UserAllocationRatio: Get<Permill>;

    /// TOL Zap adapter for token-driven liquidity provisioning
    type TolZapAdapter: TolZapInterface<Self::Balance>;

    /// Runtime glue hook executed on curve creation
    type DomainGlueHook: DomainGlueHook;

    /// Weight information
    type WeightInfo: WeightInfo;
  }

  #[pallet::pallet]
  #[pallet::storage_version(STORAGE_VERSION)]
  pub struct Pallet<T>(_);

  /// The current storage version.
  const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

  /// Balance type
  pub type Balance = u128;

  /// Price type for token minting
  pub type Price = Balance;

  /// Slope type for linear price ceiling
  pub type Slope = Balance;

  /// Curve configuration for token minting
  #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
  pub struct CurveConfig {
    /// Initial price of the token
    pub initial_price: Price,
    /// Slope parameter for linear price ceiling
    pub slope: Slope,
    /// Initial issuance at curve creation
    pub initial_issuance: Balance,
    /// Foreign asset used for minting
    pub foreign_asset: AssetKind,
    /// Native asset ID
    pub native_asset: AssetKind,
  }

  /// Storage for token curves
  #[pallet::storage]
  pub type TokenCurves<T: Config> = StorageMap<_, Blake2_128Concat, AssetKind, CurveConfig>;

  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    /// Curve created for a token
    CurveCreated {
      token_asset: AssetKind,
      initial_price: Price,
      slope: Slope,
      foreign_asset: AssetKind,
    },
    /// Curve updated
    CurveUpdated {
      token_asset: AssetKind,
      new_slope: Slope,
    },
    /// Zap allocation distributed
    ZapAllocationDistributed {
      token_asset: AssetKind,
      user_allocation: Balance,
      zap_allocation: Balance,
      foreign_amount: Balance,
    },
  }

  #[pallet::error]
  pub enum Error<T> {
    /// Curve already exists for this token
    CurveAlreadyExists,
    /// No curve exists for this token
    NoCurveExists,
    /// Insufficient balance for operation
    InsufficientBalance,
    /// Exceeds maximum supply
    ExceedsMaxSupply,
    /// Arithmetic overflow occurred
    ArithmeticOverflow,
    /// Invalid parameters provided
    InvalidParameters,
    /// Foreign asset does not match the curve collateral configuration
    InvalidForeignAsset,
    /// Zero amount not allowed
    ZeroAmount,
  }

  #[pallet::call]
  impl<T: Config> Pallet<T> {
    /// Create a new bonding curve for a token
    #[pallet::call_index(0)]
    #[pallet::weight(T::WeightInfo::create_curve())]
    pub fn create_curve(
      origin: OriginFor<T>,
      token_asset: AssetKind,
      foreign_asset: AssetKind,
      initial_price: Price,
      slope: Slope,
    ) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;

      // Ensure curve doesn't already exist
      ensure!(
        !TokenCurves::<T>::contains_key(token_asset),
        Error::<T>::CurveAlreadyExists
      );

      // Validate parameters
      ensure!(
        initial_price > Balance::from(0u32) || slope > Balance::from(0u32),
        Error::<T>::InvalidParameters
      );

      T::DomainGlueHook::on_curve_created(token_asset, foreign_asset)?;

      let curve = CurveConfig {
        initial_price,
        slope,
        initial_issuance: T::Currency::total_issuance().unique_saturated_into(),
        foreign_asset,
        native_asset: AssetKind::Native,
      };

      TokenCurves::<T>::insert(token_asset, curve);

      Self::deposit_event(Event::CurveCreated {
        token_asset,
        initial_price,
        slope,
        foreign_asset,
      });

      Ok(())
    }

    /// Update curve parameters (governance only)
    #[pallet::call_index(1)]
    #[pallet::weight(T::WeightInfo::update_curve())]
    pub fn update_curve(
      origin: OriginFor<T>,
      token_asset: AssetKind,
      new_slope: Slope,
    ) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;

      let mut curve = TokenCurves::<T>::get(token_asset).ok_or(Error::<T>::NoCurveExists)?;
      ensure!(
        new_slope > Balance::from(0u32),
        Error::<T>::InvalidParameters
      );

      curve.slope = new_slope;
      TokenCurves::<T>::insert(token_asset, curve);

      Self::deposit_event(Event::CurveUpdated {
        token_asset,
        new_slope,
      });

      Ok(())
    }
  }

  impl<T: Config> Pallet<T> {
    /// Calculate current spot price
    fn calculate_spot_price(curve: &CurveConfig) -> Price {
      let total_issuance: u128 = T::Currency::total_issuance().unique_saturated_into();
      let effective_supply = total_issuance.saturating_sub(curve.initial_issuance);
      let slope_contribution = curve.slope.saturating_mul(effective_supply);
      let precision: u128 = T::Precision::get().unique_saturated_into();
      let normalized = slope_contribution / precision;
      curve.initial_price.saturating_add(normalized)
    }

    /// Check if a bonding curve exists for the given token asset
    pub fn has_curve(asset_id: AssetKind) -> bool {
      TokenCurves::<T>::contains_key(asset_id)
    }

    /// Get curve configuration
    pub fn get_curve(asset_id: AssetKind) -> Option<CurveConfig> {
      TokenCurves::<T>::get(asset_id)
    }

    /// Get the account ID of the TMC pallet
    pub fn account_id() -> T::AccountId {
      T::PalletId::get().into_account_truncating()
    }

    /// Calculate how much Native tokens the user receives for the foreign payment
    pub fn calculate_user_receives(
      token_asset: AssetKind,
      foreign_amount: Balance,
    ) -> Result<Balance, DispatchError> {
      let curve = TokenCurves::<T>::get(token_asset).ok_or(Error::<T>::NoCurveExists)?;

      let initial_price = curve.initial_price;
      let slope = curve.slope;
      let precision = T::Precision::get();

      // Handle zero slope (constant price)
      if slope.is_zero() {
        if initial_price.is_zero() {
          return Ok(Zero::zero());
        }
        // Linear projection: Cost = Price * Amount -> Amount = Cost / Price
        // Price is scaled by Precision: Cost = (Price_stored / Precision) * Amount
        // Amount = Cost * Precision / Price_stored
        let amount_val: u128 = foreign_amount.unique_saturated_into();
        let price_val: u128 = initial_price.unique_saturated_into();
        let precision_val: u128 = precision.unique_saturated_into();

        let result = U256::from(amount_val)
          .saturating_mul(U256::from(precision_val))
          .checked_div(U256::from(price_val))
          .unwrap_or(U256::zero());

        if result > U256::from(u128::MAX) {
          return Err(Error::<T>::ArithmeticOverflow.into());
        }

        return Ok(result.as_u128().unique_saturated_into());
      }

      // Use quadratic formula to solve for Delta S (amount to mint)
      // derived from Cost = Integral(P(s) ds)
      // Delta S = (sqrt((K*P)^2 + 2*m*K*Cost) - K*P) / m

      let p_current = Self::calculate_spot_price(&curve);

      // Convert to u128 explicitly to avoid U256::from ambiguity
      let k_val: u128 = precision.unique_saturated_into();
      let m_val: u128 = slope.unique_saturated_into();
      let p_val: u128 = p_current.unique_saturated_into();
      let cost_val: u128 = foreign_amount.unique_saturated_into();

      let k_u256 = U256::from(k_val);
      let m_u256 = U256::from(m_val);
      let p_u256 = U256::from(p_val);
      let cost_u256 = U256::from(cost_val);

      // K * P
      let kp = k_u256.saturating_mul(p_u256);

      // (K * P)^2
      let kp_sq = kp.saturating_mul(kp);

      // 2 * m * K^2 * Cost (scaled for precision)
      let two_m_k_cost = U256::from(2)
        .saturating_mul(m_u256)
        .saturating_mul(k_u256)
        .saturating_mul(k_u256)
        .saturating_mul(cost_u256);

      // Inside sqrt
      let inside_sqrt = kp_sq.saturating_add(two_m_k_cost);

      // Sqrt
      let sqrt_res = inside_sqrt.integer_sqrt();

      // Numerator: sqrt - KP
      if sqrt_res < kp {
        return Ok(Zero::zero());
      }
      let numerator = sqrt_res.saturating_sub(kp);

      // Result: Numerator / m
      let result_u256 = numerator
        .checked_div(m_u256)
        .ok_or(Error::<T>::ArithmeticOverflow)?;

      // Convert back to Balance (u128)
      if result_u256 > U256::from(u128::MAX) {
        return Err(Error::<T>::ArithmeticOverflow.into());
      }

      Ok(result_u256.as_u128().unique_saturated_into())
    }

    /// Execute mint through bonding curve with user/TOL distribution
    pub fn mint_with_distribution(
      who: &T::AccountId,
      token_asset: AssetKind,
      foreign_asset: AssetKind,
      foreign_amount: Balance,
    ) -> Result<Balance, DispatchError> {
      ensure!(foreign_amount > Balance::from(0u32), Error::<T>::ZeroAmount);

      let curve = TokenCurves::<T>::get(token_asset).ok_or(Error::<T>::NoCurveExists)?;
      ensure!(
        curve.foreign_asset == foreign_asset,
        Error::<T>::InvalidForeignAsset
      );

      // Calculate tokens to mint using integral calculus
      let native_token_amount = Self::calculate_user_receives(token_asset, foreign_amount)?;

      // Transfer foreign tokens from user to zap manager for liquidity provisioning
      match foreign_asset {
        AssetKind::Native => {
          T::Currency::transfer(
            who,
            &T::ZapManagerAccount::get(),
            foreign_amount,
            Preservation::Expendable,
          )?;
        }
        AssetKind::Local(id) | AssetKind::Foreign(id) => {
          T::Assets::transfer(
            id,
            who,
            &T::ZapManagerAccount::get(),
            foreign_amount,
            Preservation::Expendable,
          )?;
        }
      }

      // Calculate allocations: 1/3 native tokens to user, remainder to zap manager
      let user_allocation = T::UserAllocationRatio::get().mul_floor(native_token_amount);
      let zap_allocation = native_token_amount.saturating_sub(user_allocation);

      // Mint native tokens to user
      T::Currency::mint_into(who, user_allocation)?;

      // Mint native tokens to zap manager account - zap-manager will handle liquidity provisioning
      T::Currency::mint_into(&T::ZapManagerAccount::get(), zap_allocation)?;

      // Emit token-driven coordination event
      Self::deposit_event(Event::ZapAllocationDistributed {
        token_asset,
        user_allocation,
        zap_allocation,
        foreign_amount,
      });

      Ok(native_token_amount)
    }
  }

  #[pallet::genesis_config]
  #[derive(frame::prelude::DefaultNoBound)]
  pub struct GenesisConfig<T: Config> {
    #[serde(skip)]
    pub _marker: core::marker::PhantomData<T>,
  }

  #[pallet::genesis_build]
  impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
    fn build(&self) {
      frame_system::Pallet::<T>::inc_providers(&Pallet::<T>::account_id());
    }
  }
}
