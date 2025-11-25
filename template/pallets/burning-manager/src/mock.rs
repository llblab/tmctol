extern crate alloc;

use crate as pallet_burning_manager;
use alloc::vec::Vec;
use polkadot_sdk::frame_support::traits::fungibles::Mutate;
use polkadot_sdk::frame_support::traits::tokens::{Fortitude, Precision, Preservation};
use polkadot_sdk::frame_support::{
  construct_runtime, derive_impl,
  traits::{ConstU128, ConstU32, Get},
  PalletId,
};
use polkadot_sdk::frame_system;
use polkadot_sdk::sp_runtime::{
  testing::H256,
  traits::{BlakeTwo256, IdentityLookup},
  BuildStorage, DispatchError, Permill,
};
use primitives::AssetKind;
use std::cell::RefCell;
use std::collections::BTreeMap;

// State containers for stateful mocks
thread_local! {
    // AMM Pools: (AssetA, AssetB) -> (ReserveA, ReserveB)
    // Key is sorted: (min, max) for canonical ordering
    pub static POOLS: RefCell<BTreeMap<(AssetKind, AssetKind), (u128, u128)>> = const { RefCell::new(BTreeMap::new()) };

    // Oracle Prices: (AssetA, AssetB) -> Price
    pub static ORACLE_PRICES: RefCell<BTreeMap<(AssetKind, AssetKind), u128>> = const { RefCell::new(BTreeMap::new()) };
}

// Helper methods to setup state
pub fn set_pool(asset_a: AssetKind, asset_b: AssetKind, reserve_a: u128, reserve_b: u128) {
  let key = if asset_a < asset_b {
    (asset_a, asset_b)
  } else {
    (asset_b, asset_a)
  };
  // Store reserves corresponding to sorted key (ReserveMin, ReserveMax)
  let reserves = if asset_a < asset_b {
    (reserve_a, reserve_b)
  } else {
    (reserve_b, reserve_a)
  };
  POOLS.with(|p| p.borrow_mut().insert(key, reserves));
}

pub fn set_oracle_price(asset_a: AssetKind, asset_b: AssetKind, price: u128) {
  ORACLE_PRICES.with(|p| p.borrow_mut().insert((asset_a, asset_b), price));
}

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
  pub struct Test {
    System: frame_system,
    Balances: polkadot_sdk::pallet_balances,
    Assets: polkadot_sdk::pallet_assets,
    BurningManager: pallet_burning_manager,
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
impl polkadot_sdk::pallet_assets::BenchmarkHelper<u32> for AssetBenchmarkHelper {
  fn create_asset_id_parameter(id: u32) -> u32 {
    id
  }
}

pub struct MockAssetConversion;
impl pallet_burning_manager::AssetConversionApi<u64, u128> for MockAssetConversion {
  fn get_pool_id(_asset1: AssetKind, _asset2: AssetKind) -> Option<[u8; 32]> {
    Some([0u8; 32])
  }

  fn get_pool_reserves(_pool_id: [u8; 32]) -> Option<(u128, u128)> {
    // This mock assumes there's only one relevant pool or we check all.
    // For simplicity, we just take the first entry if we used [u8;32] in logic.
    // But real tests will likely set up a specific pool.
    // Since the trait forces [u8; 32], we can't easily map back to AssetKind keys without storage.
    // HACK: We will try to return (1000, 1000) as default if empty, but look in POOLS.
    // Actually, since the trait interface is restrictive here (returning [u8; 32]),
    // our logic inside BurningManager mainly calls `swap_exact_tokens_for_tokens`.
    // The `get_pool_reserves` is mostly for validation if called directly.
    Some((1000, 1000))
  }

  fn swap_exact_tokens_for_tokens(
    who: &u64,
    path: Vec<AssetKind>,
    amount_in: u128,
    min_amount_out: u128,
  ) -> Result<u128, DispatchError> {
    let asset_in = *path.first().ok_or(DispatchError::Other("Empty path"))?;
    let asset_out = *path.last().ok_or(DispatchError::Other("Empty path"))?;

    let key = if asset_in < asset_out {
      (asset_in, asset_out)
    } else {
      (asset_out, asset_in)
    };

    println!("SWAP Key: {key:?}, In: {asset_in:?}, Out: {asset_out:?}");

    let (res_a, res_b) = POOLS
      .with(|p| {
        let pools = p.borrow();
        println!("Available Pools: {:?}", pools.keys());
        pools.get(&key).cloned()
      })
      .ok_or(DispatchError::Other("Pool not found"))?;

    // Determine ReserveIn and ReserveOut based on sorted Key vs Swap Direction
    // If key is (A, B) where A < B:
    //   If swap A -> B: ResIn = A, ResOut = B
    //   If swap B -> A: ResIn = B, ResOut = A
    let (reserve_in, reserve_out) = if asset_in < asset_out {
      (res_a, res_b)
    } else {
      (res_b, res_a)
    };

    if reserve_in == 0 || reserve_out == 0 {
      return Err(DispatchError::Other("Empty reserves"));
    }

    // XYK Swap Math: y_out = (x_in * y_res) / (x_res + x_in)
    let amount_out =
      (amount_in.saturating_mul(reserve_out)) / (reserve_in.saturating_add(amount_in));

    if amount_out < min_amount_out {
      return Err(DispatchError::Other("Slippage exceeded"));
    }

    // Update Reserves
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

    POOLS.with(|p| p.borrow_mut().insert(key, (new_res_a, new_res_b)));

    // Execute Transfers (Burn/Mint Logic)
    match asset_in {
      AssetKind::Native => {
        let _ = <Balances as polkadot_sdk::frame_support::traits::Currency<u64>>::withdraw(
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

    // Note: Burning Manager usually burns the output anyway, but for the mock we should mint it
    // so the pallet logic can burn it later.
    match asset_out {
      AssetKind::Native => {
        let _ = <Balances as polkadot_sdk::frame_support::traits::Currency<u64>>::deposit_creating(
          who, amount_out,
        );
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <Assets as Mutate<u64>>::mint_into(id, who, amount_out)?;
      }
    }

    Ok(amount_out)
  }
}

pub struct MockPriceTools;
impl pallet_burning_manager::PriceTools<AssetKind, u128> for MockPriceTools {
  fn quote_spot_price(
    asset_from: AssetKind,
    asset_to: AssetKind,
    amount: u128,
  ) -> Result<u128, DispatchError> {
    // Identity check: if quoting against self, value is 1:1
    if asset_from == asset_to {
      return Ok(amount);
    }

    // Quote based on pools
    let key = if asset_from < asset_to {
      (asset_from, asset_to)
    } else {
      (asset_to, asset_from)
    };

    let (res_a, res_b) = POOLS
      .with(|p| p.borrow().get(&key).cloned())
      .ok_or(DispatchError::Other("Pool not found"))?;

    let (reserve_in, reserve_out) = if asset_from < asset_to {
      (res_a, res_b)
    } else {
      (res_b, res_a)
    };

    if reserve_in == 0 || reserve_out == 0 {
      return Ok(0);
    }

    // Simple quote: out = amount * res_out / res_in
    Ok((amount * reserve_out) / reserve_in)
  }

  fn get_oracle_price(asset_from: AssetKind, asset_to: AssetKind) -> Option<u128> {
    // Default to 1:1 (1_000_000 precision) if not set, to support tests that don't set oracle
    ORACLE_PRICES.with(|p| {
      p.borrow()
        .get(&(asset_from, asset_to))
        .cloned()
        .or(Some(1_000_000))
    })
  }
}

pub struct PalletIdStub;
impl Get<PalletId> for PalletIdStub {
  fn get() -> PalletId {
    PalletId(*b"py/burnm")
  }
}

pub struct SlippageToleranceStub;
impl Get<Permill> for SlippageToleranceStub {
  fn get() -> Permill {
    Permill::from_percent(2)
  }
}

pub struct ReferenceAssetStub;
impl Get<AssetKind> for ReferenceAssetStub {
  fn get() -> AssetKind {
    AssetKind::Local(1)
  }
}

impl pallet_burning_manager::Config for Test {
  #[cfg(feature = "runtime-benchmarks")]
  type BenchmarkHelper = BurningManagerBenchmarkHelper;
  type AdminOrigin = polkadot_sdk::frame_system::EnsureRoot<u64>;
  type Assets = Assets;
  type Currency = Balances;
  type AssetConversion = MockAssetConversion;
  type PalletId = PalletIdStub;
  type ReferenceAsset = ReferenceAssetStub;
  type DefaultMinBurnNative =
    ConstU128<{ primitives::ecosystem::params::BURNING_MANAGER_MIN_BURN_NATIVE }>;
  type DefaultDustThreshold =
    ConstU128<{ primitives::ecosystem::params::BURNING_MANAGER_DUST_THRESHOLD }>;
  type Precision = ConstU128<{ primitives::ecosystem::params::PRECISION }>;
  type DefaultSlippageTolerance = SlippageToleranceStub;
  type WeightInfo = ();
  type PriceTools = MockPriceTools;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct BurningManagerBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl crate::BenchmarkHelper<primitives::AssetKind, u64, u128> for BurningManagerBenchmarkHelper {
  fn ensure_funded(
    who: &u64,
    asset: primitives::AssetKind,
    amount: u128,
  ) -> polkadot_sdk::sp_runtime::DispatchResult {
    use polkadot_sdk::frame_support::traits::fungibles::Mutate;
    use polkadot_sdk::frame_support::traits::Currency;
    match asset {
      primitives::AssetKind::Native => {
        let _ = Balances::deposit_creating(who, amount);
      }
      primitives::AssetKind::Local(id) | primitives::AssetKind::Foreign(id) => {
        let _ = Assets::force_create(frame_system::RawOrigin::Root.into(), id, 1, true, 1);
        Assets::mint_into(id, who, amount)?;
      }
    }
    Ok(())
  }

  fn create_asset(asset: primitives::AssetKind) -> polkadot_sdk::sp_runtime::DispatchResult {
    if let primitives::AssetKind::Local(id) = asset {
      let _ = Assets::force_create(frame_system::RawOrigin::Root.into(), id, 1, true, 1);
    }
    Ok(())
  }

  fn create_pool(
    asset1: primitives::AssetKind,
    asset2: primitives::AssetKind,
  ) -> polkadot_sdk::sp_runtime::DispatchResult {
    let key = if asset1 < asset2 {
      (asset1, asset2)
    } else {
      (asset2, asset1)
    };
    POOLS.with(|p| p.borrow_mut().insert(key, (0, 0)));
    Ok(())
  }

  fn add_liquidity(
    _who: &u64,
    asset1: primitives::AssetKind,
    asset2: primitives::AssetKind,
    amount1: u128,
    amount2: u128,
  ) -> polkadot_sdk::sp_runtime::DispatchResult {
    let key = if asset1 < asset2 {
      (asset1, asset2)
    } else {
      (asset2, asset1)
    };

    POOLS.with(|p| {
      let mut pools = p.borrow_mut();
      let (r1, r2) = pools.get(&key).cloned().unwrap_or((0, 0));

      let (new_r1, new_r2) = if key.0 == asset1 {
        (r1 + amount1, r2 + amount2)
      } else {
        (r1 + amount2, r2 + amount1)
      };

      pools.insert(key, (new_r1, new_r2));
    });
    Ok(())
  }
}

pub fn new_test_ext() -> polkadot_sdk::sp_io::TestExternalities {
  let mut t = frame_system::GenesisConfig::<Test>::default()
    .build_storage()
    .unwrap();

  polkadot_sdk::pallet_assets::GenesisConfig::<Test> {
    assets: alloc::vec![(1, 1, true, 1)], // Asset 1, owner 1, Sufficient, min_bal 1
    metadata: alloc::vec![],
    accounts: alloc::vec![],
    reserves: alloc::vec![],
    next_asset_id: None,
  }
  .assimilate_storage(&mut t)
  .unwrap();

  // Reset State
  POOLS.with(|p| p.borrow_mut().clear());
  ORACLE_PRICES.with(|p| p.borrow_mut().clear());

  t.into()
}
