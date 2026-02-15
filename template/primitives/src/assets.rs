use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

/// This enum serves as the single source of truth for asset types across all pallets,
/// enabling type-safe interactions between Axial Router, TMC, Burning Manager, and other actors.
///
/// - `Native`: The system's native token (managed by pallet-balances).
/// - `Local(u32)`: Local synthetic assets (managed by pallet-assets).
#[derive(
  Clone,
  Copy,
  Debug,
  Decode,
  DecodeWithMemTracking,
  Default,
  Encode,
  Eq,
  MaxEncodedLen,
  Ord,
  PartialEq,
  PartialOrd,
  TypeInfo,
  Serialize,
  Deserialize,
)]
pub enum AssetKind {
  /// Native token managed by pallet-balances
  #[default]
  Native,
  /// Local asset managed by pallet-assets
  Local(u32),
  /// Foreign asset managed by pallet-assets via XCM mapping (0xF... namespace)
  Foreign(u32),
}

impl From<u32> for AssetKind {
  fn from(asset_id: u32) -> Self {
    AssetKind::Local(asset_id)
  }
}

// Bitmask Architecture for Asset Classification
//
// 32-bit ID Structure:
// [ 4 bits: Type ] [ 28 bits: Index/ID ]
//
// Types:
// 0x0... -> Native (Reserved, though AssetKind::Native is usually used)
// 0x1... -> Standard Tokens (DOT, KSM, etc.)
// 0x2... -> Stablecoins (USDT, USDC, etc.)
// 0x3... -> Liquid Staking Tokens (vDOT, vKSM, etc.)
// 0x4... -> LP Tokens
// 0xF... -> Foreign/XCM Assets

pub const MASK_TYPE: u32 = 0xF000_0000;
pub const MASK_INDEX: u32 = 0x0FFF_FFFF;

pub const TYPE_STD: u32 = 0x1000_0000;
pub const TYPE_STABLE: u32 = 0x2000_0000;
pub const TYPE_VTOKEN: u32 = 0x3000_0000;
pub const TYPE_LP: u32 = 0x4000_0000;
pub const TYPE_FOREIGN: u32 = 0xF000_0000;

/// Helper trait to inspect AssetKind properties
pub trait AssetInspector {
  fn is_native(&self) -> bool;
  fn local_id(&self) -> Option<u32>;

  // Bitmask checks
  fn is_std(&self) -> bool;
  fn is_stable(&self) -> bool;
  fn is_vtoken(&self) -> bool;
  fn is_lp(&self) -> bool;
  fn is_foreign(&self) -> bool;
}

impl AssetInspector for AssetKind {
  fn is_native(&self) -> bool {
    matches!(self, AssetKind::Native)
  }

  fn local_id(&self) -> Option<u32> {
    match self {
      AssetKind::Local(id) | AssetKind::Foreign(id) => Some(*id),
      _ => None,
    }
  }

  fn is_std(&self) -> bool {
    match self {
      AssetKind::Local(id) => (id & MASK_TYPE) == TYPE_STD,
      _ => false,
    }
  }

  fn is_stable(&self) -> bool {
    match self {
      AssetKind::Local(id) => (id & MASK_TYPE) == TYPE_STABLE,
      _ => false,
    }
  }

  fn is_vtoken(&self) -> bool {
    match self {
      AssetKind::Local(id) => (id & MASK_TYPE) == TYPE_VTOKEN,
      _ => false,
    }
  }

  fn is_lp(&self) -> bool {
    match self {
      AssetKind::Local(id) => (id & MASK_TYPE) == TYPE_LP,
      _ => false,
    }
  }

  fn is_foreign(&self) -> bool {
    match self {
      AssetKind::Foreign(_) => true,
      AssetKind::Local(id) => (id & MASK_TYPE) == TYPE_FOREIGN,
      _ => false,
    }
  }
}

/// Trait for type conversions with additional context
pub trait TryConvertFrom<T, Context> {
  type Error;
  fn try_convert_from(value: T, context: Context) -> Result<Self, Self::Error>
  where
    Self: Sized;
}

/// Helper to construct compile-time IDs
const fn make_id(type_mask: u32, index: u32) -> u32 {
  type_mask | (index & MASK_INDEX)
}

/// Well-known asset constants serving as system defaults
pub mod well_known {
  use super::*;

  // Standard Tokens (0x1...)
  pub const DOT: u32 = make_id(TYPE_STD, 1);
  pub const KSM: u32 = make_id(TYPE_STD, 2);
  pub const ETH: u32 = make_id(TYPE_STD, 3);
  pub const BTC: u32 = make_id(TYPE_STD, 4);

  // Stablecoins (0x2...)
  pub const USDT: u32 = make_id(TYPE_STABLE, 1);
  pub const USDC: u32 = make_id(TYPE_STABLE, 2);
  pub const DAI: u32 = make_id(TYPE_STABLE, 3);

  // Liquid Staking Tokens (0x3...)
  pub const VDOT: u32 = make_id(TYPE_VTOKEN, 1); // vDOT
  pub const VKSM: u32 = make_id(TYPE_VTOKEN, 2); // vKSM
}

/// Metadata container for currencies
#[derive(Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Clone, Debug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct CurrencyMetadata {
  pub name: Vec<u8>,
  pub symbol: Vec<u8>,
  pub decimals: u8,
}

/// Helper to resolve metadata for well-known assets (off-chain / view logic)
pub fn get_well_known_metadata(asset: AssetKind) -> Option<CurrencyMetadata> {
  match asset {
    AssetKind::Native => Some(CurrencyMetadata {
      name: b"Native Token".to_vec(),
      symbol: b"NATIVE".to_vec(),
      decimals: 12,
    }),
    AssetKind::Local(id) => match id {
      well_known::DOT => Some(CurrencyMetadata {
        name: b"Polkadot".to_vec(),
        symbol: b"DOT".to_vec(),
        decimals: 10,
      }),
      well_known::KSM => Some(CurrencyMetadata {
        name: b"Kusama".to_vec(),
        symbol: b"KSM".to_vec(),
        decimals: 12,
      }),
      well_known::ETH => Some(CurrencyMetadata {
        name: b"Ethereum".to_vec(),
        symbol: b"ETH".to_vec(),
        decimals: 18,
      }),
      well_known::BTC => Some(CurrencyMetadata {
        name: b"Bitcoin".to_vec(),
        symbol: b"BTC".to_vec(),
        decimals: 8,
      }),

      well_known::USDT => Some(CurrencyMetadata {
        name: b"Tether USD".to_vec(),
        symbol: b"USDT".to_vec(),
        decimals: 6,
      }),
      well_known::USDC => Some(CurrencyMetadata {
        name: b"USD Coin".to_vec(),
        symbol: b"USDC".to_vec(),
        decimals: 6,
      }),
      well_known::DAI => Some(CurrencyMetadata {
        name: b"Dai Stablecoin".to_vec(),
        symbol: b"DAI".to_vec(),
        decimals: 18,
      }),

      well_known::VDOT => Some(CurrencyMetadata {
        name: b"Liquid DOT".to_vec(),
        symbol: b"vDOT".to_vec(),
        decimals: 10,
      }),
      well_known::VKSM => Some(CurrencyMetadata {
        name: b"Liquid KSM".to_vec(),
        symbol: b"vKSM".to_vec(),
        decimals: 12,
      }),

      _ => None,
    },
    AssetKind::Foreign(_) => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_well_known_ids() {
    assert_eq!(well_known::DOT & MASK_TYPE, TYPE_STD);
    assert_eq!(well_known::USDT & MASK_TYPE, TYPE_STABLE);
    assert_eq!(well_known::VDOT & MASK_TYPE, TYPE_VTOKEN);
  }

  #[test]
  fn test_asset_inspection() {
    let dot = AssetKind::Local(well_known::DOT);
    assert!(dot.is_std());
    assert!(!dot.is_stable());

    let usdt = AssetKind::Local(well_known::USDT);
    assert!(usdt.is_stable());
    assert!(!usdt.is_std());

    let native = AssetKind::Native;
    assert!(native.is_native());
    assert!(!native.is_stable());
  }

  #[test]
  fn test_metadata() {
    let meta = get_well_known_metadata(AssetKind::Native).unwrap();
    assert_eq!(meta.symbol, b"NATIVE".to_vec());

    let meta = get_well_known_metadata(AssetKind::Local(well_known::USDT)).unwrap();
    assert_eq!(meta.symbol, b"USDT".to_vec());
  }

  #[test]
  fn test_bitmask_boundaries() {
    // Test boundary between Standard (0x1...) and Stable (0x2...)
    let max_std = AssetKind::Local(TYPE_STD | MASK_INDEX);
    let min_stable = AssetKind::Local(TYPE_STABLE);

    assert!(max_std.is_std());
    assert!(!max_std.is_stable());

    assert!(min_stable.is_stable());
    assert!(!min_stable.is_std());

    // Test boundary between Stable (0x2...) and VToken (0x3...)
    let max_stable = AssetKind::Local(TYPE_STABLE | MASK_INDEX);
    let min_vtoken = AssetKind::Local(TYPE_VTOKEN);

    assert!(max_stable.is_stable());
    assert!(!max_stable.is_vtoken());

    assert!(min_vtoken.is_vtoken());
    assert!(!min_vtoken.is_stable());
  }

  #[test]
  fn test_lp_namespace_isolation() {
    // LP Tokens use 0x4... prefix
    let lp_token = AssetKind::Local(TYPE_LP | 12345);

    assert!(lp_token.is_lp());
    assert!(!lp_token.is_std());
    assert!(!lp_token.is_stable());
    assert!(!lp_token.is_vtoken());
    assert!(!lp_token.is_foreign());

    // Try to spoof LP token with other prefix
    let spoofed_lp = AssetKind::Local(TYPE_STD | 12345);
    assert!(!spoofed_lp.is_lp());
  }

  #[test]
  fn test_foreign_asset_isolation() {
    // Foreign assets use 0xF... prefix
    let foreign_asset = AssetKind::Foreign(TYPE_FOREIGN | 12345);

    assert!(foreign_asset.is_foreign());
    assert!(!foreign_asset.is_native());
    assert!(!foreign_asset.is_std());
    assert!(!foreign_asset.is_stable());
    assert_eq!(foreign_asset.local_id(), Some(TYPE_FOREIGN | 12345));

    // Verify that other types don't get confused for foreign
    let std_asset = AssetKind::Local(TYPE_STD | 12345);
    assert!(!std_asset.is_foreign());

    // Native enum variant check
    assert!(!AssetKind::Native.is_foreign());
  }
}
