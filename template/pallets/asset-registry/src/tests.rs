use crate::{Error, Event, mock::*};
use frame::deps::frame_support::{assert_noop, assert_ok};
use polkadot_sdk::staging_xcm::latest::{Junction::Parachain, Junctions, Location};
use polkadot_sdk::{frame_system, pallet_assets};
use primitives::assets::{CurrencyMetadata, TYPE_FOREIGN};
use std::sync::Arc;

#[test]
fn register_foreign_asset_works() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    // Location mapping to ID 1000 via MockLocationToAssetId
    let location = Location::new(1, Junctions::X1(Arc::new([Parachain(1000)])));
    let metadata = CurrencyMetadata {
      name: b"Sibling Token".to_vec(),
      symbol: b"SIBL".to_vec(),
      decimals: 12,
    };
    let min_balance = 10;
    let is_sufficient = true;

    // 1. Register
    assert_ok!(crate::Pallet::<Test>::register_foreign_asset(
      RuntimeOrigin::root(),
      location.clone(),
      metadata.clone(),
      min_balance,
      is_sufficient
    ));

    // 2. Verify Storage Persistence
    assert_eq!(
      crate::Pallet::<Test>::location_to_asset(&location),
      Some(1000)
    );

    // 3. Verify Event
    frame_system::Pallet::<Test>::assert_last_event(RuntimeEvent::AssetRegistry(
      Event::ForeignAssetRegistered {
        asset_id: 1000,
        location: location.clone(),
        symbol: metadata.symbol.clone(),
      },
    ));

    // 4. Verify Assets Pallet State
    // Check Metadata
    let stored_metadata = polkadot_sdk::pallet_assets::Metadata::<Test>::get(1000);
    assert_eq!(stored_metadata.name, metadata.name);
    assert_eq!(stored_metadata.symbol, metadata.symbol);
    assert_eq!(stored_metadata.decimals, metadata.decimals);
  });
}

#[test]
fn register_duplicate_fails() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let location = Location::new(1, Junctions::X1(Arc::new([Parachain(1000)])));
    let metadata = CurrencyMetadata {
      name: b"Sibling Token".to_vec(),
      symbol: b"SIBL".to_vec(),
      decimals: 12,
    };

    // First registration works
    assert_ok!(crate::Pallet::<Test>::register_foreign_asset(
      RuntimeOrigin::root(),
      location.clone(),
      metadata.clone(),
      10,
      true
    ));

    // Second registration fails
    assert_noop!(
      crate::Pallet::<Test>::register_foreign_asset(
        RuntimeOrigin::root(),
        location,
        metadata,
        10,
        true
      ),
      Error::<Test>::AssetAlreadyRegistered
    );
  });
}

#[test]
fn register_collision_fails() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let location = Location::new(1, Junctions::X1(Arc::new([Parachain(1000)])));
    let metadata = CurrencyMetadata {
      name: b"Sibling Token".to_vec(),
      symbol: b"SIBL".to_vec(),
      decimals: 12,
    };

    // Pre-occupy ID 1000 (Mock maps Parachain(1000) -> 1000)
    assert_ok!(pallet_assets::Pallet::<Test>::force_create(
      RuntimeOrigin::root(),
      1000,
      1,    // owner
      true, // is_sufficient
      10    // min_balance
    ));

    // Attempt to register foreign asset mapping to same ID
    assert_noop!(
      crate::Pallet::<Test>::register_foreign_asset(
        RuntimeOrigin::root(),
        location,
        metadata,
        10,
        true
      ),
      Error::<Test>::AssetIdCollision
    );
  });
}

#[test]
fn register_foreign_asset_fails_bad_origin() {
  new_test_ext().execute_with(|| {
    let location = Location::new(1, Junctions::X1(Arc::new([Parachain(1000)])));
    let metadata = CurrencyMetadata {
      name: b"Sibling Token".to_vec(),
      symbol: b"SIBL".to_vec(),
      decimals: 12,
    };

    // Attempt with signed origin (Mock requires Root)
    assert_noop!(
      crate::Pallet::<Test>::register_foreign_asset(
        RuntimeOrigin::signed(2),
        location,
        metadata,
        10,
        true
      ),
      polkadot_sdk::sp_runtime::DispatchError::BadOrigin
    );
  });
}

#[test]
fn link_existing_asset_emits_symbol() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let location = Location::new(1, Junctions::X1(Arc::new([Parachain(3001)])));
    let asset_id = TYPE_FOREIGN | 77;

    assert_ok!(pallet_assets::Pallet::<Test>::force_create(
      RuntimeOrigin::root(),
      asset_id,
      1,
      true,
      1
    ));
    assert_ok!(pallet_assets::Pallet::<Test>::force_set_metadata(
      RuntimeOrigin::root(),
      asset_id,
      b"Linked Token".to_vec(),
      b"LNK".to_vec(),
      12,
      false
    ));

    assert_ok!(crate::Pallet::<Test>::link_existing_asset(
      RuntimeOrigin::root(),
      location.clone(),
      asset_id
    ));

    frame_system::Pallet::<Test>::assert_last_event(RuntimeEvent::AssetRegistry(
      Event::ForeignAssetRegistered {
        asset_id,
        location,
        symbol: b"LNK".to_vec(),
      },
    ));
  });
}

#[test]
fn migrate_location_key_emits_event() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let old_location = Location::new(1, Junctions::X1(Arc::new([Parachain(8000)])));
    let new_location = Location::new(1, Junctions::X1(Arc::new([Parachain(8001)])));
    let metadata = CurrencyMetadata {
      name: b"Foreign Token".to_vec(),
      symbol: b"FRGN".to_vec(),
      decimals: 12,
    };

    assert_ok!(crate::Pallet::<Test>::register_foreign_asset(
      RuntimeOrigin::root(),
      old_location.clone(),
      metadata,
      10,
      true
    ));

    let asset_id = crate::Pallet::<Test>::location_to_asset(&old_location).unwrap();

    assert_ok!(crate::Pallet::<Test>::migrate_location_key(
      RuntimeOrigin::root(),
      old_location.clone(),
      new_location.clone()
    ));

    frame_system::Pallet::<Test>::assert_last_event(RuntimeEvent::AssetRegistry(
      Event::MigrationApplied {
        asset_id,
        old_location,
        new_location,
      },
    ));
  });
}
