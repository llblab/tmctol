#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

// Re-export pallet items for runtime construction
pub use pallet::*;

pub mod types;
pub use types::AssetKind;

#[cfg(test)]
pub mod tests;

#[cfg(test)]
pub mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

use polkadot_sdk::frame_support::pallet_prelude::*;
use polkadot_sdk::frame_support::traits::fungible::{
  Inspect as NativeInspect, Mutate as NativeMutate,
};
use polkadot_sdk::frame_support::traits::fungibles::{Inspect, Mutate};
use polkadot_sdk::frame_support::traits::tokens::{Fortitude, Precision, Preservation};

use polkadot_sdk::frame_system::pallet_prelude::*;
use polkadot_sdk::sp_core::U256;
use polkadot_sdk::sp_runtime::traits::{
  AccountIdConversion, AtLeast32BitUnsigned, UniqueSaturatedInto, Zero,
};
use polkadot_sdk::sp_runtime::Permill;

/// Interface for TOL Zap algorithm integration
pub trait TolZapInterface<Balance> {
  /// Execute Zap algorithm after minting for TOL distribution
  fn execute_zap_after_minting(
    token_asset: AssetKind,
    total_tol: Balance,
    foreign_amount: Balance,
  ) -> Result<(Balance, Balance), polkadot_sdk::sp_runtime::DispatchError>;

  /// Add tokens to zap buffer for future processing
  fn add_to_zap_buffer(token_asset: AssetKind, total_native: Balance, total_foreign: Balance);
}

#[polkadot_sdk::frame_support::pallet(dev_mode)]
pub mod pallet {
  use super::WeightInfo;
  use super::*;

  #[pallet::config]
  pub trait Config: polkadot_sdk::frame_system::Config {
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
    type PalletId: Get<polkadot_sdk::frame_support::PalletId>;

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

    /// Distribution ratio for user allocation (33.3%)
    type UserAllocationRatio: Get<Permill>;

    /// Distribution ratio for zap manager allocation (66.6%)
    type ZapAllocationRatio: Get<Permill>;
    /// Burn percentage for bidirectional compression (20%)
    type BurnPercentage: Get<Permill>;

    /// TOL Zap adapter for token-driven liquidity provisioning
    type TolZapAdapter: TolZapInterface<Self::Balance>;

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
    /// Current supply of the token
    pub current_supply: Balance,
    /// Total minted tokens
    pub total_minted: Balance,
    /// Foreign asset used for minting
    pub foreign_asset: AssetKind,
    /// Native asset ID
    pub native_asset: AssetKind,
  }

  /// Storage for token curves
  #[pallet::storage]
  pub type TokenCurves<T: Config> = StorageMap<_, Blake2_128Concat, AssetKind, CurveConfig>;

  /// Whether minting is currently paused
  #[pallet::storage]
  #[pallet::getter(fn minting_paused)]
  pub type MintingPaused<T: Config> = StorageValue<_, bool, ValueQuery>;

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
    /// Tokens minted through bonding curve
    TokensMinted {
      who: T::AccountId,
      token_asset: AssetKind,
      foreign_amount: Balance,
      token_amount: Balance,
      spot_price: Price,
    },
    /// Curve updated
    CurveUpdated {
      token_asset: AssetKind,
      new_slope: Slope,
    },
    /// Minting has been paused
    MintingPaused,
    /// Minting has been unpaused
    MintingUnpaused,
    /// Tokens burned through bonding curve
    TokensBurned {
      token_asset: AssetKind,
      amount: Balance,
      new_supply: Balance,
      new_ceiling: Price,
    },
    /// TOL distributed to buckets
    TolDistributed {
      token_asset: AssetKind,
      bucket_a: Balance,
      bucket_b: Balance,
      bucket_c: Balance,
      bucket_d: Balance,
      total_tol: Balance,
    },
    /// Zap allocation distributed
    ZapAllocationDistributed {
      token_asset: AssetKind,
      user_allocation: Balance,
      zap_allocation: Balance,
      foreign_amount: Balance,
    },
    /// Zap operation failed but tokens were transferred
    ZapError {
      token_asset: AssetKind,
      native_amount: Balance,
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
    /// Zero amount not allowed
    ZeroAmount,
    /// Insufficient tokens to burn
    InsufficientTokensToBurn,
    /// Minting is currently paused
    MintingIsPaused,
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
      let _who = ensure_signed(origin)?;

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

      let curve = CurveConfig {
        initial_price,
        slope,
        current_supply: Balance::from(0u32),
        total_minted: Balance::from(0u32),
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

    /// Mint tokens through bonding curve
    #[pallet::call_index(1)]
    #[pallet::weight(T::WeightInfo::mint_tokens())]
    pub fn mint_tokens(
      origin: OriginFor<T>,
      token_asset: AssetKind,
      foreign_amount: Balance,
    ) -> DispatchResult {
      let who = ensure_signed(origin)?;

      // Check if minting is paused
      ensure!(!MintingPaused::<T>::get(), Error::<T>::MintingIsPaused);

      ensure!(foreign_amount > Balance::from(0u32), Error::<T>::ZeroAmount);

      // Get curve configuration
      let mut curve = TokenCurves::<T>::get(token_asset).ok_or(Error::<T>::NoCurveExists)?;

      // Calculate tokens to mint using integral calculus
      let token_amount = Self::calculate_user_receives(token_asset, foreign_amount)?;

      // Transfer foreign tokens from user to treasury
      match curve.foreign_asset {
        AssetKind::Native => {
          T::Currency::transfer(
            &who,
            &T::TreasuryAccount::get(),
            foreign_amount,
            Preservation::Expendable,
          )?;
        }
        AssetKind::Local(id) | AssetKind::Foreign(id) => {
          T::Assets::transfer(
            id,
            &who,
            &T::TreasuryAccount::get(),
            foreign_amount,
            Preservation::Expendable,
          )?;
        }
      }

      // Calculate allocations: 33.3% to user, 66.6% to zap manager
      let user_allocation = Self::calculate_user_allocation(token_amount);
      let zap_allocation = Self::calculate_zap_allocation(token_amount);

      // Mint tokens
      match token_asset {
        AssetKind::Native => {
          // Mint native tokens
          T::Currency::mint_into(&who, user_allocation)?;
          T::Currency::mint_into(&T::ZapManagerAccount::get(), zap_allocation)?;
        }
        AssetKind::Local(id) | AssetKind::Foreign(id) => {
          // Mint local/foreign-mapped tokens
          T::Assets::mint_into(id, &who, user_allocation)?;
          T::Assets::mint_into(id, &T::ZapManagerAccount::get(), zap_allocation)?;
        }
      }

      // Token-driven coordination: direct token transfer to zap manager account
      // This implements the token-driven coordination pattern where 66.6% of minted tokens
      // are automatically sent to the zap manager account for liquidity provisioning
      Self::deposit_event(Event::ZapAllocationDistributed {
        token_asset,
        user_allocation,
        zap_allocation,
        foreign_amount,
      });

      // Update curve state
      curve.current_supply = curve.current_supply.saturating_add(token_amount);
      curve.total_minted = curve.total_minted.saturating_add(token_amount);
      TokenCurves::<T>::insert(token_asset, curve.clone());

      // Calculate spot price for event
      let spot_price = Self::calculate_spot_price(&curve);

      Self::deposit_event(Event::TokensMinted {
        who,
        token_asset,
        foreign_amount,
        token_amount,
        spot_price,
      });

      Ok(())
    }

    /// Update curve parameters (governance only)
    #[pallet::call_index(2)]
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

    /// Burn tokens for supply compression
    #[pallet::call_index(3)]
    #[pallet::weight(T::WeightInfo::burn_tokens())]
    pub fn burn_tokens(
      origin: OriginFor<T>,
      token_asset: AssetKind,
      amount: Balance,
    ) -> DispatchResult {
      let _who = ensure_signed(origin)?;

      ensure!(amount > Balance::from(0u32), Error::<T>::ZeroAmount);

      let mut curve = TokenCurves::<T>::get(token_asset).ok_or(Error::<T>::NoCurveExists)?;

      // Check treasury balance
      let treasury_balance = match token_asset {
        AssetKind::Native => T::Currency::balance(&T::TreasuryAccount::get()),
        AssetKind::Local(id) | AssetKind::Foreign(id) => {
          T::Assets::balance(id, &T::TreasuryAccount::get())
        }
      };

      ensure!(
        treasury_balance >= amount,
        Error::<T>::InsufficientTokensToBurn
      );

      // Burn tokens from treasury
      match token_asset {
        AssetKind::Native => {
          T::Currency::burn_from(
            &T::TreasuryAccount::get(),
            amount,
            Preservation::Expendable,
            Precision::BestEffort,
            Fortitude::Polite,
          )?;
        }
        AssetKind::Local(id) | AssetKind::Foreign(id) => {
          T::Assets::burn_from(
            id,
            &T::TreasuryAccount::get(),
            amount,
            Preservation::Expendable,
            Precision::BestEffort,
            Fortitude::Polite,
          )?;
        }
      }

      // Update curve state
      curve.current_supply = curve.current_supply.saturating_sub(amount);
      TokenCurves::<T>::insert(token_asset, curve.clone());

      let new_ceiling = Self::calculate_spot_price(&curve);

      Self::deposit_event(Event::TokensBurned {
        token_asset,
        amount,
        new_supply: curve.current_supply,
        new_ceiling,
      });

      Ok(())
    }

    /// Pause minting operations (governance only)
    #[pallet::call_index(4)]
    #[pallet::weight(T::WeightInfo::pause_minting())]
    pub fn pause_minting(origin: OriginFor<T>) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;

      MintingPaused::<T>::put(true);

      Self::deposit_event(Event::MintingPaused);

      Ok(())
    }

    /// Unpause minting operations (governance only)
    #[pallet::call_index(5)]
    #[pallet::weight(T::WeightInfo::unpause_minting())]
    pub fn unpause_minting(origin: OriginFor<T>) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;

      MintingPaused::<T>::put(false);

      Self::deposit_event(Event::MintingUnpaused);

      Ok(())
    }
  }

  impl<T: Config> Pallet<T> {
    /// Internal burn function without Origin check for secure inter-pallet calls
    pub fn do_burn_internal(token_asset: AssetKind, amount: Balance) -> DispatchResult {
      ensure!(amount > Balance::from(0u32), Error::<T>::ZeroAmount);

      let mut curve = TokenCurves::<T>::get(token_asset).ok_or(Error::<T>::NoCurveExists)?;

      // Check treasury balance
      let treasury_balance = match token_asset {
        AssetKind::Native => T::Currency::balance(&T::TreasuryAccount::get()),
        AssetKind::Local(id) | AssetKind::Foreign(id) => {
          T::Assets::balance(id, &T::TreasuryAccount::get())
        }
      };

      ensure!(
        treasury_balance >= amount,
        Error::<T>::InsufficientTokensToBurn
      );

      // Burn tokens from treasury
      match token_asset {
        AssetKind::Native => {
          T::Currency::burn_from(
            &T::TreasuryAccount::get(),
            amount,
            Preservation::Expendable,
            Precision::BestEffort,
            Fortitude::Polite,
          )?;
        }
        AssetKind::Local(id) | AssetKind::Foreign(id) => {
          T::Assets::burn_from(
            id,
            &T::TreasuryAccount::get(),
            amount,
            Preservation::Expendable,
            Precision::BestEffort,
            Fortitude::Polite,
          )?;
        }
      }

      // Update curve state
      curve.current_supply = curve.current_supply.saturating_sub(amount);
      TokenCurves::<T>::insert(token_asset, curve.clone());

      let new_ceiling = Self::calculate_spot_price(&curve);

      Self::deposit_event(Event::TokensBurned {
        token_asset,
        amount,
        new_supply: curve.current_supply,
        new_ceiling,
      });

      Ok(())
    }

    /// Calculate current spot price
    fn calculate_spot_price(curve: &CurveConfig) -> Price {
      // P(S) = P₀ + m·S/Precision
      // Use multiplication by reciprocal to avoid division issues
      let slope_contribution = curve.slope.saturating_mul(curve.current_supply);

      // For now, use simplified calculation since we're in dev_mode
      // In production, this would use proper fixed-point arithmetic
      let precision: u128 = T::Precision::get().unique_saturated_into();
      let normalized_contribution = slope_contribution / precision;

      curve.initial_price.saturating_add(normalized_contribution)
    }

    /// Calculate user allocation (33.3% of total)
    fn calculate_user_allocation(total_amount: Balance) -> Balance {
      // User gets 33.3% (100% - 66.7% TOL)
      total_amount.saturating_sub(Self::calculate_tol_total(total_amount))
    }

    /// Calculate total TOL allocation (66.7% of total)
    fn calculate_tol_total(total_amount: Balance) -> Balance {
      // TOL gets 66.7% of total
      Permill::from_rational(667u32, 1000u32).mul_floor(total_amount)
    }

    /// Calculate zap manager allocation (66.6% of total)
    fn calculate_zap_allocation(total_amount: Balance) -> Balance {
      T::ZapAllocationRatio::get().mul_floor(total_amount)
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
      foreign_asset: AssetKind,
      foreign_amount: Balance,
    ) -> Result<Balance, DispatchError> {
      let curve = TokenCurves::<T>::get(foreign_asset).ok_or(Error::<T>::NoCurveExists)?;

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
      foreign_asset: AssetKind,
      foreign_amount: Balance,
    ) -> Result<Balance, DispatchError> {
      ensure!(foreign_amount > Balance::from(0u32), Error::<T>::ZeroAmount);

      let mut curve = TokenCurves::<T>::get(foreign_asset).ok_or(Error::<T>::NoCurveExists)?;

      // Calculate tokens to mint using integral calculus
      let native_token_amount = Self::calculate_user_receives(foreign_asset, foreign_amount)?;

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

      // Calculate allocations: 33.3% native tokens to user, 66.6% native tokens to zap manager
      let user_allocation = Self::calculate_user_allocation(native_token_amount);
      let zap_allocation = Self::calculate_zap_allocation(native_token_amount);

      // Mint native tokens to user
      T::Currency::mint_into(who, user_allocation)?;

      // Mint native tokens to zap manager account - zap-manager will handle liquidity provisioning
      T::Currency::mint_into(&T::ZapManagerAccount::get(), zap_allocation)?;

      // Emit token-driven coordination event
      Self::deposit_event(Event::ZapAllocationDistributed {
        token_asset: AssetKind::Native,
        user_allocation,
        zap_allocation,
        foreign_amount,
      });

      // Update curve state
      curve.current_supply = curve.current_supply.saturating_add(native_token_amount);
      curve.total_minted = curve.total_minted.saturating_add(native_token_amount);
      TokenCurves::<T>::insert(foreign_asset, curve);

      Ok(native_token_amount)
    }
  }
}
