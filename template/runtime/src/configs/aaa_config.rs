//! AAA pallet configuration for the TMCTOL parachain runtime.
//!
//! Wires the two adapter traits (`AssetOps`, `DexOps`) to concrete runtime pallets:
//! - Native token: `pallet-balances`
//! - Foreign assets: `pallet-assets`
//! - Swaps: Axial Router
//! - Liquidity: Asset Conversion

use super::*;
use primitives::{AssetKind, ecosystem};

use polkadot_sdk::frame_support::traits::{
  Currency, Get,
  fungible::Inspect as NativeInspect,
  fungibles::{Inspect as FungiblesInspect, Mutate as FungiblesMutate},
  tokens::{Fortitude, Precision, Preservation},
};
use polkadot_sdk::pallet_asset_conversion::PoolLocator;
use polkadot_sdk::sp_runtime::{DispatchError, TokenError};

use crate::{AssetConversion, RuntimeOrigin};
use pallet_aaa::{AssetOps, DexOps};

parameter_types! {
  pub const AaaMinWindowLength: BlockNumber = 100;
  pub const AaaPalletId: PalletId = PalletId(*ecosystem::pallet_ids::AAA_PALLET_ID);
  pub const AaaNativeAssetId: AssetKind = AssetKind::Native;
  pub const AaaMaxPipelineSteps: u32 = 10;
  pub const AaaMaxUserPipelineSteps: u32 = 3;
  pub const AaaMaxSystemPipelineSteps: u32 = 10;
  pub const AaaMaxConditionsPerStep: u32 = 4;
  pub const AaaMaxOwnedAaas: u32 = 16;
  pub const AaaMaxOwnerSlots: u16 = 256;
  pub const AaaMaxReadyRingLength: u32 = 64;
  pub const AaaMaxDeferredRingLength: u32 = 64;
  pub const AaaMaxDeferredRetriesPerBlock: u32 = 16;
  pub const AaaMaxSystemExecutionsPerBlock: u32 = 16;
  pub const AaaMaxUserExecutionsPerBlock: u32 = 32;
  pub const AaaFairnessWeightSystem: u32 = 1;
  pub const AaaFairnessWeightUser: u32 = 3;
  pub const AaaMaxAddressEventInboxCount: u32 = 16;
  pub const AaaMaxPoolScan: u32 = 64;
  /// Per-step flat evaluation cost (§3.3). 0 native = free evaluation in MVP.
  pub const AaaStepBaseFee: u128 = 0;
  /// Per-condition balance read cost (§3.3).
  pub const AaaConditionReadFee: u128 = 0;
  /// Storage rent per block for User AAA (§3.2). ~0.144 native/day at 6s blocks.
  pub const AaaRentPerBlock: u128 = 1_000_000;
  /// Max rent accrual cap (§3.2) — prevents unbounded debt. 100 native tokens.
  pub const AaaMaxRentAccrual: Balance = 100 * 1_000_000_000_000u128;
  pub const AaaMaxRefundableAssets: u32 = 16;
  pub const AaaMaxConsecutiveFailures: u32 = 10;
  pub const AaaMinUserBalance: Balance = 5 * ExistentialDeposit::get();
  pub const AaaMaxSweepPerBlock: u32 = 5;
  /// Cost of one asset transfer during refund (§4.2). ~0.0001 native.
  pub const AaaRefundTransferCost: Balance = 100_000_000;
}

parameter_types! {
  pub const AaaBudgetPct: polkadot_sdk::sp_runtime::Permill =
    polkadot_sdk::sp_runtime::Permill::from_percent(30);
}

/// Fee sink — receives AAA evaluation and execution fees.
pub struct AaaFeeRecipient;
impl Get<crate::AccountId> for AaaFeeRecipient {
  fn get() -> crate::AccountId {
    use polkadot_sdk::sp_runtime::traits::AccountIdConversion;
    AaaPalletId::get().into_account_truncating()
  }
}

pub struct TmctolAssetOps;

impl AssetOps<AccountId, AssetKind, Balance> for TmctolAssetOps {
  fn transfer(
    from: &AccountId,
    to: &AccountId,
    asset: AssetKind,
    amount: Balance,
  ) -> Result<(), DispatchError> {
    match asset {
      AssetKind::Native => <Balances as Currency<AccountId>>::transfer(
        from,
        to,
        amount,
        polkadot_sdk::frame_support::traits::ExistenceRequirement::AllowDeath,
      ),
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesMutate<AccountId>>::transfer(
          id,
          from,
          to,
          amount,
          Preservation::Expendable,
        )?;
        Ok(())
      }
    }
  }

  fn burn(who: &AccountId, asset: AssetKind, amount: Balance) -> Result<(), DispatchError> {
    match asset {
      AssetKind::Native => {
        let (_, remainder) = <Balances as Currency<AccountId>>::slash(who, amount);
        if remainder > 0 {
          return Err(DispatchError::Token(TokenError::FundsUnavailable));
        }
        Ok(())
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesMutate<AccountId>>::burn_from(
          id,
          who,
          amount,
          Preservation::Expendable,
          Precision::Exact,
          Fortitude::Polite,
        )?;
        Ok(())
      }
    }
  }

  fn mint(to: &AccountId, asset: AssetKind, amount: Balance) -> Result<(), DispatchError> {
    match asset {
      AssetKind::Native => {
        let _ = <Balances as Currency<AccountId>>::deposit_creating(to, amount);
        Ok(())
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesMutate<AccountId>>::mint_into(id, to, amount)?;
        Ok(())
      }
    }
  }

  fn balance(who: &AccountId, asset: AssetKind) -> Balance {
    match asset {
      AssetKind::Native => <Balances as NativeInspect<AccountId>>::balance(who),
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, who)
      }
    }
  }
}

pub struct TmctolDexOps;

impl DexOps<AccountId, AssetKind, Balance> for TmctolDexOps {
  fn swap_exact_in(
    who: &AccountId,
    asset_in: AssetKind,
    asset_out: AssetKind,
    amount_in: Balance,
    min_out: Balance,
  ) -> Result<Balance, DispatchError> {
    pallet_axial_router::Pallet::<Runtime>::execute_swap_for(
      who, asset_in, asset_out, amount_in, min_out, who,
    )
  }

  fn swap_exact_out(
    who: &AccountId,
    asset_in: AssetKind,
    asset_out: AssetKind,
    amount_out: Balance,
    max_in: Balance,
  ) -> Result<Balance, DispatchError> {
    let quote =
      pallet_axial_router::Pallet::<Runtime>::quote_price(asset_in, asset_out, amount_out)?;
    if quote > max_in {
      return Err(DispatchError::Other("SlippageExceeded"));
    }
    let _ = pallet_axial_router::Pallet::<Runtime>::execute_swap_for(
      who, asset_in, asset_out, quote, amount_out, who,
    )?;
    Ok(quote)
  }

  fn get_quote(asset_in: AssetKind, asset_out: AssetKind, amount_in: Balance) -> Option<Balance> {
    pallet_axial_router::Pallet::<Runtime>::quote_price(asset_in, asset_out, amount_in).ok()
  }

  fn add_liquidity(
    who: &AccountId,
    asset_a: AssetKind,
    asset_b: AssetKind,
    amount_a: Balance,
    amount_b: Balance,
  ) -> Result<(Balance, Balance, Balance), DispatchError> {
    use alloc::boxed::Box;
    let lp_before = Self::lp_balance(who, asset_a, asset_b);
    AssetConversion::add_liquidity(
      RuntimeOrigin::signed(who.clone()),
      Box::new(asset_a),
      Box::new(asset_b),
      amount_a,
      amount_b,
      0,
      0,
      who.clone(),
    )?;
    let lp_after = Self::lp_balance(who, asset_a, asset_b);
    let lp_minted = lp_after.saturating_sub(lp_before);
    Ok((amount_a, amount_b, lp_minted))
  }

  fn remove_liquidity(
    who: &AccountId,
    lp_asset: AssetKind,
    lp_amount: Balance,
  ) -> Result<(Balance, Balance), DispatchError> {
    use alloc::boxed::Box;
    let lp_id = match lp_asset {
      AssetKind::Local(id) => id,
      _ => return Err(DispatchError::Other("LP asset must be Local")),
    };
    let (asset_a, asset_b) =
      Self::pool_pair_for_lp(lp_id).ok_or(DispatchError::Other("Pool not found for LP token"))?;
    let before_a = TmctolAssetOps::balance(who, asset_a);
    let before_b = TmctolAssetOps::balance(who, asset_b);
    AssetConversion::remove_liquidity(
      RuntimeOrigin::signed(who.clone()),
      Box::new(asset_a),
      Box::new(asset_b),
      lp_amount,
      0,
      0,
      who.clone(),
    )?;
    let after_a = TmctolAssetOps::balance(who, asset_a);
    let after_b = TmctolAssetOps::balance(who, asset_b);
    Ok((
      after_a.saturating_sub(before_a),
      after_b.saturating_sub(before_b),
    ))
  }

  fn get_pool_reserves(asset_a: AssetKind, asset_b: AssetKind) -> Option<(Balance, Balance)> {
    AssetConversion::get_reserves(asset_a, asset_b).ok()
  }
}

impl TmctolDexOps {
  fn lp_balance(who: &AccountId, asset_a: AssetKind, asset_b: AssetKind) -> Balance {
    let pool_id =
      <Runtime as pallet_asset_conversion::Config>::PoolLocator::pool_id(&asset_a, &asset_b).ok();
    let Some(pool_id) = pool_id else {
      return 0;
    };
    let Some(pool_info) = pallet_asset_conversion::Pools::<Runtime>::get(pool_id) else {
      return 0;
    };
    <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(
      pool_info.lp_token,
      who,
    )
  }

  fn pool_pair_for_lp(lp_token_id: u32) -> Option<(AssetKind, AssetKind)> {
    let mut scanned = 0u32;
    for (pool_key, pool_info) in pallet_asset_conversion::Pools::<Runtime>::iter() {
      if scanned >= <Runtime as pallet_aaa::Config>::MaxAdapterScan::get() {
        break;
      }
      scanned = scanned.saturating_add(1);
      if pool_info.lp_token == lp_token_id {
        return Some(pool_key);
      }
    }
    None
  }
}

impl pallet_aaa::Config for Runtime {
  type AssetId = AssetKind;
  type Balance = Balance;
  type NativeAssetId = AaaNativeAssetId;
  type AssetOps = TmctolAssetOps;
  type DexOps = TmctolDexOps;
  type MinWindowLength = AaaMinWindowLength;
  type PalletId = AaaPalletId;
  type SystemOrigin = EnsureRoot<AccountId>;
  type GlobalBreakerOrigin = EnsureRoot<AccountId>;
  type MaxPipelineSteps = AaaMaxPipelineSteps;
  type MaxUserPipelineSteps = AaaMaxUserPipelineSteps;
  type MaxSystemPipelineSteps = AaaMaxSystemPipelineSteps;
  type MaxConditionsPerStep = AaaMaxConditionsPerStep;
  type MaxOwnedAaas = AaaMaxOwnedAaas;
  type MaxOwnerSlots = AaaMaxOwnerSlots;
  type MaxReadyRingLength = AaaMaxReadyRingLength;
  type MaxDeferredRingLength = AaaMaxDeferredRingLength;
  type MaxDeferredRetriesPerBlock = AaaMaxDeferredRetriesPerBlock;
  type MaxSystemExecutionsPerBlock = AaaMaxSystemExecutionsPerBlock;
  type MaxUserExecutionsPerBlock = AaaMaxUserExecutionsPerBlock;
  type FairnessWeightSystem = AaaFairnessWeightSystem;
  type FairnessWeightUser = AaaFairnessWeightUser;
  type MaxAddressEventInboxCount = AaaMaxAddressEventInboxCount;
  type MaxAdapterScan = AaaMaxPoolScan;
  type AaaBudgetPct = AaaBudgetPct;
  type RentPerBlock = AaaRentPerBlock;
  type MaxRentAccrual = AaaMaxRentAccrual;
  type StepBaseFee = AaaStepBaseFee;
  type ConditionReadFee = AaaConditionReadFee;
  type WeightToFee = crate::WeightToFee;
  // Runtime binds task upper bounds so fee admission stays chain-specific and auditable
  type TaskWeightInfo = pallet_aaa::weights::SubstrateTaskWeightInfo<Runtime>;
  #[cfg(feature = "runtime-benchmarks")]
  type BenchmarkHelper = RuntimeAaaBenchmarkHelper;
  type FeeSink = AaaFeeRecipient;
  type MaxRefundableAssets = AaaMaxRefundableAssets;
  type MaxConsecutiveFailures = AaaMaxConsecutiveFailures;
  type MinUserBalance = AaaMinUserBalance;
  type MaxSweepPerBlock = AaaMaxSweepPerBlock;
  type RefundTransferCost = AaaRefundTransferCost;
  type WeightInfo = crate::weights::pallet_aaa::SubstrateWeight<Runtime>;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct RuntimeAaaBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl RuntimeAaaBenchmarkHelper {
  fn ensure_local_asset(asset_id: u32, owner: &AccountId) -> Result<(), DispatchError> {
    if !<pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::asset_exists(asset_id) {
      pallet_assets::Pallet::<Runtime>::force_create(
        RuntimeOrigin::root(),
        asset_id,
        polkadot_sdk::sp_runtime::MultiAddress::Id(owner.clone()),
        true,
        1,
      )?;
    }
    Ok(())
  }
}

#[cfg(feature = "runtime-benchmarks")]
impl pallet_aaa::BenchmarkHelper<AccountId, AssetKind, Balance> for RuntimeAaaBenchmarkHelper {
  fn setup_remove_liquidity_max_k(
    owner: &AccountId,
    max_scan: u32,
  ) -> Result<(AssetKind, Balance), DispatchError> {
    if max_scan == 0 {
      return Err(DispatchError::Other("MaxAdapterScanZero"));
    }
    let liquidity = 1_000_000_000_000u128;
    let native_seed = liquidity.saturating_mul(max_scan.saturating_add(1) as u128);
    let _ = <Balances as Currency<AccountId>>::deposit_creating(owner, native_seed);
    let mut target_lp: Option<(AssetKind, Balance)> = None;
    for i in 0..max_scan {
      let asset_a_id = 100_000u32.saturating_add(i.saturating_mul(2));
      let asset_b_id = asset_a_id.saturating_add(1);
      Self::ensure_local_asset(asset_a_id, owner)?;
      Self::ensure_local_asset(asset_b_id, owner)?;
      <pallet_assets::Pallet<Runtime> as FungiblesMutate<AccountId>>::mint_into(
        asset_a_id, owner, liquidity,
      )?;
      <pallet_assets::Pallet<Runtime> as FungiblesMutate<AccountId>>::mint_into(
        asset_b_id, owner, liquidity,
      )?;
      let asset_a = AssetKind::Local(asset_a_id);
      let asset_b = AssetKind::Local(asset_b_id);
      AssetConversion::create_pool(
        RuntimeOrigin::signed(owner.clone()),
        alloc::boxed::Box::new(asset_a),
        alloc::boxed::Box::new(asset_b),
      )?;
      AssetConversion::add_liquidity(
        RuntimeOrigin::signed(owner.clone()),
        alloc::boxed::Box::new(asset_a),
        alloc::boxed::Box::new(asset_b),
        liquidity,
        liquidity,
        0,
        0,
        owner.clone(),
      )?;
      let pool_id =
        <Runtime as pallet_asset_conversion::Config>::PoolLocator::pool_id(&asset_a, &asset_b)
          .map_err(|_| DispatchError::Other("PoolIdUnavailable"))?;
      let pool_info = pallet_asset_conversion::Pools::<Runtime>::get(pool_id)
        .ok_or(DispatchError::Other("PoolNotCreated"))?;
      if i.saturating_add(1) == max_scan {
        let lp_amount = <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(
          pool_info.lp_token,
          owner,
        );
        target_lp = Some((AssetKind::Local(pool_info.lp_token), lp_amount));
      }
    }
    target_lp.ok_or(DispatchError::Other("TargetLpMissing"))
  }
}
