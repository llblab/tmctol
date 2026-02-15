//! Runtime integration tests for the parachain.

#[cfg(test)]
pub mod aaa_integration_tests;
#[cfg(test)]
pub mod asset_convertion_integration_tests;
#[cfg(test)]
pub mod asset_registry_integration_tests;
#[cfg(test)]
pub mod axial_router_integration_tests;
#[cfg(test)]
pub mod burning_manager_integration_tests;
#[cfg(test)]
pub mod common;
#[cfg(test)]
pub mod economic_metrics;
#[cfg(test)]
pub mod load_testing;
#[cfg(test)]
pub mod tmctol_integration_tests;
#[cfg(test)]
pub mod treasury_owned_liquidity_integration_tests;
#[cfg(test)]
pub mod zap_manager_integration_tests;
#[cfg(test)]
pub mod zap_manager_load_tests;
