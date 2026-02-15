extern crate alloc;

use crate as pallet_treasury_owned_liquidity;
use alloc::vec::Vec;
use codec::Encode;
use polkadot_sdk::frame_support::{
  PalletId, construct_runtime, derive_impl,
  traits::{
    ConstU32, ConstU128, Currency, Get,
    fungibles::Mutate,
    tokens::{Fortitude, Precision, Preservation},
  },
};
use polkadot_sdk::frame_system;
use polkadot_sdk::sp_io::hashing::blake2_256;
use polkadot_sdk::sp_runtime::{
  BuildStorage, DispatchError, Permill,
  testing::H256,
  traits::{BlakeTwo256, IdentityLookup},
};
use primitives::{AssetInspector, AssetKind, assets::TYPE_LP};
use std::cell::RefCell;
use std::collections::BTreeMap;

// State for Mocks
thread_local! {
    pub static POOLS: RefCell<BTreeMap<[u8; 32], (u128, u128)>> = const { RefCell::new(BTreeMap::new()) };
    // Map sorted assets to pool ID
    pub static ASSET_POOLS: RefCell<BTreeMap<(AssetKind, AssetKind), [u8; 32]>> = const { RefCell::new(BTreeMap::new()) };
}

pub const TEST_LP_ASSET_ID: u32 = TYPE_LP | 1;

pub fn set_pool(asset_a: AssetKind, asset_b: AssetKind, reserve_a: u128, reserve_b: u128) {
  let (min, max) = if asset_a < asset_b {
    (asset_a, asset_b)
  } else {
    (asset_b, asset_a)
  };
  let id = blake2_256(&(min, max).encode());
  POOLS.with(|p| p.borrow_mut().insert(id, (reserve_a, reserve_b)));
  ASSET_POOLS.with(|p| p.borrow_mut().insert((min, max), id));
}

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
  pub struct Test {
    System: frame_system,
    Balances: polkadot_sdk::pallet_balances,
    Assets: polkadot_sdk::pallet_assets,
    TreasuryOwnedLiquidity: pallet_treasury_owned_liquidity,
  }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
  type Block = Block;
  type AccountId = u64;
  type Lookup = IdentityLookup<Self::AccountId>;
  type Hash = H256;
  type Hashing = BlakeTwo256;
  type AccountData = polkadot_sdk::pallet_balances::AccountData<u128>;
}

impl polkadot_sdk::pallet_balances::Config for Test {
  type MaxLocks = ();
  type MaxReserves = ();
  type ReserveIdentifier = [u8; 8];
  type Balance = u128;
  type DustRemoval = ();
  type RuntimeEvent = RuntimeEvent;
  type ExistentialDeposit = ConstU128<1>;
  type AccountStore = System;
  type WeightInfo = ();
  type FreezeIdentifier = ();
  type MaxFreezes = ();
  type RuntimeHoldReason = ();
  type RuntimeFreezeReason = ();
  type DoneSlashHandler = ();
}

impl polkadot_sdk::pallet_assets::Config for Test {
  type RuntimeEvent = RuntimeEvent;
  type Balance = u128;
  type AssetId = u32;
  type AssetIdParameter = u32;
  type Currency = Balances;
  type CreateOrigin = polkadot_sdk::frame_support::traits::AsEnsureOriginWithArg<
    frame_system::EnsureSigned<Self::AccountId>,
  >;
  type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
  type AssetDeposit = ConstU128<1>;
  type AssetAccountDeposit = ConstU128<1>;
  type MetadataDepositBase = ConstU128<1>;
  type MetadataDepositPerByte = ConstU128<1>;
  type ApprovalDeposit = ConstU128<1>;
  type StringLimit = ConstU32<50>;
  type Freezer = ();
  type Extra = ();
  type ReserveData = ();
  type CallbackHandle = ();
  type WeightInfo = ();
  type RemoveItemsLimit = ConstU32<5>;
  type Holder = ();
  #[cfg(feature = "runtime-benchmarks")]
  type BenchmarkHelper = AssetBenchmarkHelper;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct AssetBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl polkadot_sdk::pallet_assets::BenchmarkHelper<u32, ()> for AssetBenchmarkHelper {
  fn create_asset_id_parameter(id: u32) -> u32 {
    id
  }
  fn create_reserve_id_parameter(_id: u32) -> () {
    ()
  }
}

// Mock Asset Conversion
pub struct MockAssetConversion;
impl pallet_treasury_owned_liquidity::AssetConversionApi<u64, u128> for MockAssetConversion {
  fn get_pool_id(asset1: AssetKind, asset2: AssetKind) -> Option<[u8; 32]> {
    let (min, max) = if asset1 < asset2 {
      (asset1, asset2)
    } else {
      (asset2, asset1)
    };
    ASSET_POOLS.with(|p| p.borrow().get(&(min, max)).cloned())
  }

  fn get_pool_reserves(pool_id: [u8; 32]) -> Option<(u128, u128)> {
    POOLS.with(|p| p.borrow().get(&pool_id).cloned())
  }

  fn quote_price_exact_tokens_for_tokens(
    asset1: AssetKind,
    asset2: AssetKind,
    amount_in: u128,
    include_fee: bool,
  ) -> Option<u128> {
    let pool_id = Self::get_pool_id(asset1, asset2)?;
    let (res_a, res_b) = Self::get_pool_reserves(pool_id)?;
    let (reserve_in, reserve_out) = if asset1 < asset2 {
      (res_a, res_b)
    } else {
      (res_b, res_a)
    };
    if reserve_in == 0 || reserve_out == 0 {
      return None;
    }
    let amount_in_adjusted = if include_fee {
      amount_in.saturating_mul(997) / 1000
    } else {
      amount_in
    };
    let numerator = amount_in_adjusted.saturating_mul(reserve_out);
    let denominator = reserve_in.saturating_add(amount_in_adjusted);
    if denominator == 0 {
      return None;
    }
    Some(numerator / denominator)
  }

  fn swap_exact_tokens_for_tokens(
    who: &u64,
    path: Vec<AssetKind>,
    amount_in: u128,
    min_amount_out: u128,
    recipient: u64,
    _keep_alive: bool,
  ) -> Result<u128, DispatchError> {
    if path.len() < 2 {
      return Err(DispatchError::Other("Invalid path"));
    }
    let asset_in = path[0];
    let asset_out = path[path.len() - 1];
    let amount_out =
      Self::quote_price_exact_tokens_for_tokens(asset_in, asset_out, amount_in, true)
        .ok_or(DispatchError::Other("Quote failed"))?;
    if amount_out < min_amount_out {
      return Err(DispatchError::Other("Slippage exceeded"));
    }
    let pool_id =
      Self::get_pool_id(asset_in, asset_out).ok_or(DispatchError::Other("Pool not found"))?;
    POOLS.with(|p| {
      let mut pools = p.borrow_mut();
      let (res_a, res_b) = pools.get(&pool_id).unwrap();
      let (new_res_a, new_res_b) = if asset_in < asset_out {
        (
          res_a.saturating_add(amount_in),
          res_b.saturating_sub(amount_out),
        )
      } else {
        (
          res_a.saturating_sub(amount_out),
          res_b.saturating_add(amount_in),
        )
      };
      pools.insert(pool_id, (new_res_a, new_res_b));
    });
    match asset_in {
      AssetKind::Native => {
        let _ = <Balances as Currency<u64>>::withdraw(
          who,
          amount_in,
          polkadot_sdk::frame_support::traits::WithdrawReasons::TRANSFER,
          polkadot_sdk::frame_support::traits::ExistenceRequirement::KeepAlive,
        )?;
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <Assets as Mutate<u64>>::burn_from(
          id,
          who,
          amount_in,
          Preservation::Expendable,
          Precision::Exact,
          Fortitude::Polite,
        )?;
      }
    }
    match asset_out {
      AssetKind::Native => {
        let _ = <Balances as Currency<u64>>::deposit_creating(&recipient, amount_out);
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <Assets as Mutate<u64>>::mint_into(id, &recipient, amount_out)?;
      }
    }
    Ok(amount_out)
  }

  fn remove_liquidity(
    who: &u64,
    asset1: AssetKind,
    asset2: AssetKind,
    lp_amount: u128,
  ) -> Result<(u128, u128), DispatchError> {
    if lp_amount == 0 {
      return Err(DispatchError::Other("LP amount is zero"));
    }
    let pool_id =
      Self::get_pool_id(asset1, asset2).ok_or(DispatchError::Other("Pool not found"))?;
    let (amount1, amount2) = POOLS.with(|p| {
      let mut pools = p.borrow_mut();
      let (res_a, res_b) = pools
        .get(&pool_id)
        .copied()
        .ok_or(DispatchError::Other("Pool not found"))?;
      let amount_a = lp_amount.min(res_a);
      let amount_b = lp_amount.min(res_b);
      pools.insert(
        pool_id,
        (
          res_a.saturating_sub(amount_a),
          res_b.saturating_sub(amount_b),
        ),
      );
      Ok::<(u128, u128), DispatchError>(if asset1 < asset2 {
        (amount_a, amount_b)
      } else {
        (amount_b, amount_a)
      })
    })?;
    match asset1 {
      AssetKind::Native => {
        let _ = <Balances as Currency<u64>>::deposit_creating(who, amount1);
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <Assets as Mutate<u64>>::mint_into(id, who, amount1)?;
      }
    }
    match asset2 {
      AssetKind::Native => {
        let _ = <Balances as Currency<u64>>::deposit_creating(who, amount2);
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <Assets as Mutate<u64>>::mint_into(id, who, amount2)?;
      }
    }
    Ok((amount1, amount2))
  }

  fn get_pool_pair_for_lp(lp_token_id: u32) -> Option<(AssetKind, AssetKind)> {
    let lp_asset = AssetKind::Local(lp_token_id);
    if lp_asset.is_lp() {
      return Some((AssetKind::Native, AssetKind::Local(1)));
    }
    None
  }

  fn initialize_lp_asset_namespace() {}
}

pub struct PalletIdStub;
impl Get<PalletId> for PalletIdStub {
  fn get() -> PalletId {
    PalletId(*b"py/trsry")
  }
}

pub struct AccountIdStub;
impl Get<u64> for AccountIdStub {
  fn get() -> u64 {
    1000
  }
}

pub struct AllocationStub;
impl Get<u32> for AllocationStub {
  fn get() -> u32 {
    250_000 // 25%
  }
}

pub struct RatioStub;
impl Get<Permill> for RatioStub {
  fn get() -> Permill {
    Permill::from_percent(25)
  }
}

#[cfg(feature = "runtime-benchmarks")]
pub struct RuntimeBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl crate::BenchmarkHelper<u64> for RuntimeBenchmarkHelper {
  fn create_asset(asset_id: u32) -> polkadot_sdk::sp_runtime::DispatchResult {
    if !<Assets as polkadot_sdk::frame_support::traits::fungibles::Inspect<u64>>::asset_exists(
      asset_id,
    ) {
      Assets::force_create(RuntimeOrigin::root(), asset_id, 1, true, 1)?;
    }
    Ok(())
  }

  fn fund_account(
    who: &u64,
    asset: primitives::AssetKind,
    amount: u128,
  ) -> polkadot_sdk::sp_runtime::DispatchResult {
    match asset {
      primitives::AssetKind::Native => {
        let _ = Balances::deposit_creating(who, amount);
      }
      primitives::AssetKind::Local(id) | primitives::AssetKind::Foreign(id) => {
        <Assets as Mutate<u64>>::mint_into(id, who, amount)?;
      }
    }
    Ok(())
  }
}

impl pallet_treasury_owned_liquidity::Config for Test {
  type Assets = Assets;
  type Currency = Balances;
  type TreasuryAccount = AccountIdStub;
  type PalletId = PalletIdStub;
  type Precision = ConstU128<{ primitives::ecosystem::params::PRECISION }>;
  type BucketAAllocation = AllocationStub;
  type BucketBAllocation = AllocationStub;
  type BucketCAllocation = AllocationStub;
  type BucketDAllocation = AllocationStub;
  type BucketAAccount = AccountIdStub;
  type BucketBAccount = AccountIdStub;
  type BucketCAccount = AccountIdStub;
  type BucketDAccount = AccountIdStub;
  type ZapManagerAccount = AccountIdStub;
  type BurningManagerAccount = AccountIdStub;
  type BucketARatio = RatioStub;
  type BucketBRatio = RatioStub;
  type BucketCRatio = RatioStub;
  type AssetConversion = MockAssetConversion;
  type MinSwapForeign = ConstU128<{ primitives::ecosystem::params::TOL_MIN_SWAP_FOREIGN }>;
  type MaxPriceDeviation = RatioStub;
  type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
  type WeightInfo = ();
  type MaxTolRequestsPerBlock = ConstU32<10>;
  type MaxNonLpSweepsPerBlock = ConstU32<16>;
  type MaxTolDomains = ConstU32<32>;
  #[cfg(feature = "runtime-benchmarks")]
  type BenchmarkHelper = RuntimeBenchmarkHelper;
}

pub fn new_test_ext() -> polkadot_sdk::sp_io::TestExternalities {
  let mut t = frame_system::GenesisConfig::<Test>::default()
    .build_storage()
    .unwrap();
  polkadot_sdk::pallet_assets::GenesisConfig::<Test> {
    assets: alloc::vec![(1, 1, true, 1), (TEST_LP_ASSET_ID, 1, true, 1)],
    metadata: alloc::vec![],
    accounts: alloc::vec![],
    reserves: alloc::vec![],
    next_asset_id: None,
  }
  .assimilate_storage(&mut t)
  .unwrap();
  pallet_treasury_owned_liquidity::GenesisConfig::<Test>::default()
    .assimilate_storage(&mut t)
    .unwrap();
  POOLS.with(|p| p.borrow_mut().clear());
  ASSET_POOLS.with(|p| p.borrow_mut().clear());
  // Default pool for tests that expect one
  set_pool(AssetKind::Native, AssetKind::Local(1), 1000, 1000);
  t.into()
}
