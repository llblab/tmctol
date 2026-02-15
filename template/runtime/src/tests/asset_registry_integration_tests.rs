use crate::tests::common::{ALICE, new_test_ext, saturate_active_tol_domains};
use crate::{AssetRegistry, Assets, RuntimeOrigin, TreasuryOwnedLiquidity};
use polkadot_sdk::frame_support::traits::fungibles::Inspect;
use polkadot_sdk::frame_support::{assert_noop, assert_ok};
use polkadot_sdk::pallet_assets;
use polkadot_sdk::staging_xcm as xcm;
use primitives::AssetKind;
use primitives::assets::{CurrencyMetadata, TYPE_FOREIGN, TYPE_STD};
use std::sync::Arc;
use xcm::latest::{Junction::Parachain, Junctions, Location};

fn sample_location(para_id: u32) -> Location {
  Location::new(1, Junctions::X1(Arc::new([Parachain(para_id)])))
}

fn sample_metadata() -> CurrencyMetadata {
  CurrencyMetadata {
    name: b"Foreign Token".to_vec(),
    symbol: b"FRGN".to_vec(),
    decimals: 12,
  }
}

#[test]
fn register_foreign_asset_creates_mapping_and_metadata() {
  new_test_ext().execute_with(|| {
    let location = sample_location(1000);
    let metadata = sample_metadata();

    assert_ok!(AssetRegistry::register_foreign_asset(
      RuntimeOrigin::root(),
      location.clone(),
      metadata.clone(),
      1,
      true
    ));

    let stored_id = AssetRegistry::location_to_asset(location.clone()).expect("id stored");
    assert!(Assets::asset_exists(stored_id));
    let stored_metadata = pallet_assets::Metadata::<crate::Runtime>::get(stored_id);
    assert_eq!(stored_metadata.name, metadata.name);
    assert_eq!(stored_metadata.symbol, metadata.symbol);
    assert_eq!(stored_metadata.decimals, metadata.decimals);
  });
}

#[test]
fn register_foreign_asset_auto_bootstraps_tol_domain_binding() {
  new_test_ext().execute_with(|| {
    let location = sample_location(1010);
    let metadata = sample_metadata();

    assert_ok!(AssetRegistry::register_foreign_asset(
      RuntimeOrigin::root(),
      location.clone(),
      metadata,
      1,
      true
    ));

    let asset_id = AssetRegistry::location_to_asset(location).expect("id stored");
    let token_asset = AssetKind::Foreign(asset_id);

    assert_eq!(
      TreasuryOwnedLiquidity::token_tol_binding(token_asset),
      Some(asset_id)
    );
    assert!(TreasuryOwnedLiquidity::tol_configuration(asset_id).is_some());
  });
}

#[test]
fn register_foreign_asset_fails_fast_when_tol_domain_capacity_is_reached() {
  new_test_ext().execute_with(|| {
    saturate_active_tol_domains(50_000);

    let location = sample_location(2026);
    let res = AssetRegistry::register_foreign_asset(
      RuntimeOrigin::root(),
      location.clone(),
      sample_metadata(),
      1,
      true,
    );

    assert_noop!(
      res,
      pallet_treasury_owned_liquidity::Error::<crate::Runtime>::TooManyTolDomains
    );
    assert_eq!(AssetRegistry::location_to_asset(location), None);
  });
}

#[test]
fn register_foreign_asset_with_id_respects_mask_and_collision() {
  new_test_ext().execute_with(|| {
    let location = sample_location(2000);
    let asset_id = TYPE_FOREIGN | 123;
    let metadata = sample_metadata();

    assert_ok!(AssetRegistry::register_foreign_asset_with_id(
      RuntimeOrigin::root(),
      location.clone(),
      asset_id,
      metadata,
      1,
      true
    ));
    assert_eq!(AssetRegistry::location_to_asset(location), Some(asset_id));
    assert!(Assets::asset_exists(asset_id));
  });

  new_test_ext().execute_with(|| {
    // Pre-occupy the ID
    let asset_id = TYPE_FOREIGN | 123;
    assert_ok!(Assets::force_create(
      RuntimeOrigin::root(),
      asset_id,
      ALICE.into(),
      true,
      1
    ));

    let res = AssetRegistry::register_foreign_asset_with_id(
      RuntimeOrigin::root(),
      sample_location(2001),
      asset_id,
      sample_metadata(),
      1,
      true,
    );
    assert_noop!(
      res,
      pallet_asset_registry::Error::<crate::Runtime>::AssetIdCollision
    );
  });
}

#[test]
fn register_foreign_asset_with_id_rejects_bad_mask() {
  new_test_ext().execute_with(|| {
    let bad_id = TYPE_STD | 7;
    let res = AssetRegistry::register_foreign_asset_with_id(
      RuntimeOrigin::root(),
      sample_location(3000),
      bad_id,
      sample_metadata(),
      1,
      true,
    );
    assert_noop!(
      res,
      pallet_asset_registry::Error::<crate::Runtime>::InvalidAssetIdMask
    );
  });
}

#[test]
fn link_existing_asset_works() {
  new_test_ext().execute_with(|| {
    let location = sample_location(4000);
    let asset_id = TYPE_FOREIGN | 42;

    // Pre-create the asset
    assert_ok!(Assets::force_create(
      RuntimeOrigin::root(),
      asset_id,
      ALICE.into(),
      true,
      1
    ));

    assert_ok!(AssetRegistry::link_existing_asset(
      RuntimeOrigin::root(),
      location.clone(),
      asset_id
    ));

    assert_eq!(
      AssetRegistry::location_to_asset(location.clone()),
      Some(asset_id)
    );
    assert!(Assets::asset_exists(asset_id));
    // Second link should fail
    let res = AssetRegistry::link_existing_asset(RuntimeOrigin::root(), location, asset_id);
    assert_noop!(
      res,
      pallet_asset_registry::Error::<crate::Runtime>::AssetAlreadyRegistered
    );
  });
}

#[test]
fn repeated_link_existing_asset_keeps_domain_state_idempotent() {
  new_test_ext().execute_with(|| {
    let asset_id = TYPE_FOREIGN | 77;
    let token_asset = AssetKind::Foreign(asset_id);

    assert_ok!(Assets::force_create(
      RuntimeOrigin::root(),
      asset_id,
      ALICE.into(),
      true,
      1
    ));

    assert_ok!(AssetRegistry::link_existing_asset(
      RuntimeOrigin::root(),
      sample_location(4010),
      asset_id
    ));
    let domains_after_first = TreasuryOwnedLiquidity::active_tol_domains();
    assert_eq!(domains_after_first.len(), 1);
    assert!(domains_after_first.contains(&asset_id));

    assert_ok!(AssetRegistry::link_existing_asset(
      RuntimeOrigin::root(),
      sample_location(4011),
      asset_id
    ));
    let domains_after_second = TreasuryOwnedLiquidity::active_tol_domains();
    assert_eq!(domains_after_second.len(), 1);
    assert!(domains_after_second.contains(&asset_id));
    assert_eq!(
      TreasuryOwnedLiquidity::token_tol_binding(token_asset),
      Some(asset_id)
    );
  });
}

#[test]
fn link_existing_asset_rejects_unknown_asset() {
  new_test_ext().execute_with(|| {
    let res = AssetRegistry::link_existing_asset(
      RuntimeOrigin::root(),
      sample_location(5000),
      TYPE_FOREIGN | 99,
    );
    assert_noop!(
      res,
      pallet_asset_registry::Error::<crate::Runtime>::AssetNotFound
    );
  });
}

#[test]
fn migrate_location_key_moves_mapping() {
  new_test_ext().execute_with(|| {
    let old_location = sample_location(6000);
    let new_location = sample_location(6001);
    let metadata = sample_metadata();

    assert_ok!(AssetRegistry::register_foreign_asset(
      RuntimeOrigin::root(),
      old_location.clone(),
      metadata,
      1,
      true
    ));

    let id = AssetRegistry::location_to_asset(old_location.clone()).unwrap();

    assert_ok!(AssetRegistry::migrate_location_key(
      RuntimeOrigin::root(),
      old_location.clone(),
      new_location.clone()
    ));

    assert_eq!(AssetRegistry::location_to_asset(old_location), None);
    assert_eq!(AssetRegistry::location_to_asset(new_location), Some(id));
  });
}

#[test]
fn migrate_location_key_rejects_duplicate_target() {
  new_test_ext().execute_with(|| {
    let old_location = sample_location(7000);
    let new_location = sample_location(7001);

    // First register at new_location
    assert_ok!(AssetRegistry::register_foreign_asset(
      RuntimeOrigin::root(),
      new_location.clone(),
      sample_metadata(),
      1,
      true
    ));

    // Register another at old_location
    assert_ok!(AssetRegistry::register_foreign_asset(
      RuntimeOrigin::root(),
      old_location.clone(),
      sample_metadata(),
      1,
      true
    ));

    let res =
      AssetRegistry::migrate_location_key(RuntimeOrigin::root(), old_location, new_location);
    assert_noop!(
      res,
      pallet_asset_registry::Error::<crate::Runtime>::AssetAlreadyRegistered
    );
  });
}

#[test]
fn convert_unknown_location_returns_none() {
  new_test_ext().execute_with(|| {
    let missing = sample_location(9999);
    assert_eq!(AssetRegistry::location_to_asset(missing), None);
  });
}
