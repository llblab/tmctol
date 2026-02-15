extern crate alloc;

use crate as pallet_zap_manager;
use polkadot_sdk::frame_support::traits::fungibles::{Inspect, Mutate};
use polkadot_sdk::frame_support::traits::tokens::{Fortitude, Precision, Preservation};
use polkadot_sdk::frame_support::{
  PalletId, construct_runtime, derive_impl,
  traits::{ConstU32, ConstU64, ConstU128, Get},
};
use polkadot_sdk::frame_system;
use polkadot_sdk::sp_runtime::{
  BuildStorage, DispatchError,
  testing::H256,
  traits::{BlakeTwo256, IdentityLookup, IntegerSquareRoot},
};
use primitives::AssetKind;
use std::cell::RefCell;
use std::collections::BTreeMap;

thread_local! {
    pub static POOLS: RefCell<BTreeMap<(AssetKind, AssetKind), (u128, u128)>> = const { RefCell::new(BTreeMap::new()) };
    pub static LP_TOKENS: RefCell<BTreeMap<(AssetKind, AssetKind), AssetKind>> = const { RefCell::new(BTreeMap::new()) };
    pub static NEXT_LP_ID: RefCell<u32> = const { RefCell::new(100) };
    pub static ORACLE_PRICES: RefCell<BTreeMap<(AssetKind, AssetKind), u128>> = const { RefCell::new(BTreeMap::new()) };
}

pub fn set_pool(asset_a: AssetKind, asset_b: AssetKind, reserve_a: u128, reserve_b: u128) {
  let key = if asset_a < asset_b {
    (asset_a, asset_b)
  } else {
    (asset_b, asset_a)
  };
  POOLS.with(|p| p.borrow_mut().insert(key, (reserve_a, reserve_b)));
}

pub fn get_pool_reserves(asset_a: AssetKind, asset_b: AssetKind) -> Option<(u128, u128)> {
  let key = if asset_a < asset_b {
    (asset_a, asset_b)
  } else {
    (asset_b, asset_a)
  };
  POOLS.with(|p| p.borrow().get(&key).cloned())
}

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
  pub struct Test {
    System: frame_system,
    Balances: polkadot_sdk::pallet_balances,
    Assets: polkadot_sdk::pallet_assets,
    ZapManager: pallet_zap_manager,
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
  type CallbackHandle = ();
  type WeightInfo = ();
  type RemoveItemsLimit = ConstU32<5>;
  type Holder = ();
  type ReserveData = ();
  #[cfg(feature = "runtime-benchmarks")]
  type BenchmarkHelper = ();
}

pub struct MockPriceOracle;
impl pallet_zap_manager::PriceOracle<u128> for MockPriceOracle {
  fn get_ema_price(asset_in: AssetKind, asset_out: AssetKind) -> Option<u128> {
    let (min, max) = if asset_in < asset_out {
      (asset_in, asset_out)
    } else {
      (asset_out, asset_in)
    };
    ORACLE_PRICES.with(|p| p.borrow().get(&(min, max)).cloned())
  }

  fn validate_price_deviation(
    asset_in: AssetKind,
    asset_out: AssetKind,
    current_price: u128,
  ) -> Result<(), DispatchError> {
    if let Some(oracle_price) = Self::get_ema_price(asset_in, asset_out) {
      let delta = current_price.abs_diff(oracle_price);
      let allowed_delta = oracle_price / 5;

      if delta > allowed_delta {
        return Err(DispatchError::Other("Price Deviation Exceeded"));
      }
    }
    Ok(())
  }
}

pub struct MockAssetConversion;
impl pallet_zap_manager::AssetConversionApi<u64, u128> for MockAssetConversion {
  fn get_pool_id(asset1: AssetKind, asset2: AssetKind) -> Option<AssetKind> {
    let key = if asset1 < asset2 {
      (asset1, asset2)
    } else {
      (asset2, asset1)
    };
    LP_TOKENS.with(|lp| lp.borrow().get(&key).cloned())
  }

  fn get_pool_reserves(pool_id: AssetKind) -> Option<(u128, u128)> {
    LP_TOKENS.with(|lp| {
      let lp_map = lp.borrow();
      for ((a1, a2), lp_token) in lp_map.iter() {
        if *lp_token == pool_id {
          return POOLS.with(|p| p.borrow().get(&(*a1, *a2)).cloned());
        }
      }
      None
    })
  }

  fn create_pool(asset1: AssetKind, asset2: AssetKind) -> Result<AssetKind, DispatchError> {
    let key = if asset1 < asset2 {
      (asset1, asset2)
    } else {
      (asset2, asset1)
    };

    if LP_TOKENS.with(|lp| lp.borrow().contains_key(&key)) {
      return Err(DispatchError::Other("Pool already exists"));
    }

    let lp_id = NEXT_LP_ID.with(|n| {
      let mut id = n.borrow_mut();
      let current = *id;
      *id += 1;
      current
    });
    let lp_asset = AssetKind::Local(lp_id);

    LP_TOKENS.with(|lp| lp.borrow_mut().insert(key, lp_asset));
    POOLS.with(|p| p.borrow_mut().insert(key, (0, 0)));

    if !Assets::asset_exists(lp_id) {
      let _ = Assets::force_create(frame_system::RawOrigin::Root.into(), lp_id, 1, true, 1);
    }

    Ok(lp_asset)
  }

  fn add_liquidity(
    who: &u64,
    asset1: AssetKind,
    asset2: AssetKind,
    amount1_desired: u128,
    amount2_desired: u128,
    _amount1_min: u128,
    _amount2_min: u128,
  ) -> Result<(u128, u128, u128), DispatchError> {
    let key = if asset1 < asset2 {
      (asset1, asset2)
    } else {
      (asset2, asset1)
    };

    let (reserve1, reserve2) = POOLS
      .with(|p| p.borrow().get(&key).cloned())
      .ok_or(DispatchError::Other("Pool not found"))?;

    let lp_token = LP_TOKENS
      .with(|lp| lp.borrow().get(&key).cloned())
      .ok_or(DispatchError::Other("LP token not found"))?;

    let (amount1, amount2, shares) = if reserve1 == 0 && reserve2 == 0 {
      let shares = (amount1_desired * amount2_desired).integer_sqrt();
      (amount1_desired, amount2_desired, shares)
    } else {
      let amount2_optimal = (amount1_desired * reserve2) / reserve1;
      if amount2_optimal <= amount2_desired {
        let shares = (amount1_desired * 1_000_000_000) / reserve1;
        (amount1_desired, amount2_optimal, shares)
      } else {
        let amount1_optimal = (amount2_desired * reserve1) / reserve2;
        let shares = (amount2_desired * 1_000_000_000) / reserve2;
        (amount1_optimal, amount2_desired, shares)
      }
    };

    POOLS.with(|p| {
      p.borrow_mut()
        .insert(key, (reserve1 + amount1, reserve2 + amount2))
    });

    match asset1 {
      AssetKind::Native => {
        let _ = <Balances as polkadot_sdk::frame_support::traits::Currency<u64>>::withdraw(
          who,
          amount1,
          polkadot_sdk::frame_support::traits::WithdrawReasons::TRANSFER,
          polkadot_sdk::frame_support::traits::ExistenceRequirement::KeepAlive,
        )?;
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <Assets as Mutate<u64>>::burn_from(
          id,
          who,
          amount1,
          Preservation::Expendable,
          Precision::Exact,
          Fortitude::Polite,
        )?;
      }
    }

    match asset2 {
      AssetKind::Native => {
        let _ = <Balances as polkadot_sdk::frame_support::traits::Currency<u64>>::withdraw(
          who,
          amount2,
          polkadot_sdk::frame_support::traits::WithdrawReasons::TRANSFER,
          polkadot_sdk::frame_support::traits::ExistenceRequirement::KeepAlive,
        )?;
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <Assets as Mutate<u64>>::burn_from(
          id,
          who,
          amount2,
          Preservation::Expendable,
          Precision::Exact,
          Fortitude::Polite,
        )?;
      }
    }

    if let AssetKind::Local(id) | AssetKind::Foreign(id) = lp_token {
      <Assets as Mutate<u64>>::mint_into(id, who, shares)?;
    }

    Ok((amount1, amount2, shares))
  }

  fn swap_exact_tokens_for_tokens(
    who: &u64,
    asset_in: AssetKind,
    asset_out: AssetKind,
    amount_in: u128,
    _amount_out_min: u128,
  ) -> Result<u128, DispatchError> {
    let key = if asset_in < asset_out {
      (asset_in, asset_out)
    } else {
      (asset_out, asset_in)
    };

    let (mut reserve_in, mut reserve_out) = POOLS
      .with(|p| p.borrow().get(&key).cloned())
      .ok_or(DispatchError::Other("Pool not found"))?;

    if key.0 != asset_in {
      core::mem::swap(&mut reserve_in, &mut reserve_out);
    }

    // XYK: amount_out = (amount_in * reserve_out) / (reserve_in + amount_in)
    let amount_out = amount_in
      .checked_mul(reserve_out)
      .and_then(|v| v.checked_div(reserve_in.saturating_add(amount_in)))
      .ok_or(DispatchError::Arithmetic(
        polkadot_sdk::sp_runtime::ArithmeticError::Overflow,
      ))?;

    if amount_out == 0 {
      return Err(DispatchError::Other("Insufficient output amount"));
    }

    // Burn input tokens from who
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

    // Mint output tokens to who
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

    // Update reserves
    let new_reserve_in = reserve_in.saturating_add(amount_in);
    let new_reserve_out = reserve_out.saturating_sub(amount_out);

    let (final_r1, final_r2) = if key.0 == asset_in {
      (new_reserve_in, new_reserve_out)
    } else {
      (new_reserve_out, new_reserve_in)
    };

    POOLS.with(|p| p.borrow_mut().insert(key, (final_r1, final_r2)));

    Ok(amount_out)
  }
}

pub struct PalletIdStub;
impl Get<PalletId> for PalletIdStub {
  fn get() -> PalletId {
    PalletId(*b"py/zapmg")
  }
}

pub struct MockTolAccountResolver;
impl pallet_zap_manager::TolAccountResolver<u64> for MockTolAccountResolver {
  fn resolve_tol_account(_token_asset: AssetKind) -> u64 {
    999
  }
}

impl pallet_zap_manager::Config for Test {
  type Assets = Assets;
  type Currency = Balances;
  type AssetConversion = MockAssetConversion;
  type PriceOracle = MockPriceOracle;
  type PalletId = PalletIdStub;
  type TolAccountResolver = MockTolAccountResolver;
  type MinSwapForeign = ConstU128<{ primitives::ecosystem::params::ZAP_MANAGER_MIN_SWAP_FOREIGN }>;
  type DustThreshold = ConstU128<{ primitives::ecosystem::params::ZAP_MANAGER_DUST_THRESHOLD }>;
  type RetryCooldown =
    ConstU64<{ primitives::ecosystem::params::ZAP_MANAGER_RETRY_COOLDOWN as u64 }>;
  type WeightInfo = ();
  type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
}

pub fn new_test_ext() -> polkadot_sdk::sp_io::TestExternalities {
  let mut t = frame_system::GenesisConfig::<Test>::default()
    .build_storage()
    .unwrap();

  polkadot_sdk::pallet_assets::GenesisConfig::<Test> {
    assets: alloc::vec![(1, 1, true, 1)],
    metadata: alloc::vec![],
    accounts: alloc::vec![],
    reserves: alloc::vec![],
    next_asset_id: None,
  }
  .assimilate_storage(&mut t)
  .unwrap();

  // ZM genesis: pallet account gets provider ref (ED-free)
  pallet_zap_manager::GenesisConfig::<Test>::default()
    .assimilate_storage(&mut t)
    .unwrap();

  POOLS.with(|p| p.borrow_mut().clear());
  LP_TOKENS.with(|lp| lp.borrow_mut().clear());
  NEXT_LP_ID.with(|n| *n.borrow_mut() = 100);
  ORACLE_PRICES.with(|p| p.borrow_mut().clear());

  t.into()
}
