use crate as pallet_aaa;
use frame::prelude::*;
use polkadot_sdk::{
  frame_support::{
    PalletId, construct_runtime,
    traits::{ConstU16, ConstU32, ConstU128, Get},
  },
  frame_system::EnsureRoot,
  sp_runtime::{
    BuildStorage,
    traits::{BlakeTwo256, IdentityLookup},
  },
};

use alloc::vec;
use core::cell::RefCell;

use crate::{AssetOps, DexOps};

type Block = polkadot_sdk::frame_system::mocking::MockBlock<Test>;
pub type AccountId = u64;
pub type Balance = u128;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;

#[derive(
  Clone,
  Copy,
  Debug,
  Default,
  Decode,
  DecodeWithMemTracking,
  Encode,
  Eq,
  Ord,
  PartialEq,
  PartialOrd,
  TypeInfo,
  MaxEncodedLen,
  serde::Serialize,
  serde::Deserialize,
)]
pub enum TestAsset {
  #[default]
  Native,
  Local(u32),
}

construct_runtime!(
  pub enum Test {
    System: polkadot_sdk::frame_system,
    Balances: polkadot_sdk::pallet_balances,
    AAA: pallet_aaa,
  }
);

impl polkadot_sdk::frame_system::Config for Test {
  type BaseCallFilter = polkadot_sdk::frame_support::traits::Everything;
  type BlockWeights = ();
  type BlockLength = ();
  type DbWeight = ();
  type RuntimeOrigin = RuntimeOrigin;
  type RuntimeCall = RuntimeCall;
  type Nonce = u64;
  type Hash = polkadot_sdk::sp_core::H256;
  type Hashing = BlakeTwo256;
  type AccountId = AccountId;
  type Lookup = IdentityLookup<Self::AccountId>;
  type Block = Block;
  type RuntimeEvent = RuntimeEvent;
  type BlockHashCount = polkadot_sdk::frame_support::traits::ConstU64<250>;
  type Version = ();
  type PalletInfo = PalletInfo;
  type AccountData = polkadot_sdk::pallet_balances::AccountData<Balance>;
  type OnNewAccount = ();
  type OnKilledAccount = ();
  type SystemWeightInfo = ();
  type SS58Prefix = ();
  type OnSetCode = ();
  type MaxConsumers = ConstU32<16>;
  type RuntimeTask = ();
  type ExtensionsWeightInfo = ();
  type SingleBlockMigrations = ();
  type MultiBlockMigrator = ();
  type PreInherents = ();
  type PostInherents = ();
  type PostTransactions = ();
}

impl polkadot_sdk::pallet_balances::Config for Test {
  type MaxLocks = ConstU32<50>;
  type MaxReserves = ();
  type ReserveIdentifier = [u8; 8];
  type Balance = Balance;
  type RuntimeEvent = RuntimeEvent;
  type DustRemoval = ();
  type ExistentialDeposit = ConstU128<1>;
  type AccountStore = System;
  type WeightInfo = ();
  type FreezeIdentifier = ();
  type MaxFreezes = ();
  type RuntimeHoldReason = RuntimeHoldReason;
  type RuntimeFreezeReason = RuntimeFreezeReason;
  type DoneSlashHandler = ();
}

pub struct AaaPalletId;
impl Get<PalletId> for AaaPalletId {
  fn get() -> PalletId {
    PalletId(*b"py/aaa00")
  }
}

pub struct NativeAsset;
impl Get<TestAsset> for NativeAsset {
  fn get() -> TestAsset {
    TestAsset::Native
  }
}

pub struct AaaBudget;
impl Get<frame::prelude::Permill> for AaaBudget {
  fn get() -> frame::prelude::Permill {
    frame::prelude::Permill::from_percent(30)
  }
}

thread_local! {
  static ASSET_BALANCES: RefCell<alloc::collections::BTreeMap<(AccountId, TestAsset), Balance>> =
    RefCell::new(alloc::collections::BTreeMap::new());

  static BURNED: RefCell<alloc::collections::BTreeMap<TestAsset, Balance>> =
    RefCell::new(alloc::collections::BTreeMap::new());

  static MINTED: RefCell<alloc::collections::BTreeMap<TestAsset, Balance>> =
    RefCell::new(alloc::collections::BTreeMap::new());

  static POOL_RESERVES: RefCell<alloc::collections::BTreeMap<(TestAsset, TestAsset), (Balance, Balance)>> =
    RefCell::new(alloc::collections::BTreeMap::new());
}

pub fn reset_mock_adapters() {
  ASSET_BALANCES.with(|b| b.borrow_mut().clear());
  BURNED.with(|b| b.borrow_mut().clear());
  MINTED.with(|b| b.borrow_mut().clear());
  POOL_RESERVES.with(|b| b.borrow_mut().clear());
}

pub fn set_asset_balance(who: AccountId, asset: TestAsset, amount: Balance) {
  ASSET_BALANCES.with(|b| {
    b.borrow_mut().insert((who, asset), amount);
  });
}

pub fn get_asset_balance(who: AccountId, asset: TestAsset) -> Balance {
  ASSET_BALANCES.with(|b| b.borrow().get(&(who, asset)).copied().unwrap_or(0))
}

pub fn get_total_burned(asset: TestAsset) -> Balance {
  BURNED.with(|b| b.borrow().get(&asset).copied().unwrap_or(0))
}

pub fn get_total_minted(asset: TestAsset) -> Balance {
  MINTED.with(|b| b.borrow().get(&asset).copied().unwrap_or(0))
}

pub fn set_pool_reserves(asset_a: TestAsset, asset_b: TestAsset, ra: Balance, rb: Balance) {
  let (key, val) = if asset_a <= asset_b {
    ((asset_a, asset_b), (ra, rb))
  } else {
    ((asset_b, asset_a), (rb, ra))
  };
  POOL_RESERVES.with(|p| {
    p.borrow_mut().insert(key, val);
  });
}

pub struct MockAssetOps;

impl AssetOps<AccountId, TestAsset, Balance> for MockAssetOps {
  fn transfer(
    from: &AccountId,
    to: &AccountId,
    asset: TestAsset,
    amount: Balance,
  ) -> Result<(), DispatchError> {
    match asset {
      TestAsset::Native => {
        use polkadot_sdk::frame_support::traits::Currency;
        <Balances as Currency<AccountId>>::transfer(
          from,
          to,
          amount,
          polkadot_sdk::frame_support::traits::ExistenceRequirement::AllowDeath,
        )
      }
      _ => ASSET_BALANCES.with(|b| {
        let mut map = b.borrow_mut();
        let src = map.get(&(*from, asset)).copied().unwrap_or(0);
        if src < amount {
          return Err(DispatchError::Token(
            polkadot_sdk::sp_runtime::TokenError::FundsUnavailable,
          ));
        }
        map.insert((*from, asset), src - amount);
        let dst = map.get(&(*to, asset)).copied().unwrap_or(0);
        map.insert((*to, asset), dst + amount);
        Ok(())
      }),
    }
  }

  fn burn(who: &AccountId, asset: TestAsset, amount: Balance) -> Result<(), DispatchError> {
    match asset {
      TestAsset::Native => {
        use polkadot_sdk::frame_support::traits::Currency;
        let (_, remainder) = <Balances as Currency<AccountId>>::slash(who, amount);
        if remainder > 0 {
          return Err(DispatchError::Token(
            polkadot_sdk::sp_runtime::TokenError::FundsUnavailable,
          ));
        }
        Ok(())
      }
      _ => ASSET_BALANCES.with(|b| {
        let mut map = b.borrow_mut();
        let bal = map.get(&(*who, asset)).copied().unwrap_or(0);
        if bal < amount {
          return Err(DispatchError::Token(
            polkadot_sdk::sp_runtime::TokenError::FundsUnavailable,
          ));
        }
        map.insert((*who, asset), bal - amount);
        BURNED.with(|br| {
          let mut bm = br.borrow_mut();
          let prev = bm.get(&asset).copied().unwrap_or(0);
          bm.insert(asset, prev + amount);
        });
        Ok(())
      }),
    }
  }

  fn mint(to: &AccountId, asset: TestAsset, amount: Balance) -> Result<(), DispatchError> {
    match asset {
      TestAsset::Native => {
        use polkadot_sdk::frame_support::traits::Currency;
        let _ = <Balances as Currency<AccountId>>::deposit_creating(to, amount);
        Ok(())
      }
      _ => ASSET_BALANCES.with(|b| {
        let mut map = b.borrow_mut();
        let bal = map.get(&(*to, asset)).copied().unwrap_or(0);
        map.insert((*to, asset), bal + amount);
        MINTED.with(|m| {
          let mut mm = m.borrow_mut();
          let prev = mm.get(&asset).copied().unwrap_or(0);
          mm.insert(asset, prev + amount);
        });
        Ok(())
      }),
    }
  }

  fn balance(who: &AccountId, asset: TestAsset) -> Balance {
    match asset {
      TestAsset::Native => {
        use polkadot_sdk::frame_support::traits::Currency;
        <Balances as Currency<AccountId>>::free_balance(who)
      }
      _ => ASSET_BALANCES.with(|b| b.borrow().get(&(*who, asset)).copied().unwrap_or(0)),
    }
  }
}

pub struct MockDexOps;

impl DexOps<AccountId, TestAsset, Balance> for MockDexOps {
  fn swap_exact_in(
    who: &AccountId,
    asset_in: TestAsset,
    asset_out: TestAsset,
    amount_in: Balance,
    min_out: Balance,
  ) -> Result<Balance, DispatchError> {
    let (ri, ro) = Self::get_reserves(asset_in, asset_out)?;
    let amount_out = amount_in.saturating_mul(ro) / (ri.saturating_add(amount_in));
    if amount_out < min_out {
      return Err(DispatchError::Other("SlippageExceeded"));
    }
    MockAssetOps::transfer(who, &u64::MAX, asset_in, amount_in)?;
    MockAssetOps::transfer(&u64::MAX, who, asset_out, amount_out)?;
    Ok(amount_out)
  }

  fn swap_exact_out(
    who: &AccountId,
    asset_in: TestAsset,
    asset_out: TestAsset,
    amount_out: Balance,
    max_in: Balance,
  ) -> Result<Balance, DispatchError> {
    let (ri, ro) = Self::get_reserves(asset_in, asset_out)?;
    if amount_out >= ro {
      return Err(DispatchError::Other("InsufficientLiquidity"));
    }
    let amount_in = ri
      .saturating_mul(amount_out)
      .checked_div(ro.saturating_sub(amount_out))
      .unwrap_or(Balance::MAX)
      .saturating_add(1);
    if amount_in > max_in {
      return Err(DispatchError::Other("SlippageExceeded"));
    }
    MockAssetOps::transfer(who, &u64::MAX, asset_in, amount_in)?;
    MockAssetOps::transfer(&u64::MAX, who, asset_out, amount_out)?;
    Ok(amount_in)
  }

  fn get_quote(asset_in: TestAsset, asset_out: TestAsset, amount_in: Balance) -> Option<Balance> {
    let (ri, ro) = Self::get_reserves(asset_in, asset_out).ok()?;
    Some(amount_in.saturating_mul(ro) / ri.saturating_add(amount_in))
  }

  fn add_liquidity(
    _who: &AccountId,
    _asset_a: TestAsset,
    _asset_b: TestAsset,
    amount_a: Balance,
    amount_b: Balance,
  ) -> Result<(Balance, Balance, Balance), DispatchError> {
    let lp_minted = integer_sqrt(amount_a.saturating_mul(amount_b));
    Ok((amount_a, amount_b, lp_minted))
  }

  fn remove_liquidity(
    _who: &AccountId,
    _lp_asset: TestAsset,
    lp_amount: Balance,
  ) -> Result<(Balance, Balance), DispatchError> {
    let half = lp_amount / 2;
    Ok((half, half))
  }

  fn get_pool_reserves(asset_a: TestAsset, asset_b: TestAsset) -> Option<(Balance, Balance)> {
    let key = if asset_a <= asset_b {
      (asset_a, asset_b)
    } else {
      (asset_b, asset_a)
    };
    POOL_RESERVES.with(|p| {
      let map = p.borrow();
      let (ra, rb) = map.get(&key).copied()?;
      if asset_a <= asset_b {
        Some((ra, rb))
      } else {
        Some((rb, ra))
      }
    })
  }
}

impl MockDexOps {
  fn get_reserves(
    asset_in: TestAsset,
    asset_out: TestAsset,
  ) -> Result<(Balance, Balance), DispatchError> {
    let key = if asset_in <= asset_out {
      (asset_in, asset_out)
    } else {
      (asset_out, asset_in)
    };
    POOL_RESERVES.with(|p| {
      let map = p.borrow();
      let (ra, rb) = map
        .get(&key)
        .copied()
        .ok_or(DispatchError::Other("NoPool"))?;
      if asset_in <= asset_out {
        Ok((ra, rb))
      } else {
        Ok((rb, ra))
      }
    })
  }
}

#[cfg(feature = "runtime-benchmarks")]
pub struct MockBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl crate::BenchmarkHelper<AccountId, TestAsset, Balance> for MockBenchmarkHelper {
  fn setup_remove_liquidity_max_k(
    owner: &AccountId,
    _max_scan: u32,
  ) -> Result<(TestAsset, Balance), DispatchError> {
    let lp_asset = TestAsset::Local(1);
    let lp_amount = 1_000_000u128;
    MockAssetOps::mint(owner, lp_asset, lp_amount)?;
    Ok((lp_asset, lp_amount))
  }
}

fn integer_sqrt(n: u128) -> u128 {
  if n == 0 {
    return 0;
  }
  let mut x = n;
  let mut y = x.div_ceil(2);
  while y < x {
    x = y;
    y = (x + n / x) / 2;
  }
  x
}

pub struct TestRentPerBlock;
impl Get<Balance> for TestRentPerBlock {
  fn get() -> Balance {
    1_000_000
  }
}

pub struct TestMaxRentAccrual;
impl Get<Balance> for TestMaxRentAccrual {
  fn get() -> Balance {
    1_000 * 1_000_000
  }
}

pub struct TestStepBaseFee;
impl Get<Balance> for TestStepBaseFee {
  fn get() -> Balance {
    0
  }
}

pub struct TestConditionReadFee;
impl Get<Balance> for TestConditionReadFee {
  fn get() -> Balance {
    0
  }
}

pub struct TestWeightToFee;
impl polkadot_sdk::sp_weights::WeightToFee for TestWeightToFee {
  type Balance = Balance;
  fn weight_to_fee(_weight: &polkadot_sdk::sp_weights::Weight) -> Self::Balance {
    0
  }
}

pub struct TestFeeSink;
impl Get<AccountId> for TestFeeSink {
  fn get() -> AccountId {
    999
  }
}

pub struct TestRefundTransferCost;
impl Get<Balance> for TestRefundTransferCost {
  fn get() -> Balance {
    101
  }
}

pub struct TestMaxConsecutiveFailures;
impl Get<u32> for TestMaxConsecutiveFailures {
  fn get() -> u32 {
    3
  }
}

pub struct TestMinUserBalance;
impl Get<Balance> for TestMinUserBalance {
  fn get() -> Balance {
    50
  }
}

pub struct TestMaxSweepPerBlock;
impl Get<u32> for TestMaxSweepPerBlock {
  fn get() -> u32 {
    3
  }
}

impl pallet_aaa::Config for Test {
  type AssetId = TestAsset;
  type Balance = Balance;
  type NativeAssetId = NativeAsset;
  type AssetOps = MockAssetOps;
  type DexOps = MockDexOps;
  type MinWindowLength = frame::traits::ConstU64<100>;
  type PalletId = AaaPalletId;
  type SystemOrigin = EnsureRoot<AccountId>;
  type GlobalBreakerOrigin = EnsureRoot<AccountId>;
  type MaxPipelineSteps = ConstU32<10>;
  type MaxUserPipelineSteps = ConstU32<3>;
  type MaxSystemPipelineSteps = ConstU32<10>;
  type MaxConditionsPerStep = ConstU32<4>;
  type MaxOwnedAaas = ConstU32<16>;
  type MaxOwnerSlots = ConstU16<64>;
  type MaxReadyRingLength = ConstU32<32>;
  type MaxDeferredRingLength = ConstU32<32>;
  type MaxDeferredRetriesPerBlock = ConstU32<8>;
  type MaxSystemExecutionsPerBlock = ConstU32<1>;
  type MaxUserExecutionsPerBlock = ConstU32<2>;
  type FairnessWeightSystem = ConstU32<1>;
  type FairnessWeightUser = ConstU32<2>;
  type MaxAddressEventInboxCount = ConstU32<8>;
  type MaxAdapterScan = ConstU32<64>;
  type AaaBudgetPct = AaaBudget;
  type RentPerBlock = TestRentPerBlock;
  type MaxRentAccrual = TestMaxRentAccrual;
  type StepBaseFee = TestStepBaseFee;
  type ConditionReadFee = TestConditionReadFee;
  type WeightToFee = TestWeightToFee;
  type TaskWeightInfo = ();
  #[cfg(feature = "runtime-benchmarks")]
  type BenchmarkHelper = MockBenchmarkHelper;
  type FeeSink = TestFeeSink;
  type MaxRefundableAssets = ConstU32<16>;
  type MaxConsecutiveFailures = TestMaxConsecutiveFailures;
  type MinUserBalance = TestMinUserBalance;
  type MaxSweepPerBlock = TestMaxSweepPerBlock;
  type RefundTransferCost = TestRefundTransferCost;
  type WeightInfo = ();
}

pub const TEST_INITIAL_BALANCE: Balance = 10_000_000_000_000;

pub fn new_test_ext() -> polkadot_sdk::sp_io::TestExternalities {
  let mut t = polkadot_sdk::frame_system::GenesisConfig::<Test>::default()
    .build_storage()
    .unwrap();

  polkadot_sdk::pallet_balances::GenesisConfig::<Test> {
    balances: vec![
      (ALICE, TEST_INITIAL_BALANCE),
      (BOB, TEST_INITIAL_BALANCE),
      (CHARLIE, TEST_INITIAL_BALANCE),
      (0, TEST_INITIAL_BALANCE),
      (255, TEST_INITIAL_BALANCE),
      (999, 1), // FeeSink ED
    ],
    dev_accounts: None,
  }
  .assimilate_storage(&mut t)
  .unwrap();

  let mut ext = polkadot_sdk::sp_io::TestExternalities::new(t);
  ext.execute_with(|| {
    reset_mock_adapters();
  });
  ext
}
