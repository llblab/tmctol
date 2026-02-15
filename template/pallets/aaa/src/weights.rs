#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use core::marker::PhantomData;
use polkadot_sdk::frame_support::{
  traits::Get,
  weights::{constants::RocksDbWeight, Weight},
};

pub trait WeightInfo {
  fn create_user_aaa() -> Weight;
  fn create_system_aaa() -> Weight;
  fn pause_aaa() -> Weight;
  fn resume_aaa() -> Weight;
  fn manual_trigger() -> Weight;
  fn fund_aaa() -> Weight;
  fn refund_and_close() -> Weight;
  fn update_policy() -> Weight;
  fn update_schedule() -> Weight;
  fn update_refund_assets() -> Weight;
  fn set_global_circuit_breaker() -> Weight;
  fn permissionless_sweep() -> Weight;
}

pub trait TaskWeightInfo {
  fn transfer() -> Weight;
  fn split_transfer(legs: u32) -> Weight;
  fn swap_exact_in() -> Weight;
  fn swap_exact_out() -> Weight;
  fn add_liquidity() -> Weight;
  fn remove_liquidity() -> Weight;
  fn burn() -> Weight;
  fn mint() -> Weight;
  fn noop() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: polkadot_sdk::frame_system::Config + crate::Config> WeightInfo for SubstrateWeight<T> {
  fn create_user_aaa() -> Weight {
    let slot_scan_reads = u64::from(T::MaxOwnerSlots::get());
    Weight::from_parts(25_000_000, 2000)
      .saturating_add(T::DbWeight::get().reads(slot_scan_reads.saturating_add(4)))
      .saturating_add(T::DbWeight::get().writes(6))
  }

  fn create_system_aaa() -> Weight {
    let slot_scan_reads = u64::from(T::MaxOwnerSlots::get());
    Weight::from_parts(25_000_000, 2000)
      .saturating_add(T::DbWeight::get().reads(slot_scan_reads.saturating_add(4)))
      .saturating_add(T::DbWeight::get().writes(6))
  }

  fn pause_aaa() -> Weight {
    Weight::from_parts(15_000_000, 1200)
      .saturating_add(T::DbWeight::get().reads(1))
      .saturating_add(T::DbWeight::get().writes(2))
  }

  fn resume_aaa() -> Weight {
    Weight::from_parts(15_000_000, 1200)
      .saturating_add(T::DbWeight::get().reads(1))
      .saturating_add(T::DbWeight::get().writes(2))
  }

  fn manual_trigger() -> Weight {
    Weight::from_parts(12_000_000, 1200)
      .saturating_add(T::DbWeight::get().reads(1))
      .saturating_add(T::DbWeight::get().writes(2))
  }

  fn fund_aaa() -> Weight {
    Weight::from_parts(20_000_000, 1800)
      .saturating_add(T::DbWeight::get().reads(2))
      .saturating_add(T::DbWeight::get().writes(2))
  }

  fn refund_and_close() -> Weight {
    Weight::from_parts(30_000_000, 2200)
      .saturating_add(T::DbWeight::get().reads(3))
      .saturating_add(T::DbWeight::get().writes(4))
  }

  fn update_policy() -> Weight {
    Weight::from_parts(12_000_000, 900)
      .saturating_add(T::DbWeight::get().reads(1))
      .saturating_add(T::DbWeight::get().writes(1))
  }

  fn update_schedule() -> Weight {
    Weight::from_parts(12_000_000, 900)
      .saturating_add(T::DbWeight::get().reads(1))
      .saturating_add(T::DbWeight::get().writes(1))
  }

  fn update_refund_assets() -> Weight {
    Weight::from_parts(15_000_000, 1200)
      .saturating_add(T::DbWeight::get().reads(1))
      .saturating_add(T::DbWeight::get().writes(1))
  }

  fn set_global_circuit_breaker() -> Weight {
    Weight::from_parts(8_000_000, 600)
      .saturating_add(T::DbWeight::get().writes(1))
  }

  fn permissionless_sweep() -> Weight {
    Weight::from_parts(18_000_000, 1200)
      .saturating_add(T::DbWeight::get().reads(2))
      .saturating_add(T::DbWeight::get().writes(1))
  }
}

pub struct SubstrateTaskWeightInfo<T>(PhantomData<T>);
impl<T: polkadot_sdk::frame_system::Config + crate::Config> TaskWeightInfo
  for SubstrateTaskWeightInfo<T>
{
  fn transfer() -> Weight {
    Weight::from_parts(10_000_000, 1000)
      .saturating_add(T::DbWeight::get().reads_writes(2, 2))
  }

  fn split_transfer(legs: u32) -> Weight {
    // +1 transfer path is reserved for explicit remainder_to target assignment
    let bounded_legs = u64::from(legs.min(17));
    Weight::from_parts(
      10_000_000u64.saturating_add(bounded_legs.saturating_mul(2_500_000)),
      1200u64.saturating_add(bounded_legs.saturating_mul(64)),
    )
    .saturating_add(T::DbWeight::get().reads_writes(2, bounded_legs.saturating_add(1)))
  }

  fn swap_exact_in() -> Weight {
    Weight::from_parts(60_000_000, 2200)
      .saturating_add(T::DbWeight::get().reads_writes(8, 8))
  }

  fn swap_exact_out() -> Weight {
    Weight::from_parts(60_000_000, 2200)
      .saturating_add(T::DbWeight::get().reads_writes(8, 8))
  }

  fn add_liquidity() -> Weight {
    Weight::from_parts(70_000_000, 2600)
      .saturating_add(T::DbWeight::get().reads_writes(10, 10))
  }

  fn remove_liquidity() -> Weight {
    let max_scan = u64::from(T::MaxAdapterScan::get());
    Weight::from_parts(
      70_000_000u64.saturating_add(max_scan.saturating_mul(2_000_000)),
      2600u64.saturating_add(max_scan.saturating_mul(64)),
    )
    .saturating_add(T::DbWeight::get().reads_writes(
      10u64.saturating_add(max_scan),
      10,
    ))
  }

  fn burn() -> Weight {
    Weight::from_parts(10_000_000, 1000)
      .saturating_add(T::DbWeight::get().reads_writes(1, 1))
  }

  fn mint() -> Weight {
    Weight::from_parts(10_000_000, 1000)
      .saturating_add(T::DbWeight::get().reads_writes(1, 1))
  }

  fn noop() -> Weight {
    Weight::from_parts(1_000_000, 0)
  }
}

impl WeightInfo for () {
  fn create_user_aaa() -> Weight { Weight::from_parts(25_000_000, 2000) }
  fn create_system_aaa() -> Weight { Weight::from_parts(25_000_000, 2000) }
  fn pause_aaa() -> Weight { Weight::from_parts(15_000_000, 1200) }
  fn resume_aaa() -> Weight { Weight::from_parts(15_000_000, 1200) }
  fn manual_trigger() -> Weight { Weight::from_parts(12_000_000, 1200) }
  fn fund_aaa() -> Weight { Weight::from_parts(20_000_000, 1800) }
  fn refund_and_close() -> Weight { Weight::from_parts(30_000_000, 2200) }
  fn update_policy() -> Weight { Weight::from_parts(12_000_000, 900) }
  fn update_schedule() -> Weight { Weight::from_parts(12_000_000, 900) }
  fn update_refund_assets() -> Weight { Weight::from_parts(15_000_000, 1200) }
  fn set_global_circuit_breaker() -> Weight { Weight::from_parts(8_000_000, 600) }
  fn permissionless_sweep() -> Weight { Weight::from_parts(18_000_000, 1200) }
}

impl TaskWeightInfo for () {
  fn transfer() -> Weight { Weight::from_parts(10_000_000, 1000) }
  fn split_transfer(legs: u32) -> Weight {
    let bounded_legs = u64::from(legs.min(17));
    Weight::from_parts(10_000_000u64.saturating_add(bounded_legs.saturating_mul(2_500_000)), 1200)
  }
  fn swap_exact_in() -> Weight { Weight::from_parts(60_000_000, 2200) }
  fn swap_exact_out() -> Weight { Weight::from_parts(60_000_000, 2200) }
  fn add_liquidity() -> Weight { Weight::from_parts(70_000_000, 2600) }
  fn remove_liquidity() -> Weight { Weight::from_parts(70_000_000, 2600) }
  fn burn() -> Weight { Weight::from_parts(10_000_000, 1000) }
  fn mint() -> Weight { Weight::from_parts(10_000_000, 1000) }
  fn noop() -> Weight { Weight::from_parts(1_000_000, 0) }
}
