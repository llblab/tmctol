#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items for runtime construction
pub use crate::pallet::*;

pub mod types;
pub use types::{AssetKind, *};

#[cfg(test)]
pub mod tests;

#[cfg(test)]
pub mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

use polkadot_sdk::frame_support::pallet_prelude::*;

use polkadot_sdk::frame_system::pallet_prelude::*;
use polkadot_sdk::sp_runtime;

use polkadot_sdk::sp_runtime::Permill;
use scale_info::prelude::vec::Vec;

/// Route comparison result for optimal path selection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteComparison {
  /// Expected output amount
  pub expected_output: Balance,
  /// Route path (asset kinds)
  pub path: Vec<AssetKind>,
  /// Route mechanism type
  pub mechanism: RouteMechanism,
  /// Price impact percentage
  pub price_impact: Permill,
  /// Total fees (router + AMM)
  pub total_fees: Balance,
}

/// Route mechanism types for advanced routing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteMechanism {
  /// Direct XYK pool swap
  DirectXyk { pool_id: (AssetKind, AssetKind) },
  /// Direct mint via TMC curve
  DirectMint { foreign_asset: AssetKind },
  /// Multi-hop through Native
  MultiHopNative { hops: Vec<AssetKind> },
}

impl RouteComparison {
  /// Create new route comparison
  pub fn new(
    expected_output: Balance,
    path: Vec<AssetKind>,
    mechanism: RouteMechanism,
    price_impact: Permill,
    total_fees: Balance,
  ) -> Self {
    Self {
      expected_output,
      path,
      mechanism,
      price_impact,
      total_fees,
    }
  }

  /// Calculate route efficiency score (higher is better)
  pub fn efficiency_score(&self) -> Balance {
    // Higher output with lower price impact is better
    let base_score = self.expected_output;
    let impact_penalty = self.price_impact.mul_floor(base_score) / 1000u128;
    let fee_penalty = self.total_fees;

    base_score
      .saturating_sub(impact_penalty)
      .saturating_sub(fee_penalty)
  }
}

#[polkadot_sdk::frame_support::pallet(dev_mode)]
pub mod pallet {
  use super::*;
  use crate::types::{AssetConversionApi, AssetKind, FeeRoutingAdapter, PriceOracle, TmcInterface};
  use core::convert::TryInto;
  use polkadot_sdk::sp_runtime::traits::AccountIdConversion;
  use scale_info::prelude::vec;

  /// The pallet module, the place to define dispatchable calls, storage items, events, errors, etc.
  ///
  /// ## Oracle Updates
  /// Oracle price updates happen **pre-swap** using Spot Price calculated from current pool reserves.
  /// The implementation detects asset order in pools to prevent price inversion - it identifies which
  /// reserve corresponds to the 'from' asset and calculates price correctly regardless of internal
  /// pool ordering.
  ///
  /// ## Fee Routing
  /// Fees are routed via **one-hop** transfer directly to the burning manager account for immediate
  /// processing through token-driven coordination mechanisms.
  ///
  /// ## Dust Protection
  /// `MinSwapForeign` acts as a hard floor for input amounts, preventing micro-transactions that
  /// could spam the system while maintaining economic efficiency for legitimate swaps.
  #[pallet::pallet]
  pub struct Pallet<T>(polkadot_sdk::frame_support::pallet_prelude::PhantomData<T>);

  #[pallet::config]
  pub trait Config: polkadot_sdk::frame_system::Config {
    /// Native currency interface for native token transfers
    type Currency: polkadot_sdk::frame_support::traits::Currency<Self::AccountId>;

    /// Asset management interface
    type Assets: polkadot_sdk::frame_support::traits::fungibles::Inspect<
        Self::AccountId,
        AssetId = u32,
        Balance = Balance,
      > + polkadot_sdk::frame_support::traits::fungibles::Mutate<Self::AccountId>;

    /// TMC pallet interface
    type TmcPallet: crate::types::TmcInterface<Self::AccountId, Balance>;

    /// Asset conversion API for XYK pools
    type AssetConversion: crate::types::AssetConversionApi<Self::AccountId, Balance>;

    /// Origin that can perform governance operations
    type AdminOrigin: polkadot_sdk::frame_support::traits::EnsureOrigin<Self::RuntimeOrigin>;

    /// Native asset (AssetKind)
    #[pallet::constant]
    type NativeAsset: Get<AssetKind>;

    /// Default router fee as Permill (default: 0.5%)
    #[pallet::constant]
    type DefaultRouterFee: Get<Permill>;

    /// Precision constant for all calculations (10^12)
    #[pallet::constant]
    type Precision: Get<Balance>;

    /// EMA oracle half-life in blocks (100 blocks ~ 10 minutes at 6s/block)
    #[pallet::constant]
    type EmaHalfLife: Get<u32>;

    /// Maximum price deviation allowed (20%)
    #[pallet::constant]
    type MaxPriceDeviation: Get<Permill>;

    /// Maximum number of hops in multi-hop routing
    #[pallet::constant]
    type MaxHops: Get<u32>;

    /// Fee manager interface
    type FeeAdapter: FeeRoutingAdapter<Self::AccountId, Balance>;

    /// Burning manager account for fee processing
    #[pallet::constant]
    type BurningManagerAccount: Get<Self::AccountId>;

    /// Price oracle for manipulation-resistant pricing
    type PriceOracle: PriceOracle<Balance>;

    /// Minimum foreign amount for swapping (threshold for buffer processing)
    #[pallet::constant]
    type MinSwapForeign: Get<Balance>;

    /// Weight information
    type WeightInfo: WeightInfo;

    /// Helper for benchmarking
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper: crate::types::BenchmarkHelper<
      crate::types::AssetKind,
      Self::AccountId,
      u128,
    >;
  }

  /// Balance type
  pub type Balance = u128;

  /// Router configuration
  #[derive(
    Clone,
    codec::Encode,
    codec::Decode,
    Eq,
    PartialEq,
    sp_runtime::RuntimeDebug,
    scale_info::TypeInfo,
    codec::MaxEncodedLen,
    Default,
  )]
  pub struct RouterConfig {
    /// Native asset
    pub native_asset: AssetKind,
    /// Router fee in basis points
    pub router_fee_bps: u32,
  }

  /// A simple route structure
  #[derive(
    Clone,
    codec::Encode,
    codec::Decode,
    Eq,
    PartialEq,
    sp_runtime::RuntimeDebug,
    scale_info::TypeInfo,
    codec::MaxEncodedLen,
  )]
  pub struct Route {
    /// Source asset
    pub from: AssetKind,
    /// Destination asset
    pub to: AssetKind,
    /// Expected output amount
    pub expected_output: Balance,
  }

  /// Router configuration storage
  #[pallet::storage]
  pub type RouterConfiguration<T: Config> = StorageValue<_, RouterConfig, ValueQuery>;

  /// Assets tracked by the oracle for price monitoring
  #[pallet::storage]
  pub type TrackedAssets<T: Config> = StorageValue<_, Vec<AssetKind>, ValueQuery>;

  /// Current router fee (can be updated by governance)
  #[pallet::storage]
  #[pallet::getter(fn router_fee)]
  pub type RouterFee<T: Config> = StorageValue<_, Permill, ValueQuery, T::DefaultRouterFee>;

  /// EMA price storage for asset pairs (key: (asset_in, asset_out))
  #[pallet::storage]
  pub type EmaPrices<T: Config> = StorageDoubleMap<
    _,
    Blake2_128Concat,
    AssetKind,
    Blake2_128Concat,
    AssetKind,
    Balance,
    ValueQuery,
  >;

  /// Last update block for EMA prices
  #[pallet::storage]
  pub type EmaLastUpdate<T: Config> = StorageDoubleMap<
    _,
    Blake2_128Concat,
    AssetKind,
    Blake2_128Concat,
    AssetKind,
    BlockNumberFor<T>,
    ValueQuery,
  >;

  /// Total value locked for asset pairs (for TVL-weighted EMA)
  #[pallet::storage]
  pub type PairTvl<T: Config> = StorageDoubleMap<
    _,
    Blake2_128Concat,
    AssetKind,
    Blake2_128Concat,
    AssetKind,
    Balance,
    ValueQuery,
  >;

  #[pallet::error]
  pub enum Error<T> {
    /// No viable route found between tokens
    NoRouteFound,
    /// Identical source and target assets
    IdenticalAssets,
    /// Amount is zero
    ZeroAmount,
    /// Amount below minimum swap threshold
    AmountTooLow,
    /// Insufficient liquidity in pools
    InsufficientLiquidity,
    /// Output amount below minimum acceptable
    SlippageExceeded,
    /// Transaction deadline passed
    DeadlinePassed,
    /// Fee processing failed
    FeeRoutingFailed,
    /// Accumulated balance processing failed

    /// Price deviation exceeds maximum allowed
    PriceDeviationExceeded,
    /// Invalid price oracle data
    InvalidOracleData,
    /// Multi-hop routing path too long
    PathTooLong,
    /// No viable multi-hop route found
    NoMultiHopRoute,
  }

  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    /// Swap successfully executed
    SwapExecuted {
      who: T::AccountId,
      from: AssetKind,
      to: AssetKind,
      amount_in: Balance,
      amount_out: Balance,
    },
    /// Fee collected and routed
    FeeCollected {
      asset: AssetKind,
      amount: Balance,
      source: T::AccountId,
      collector: T::AccountId,
    },
    /// EMA price updated
    EmaPriceUpdated {
      asset_in: AssetKind,
      asset_out: AssetKind,
      price: Balance,
      tvl: Balance,
    },
    /// Price deviation detected
    PriceDeviationDetected {
      asset_in: AssetKind,
      asset_out: AssetKind,
      current_price: Balance,
      ema_price: Balance,
      deviation: u32,
    },
    /// Asset added to tracking list
    TrackedAssetAdded { asset: AssetKind },
    /// Router fee updated
    RouterFeeUpdated { old_fee: Permill, new_fee: Permill },
  }

  impl<T: Config> From<DispatchError> for Error<T> {
    fn from(_: DispatchError) -> Self {
      Error::<T>::FeeRoutingFailed
    }
  }

  #[pallet::call]
  impl<T: Config> Pallet<T> {
    /// Execute a token swap through the router
    #[pallet::call_index(0)]
    #[pallet::weight(100_000_000)]
    pub fn swap(
      origin: OriginFor<T>,
      from: AssetKind,
      to: AssetKind,
      amount_in: Balance,
      min_amount_out: Balance,
      recipient: T::AccountId,
      deadline: BlockNumberFor<T>,
    ) -> DispatchResult {
      let who = ensure_signed(origin)?;

      // Validate inputs
      ensure!(from != to, Error::<T>::IdenticalAssets);
      ensure!(amount_in > 0, Error::<T>::ZeroAmount);
      ensure!(
        amount_in >= T::MinSwapForeign::get(),
        Error::<T>::AmountTooLow
      );
      ensure!(
        polkadot_sdk::frame_system::Pallet::<T>::block_number() <= deadline,
        Error::<T>::DeadlinePassed
      );

      // Update oracle using pre-swap pool reserves to prevent manipulation
      Self::update_oracle_from_reserves(from, to)?;

      // Use advanced route selection with optimal path finding
      let amount_out =
        Self::execute_optimal_route(&who, from, to, amount_in, min_amount_out, &recipient)?;

      // Collect router fee
      Self::collect_router_fee(from, amount_in, &who)?;

      // Emit event with actual amounts
      Self::deposit_event(Event::SwapExecuted {
        who: who.clone(),
        from,
        to,
        amount_in,
        amount_out,
      });

      Ok(())
    }

    /// Add asset to tracked assets list for oracle monitoring (governance only)
    #[pallet::call_index(1)]
    #[pallet::weight(10_000)]
    pub fn add_tracked_asset(origin: OriginFor<T>, asset: AssetKind) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;

      TrackedAssets::<T>::mutate(|assets_list| {
        if !assets_list.contains(&asset) {
          assets_list.push(asset);
        }
      });

      Self::deposit_event(Event::TrackedAssetAdded { asset });
      Ok(())
    }

    /// Update router fee (governance only)
    #[pallet::call_index(2)]
    #[pallet::weight(T::WeightInfo::update_router_fee())]
    pub fn update_router_fee(origin: OriginFor<T>, new_fee: Permill) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;

      let old_fee = RouterFee::<T>::get();
      RouterFee::<T>::put(new_fee);

      Self::deposit_event(Event::RouterFeeUpdated { old_fee, new_fee });
      Ok(())
    }
  }

  impl<T: Config> Pallet<T> {
    /// Execute direct swap through asset conversion
    fn execute_direct_swap(
      who: &T::AccountId,
      path: &[AssetKind],
      amount_in: Balance,
      min_amount_out: Balance,
      recipient: &T::AccountId,
    ) -> Result<Balance, DispatchError> {
      if path.len() < 2 {
        return Err(Error::<T>::NoRouteFound.into());
      }

      T::AssetConversion::swap_exact_tokens_for_tokens(
        who.clone(),
        path.to_vec(),
        amount_in,
        min_amount_out,
        recipient.clone(),
        true, // keep_alive
      )
    }

    /// Execute optimal route selection with advanced routing
    fn execute_optimal_route(
      who: &T::AccountId,
      from: AssetKind,
      to: AssetKind,
      amount_in: Balance,
      min_amount_out: Balance,
      recipient: &T::AccountId,
    ) -> Result<Balance, DispatchError> {
      // Find optimal route using advanced selection
      let route_comparison =
        Self::find_optimal_route(from, to, amount_in).ok_or(Error::<T>::NoRouteFound)?;

      // Validate price protection for the route
      Self::validate_price_protection(
        &route_comparison.path,
        amount_in,
        min_amount_out,
        route_comparison.expected_output,
      )?;

      // Execute the selected route
      let amount_out = match route_comparison.mechanism {
        RouteMechanism::DirectMint { foreign_asset } => {
          // TMC Mints Native Token.
          // foreign_asset is the Collateral (from).
          T::TmcPallet::mint_with_distribution(who, foreign_asset, amount_in)?
        }
        _ => Self::execute_direct_swap(
          who,
          &route_comparison.path,
          amount_in,
          min_amount_out,
          recipient,
        )?,
      };

      Ok(amount_out)
    }

    /// Validate price protection before swap execution
    fn validate_price_protection(
      path: &[AssetKind],
      amount_in: Balance,
      min_amount_out: Balance,
      expected_output: Balance,
    ) -> Result<(), Error<T>> {
      // Basic slippage check on the quote
      if expected_output < min_amount_out {
        return Err(Error::<T>::SlippageExceeded);
      }

      if path.len() < 2 {
        return Err(Error::<T>::NoRouteFound);
      }

      let from = path.first().copied().unwrap();
      let to = path.last().copied().unwrap();

      if from == to {
        return Err(Error::<T>::IdenticalAssets);
      }

      if path.len() == 2 {
        let router_fee = Self::calculate_router_fee(amount_in);
        let amount_after_fee = amount_in.saturating_sub(router_fee);
        let current_output = expected_output; // Use pre-calculated output to avoid double DB read

        let current_price_normalized = current_output
          .saturating_mul(T::Precision::get())
          .saturating_div(amount_after_fee);

        if T::PriceOracle::validate_price_deviation(from, to, current_price_normalized).is_err() {
          return Err(Error::<T>::NoRouteFound);
        }
      } else {
        let router_fee = Self::calculate_router_fee(amount_in);
        let amount_after_fee = amount_in.saturating_sub(router_fee);
        Self::quote_multi_hop_route(path, amount_after_fee).ok_or(Error::<T>::NoRouteFound)?;
      }

      Ok(())
    }

    /// Get tracked assets for oracle updates
    pub fn get_tracked_assets() -> Option<Vec<AssetKind>> {
      let tracked = TrackedAssets::<T>::get();
      if tracked.is_empty() {
        None
      } else {
        Some(tracked)
      }
    }

    /// Update oracle using pre-swap pool reserves to prevent manipulation
    fn update_oracle_from_reserves(from: AssetKind, to: AssetKind) -> Result<(), Error<T>> {
      if let Some(pool_id) = T::AssetConversion::get_pool_id(from, to) {
        if let Some((res_a, res_b)) = T::AssetConversion::get_pool_reserves(pool_id) {
          // CORRECT: Identify which reserve matches the 'from' asset
          let (reserve_in, reserve_out) = if pool_id.0 == from {
            (res_a, res_b)
          } else {
            (res_b, res_a) // Flip reserves if pool is sorted differently
          };

          if !reserve_in.is_zero() {
            let spot_price = reserve_out
              .saturating_mul(T::Precision::get())
              .saturating_div(reserve_in);

            let tvl = res_a.saturating_add(res_b);

            T::PriceOracle::update_ema_price(from, to, spot_price, tvl)
              .map_err(|_| Error::<T>::InvalidOracleData)?;
          }
        }
      }
      Ok(())
    }

    /// Collect router fee with advanced accumulated balance processing
    fn collect_router_fee(
      fee_asset: AssetKind,
      trade_amount: Balance,
      who: &T::AccountId,
    ) -> Result<(), Error<T>> {
      let fee_amount = Self::calculate_router_fee(trade_amount);

      if fee_amount == 0 {
        return Ok(());
      }

      // Anti-self-taxation: system operations are fee-free
      if who == &Self::account_id() {
        return Ok(());
      }

      // Direct one-hop transfer to burning manager account
      T::FeeAdapter::route_fee(who, fee_asset, fee_amount)
        .map_err(|_| Error::<T>::FeeRoutingFailed)?;

      Self::deposit_event(Event::<T>::FeeCollected {
        asset: fee_asset,
        amount: fee_amount,
        source: who.clone(),
        collector: T::BurningManagerAccount::get(),
      });
      Ok(())
    }

    /// Get pallet account ID
    pub fn account_id() -> T::AccountId {
      polkadot_sdk::frame_support::PalletId(*b"axialrt0").into_account_truncating()
    }

    /// Get quote for swapping from asset_from to asset_to with amount_in
    pub fn quote_price(
      asset_from: AssetKind,
      asset_to: AssetKind,
      amount_in: Balance,
    ) -> Result<Balance, DispatchError> {
      if asset_from == asset_to {
        return Err(Error::<T>::IdenticalAssets.into());
      }
      if amount_in.is_zero() {
        return Err(Error::<T>::ZeroAmount.into());
      }

      // Get quote from asset conversion pallet
      T::AssetConversion::quote_price_exact_tokens_for_tokens(asset_from, asset_to, amount_in, true)
        .ok_or_else(|| Error::<T>::NoRouteFound.into())
    }

    /// Get oracle price for asset pair
    pub fn get_oracle_price(asset_from: AssetKind, asset_to: AssetKind) -> Option<Balance> {
      T::PriceOracle::get_ema_price(asset_from, asset_to)
    }

    /// Find best multi-hop route using Native anchor
    fn find_best_multi_hop_route(
      from: AssetKind,
      to: AssetKind,
      amount_after_fee: Balance,
    ) -> Option<Vec<AssetKind>> {
      let native_asset = T::NativeAsset::get();

      // Only support Native-anchored routing for now
      if from == native_asset || to == native_asset {
        return None; // Direct route should be used
      }

      // Check if both hops have liquidity
      let hop1_quote = T::AssetConversion::quote_price_exact_tokens_for_tokens(
        from,
        native_asset,
        amount_after_fee,
        true,
      );
      let hop2_quote = if let Some(intermediate_amount) = hop1_quote {
        T::AssetConversion::quote_price_exact_tokens_for_tokens(
          native_asset,
          to,
          intermediate_amount,
          true,
        )
      } else {
        None
      };

      if hop1_quote.is_some() && hop2_quote.is_some() {
        Some(vec![from, native_asset, to])
      } else {
        None
      }
    }

    /// Advanced route selection with TMC integration
    fn find_optimal_route(
      from: AssetKind,
      to: AssetKind,
      amount_in: Balance,
    ) -> Option<RouteComparison> {
      let native_asset = T::NativeAsset::get();
      let router_fee = Self::calculate_router_fee(amount_in);
      let amount_after_fee = amount_in.saturating_sub(router_fee);
      let mut candidate_routes = Vec::new();

      // 1. Direct XYK route
      if let Some(direct_output) =
        T::AssetConversion::quote_price_exact_tokens_for_tokens(from, to, amount_after_fee, true)
      {
        let final_output = direct_output;
        let price_impact = Self::calculate_price_impact(from, to, amount_after_fee, direct_output);

        candidate_routes.push(RouteComparison::new(
          final_output,
          vec![from, to],
          RouteMechanism::DirectXyk {
            pool_id: (from, to),
          },
          price_impact,
          router_fee,
        ));
      }

      // 2. Direct mint route (if applicable)
      // TMC Mints Native Token using Foreign/Local Assets (Collateral)
      // So we support: Local/Foreign (from) -> Native (to)

      if from != native_asset && to == native_asset && T::TmcPallet::has_curve(from) {
        if let Ok(tmc_output) = T::TmcPallet::calculate_user_receives(from, amount_after_fee) {
          let final_output = tmc_output;
          let price_impact = Permill::zero(); // TMC has predictable pricing

          candidate_routes.push(RouteComparison::new(
            final_output,
            vec![from, to],
            RouteMechanism::DirectMint {
              foreign_asset: from,
            },
            price_impact,
            router_fee,
          ));
        }
      }

      // 3. Multi-hop Native route
      if from != native_asset && to != native_asset {
        if let Some(multi_hop_path) = Self::find_best_multi_hop_route(from, to, amount_after_fee) {
          if let Some(multi_hop_output) =
            Self::quote_multi_hop_route(&multi_hop_path, amount_after_fee)
          {
            let final_output = multi_hop_output;
            let price_impact = Self::calculate_multi_hop_price_impact(
              &multi_hop_path,
              amount_after_fee,
              multi_hop_output,
            );

            candidate_routes.push(RouteComparison::new(
              final_output,
              multi_hop_path,
              RouteMechanism::MultiHopNative {
                hops: vec![from, native_asset, to],
              },
              price_impact,
              router_fee,
            ));
          }
        }
      }

      // Select route with highest efficiency score
      candidate_routes
        .into_iter()
        .max_by_key(|route| route.efficiency_score())
    }

    /// Quote multi-hop route output
    fn quote_multi_hop_route(path: &[AssetKind], amount_in: Balance) -> Option<Balance> {
      if path.len() < 2 {
        return None;
      }

      let mut current_amount = amount_in;
      for window in path.windows(2) {
        let from = window[0];
        let to = window[1];

        if let Some(output) =
          T::AssetConversion::quote_price_exact_tokens_for_tokens(from, to, current_amount, true)
        {
          current_amount = output;
        } else {
          return None;
        }
      }

      Some(current_amount)
    }

    /// Calculate price impact for direct route
    fn calculate_price_impact(
      from: AssetKind,
      to: AssetKind,
      amount_in: Balance,
      amount_out: Balance,
    ) -> Permill {
      // Simplified price impact calculation
      // In production, this would use pool reserves and more sophisticated math
      if let Some(ema_price) = T::PriceOracle::get_ema_price(from, to) {
        if ema_price > 0 {
          let expected_out = amount_in.saturating_mul(ema_price) / T::Precision::get();
          if expected_out > amount_out {
            return Permill::from_rational(expected_out - amount_out, expected_out);
          }
        }
      }
      Permill::zero()
    }

    /// Calculate price impact for multi-hop route
    fn calculate_multi_hop_price_impact(
      path: &[AssetKind],
      amount_in: Balance,
      amount_out: Balance,
    ) -> Permill {
      // Simplified multi-hop price impact
      // In production, this would calculate cumulative impact across all hops
      if let Some(direct_quote) = T::AssetConversion::quote_price_exact_tokens_for_tokens(
        path[0],
        path[path.len() - 1],
        amount_in,
        true,
      ) {
        if direct_quote > amount_out {
          return Permill::from_rational(direct_quote - amount_out, direct_quote);
        }
      }
      Permill::zero()
    }

    /// Calculate router fee for a given amount
    pub fn calculate_router_fee(amount: Balance) -> Balance {
      RouterFee::<T>::get().mul_floor(amount)
    }
  }

  /// Genesis configuration
  #[pallet::genesis_config]
  pub struct GenesisConfig<T: Config> {
    /// Native asset
    pub native_asset: AssetKind,
    /// Router fee in basis points
    pub router_fee_bps: u32,
    /// Assets tracked by the oracle
    pub tracked_assets: Vec<AssetKind>,
    /// Phantom data
    pub _marker: core::marker::PhantomData<T>,
  }

  impl<T: Config> Default for GenesisConfig<T> {
    fn default() -> Self {
      Self {
        native_asset: AssetKind::Native,
        router_fee_bps: 50,
        tracked_assets: vec![AssetKind::Native], // Track native asset by default
        _marker: Default::default(),
      }
    }
  }

  #[pallet::genesis_build]
  impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
    fn build(&self) {
      RouterConfiguration::<T>::put(RouterConfig {
        native_asset: self.native_asset,
        router_fee_bps: self.router_fee_bps,
      });
      TrackedAssets::<T>::put(&self.tracked_assets);
    }
  }
}

/// Best price quote result for extrinsics
#[derive(
  Clone,
  codec::Encode,
  codec::Decode,
  Eq,
  PartialEq,
  sp_runtime::RuntimeDebug,
  scale_info::TypeInfo,
  codec::MaxEncodedLen,
)]
pub struct BestPriceQuote {
  /// Expected output amount
  pub amount_out: Balance,
}
