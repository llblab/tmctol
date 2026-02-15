#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

pub mod adapters;
pub use adapters::{AssetOps, DexOps};

pub mod weights;
pub use weights::{TaskWeightInfo, WeightInfo};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(feature = "runtime-benchmarks")]
pub trait BenchmarkHelper<AccountId, AssetId, Balance> {
  fn setup_remove_liquidity_max_k(
    owner: &AccountId,
    max_scan: u32,
  ) -> Result<(AssetId, Balance), polkadot_sdk::sp_runtime::DispatchError>;
}

#[frame::pallet]
pub mod pallet {
  use super::{AssetOps, DexOps, TaskWeightInfo, WeightInfo};
  use frame::prelude::*;
  use polkadot_sdk::{
    frame_support::{PalletId, traits::EnsureOrigin},
    sp_runtime::traits::{SaturatedConversion, Zero},
    sp_weights::WeightToFee as _,
  };

  pub type AaaId = u64;

  #[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    PartialEq,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub enum AmountSpec<Balance> {
    Fixed(Balance),
    AllBalance,
    Percentage(Permill),
  }

  #[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen,
  )]
  pub struct SplitLeg<AccountId> {
    pub to: AccountId,
    pub share: u32,
  }

  #[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen,
  )]
  pub enum TaskKind<AssetId, Balance, AccountId> {
    Transfer {
      to: AccountId,
      asset: AssetId,
      amount: AmountSpec<Balance>,
    },
    SplitTransfer {
      asset: AssetId,
      amount: AmountSpec<Balance>,
      total_shares: u32,
      legs: BoundedVec<SplitLeg<AccountId>, ConstU32<16>>,
      remainder_to: Option<AccountId>,
    },
    SwapExactIn {
      asset_in: AssetId,
      asset_out: AssetId,
      amount_in: AmountSpec<Balance>,
      min_out: Balance,
    },
    SwapExactOut {
      asset_in: AssetId,
      asset_out: AssetId,
      amount_out: Balance,
      max_in: Balance,
    },
    AddLiquidity {
      asset_a: AssetId,
      asset_b: AssetId,
      amount_a: AmountSpec<Balance>,
      amount_b: AmountSpec<Balance>,
    },
    RemoveLiquidity {
      lp_asset: AssetId,
      amount: AmountSpec<Balance>,
    },
    Burn {
      asset: AssetId,
      amount: AmountSpec<Balance>,
    },
    Mint {
      asset: AssetId,
      amount: AmountSpec<Balance>,
    },
    Noop,
  }

  #[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    PartialEq,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub enum AaaType {
    User,
    System,
  }

  #[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    PartialEq,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub enum Mutability {
    #[default]
    Mutable,
    Immutable,
  }

  // §12.3 — only Manual and CycleNonceExhausted
  #[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    PartialEq,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub enum PauseReason {
    Manual,
    CycleNonceExhausted,
  }

  // §12.1
  #[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    PartialEq,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub enum RefundReason {
    OwnerInitiated,
    RentInsolvent,
    BalanceExhausted,
    ConsecutiveFailures,
    WindowExpired,
    CycleNonceExhausted,
  }

  // §5.4 — only AbortCycle and ContinueNextStep
  #[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    PartialEq,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub enum PipelineErrorPolicy {
    AbortCycle,
    ContinueNextStep,
  }

  // §12.2
  #[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    PartialEq,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub enum DeferReason {
    QueueOverflow,
    InsufficientBudget,
  }

  #[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen,
  )]
  pub enum AssetFilter<AssetId> {
    IncludeOnly(BoundedVec<AssetId, ConstU32<16>>),
    Exclude(BoundedVec<AssetId, ConstU32<16>>),
  }

  #[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen,
  )]
  pub enum SourceFilter<AccountId> {
    Any,
    OwnerOnly,
    RefundAddressOnly,
    Whitelist(BoundedVec<AccountId, ConstU32<16>>),
  }

  #[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    PartialEq,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub enum InboxDrainMode {
    Single,
    Batch(u32),
    Drain,
  }

  #[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen,
  )]
  pub enum Trigger<AssetId, AccountId> {
    ProbabilisticTimer {
      every_blocks: u32,
      probability_ppm: u32,
    },
    OnAddressEvent {
      asset_filter: AssetFilter<AssetId>,
      source_filter: SourceFilter<AccountId>,
      drain_mode: InboxDrainMode,
    },
    Manual,
  }

  #[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen,
  )]
  pub struct Schedule<AssetId, AccountId> {
    pub trigger: Trigger<AssetId, AccountId>,
    pub cooldown_blocks: u32,
  }

  #[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    PartialEq,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub enum Condition<AssetId, Balance> {
    BalanceAbove { asset: AssetId, threshold: Balance },
    BalanceBelow { asset: AssetId, threshold: Balance },
    BalanceEquals { asset: AssetId, threshold: Balance },
    BalanceNotEquals { asset: AssetId, threshold: Balance },
  }

  #[derive(Decode, DecodeWithMemTracking, Encode, TypeInfo, MaxEncodedLen)]
  #[scale_info(skip_type_params(MaxConditionsPerStep))]
  pub struct Step<AssetId, Balance, AccountId, MaxConditionsPerStep: Get<u32>> {
    pub conditions: BoundedVec<Condition<AssetId, Balance>, MaxConditionsPerStep>,
    pub task: TaskKind<AssetId, Balance, AccountId>,
    pub on_error: PipelineErrorPolicy,
  }

  impl<AssetId: Clone, Balance: Clone, AccountId: Clone, MaxConditionsPerStep: Get<u32>> Clone
    for Step<AssetId, Balance, AccountId, MaxConditionsPerStep>
  {
    fn clone(&self) -> Self {
      Self {
        conditions: self.conditions.clone(),
        task: self.task.clone(),
        on_error: self.on_error,
      }
    }
  }

  impl<
    AssetId: core::fmt::Debug,
    Balance: core::fmt::Debug,
    AccountId: core::fmt::Debug,
    MaxConditionsPerStep: Get<u32>,
  > core::fmt::Debug for Step<AssetId, Balance, AccountId, MaxConditionsPerStep>
  {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
      f.debug_struct("Step")
        .field("conditions", &self.conditions)
        .field("task", &self.task)
        .field("on_error", &self.on_error)
        .finish()
    }
  }

  impl<AssetId: PartialEq, Balance: PartialEq, AccountId: PartialEq, MaxConditionsPerStep: Get<u32>>
    PartialEq for Step<AssetId, Balance, AccountId, MaxConditionsPerStep>
  {
    fn eq(&self, other: &Self) -> bool {
      self.conditions == other.conditions
        && self.task == other.task
        && self.on_error == other.on_error
    }
  }

  impl<AssetId: Eq, Balance: Eq, AccountId: Eq, MaxConditionsPerStep: Get<u32>> Eq
    for Step<AssetId, Balance, AccountId, MaxConditionsPerStep>
  {
  }

  #[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    PartialEq,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub struct AaaPolicy {
    pub default_error_policy: PipelineErrorPolicy,
  }

  impl Default for AaaPolicy {
    fn default() -> Self {
      Self {
        default_error_policy: PipelineErrorPolicy::AbortCycle,
      }
    }
  }

  #[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    PartialEq,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub struct ScheduleWindow<BlockNumber> {
    pub start: BlockNumber,
    pub end: BlockNumber,
  }

  #[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen,
  )]
  #[scale_info(skip_type_params(MaxRefundableAssets))]
  pub struct AaaInstance<
    AccountId,
    AssetId,
    BlockNumber,
    Pipeline,
    MaxRefundableAssets: Get<u32> + 'static,
  > {
    pub aaa_id: AaaId,
    pub sovereign_account: AccountId,
    pub owner: AccountId,
    pub owner_slot: u16,
    pub aaa_type: AaaType,
    pub mutability: Mutability,
    pub is_paused: bool,
    pub pause_reason: Option<PauseReason>,
    pub schedule: Schedule<AssetId, AccountId>,
    pub schedule_window: Option<ScheduleWindow<BlockNumber>>,
    pub pipeline: Pipeline,
    pub refund_assets: BoundedVec<AssetId, MaxRefundableAssets>,
    pub cycle_nonce: u64,
    pub consecutive_failures: u32,
    pub manual_trigger_pending: bool,
    pub policy: AaaPolicy,
    pub refund_to: AccountId,
    pub created_at: BlockNumber,
    pub updated_at: BlockNumber,
    pub last_cycle_block: BlockNumber,
    pub last_rent_block: BlockNumber,
  }

  #[pallet::config]
  pub trait Config: frame_system::Config {
    type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize + MaxEncodedLen;

    type Balance: Parameter
      + Member
      + AtLeast32BitUnsigned
      + Default
      + Copy
      + MaybeSerializeDeserialize
      + MaxEncodedLen;

    #[pallet::constant]
    type NativeAssetId: Get<Self::AssetId>;

    type AssetOps: AssetOps<Self::AccountId, Self::AssetId, Self::Balance>;
    type DexOps: DexOps<Self::AccountId, Self::AssetId, Self::Balance>;

    #[pallet::constant]
    type MinWindowLength: Get<BlockNumberFor<Self>>;

    #[pallet::constant]
    type PalletId: Get<PalletId>;

    type SystemOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    type GlobalBreakerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

    #[pallet::constant]
    type MaxPipelineSteps: Get<u32>;
    #[pallet::constant]
    type MaxUserPipelineSteps: Get<u32>;
    #[pallet::constant]
    type MaxSystemPipelineSteps: Get<u32>;
    #[pallet::constant]
    type MaxConditionsPerStep: Get<u32>;
    #[pallet::constant]
    type MaxOwnedAaas: Get<u32>;
    #[pallet::constant]
    type MaxOwnerSlots: Get<u16>;
    #[pallet::constant]
    type MaxReadyRingLength: Get<u32>;
    #[pallet::constant]
    type MaxDeferredRingLength: Get<u32>;
    #[pallet::constant]
    type MaxDeferredRetriesPerBlock: Get<u32>;
    #[pallet::constant]
    type MaxSystemExecutionsPerBlock: Get<u32>;
    #[pallet::constant]
    type MaxUserExecutionsPerBlock: Get<u32>;
    #[pallet::constant]
    type FairnessWeightSystem: Get<u32>;
    #[pallet::constant]
    type FairnessWeightUser: Get<u32>;
    #[pallet::constant]
    type MaxSweepPerBlock: Get<u32>;
    #[pallet::constant]
    type AaaBudgetPct: Get<Permill>;
    #[pallet::constant]
    type MaxAddressEventInboxCount: Get<u32>;
    #[pallet::constant]
    type MaxAdapterScan: Get<u32>;

    #[pallet::constant]
    type RentPerBlock: Get<Self::Balance>;
    #[pallet::constant]
    type MaxRentAccrual: Get<Self::Balance>;

    /// Per-step flat evaluation cost (§3.3)
    #[pallet::constant]
    type StepBaseFee: Get<Self::Balance>;
    /// Per-condition balance read cost (§3.3)
    #[pallet::constant]
    type ConditionReadFee: Get<Self::Balance>;
    /// Converts weight to fee for execution cost calculation (§3.4)
    type WeightToFee: polkadot_sdk::sp_weights::WeightToFee<Balance = Self::Balance>;
    /// Runtime-bound upper weights for every AAA task variant (§3.4)
    type TaskWeightInfo: TaskWeightInfo;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper: crate::BenchmarkHelper<Self::AccountId, Self::AssetId, Self::Balance>;

    type FeeSink: Get<Self::AccountId>;

    #[pallet::constant]
    type MaxRefundableAssets: Get<u32> + 'static;
    #[pallet::constant]
    type MaxConsecutiveFailures: Get<u32>;
    #[pallet::constant]
    type MinUserBalance: Get<Self::Balance>;
    #[pallet::constant]
    type RefundTransferCost: Get<Self::Balance>;

    type WeightInfo: WeightInfo;
  }

  pub type BalanceOf<T> = <T as Config>::Balance;
  pub type AssetIdOf<T> = <T as Config>::AssetId;

  pub type PipelineOf<T> = BoundedVec<
    Step<
      <T as Config>::AssetId,
      <T as Config>::Balance,
      <T as frame_system::Config>::AccountId,
      <T as Config>::MaxConditionsPerStep,
    >,
    <T as Config>::MaxPipelineSteps,
  >;

  pub type AaaInstanceOf<T> = AaaInstance<
    <T as frame_system::Config>::AccountId,
    <T as Config>::AssetId,
    BlockNumberFor<T>,
    PipelineOf<T>,
    <T as Config>::MaxRefundableAssets,
  >;

  #[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    PartialEq,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub struct EventInboxEntry<BlockNumber> {
    pub pending_count: u32,
    pub saturated: bool,
    pub last_event_block: BlockNumber,
  }

  #[pallet::pallet]
  pub struct Pallet<T>(_);

  #[pallet::storage]
  #[pallet::getter(fn next_aaa_id)]
  pub type NextAaaId<T> = StorageValue<_, AaaId, ValueQuery>;

  #[pallet::storage]
  pub type SweepCursor<T: Config> = StorageValue<_, AaaId, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn aaa_instances)]
  pub type AaaInstances<T: Config> =
    StorageMap<_, Blake2_128Concat, AaaId, AaaInstanceOf<T>, OptionQuery>;

  #[pallet::storage]
  #[pallet::getter(fn owner_index)]
  pub type OwnerIndex<T: Config> =
    StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<AaaId, T::MaxOwnedAaas>, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn owner_slots)]
  pub type OwnerSlots<T: Config> =
    StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, u16, AaaId, OptionQuery>;

  #[pallet::storage]
  #[pallet::getter(fn sovereign_index)]
  pub type SovereignIndex<T: Config> =
    StorageMap<_, Blake2_128Concat, T::AccountId, AaaId, OptionQuery>;

  #[pallet::storage]
  #[pallet::getter(fn ready_ring)]
  pub type ReadyRing<T: Config> =
    StorageValue<_, BoundedVec<AaaId, T::MaxReadyRingLength>, ValueQuery>;

  #[pallet::storage]
  pub type ReadyArbitrationCursor<T: Config> = StorageValue<_, u32, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn deferred_ring)]
  pub type DeferredRing<T: Config> =
    StorageValue<_, BoundedVec<AaaId, T::MaxDeferredRingLength>, ValueQuery>;

  #[pallet::storage]
  pub type DeferredCursor<T: Config> = StorageValue<_, u32, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn global_circuit_breaker)]
  pub type GlobalCircuitBreaker<T> = StorageValue<_, bool, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn event_inbox)]
  pub type EventInbox<T: Config> = StorageDoubleMap<
    _,
    Blake2_128Concat,
    AaaId,
    Blake2_128Concat,
    T::AssetId,
    EventInboxEntry<BlockNumberFor<T>>,
    OptionQuery,
  >;

  #[pallet::hooks]
  impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
      if GlobalCircuitBreaker::<T>::get() {
        return T::DbWeight::get().reads(1);
      }
      T::DbWeight::get().reads(1)
    }

    fn on_idle(_now: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
      if GlobalCircuitBreaker::<T>::get() {
        return Weight::zero();
      }
      let cycle_weight = Self::execute_cycle(remaining_weight);
      let sweep_weight = Self::execute_zombie_sweep();
      cycle_weight.saturating_add(sweep_weight)
    }
  }

  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    AAACreated {
      aaa_id: AaaId,
      aaa_type: AaaType,
      owner: T::AccountId,
      owner_slot: u16,
      mutability: Mutability,
      sovereign_account: T::AccountId,
    },
    AAAPaused {
      aaa_id: AaaId,
      reason: PauseReason,
    },
    AAAResumed {
      aaa_id: AaaId,
    },
    ManualTriggerSet {
      aaa_id: AaaId,
    },
    AAAFunded {
      aaa_id: AaaId,
      from: T::AccountId,
      amount: BalanceOf<T>,
    },
    AAARefunded {
      aaa_id: AaaId,
      reason: RefundReason,
      solvent: bool,
      to: T::AccountId,
      assets_refunded: BoundedVec<(T::AssetId, BalanceOf<T>), T::MaxRefundableAssets>,
      assets_forfeited: BoundedVec<(T::AssetId, BalanceOf<T>), T::MaxRefundableAssets>,
      native_burned: BalanceOf<T>,
    },
    AAADestroyed {
      aaa_id: AaaId,
    },
    RentCharged {
      aaa_id: AaaId,
      blocks_elapsed: u32,
      rent_due: BalanceOf<T>,
      rent_paid: BalanceOf<T>,
      rent_debt: BalanceOf<T>,
    },
    PolicyUpdated {
      aaa_id: AaaId,
    },
    ScheduleUpdated {
      aaa_id: AaaId,
    },
    RefundAssetsUpdated {
      aaa_id: AaaId,
    },
    GlobalCircuitBreakerSet {
      paused: bool,
    },
    CycleDeferred {
      aaa_id: AaaId,
      reason: DeferReason,
    },
    CycleStarted {
      aaa_id: AaaId,
      cycle_nonce: u64,
    },
    PipelineExecuted {
      aaa_id: AaaId,
      cycle_nonce: u64,
      steps_executed: u32,
    },
    PipelineFailed {
      aaa_id: AaaId,
      cycle_nonce: u64,
      failed_step: u32,
      error: DispatchError,
    },
    StepSkipped {
      aaa_id: AaaId,
      cycle_nonce: u64,
      step: u32,
    },
    StepFailed {
      aaa_id: AaaId,
      cycle_nonce: u64,
      step: u32,
      error: DispatchError,
    },
    TransferExecuted {
      aaa_id: AaaId,
      to: T::AccountId,
      asset: T::AssetId,
      amount: T::Balance,
    },
    SplitTransferExecuted {
      aaa_id: AaaId,
      asset: T::AssetId,
      total: T::Balance,
      legs_count: u32,
    },
    BurnExecuted {
      aaa_id: AaaId,
      asset: T::AssetId,
      amount: T::Balance,
    },
    MintExecuted {
      aaa_id: AaaId,
      asset: T::AssetId,
      amount: T::Balance,
    },
    SwapExecuted {
      aaa_id: AaaId,
      asset_in: T::AssetId,
      asset_out: T::AssetId,
      amount_in: T::Balance,
      amount_out: T::Balance,
    },
    LiquidityAdded {
      aaa_id: AaaId,
      asset_a: T::AssetId,
      asset_b: T::AssetId,
      amount_a: T::Balance,
      amount_b: T::Balance,
      lp_minted: T::Balance,
    },
    LiquidityRemoved {
      aaa_id: AaaId,
      lp_asset: T::AssetId,
      amount: T::Balance,
      amount_a_out: T::Balance,
      amount_b_out: T::Balance,
    },
  }

  #[pallet::error]
  pub enum Error<T> {
    AaaNotFound,
    AaaIdOverflow,
    EmptyPipeline,
    PipelineTooLong,
    OwnerIndexFull,
    OwnerSlotCapacityExceeded,
    SovereignAccountCollision,
    NotOwner,
    NotGovernance,
    NotPaused,
    AlreadyPaused,
    AmountZero,
    CycleNonceExhausted,
    TaskExecutionFailed,
    InsufficientBalance,
    InsufficientEvaluationFee,
    InsufficientExecutionFee,
    SplitTransferInvalid,
    ZeroShareLeg,
    InsufficientSplitLegs,
    DuplicateRecipient,
    RefundAssetsOverflow,
    ImmutableActor,
    MintNotAllowedForUserAaa,
    GlobalCircuitBreakerActive,
    InvalidScheduleWindow,
    WindowTooShort,
    InvalidDrainMode,
  }

  #[pallet::call]
  impl<T: Config> Pallet<T> {
    #[pallet::call_index(0)]
    #[pallet::weight(T::WeightInfo::create_user_aaa())]
    pub fn create_user_aaa(
      origin: OriginFor<T>,
      mutability: Mutability,
      schedule: Schedule<T::AssetId, T::AccountId>,
      schedule_window: Option<ScheduleWindow<BlockNumberFor<T>>>,
      pipeline: PipelineOf<T>,
      policy: AaaPolicy,
      refund_to: Option<T::AccountId>,
    ) -> DispatchResult {
      let owner = ensure_signed(origin)?;
      ensure!(
        !GlobalCircuitBreaker::<T>::get(),
        Error::<T>::GlobalCircuitBreakerActive
      );
      ensure!(!pipeline.is_empty(), Error::<T>::EmptyPipeline);
      ensure!(
        (pipeline.len() as u32) <= T::MaxUserPipelineSteps::get(),
        Error::<T>::PipelineTooLong
      );
      ensure!(
        !Self::pipeline_contains_mint(&pipeline),
        Error::<T>::MintNotAllowedForUserAaa
      );
      let target = refund_to.unwrap_or_else(|| owner.clone());
      Self::do_create_aaa(
        owner.clone(),
        owner,
        AaaType::User,
        mutability,
        schedule,
        schedule_window,
        pipeline,
        policy,
        target,
      )
    }

    #[pallet::call_index(1)]
    #[pallet::weight(T::WeightInfo::create_system_aaa())]
    pub fn create_system_aaa(
      origin: OriginFor<T>,
      owner: T::AccountId,
      schedule: Schedule<T::AssetId, T::AccountId>,
      schedule_window: Option<ScheduleWindow<BlockNumberFor<T>>>,
      pipeline: PipelineOf<T>,
      policy: AaaPolicy,
      refund_to: T::AccountId,
    ) -> DispatchResult {
      T::SystemOrigin::ensure_origin(origin)?;
      ensure!(
        !GlobalCircuitBreaker::<T>::get(),
        Error::<T>::GlobalCircuitBreakerActive
      );
      ensure!(!pipeline.is_empty(), Error::<T>::EmptyPipeline);
      ensure!(
        (pipeline.len() as u32) <= T::MaxSystemPipelineSteps::get(),
        Error::<T>::PipelineTooLong
      );
      Self::do_create_aaa(
        owner.clone(),
        owner,
        AaaType::System,
        Mutability::Mutable,
        schedule,
        schedule_window,
        pipeline,
        policy,
        refund_to,
      )
    }

    #[pallet::call_index(2)]
    #[pallet::weight(T::WeightInfo::pause_aaa())]
    pub fn pause_aaa(origin: OriginFor<T>, aaa_id: AaaId) -> DispatchResult {
      AaaInstances::<T>::try_mutate(aaa_id, |maybe| -> DispatchResult {
        let inst = maybe.as_mut().ok_or(Error::<T>::AaaNotFound)?;
        Self::ensure_control_origin(origin.clone(), inst)?;
        ensure!(
          inst.mutability == Mutability::Mutable,
          Error::<T>::ImmutableActor
        );
        ensure!(!inst.is_paused, Error::<T>::AlreadyPaused);
        inst.is_paused = true;
        inst.pause_reason = Some(PauseReason::Manual);
        inst.updated_at = frame_system::Pallet::<T>::block_number();
        Self::remove_from_ready_ring(aaa_id);
        Self::deposit_event(Event::AAAPaused {
          aaa_id,
          reason: PauseReason::Manual,
        });
        Ok(())
      })
    }

    #[pallet::call_index(3)]
    #[pallet::weight(T::WeightInfo::resume_aaa())]
    pub fn resume_aaa(origin: OriginFor<T>, aaa_id: AaaId) -> DispatchResult {
      AaaInstances::<T>::try_mutate(aaa_id, |maybe| -> DispatchResult {
        let inst = maybe.as_mut().ok_or(Error::<T>::AaaNotFound)?;
        Self::ensure_control_origin(origin.clone(), inst)?;
        ensure!(
          inst.mutability == Mutability::Mutable,
          Error::<T>::ImmutableActor
        );
        ensure!(inst.is_paused, Error::<T>::NotPaused);
        inst.is_paused = false;
        inst.pause_reason = None;
        inst.updated_at = frame_system::Pallet::<T>::block_number();
        Self::requeue_to_ready_ring(aaa_id);
        Self::deposit_event(Event::AAAResumed { aaa_id });
        Ok(())
      })
    }

    #[pallet::call_index(4)]
    #[pallet::weight(T::WeightInfo::manual_trigger())]
    pub fn manual_trigger(origin: OriginFor<T>, aaa_id: AaaId) -> DispatchResult {
      AaaInstances::<T>::try_mutate(aaa_id, |maybe| -> DispatchResult {
        let inst = maybe.as_mut().ok_or(Error::<T>::AaaNotFound)?;
        Self::ensure_control_origin(origin.clone(), inst)?;
        ensure!(!inst.is_paused, Error::<T>::AlreadyPaused);
        inst.manual_trigger_pending = true;
        inst.updated_at = frame_system::Pallet::<T>::block_number();
        Self::requeue_to_ready_ring(aaa_id);
        Self::deposit_event(Event::ManualTriggerSet { aaa_id });
        Ok(())
      })
    }

    #[pallet::call_index(5)]
    #[pallet::weight(T::WeightInfo::fund_aaa())]
    pub fn fund_aaa(origin: OriginFor<T>, aaa_id: AaaId, amount: BalanceOf<T>) -> DispatchResult {
      let who = ensure_signed(origin)?;
      ensure!(!amount.is_zero(), Error::<T>::AmountZero);
      let instance = AaaInstances::<T>::get(aaa_id).ok_or(Error::<T>::AaaNotFound)?;
      T::AssetOps::transfer(
        &who,
        &instance.sovereign_account,
        T::NativeAssetId::get(),
        amount,
      )?;
      Self::deposit_event(Event::AAAFunded {
        aaa_id,
        from: who,
        amount,
      });
      Ok(())
    }

    #[pallet::call_index(6)]
    #[pallet::weight(T::WeightInfo::refund_and_close())]
    pub fn refund_and_close(origin: OriginFor<T>, aaa_id: AaaId) -> DispatchResult {
      let instance = AaaInstances::<T>::get(aaa_id).ok_or(Error::<T>::AaaNotFound)?;
      Self::ensure_control_origin(origin, &instance)?;
      Self::do_terminal_refund(aaa_id, &instance, RefundReason::OwnerInitiated)
    }

    #[pallet::call_index(7)]
    #[pallet::weight(T::WeightInfo::update_policy())]
    pub fn update_policy(origin: OriginFor<T>, aaa_id: AaaId, policy: AaaPolicy) -> DispatchResult {
      AaaInstances::<T>::try_mutate(aaa_id, |maybe| -> DispatchResult {
        let inst = maybe.as_mut().ok_or(Error::<T>::AaaNotFound)?;
        Self::ensure_control_origin(origin.clone(), inst)?;
        ensure!(
          inst.mutability == Mutability::Mutable,
          Error::<T>::ImmutableActor
        );
        inst.policy = policy;
        inst.updated_at = frame_system::Pallet::<T>::block_number();
        Self::deposit_event(Event::PolicyUpdated { aaa_id });
        Ok(())
      })
    }

    #[pallet::call_index(8)]
    #[pallet::weight(T::WeightInfo::update_schedule())]
    pub fn update_schedule(
      origin: OriginFor<T>,
      aaa_id: AaaId,
      schedule: Schedule<T::AssetId, T::AccountId>,
      schedule_window: Option<ScheduleWindow<BlockNumberFor<T>>>,
    ) -> DispatchResult {
      Self::validate_schedule(&schedule)?;
      if let Some(ref window) = schedule_window {
        Self::validate_schedule_window(window)?;
      }
      AaaInstances::<T>::try_mutate(aaa_id, |maybe| -> DispatchResult {
        let inst = maybe.as_mut().ok_or(Error::<T>::AaaNotFound)?;
        Self::ensure_control_origin(origin.clone(), inst)?;
        ensure!(
          inst.mutability == Mutability::Mutable,
          Error::<T>::ImmutableActor
        );
        inst.schedule = schedule;
        inst.schedule_window = schedule_window;
        inst.updated_at = frame_system::Pallet::<T>::block_number();
        Self::deposit_event(Event::ScheduleUpdated { aaa_id });
        Ok(())
      })
    }

    #[pallet::call_index(9)]
    #[pallet::weight(T::WeightInfo::set_global_circuit_breaker())]
    pub fn set_global_circuit_breaker(origin: OriginFor<T>, paused: bool) -> DispatchResult {
      T::GlobalBreakerOrigin::ensure_origin(origin)?;
      GlobalCircuitBreaker::<T>::put(paused);
      Self::deposit_event(Event::GlobalCircuitBreakerSet { paused });
      Ok(())
    }

    #[pallet::call_index(10)]
    #[pallet::weight(T::WeightInfo::update_refund_assets())]
    pub fn update_refund_assets(
      origin: OriginFor<T>,
      aaa_id: AaaId,
      additional_assets: BoundedVec<T::AssetId, T::MaxRefundableAssets>,
    ) -> DispatchResult {
      T::SystemOrigin::ensure_origin(origin)?;
      AaaInstances::<T>::try_mutate(aaa_id, |maybe| -> DispatchResult {
        let inst = maybe.as_mut().ok_or(Error::<T>::AaaNotFound)?;
        ensure!(inst.aaa_type == AaaType::System, Error::<T>::NotGovernance);
        for asset in additional_assets.iter() {
          if !inst.refund_assets.contains(asset) {
            inst
              .refund_assets
              .try_push(*asset)
              .map_err(|_| Error::<T>::RefundAssetsOverflow)?;
          }
        }
        inst.updated_at = frame_system::Pallet::<T>::block_number();
        Self::deposit_event(Event::RefundAssetsUpdated { aaa_id });
        Ok(())
      })
    }

    /// §8.7 — force rent/lifecycle evaluation for a specific actor
    #[pallet::call_index(11)]
    #[pallet::weight(T::WeightInfo::permissionless_sweep())]
    pub fn permissionless_sweep(origin: OriginFor<T>, aaa_id: AaaId) -> DispatchResult {
      let _who = ensure_signed(origin)?;
      Self::evaluate_actor_liveness(aaa_id)
    }
  }

  impl<T: Config> Pallet<T> {
    pub fn weight_upper_bound(task: &TaskKind<T::AssetId, T::Balance, T::AccountId>) -> Weight {
      // Runtime owns the upper-bound contract so fee math stays aligned with chain economics
      match task {
        TaskKind::Transfer { .. } => T::TaskWeightInfo::transfer(),
        TaskKind::SplitTransfer { legs, .. } => {
          T::TaskWeightInfo::split_transfer((legs.len() as u32).saturating_add(1))
        }
        TaskKind::SwapExactIn { .. } => T::TaskWeightInfo::swap_exact_in(),
        TaskKind::SwapExactOut { .. } => T::TaskWeightInfo::swap_exact_out(),
        TaskKind::AddLiquidity { .. } => T::TaskWeightInfo::add_liquidity(),
        TaskKind::RemoveLiquidity { .. } => T::TaskWeightInfo::remove_liquidity(),
        TaskKind::Burn { .. } => T::TaskWeightInfo::burn(),
        TaskKind::Mint { .. } => T::TaskWeightInfo::mint(),
        TaskKind::Noop => T::TaskWeightInfo::noop(),
      }
    }
    pub fn sovereign_account_id(owner: &T::AccountId, owner_slot: u16) -> T::AccountId {
      let mut seed_input = owner.encode();
      seed_input.extend_from_slice(b"aaa");
      seed_input.extend_from_slice(&owner_slot.to_le_bytes());
      let seed = frame::hashing::blake2_256(&seed_input);
      // Fold seed bytes into PalletId so uniqueness is preserved even for small AccountId types
      // where `into_sub_account_truncating` may aggressively truncate the sub-account payload.
      let mut id_bytes = T::PalletId::get().0;
      for (i, b) in seed.iter().enumerate() {
        id_bytes[i % 8] ^= b;
      }
      use polkadot_sdk::sp_runtime::traits::AccountIdConversion;
      polkadot_sdk::frame_support::PalletId(id_bytes).into_sub_account_truncating(owner_slot as u64)
    }

    fn allocate_owner_slot(owner: &T::AccountId) -> Result<(u16, T::AccountId), Error<T>> {
      let owner_slot = (0..T::MaxOwnerSlots::get())
        .find(|slot| !OwnerSlots::<T>::contains_key(owner, *slot))
        .ok_or(Error::<T>::OwnerSlotCapacityExceeded)?;
      let sovereign_account = Self::sovereign_account_id(owner, owner_slot);
      if SovereignIndex::<T>::contains_key(&sovereign_account) {
        return Err(Error::<T>::SovereignAccountCollision);
      }
      Ok((owner_slot, sovereign_account))
    }

    fn do_create_aaa(
      owner_index_account: T::AccountId,
      owner: T::AccountId,
      aaa_type: AaaType,
      mutability: Mutability,
      schedule: Schedule<T::AssetId, T::AccountId>,
      schedule_window: Option<ScheduleWindow<BlockNumberFor<T>>>,
      pipeline: PipelineOf<T>,
      policy: AaaPolicy,
      refund_to: T::AccountId,
    ) -> DispatchResult {
      Self::validate_schedule(&schedule)?;
      if let Some(ref window) = schedule_window {
        Self::validate_schedule_window(window)?;
      }
      Self::validate_pipeline_shape(&pipeline)?;
      let aaa_id = NextAaaId::<T>::get();
      let next_id = aaa_id.checked_add(1).ok_or(Error::<T>::AaaIdOverflow)?;
      let now = frame_system::Pallet::<T>::block_number();
      let (owner_slot, sovereign_account) = Self::allocate_owner_slot(&owner)?;
      let refund_assets = Self::compute_refund_assets(&pipeline)?;
      let instance = AaaInstance {
        aaa_id,
        sovereign_account: sovereign_account.clone(),
        owner: owner.clone(),
        owner_slot,
        aaa_type,
        mutability,
        is_paused: false,
        pause_reason: None,
        schedule,
        schedule_window,
        pipeline,
        refund_assets,
        cycle_nonce: 0,
        consecutive_failures: 0,
        manual_trigger_pending: false,
        policy,
        refund_to,
        created_at: now,
        updated_at: now,
        last_cycle_block: Zero::zero(),
        last_rent_block: now,
      };
      OwnerIndex::<T>::try_mutate(owner_index_account, |owned| {
        owned
          .try_push(aaa_id)
          .map_err(|_| Error::<T>::OwnerIndexFull)
      })?;
      OwnerSlots::<T>::insert(owner.clone(), owner_slot, aaa_id);
      SovereignIndex::<T>::insert(sovereign_account.clone(), aaa_id);
      AaaInstances::<T>::insert(aaa_id, instance);
      NextAaaId::<T>::put(next_id);
      if !Self::try_enqueue_ready(aaa_id) {
        let _ = Self::try_enqueue_deferred(aaa_id);
        Self::deposit_event(Event::CycleDeferred {
          aaa_id,
          reason: DeferReason::QueueOverflow,
        });
      }
      Self::deposit_event(Event::AAACreated {
        aaa_id,
        owner,
        owner_slot,
        aaa_type,
        mutability,
        sovereign_account,
      });
      Ok(())
    }

    fn compute_refund_assets(
      pipeline: &PipelineOf<T>,
    ) -> Result<BoundedVec<T::AssetId, T::MaxRefundableAssets>, Error<T>> {
      let mut assets: BoundedVec<T::AssetId, T::MaxRefundableAssets> = BoundedVec::default();
      assets
        .try_push(T::NativeAssetId::get())
        .map_err(|_| Error::<T>::RefundAssetsOverflow)?;
      for step in pipeline.iter() {
        let task_assets = match &step.task {
          TaskKind::Transfer { asset, .. } => alloc::vec![*asset],
          TaskKind::SplitTransfer { asset, .. } => alloc::vec![*asset],
          TaskKind::Burn { asset, .. } => alloc::vec![*asset],
          TaskKind::Mint { asset, .. } => alloc::vec![*asset],
          TaskKind::SwapExactIn {
            asset_in,
            asset_out,
            ..
          } => alloc::vec![*asset_in, *asset_out],
          TaskKind::SwapExactOut {
            asset_in,
            asset_out,
            ..
          } => alloc::vec![*asset_in, *asset_out],
          TaskKind::AddLiquidity {
            asset_a, asset_b, ..
          } => alloc::vec![*asset_a, *asset_b],
          TaskKind::RemoveLiquidity { lp_asset, .. } => alloc::vec![*lp_asset],
          TaskKind::Noop => alloc::vec![],
        };
        for asset in task_assets {
          if !assets.contains(&asset) {
            assets
              .try_push(asset)
              .map_err(|_| Error::<T>::RefundAssetsOverflow)?;
          }
        }
      }
      Ok(assets)
    }

    fn pipeline_contains_mint(pipeline: &PipelineOf<T>) -> bool {
      pipeline
        .iter()
        .any(|step| matches!(step.task, TaskKind::Mint { .. }))
    }

    fn validate_schedule(schedule: &Schedule<T::AssetId, T::AccountId>) -> DispatchResult {
      if let Trigger::OnAddressEvent { drain_mode, .. } = &schedule.trigger {
        if let InboxDrainMode::Batch(max) = drain_mode {
          ensure!(*max > 0, Error::<T>::InvalidDrainMode);
          ensure!(
            *max <= T::MaxAddressEventInboxCount::get(),
            Error::<T>::InvalidDrainMode
          );
        }
      }
      Ok(())
    }

    fn validate_schedule_window(window: &ScheduleWindow<BlockNumberFor<T>>) -> DispatchResult {
      ensure!(window.end > window.start, Error::<T>::InvalidScheduleWindow);
      ensure!(
        window.end.saturating_sub(window.start) >= T::MinWindowLength::get(),
        Error::<T>::WindowTooShort
      );
      ensure!(
        window.start >= frame_system::Pallet::<T>::block_number(),
        Error::<T>::InvalidScheduleWindow
      );
      Ok(())
    }

    fn validate_pipeline_shape(pipeline: &PipelineOf<T>) -> DispatchResult {
      for step in pipeline.iter() {
        if let TaskKind::SplitTransfer {
          total_shares, legs, ..
        } = &step.task
        {
          Self::validate_split_transfer_legs(*total_shares, legs)?;
        }
      }
      Ok(())
    }

    fn validate_split_transfer_legs(
      total_shares: u32,
      legs: &BoundedVec<SplitLeg<T::AccountId>, ConstU32<16>>,
    ) -> DispatchResult {
      ensure!(legs.len() >= 2, Error::<T>::InsufficientSplitLegs);
      ensure!(total_shares > 0, Error::<T>::SplitTransferInvalid);
      let mut sum_shares = 0u64;
      for (idx, leg) in legs.iter().enumerate() {
        ensure!(leg.share > 0, Error::<T>::ZeroShareLeg);
        sum_shares = sum_shares.saturating_add(u64::from(leg.share));
        let duplicate = legs.iter().take(idx).any(|existing| existing.to == leg.to);
        ensure!(!duplicate, Error::<T>::DuplicateRecipient);
      }
      ensure!(
        sum_shares == u64::from(total_shares),
        Error::<T>::SplitTransferInvalid
      );
      Ok(())
    }

    fn ensure_control_origin(origin: OriginFor<T>, instance: &AaaInstanceOf<T>) -> DispatchResult {
      if let Ok(who) = ensure_signed(origin.clone()) {
        ensure!(who == instance.owner, Error::<T>::NotOwner);
        return Ok(());
      }
      T::SystemOrigin::ensure_origin(origin)?;
      ensure!(
        instance.aaa_type == AaaType::System,
        Error::<T>::NotGovernance
      );
      Ok(())
    }

    fn try_enqueue_ready(aaa_id: AaaId) -> bool {
      if DeferredRing::<T>::get().contains(&aaa_id) {
        DeferredRing::<T>::mutate(|ring| {
          if let Some(i) = ring.iter().position(|id| *id == aaa_id) {
            ring.swap_remove(i);
          }
        });
      }
      ReadyRing::<T>::mutate(|ring| {
        if ring.contains(&aaa_id) {
          return true;
        }
        ring.try_push(aaa_id).is_ok()
      })
    }

    fn try_enqueue_deferred(aaa_id: AaaId) -> bool {
      DeferredRing::<T>::mutate(|ring| {
        if ring.contains(&aaa_id) {
          return true;
        }
        ring.try_push(aaa_id).is_ok()
      })
    }

    fn remove_from_ready_ring(aaa_id: AaaId) {
      ReadyRing::<T>::mutate(|ring| {
        if let Some(i) = ring.iter().position(|id| *id == aaa_id) {
          ring.swap_remove(i);
        }
      });
      DeferredRing::<T>::mutate(|ring| {
        if let Some(i) = ring.iter().position(|id| *id == aaa_id) {
          ring.swap_remove(i);
        }
      });
    }

    fn remove_owner_index(owner: &T::AccountId, aaa_id: AaaId) {
      OwnerIndex::<T>::mutate_exists(owner, |maybe| {
        if let Some(owned) = maybe {
          if let Some(i) = owned.iter().position(|id| *id == aaa_id) {
            owned.swap_remove(i);
          }
          if owned.is_empty() {
            *maybe = None;
          }
        }
      });
    }

    fn remove_owner_slot_binding(owner: &T::AccountId, owner_slot: u16, sovereign: &T::AccountId) {
      OwnerSlots::<T>::remove(owner, owner_slot);
      SovereignIndex::<T>::remove(sovereign);
    }

    fn next_schedule_class(system_executions: u32, user_executions: u32) -> Option<AaaType> {
      let system_cap = T::MaxSystemExecutionsPerBlock::get();
      let user_cap = T::MaxUserExecutionsPerBlock::get();
      let can_system = system_executions < system_cap;
      let can_user = user_executions < user_cap;
      match (can_system, can_user) {
        (false, false) => None,
        (true, false) => Some(AaaType::System),
        (false, true) => Some(AaaType::User),
        (true, true) => {
          let system_weight = T::FairnessWeightSystem::get().max(1);
          let user_weight = T::FairnessWeightUser::get().max(1);
          let span = system_weight.saturating_add(user_weight);
          let cursor = if span == 0 {
            0
          } else {
            ReadyArbitrationCursor::<T>::get() % span
          };
          if span > 0 {
            ReadyArbitrationCursor::<T>::put((cursor.saturating_add(1)) % span);
          }
          if cursor < system_weight {
            Some(AaaType::System)
          } else {
            Some(AaaType::User)
          }
        }
      }
    }

    fn pop_ready_for_class(class: AaaType) -> Option<AaaId> {
      ReadyRing::<T>::mutate(|ring| {
        let mut idx = 0usize;
        while idx < ring.len() {
          let aaa_id = ring[idx];
          match AaaInstances::<T>::get(aaa_id) {
            None => {
              ring.swap_remove(idx);
              continue;
            }
            Some(instance) if instance.aaa_type == class => {
              return Some(ring.remove(idx));
            }
            _ => {
              idx = idx.saturating_add(1);
            }
          }
        }
        None
      })
    }

    fn pop_deferred_next() -> Option<AaaId> {
      DeferredRing::<T>::mutate(|ring| {
        if ring.is_empty() {
          DeferredCursor::<T>::put(0);
          return None;
        }
        let len = ring.len() as u32;
        let cursor = DeferredCursor::<T>::get() % len;
        let aaa_id = ring.remove(cursor as usize);
        let new_len = ring.len() as u32;
        DeferredCursor::<T>::put(if new_len == 0 { 0 } else { cursor % new_len });
        Some(aaa_id)
      })
    }

    fn retry_deferred_queue() {
      let max_retries = T::MaxDeferredRetriesPerBlock::get();
      let max_ready = T::MaxReadyRingLength::get();
      let mut retried = 0u32;
      while retried < max_retries {
        if (ReadyRing::<T>::get().len() as u32) >= max_ready {
          break;
        }
        let aaa_id = match Self::pop_deferred_next() {
          Some(id) => id,
          None => break,
        };
        if !AaaInstances::<T>::contains_key(aaa_id) {
          retried = retried.saturating_add(1);
          continue;
        }
        if !Self::try_enqueue_ready(aaa_id) {
          let _ = Self::try_enqueue_deferred(aaa_id);
          break;
        }
        retried = retried.saturating_add(1);
      }
    }

    fn execute_cycle(remaining_weight: Weight) -> Weight {
      Self::retry_deferred_queue();
      let budget_pct = T::AaaBudgetPct::get();
      let budget = budget_pct.mul_floor(remaining_weight.ref_time());
      let mut consumed: u64 = 0;
      let mut total_consumed = Weight::zero();
      let max_attempts = T::MaxReadyRingLength::get();
      let mut attempts: u32 = 0;
      let mut system_executions = 0u32;
      let mut user_executions = 0u32;
      let mut class_misses = 0u8;
      // Collect IDs to re-enqueue AFTER the loop to prevent double-firing in one block
      let mut to_requeue: alloc::vec::Vec<AaaId> = alloc::vec::Vec::new();
      while attempts < max_attempts {
        if consumed >= budget {
          break;
        }
        let class = match Self::next_schedule_class(system_executions, user_executions) {
          Some(c) => c,
          None => break,
        };
        let aaa_id = match Self::pop_ready_for_class(class) {
          Some(id) => {
            class_misses = 0;
            id
          }
          None => {
            class_misses = class_misses.saturating_add(1);
            attempts = attempts.saturating_add(1);
            if class_misses >= 2 {
              break;
            }
            continue;
          }
        };
        attempts = attempts.saturating_add(1);
        let instance = match AaaInstances::<T>::get(aaa_id) {
          Some(inst) => inst,
          None => continue,
        };
        if instance.is_paused {
          continue;
        }
        if let Some(ref window) = instance.schedule_window {
          let now = frame_system::Pallet::<T>::block_number();
          if now > window.end {
            let _ = Self::do_terminal_refund(aaa_id, &instance, RefundReason::WindowExpired);
            continue;
          }
        }
        if !Self::is_ready_for_execution(&instance) {
          to_requeue.push(aaa_id);
          continue;
        }
        let estimated_cost = 5_000_000u64.saturating_mul(instance.pipeline.len() as u64 + 1);
        if consumed.saturating_add(estimated_cost) > budget {
          Self::deposit_event(Event::CycleDeferred {
            aaa_id,
            reason: DeferReason::InsufficientBudget,
          });
          if !Self::try_enqueue_deferred(aaa_id) {
            to_requeue.push(aaa_id);
          }
          continue;
        }
        // §3.2 — lazy rent charge before anything else
        let rent_result = AaaInstances::<T>::mutate(aaa_id, |maybe_inst| {
          let inst = match maybe_inst.as_mut() {
            Some(i) => i,
            None => return (Zero::zero(), Zero::zero()),
          };
          Self::charge_rent(inst)
        });
        if !rent_result.1.is_zero() {
          if let Some(inst) = AaaInstances::<T>::get(aaa_id) {
            let _ = Self::do_terminal_refund(aaa_id, &inst, RefundReason::RentInsolvent);
          }
          continue;
        }
        // §3.1 — MinUserBalance gate (User only)
        if instance.aaa_type == AaaType::User {
          let native = T::NativeAssetId::get();
          let balance = T::AssetOps::balance(
            &AaaInstances::<T>::get(aaa_id)
              .map(|i| i.sovereign_account)
              .unwrap_or(instance.sovereign_account.clone()),
            native,
          );
          let min_balance = T::MinUserBalance::get();
          if balance < min_balance {
            if let Some(inst) = AaaInstances::<T>::get(aaa_id) {
              let _ = Self::do_terminal_refund(aaa_id, &inst, RefundReason::BalanceExhausted);
            }
            continue;
          }
        }
        if instance.aaa_type == AaaType::User {
          // Pre-flight must run before nonce/failure mutation to keep deferral non-terminal
          let mut cycle_fee_upper = T::Balance::zero();
          for step in instance.pipeline.iter() {
            let eval_fee = Self::compute_eval_fee(step.conditions.len() as u32);
            cycle_fee_upper = cycle_fee_upper.saturating_add(eval_fee);
            if !matches!(step.task, TaskKind::Noop) {
              let exec_fee = T::WeightToFee::weight_to_fee(&Self::weight_upper_bound(&step.task));
              cycle_fee_upper = cycle_fee_upper.saturating_add(exec_fee);
            }
          }
          let native = T::NativeAssetId::get();
          let balance = T::AssetOps::balance(&instance.sovereign_account, native);
          if balance < cycle_fee_upper {
            Self::deposit_event(Event::CycleDeferred {
              aaa_id,
              reason: DeferReason::InsufficientBudget,
            });
            if !Self::try_enqueue_deferred(aaa_id) {
              to_requeue.push(aaa_id);
            }
            continue;
          }
        }
        let cycle_weight = Self::execute_single_cycle(aaa_id);
        consumed = consumed.saturating_add(cycle_weight.ref_time());
        total_consumed = total_consumed.saturating_add(cycle_weight);
        match class {
          AaaType::System => {
            system_executions = system_executions.saturating_add(1);
          }
          AaaType::User => {
            user_executions = user_executions.saturating_add(1);
          }
        }
        let still_active = AaaInstances::<T>::get(aaa_id)
          .map(|inst| !inst.is_paused)
          .unwrap_or(false);
        if still_active {
          // Defer requeue to end of loop — prevents double-firing within one block
          to_requeue.push(aaa_id);
        }
      }
      // Re-enqueue survivors after processing all ready actors
      for aaa_id in to_requeue {
        Self::requeue_to_ready_ring(aaa_id);
      }
      total_consumed
    }

    fn requeue_to_ready_ring(aaa_id: AaaId) {
      if !Self::try_enqueue_ready(aaa_id) {
        let _ = Self::try_enqueue_deferred(aaa_id);
      }
    }

    pub fn is_ready_for_execution(instance: &AaaInstanceOf<T>) -> bool {
      if instance.is_paused {
        return false;
      }
      if GlobalCircuitBreaker::<T>::get() {
        return false;
      }
      if let Some(ref window) = instance.schedule_window {
        let now = frame_system::Pallet::<T>::block_number();
        if now < window.start {
          return false;
        }
      }
      // nonce exhaustion is handled in execute_single_cycle, not here
      if instance.cycle_nonce > 0 && instance.cycle_nonce < u64::MAX {
        let now = frame_system::Pallet::<T>::block_number();
        let cooldown: BlockNumberFor<T> = instance.schedule.cooldown_blocks.into();
        if now.saturating_sub(instance.last_cycle_block) < cooldown {
          return false;
        }
      }
      Self::evaluate_trigger(instance)
    }

    fn evaluate_trigger(instance: &AaaInstanceOf<T>) -> bool {
      if instance.manual_trigger_pending {
        return true;
      }
      match &instance.schedule.trigger {
        Trigger::Manual => false,
        Trigger::ProbabilisticTimer {
          every_blocks,
          probability_ppm,
        } => Self::evaluate_probabilistic_timer(instance, *every_blocks, *probability_ppm),
        Trigger::OnAddressEvent { asset_filter, .. } => {
          Self::evaluate_on_address_event(instance.aaa_id, asset_filter)
        }
      }
    }

    fn evaluate_probabilistic_timer(
      instance: &AaaInstanceOf<T>,
      every_blocks: u32,
      probability_ppm: u32,
    ) -> bool {
      let now = frame_system::Pallet::<T>::block_number();
      let cadence: BlockNumberFor<T> = every_blocks.into();
      if cadence > Zero::zero() && now.saturating_sub(instance.last_cycle_block) < cadence {
        return false;
      }
      if probability_ppm >= 1_000_000 {
        return true;
      }
      if probability_ppm == 0 {
        return false;
      }
      let parent_hash = frame_system::Pallet::<T>::parent_hash();
      let seed = Self::mix_seed(parent_hash.as_ref(), instance.aaa_id);
      (seed % 1_000_000) < probability_ppm as u64
    }

    pub fn notify_address_event(aaa_id: AaaId, asset: T::AssetId, source: &T::AccountId) {
      let instance = match AaaInstances::<T>::get(aaa_id) {
        Some(inst) => inst,
        None => return,
      };
      let (asset_filter, source_filter) = match &instance.schedule.trigger {
        Trigger::OnAddressEvent {
          asset_filter,
          source_filter,
          ..
        } => (asset_filter, source_filter),
        _ => return,
      };
      if !Self::asset_matches_filter(&asset, asset_filter) {
        return;
      }
      let source_allowed = match source_filter {
        SourceFilter::Any => true,
        SourceFilter::OwnerOnly => source == &instance.owner,
        SourceFilter::RefundAddressOnly => source == &instance.refund_to,
        SourceFilter::Whitelist(list) => list.iter().any(|a| a == source),
      };
      if !source_allowed {
        return;
      }
      let now = frame_system::Pallet::<T>::block_number();
      let max_count = T::MaxAddressEventInboxCount::get();
      EventInbox::<T>::mutate(aaa_id, asset, |maybe_entry| {
        let entry = maybe_entry.get_or_insert_with(|| EventInboxEntry {
          pending_count: 0,
          saturated: false,
          last_event_block: now,
        });
        if entry.pending_count < max_count {
          entry.pending_count = entry.pending_count.saturating_add(1);
        } else {
          entry.saturated = true;
        }
        entry.last_event_block = now;
      });
    }

    fn evaluate_on_address_event(aaa_id: AaaId, filter: &AssetFilter<T::AssetId>) -> bool {
      for (asset, entry) in EventInbox::<T>::iter_prefix(aaa_id) {
        if Self::asset_matches_filter(&asset, filter)
          && (entry.pending_count > 0 || entry.saturated)
        {
          return true;
        }
      }
      false
    }

    fn asset_matches_filter(asset: &T::AssetId, filter: &AssetFilter<T::AssetId>) -> bool {
      match filter {
        AssetFilter::IncludeOnly(set) => set.iter().any(|a| a == asset),
        AssetFilter::Exclude(set) => !set.iter().any(|a| a == asset),
      }
    }

    fn consume_address_event(
      aaa_id: AaaId,
      filter: &AssetFilter<T::AssetId>,
      drain_mode: &InboxDrainMode,
    ) {
      let mut ready_key: Option<T::AssetId> = None;
      for (asset, entry) in EventInbox::<T>::iter_prefix(aaa_id) {
        if !Self::asset_matches_filter(&asset, filter) {
          continue;
        }
        if entry.pending_count > 0 || entry.saturated {
          match &ready_key {
            None => ready_key = Some(asset),
            Some(current) if asset.encode() < current.encode() => ready_key = Some(asset),
            _ => {}
          }
        }
      }
      if let Some(key) = ready_key {
        EventInbox::<T>::mutate(aaa_id, key, |maybe_entry| {
          let Some(entry) = maybe_entry else {
            return;
          };
          match drain_mode {
            InboxDrainMode::Single => {
              if entry.pending_count > 0 {
                entry.pending_count = entry.pending_count.saturating_sub(1);
              }
              if entry.pending_count == 0 && !entry.saturated {
                *maybe_entry = None;
              }
            }
            InboxDrainMode::Batch(max) => {
              if entry.saturated {
                *maybe_entry = None;
                return;
              }
              let batch = entry.pending_count.min(*max);
              entry.pending_count = entry.pending_count.saturating_sub(batch);
              if entry.pending_count == 0 {
                *maybe_entry = None;
              }
            }
            InboxDrainMode::Drain => {
              *maybe_entry = None;
            }
          }
        });
      }
    }

    fn mix_seed(hash_bytes: &[u8], aaa_id: AaaId) -> u64 {
      let mut acc: u64 = aaa_id.wrapping_mul(0x517cc1b727220a95);
      for (i, &byte) in hash_bytes.iter().take(8).enumerate() {
        acc ^= (byte as u64) << (i * 8);
      }
      acc ^= acc >> 33;
      acc = acc.wrapping_mul(0xff51afd7ed558ccd);
      acc ^= acc >> 33;
      acc
    }

    fn charge_rent(instance: &mut AaaInstanceOf<T>) -> (BalanceOf<T>, BalanceOf<T>) {
      if instance.aaa_type == AaaType::System {
        return (Zero::zero(), Zero::zero());
      }
      let now = frame_system::Pallet::<T>::block_number();
      let native = T::NativeAssetId::get();
      let rent_per_block = T::RentPerBlock::get();
      if rent_per_block.is_zero() {
        instance.last_rent_block = now;
        return (Zero::zero(), Zero::zero());
      }
      let blocks_elapsed: u32 = now
        .saturating_sub(instance.last_rent_block)
        .try_into()
        .unwrap_or(u32::MAX);
      if blocks_elapsed == 0 {
        return (Zero::zero(), Zero::zero());
      }
      let raw_rent = rent_per_block.saturating_mul(blocks_elapsed.into());
      let rent_due = raw_rent.min(T::MaxRentAccrual::get());
      let balance = T::AssetOps::balance(&instance.sovereign_account, native);
      let fee_sink = T::FeeSink::get();
      let (rent_paid, rent_debt) = if balance >= rent_due {
        if T::AssetOps::transfer(&instance.sovereign_account, &fee_sink, native, rent_due).is_ok() {
          (rent_due, Zero::zero())
        } else {
          (Zero::zero(), rent_due)
        }
      } else {
        (Zero::zero(), rent_due)
      };
      instance.last_rent_block = now;
      Self::deposit_event(Event::RentCharged {
        aaa_id: instance.aaa_id,
        blocks_elapsed,
        rent_due,
        rent_paid,
        rent_debt,
      });
      (rent_paid, rent_debt)
    }

    fn do_terminal_refund(
      aaa_id: AaaId,
      instance: &AaaInstanceOf<T>,
      reason: RefundReason,
    ) -> DispatchResult {
      let refund_to = instance.refund_to.clone();
      let actor = &instance.sovereign_account;
      let native = T::NativeAssetId::get();
      let fee_sink = T::FeeSink::get();
      let cost_per_transfer = T::RefundTransferCost::get();
      let num_assets: BalanceOf<T> = (instance.refund_assets.len() as u32).into();
      let threshold = cost_per_transfer.saturating_mul(num_assets);
      let native_balance = T::AssetOps::balance(actor, native);
      let mut refunded: alloc::vec::Vec<(T::AssetId, BalanceOf<T>)> = alloc::vec::Vec::new();
      let mut forfeited: alloc::vec::Vec<(T::AssetId, BalanceOf<T>)> = alloc::vec::Vec::new();
      let mut native_burned = BalanceOf::<T>::zero();
      let solvent = native_balance >= threshold;
      if solvent {
        for &asset in instance.refund_assets.iter() {
          let bal = T::AssetOps::balance(actor, asset);
          if !bal.is_zero() && T::AssetOps::transfer(actor, &refund_to, asset, bal).is_ok() {
            refunded.push((asset, bal));
          }
        }
      } else {
        // §4.2 insolvent: burn native dust, forfeit non-native to FeeSink
        if !native_balance.is_zero() {
          if T::AssetOps::burn(actor, native, native_balance).is_ok() {
            native_burned = native_balance;
          }
        }
        for &asset in instance.refund_assets.iter() {
          if asset == native {
            continue;
          }
          let bal = T::AssetOps::balance(actor, asset);
          if !bal.is_zero() {
            if T::AssetOps::transfer(actor, &fee_sink, asset, bal).is_ok() {
              forfeited.push((asset, bal));
            }
          }
        }
      }
      let assets_refunded = BoundedVec::truncate_from(refunded);
      let assets_forfeited = BoundedVec::truncate_from(forfeited);
      Self::deposit_event(Event::AAARefunded {
        aaa_id,
        reason,
        solvent,
        to: refund_to,
        assets_refunded,
        assets_forfeited,
        native_burned,
      });
      Self::remove_from_ready_ring(aaa_id);
      AaaInstances::<T>::remove(aaa_id);
      Self::remove_owner_index(&instance.owner, aaa_id);
      Self::remove_owner_slot_binding(
        &instance.owner,
        instance.owner_slot,
        &instance.sovereign_account,
      );
      Self::deposit_event(Event::AAADestroyed { aaa_id });

      Ok(())
    }

    fn execute_single_cycle(aaa_id: AaaId) -> Weight {
      let base_weight = T::DbWeight::get()
        .reads(1)
        .saturating_add(T::DbWeight::get().writes(1));
      let now = frame_system::Pallet::<T>::block_number();
      let instance = match AaaInstances::<T>::get(aaa_id) {
        Some(inst) => inst,
        None => return base_weight,
      };
      // §5.7 — nonce exhaustion check
      if instance.cycle_nonce == u64::MAX {
        if instance.aaa_type == AaaType::User {
          let _ = Self::do_terminal_refund(aaa_id, &instance, RefundReason::CycleNonceExhausted);
        } else {
          AaaInstances::<T>::mutate(aaa_id, |maybe| {
            if let Some(inst) = maybe.as_mut() {
              inst.is_paused = true;
              inst.pause_reason = Some(PauseReason::CycleNonceExhausted);
              inst.updated_at = now;
            }
          });
          Self::remove_from_ready_ring(aaa_id);
          Self::deposit_event(Event::AAAPaused {
            aaa_id,
            reason: PauseReason::CycleNonceExhausted,
          });
        }
        return base_weight;
      }
      let cycle_nonce = AaaInstances::<T>::mutate(aaa_id, |maybe| {
        let inst = maybe.as_mut().expect("instance verified above");
        inst.cycle_nonce = inst.cycle_nonce.saturating_add(1);
        inst.manual_trigger_pending = false;
        inst.last_cycle_block = now;
        inst.updated_at = now;
        inst.cycle_nonce
      });
      if let Trigger::OnAddressEvent {
        ref asset_filter,
        ref drain_mode,
        ..
      } = instance.schedule.trigger
      {
        Self::consume_address_event(aaa_id, asset_filter, drain_mode);
      }
      Self::deposit_event(Event::CycleStarted {
        aaa_id,
        cycle_nonce,
      });
      let actor = instance.sovereign_account.clone();
      let is_user = instance.aaa_type == AaaType::User;
      let pipeline = &instance.pipeline;
      let mut steps_executed: u32 = 0;
      let mut pipeline_failed = false;
      // Reserve full worst-case fees up-front so task amount resolution cannot steal fee budget
      let mut reserved_fee_remaining = T::Balance::zero();
      if is_user {
        for step in pipeline.iter() {
          let eval_fee = Self::compute_eval_fee(step.conditions.len() as u32);
          reserved_fee_remaining = reserved_fee_remaining.saturating_add(eval_fee);
          if !matches!(step.task, TaskKind::Noop) {
            let exec_fee = T::WeightToFee::weight_to_fee(&Self::weight_upper_bound(&step.task));
            reserved_fee_remaining = reserved_fee_remaining.saturating_add(exec_fee);
          }
        }
      }
      for (step_idx, step) in pipeline.iter().enumerate() {
        let step_num = step_idx as u32;
        // §3.3 — evaluation fee (User only)
        if is_user {
          let eval_fee = Self::compute_eval_fee(step.conditions.len() as u32);
          if !eval_fee.is_zero() {
            reserved_fee_remaining = reserved_fee_remaining.saturating_sub(eval_fee);
            let native = T::NativeAssetId::get();
            let balance = T::AssetOps::balance(&actor, native);
            let fee_sink = T::FeeSink::get();
            if balance < eval_fee {
              let err = DispatchError::from(Error::<T>::InsufficientEvaluationFee);
              Self::deposit_event(Event::StepFailed {
                aaa_id,
                cycle_nonce,
                step: step_num,
                error: err,
              });
              pipeline_failed =
                Self::apply_error_policy(aaa_id, cycle_nonce, step_num, step.on_error, err);
              if pipeline_failed {
                break;
              }
              continue;
            }
            if T::AssetOps::transfer(&actor, &fee_sink, native, eval_fee).is_err() {
              Self::deposit_event(Event::StepFailed {
                aaa_id,
                cycle_nonce,
                step: step_num,
                error: DispatchError::Other("EvaluationFeeTransferFailed"),
              });
              pipeline_failed = Self::apply_error_policy(
                aaa_id,
                cycle_nonce,
                step_num,
                step.on_error,
                DispatchError::Other("EvaluationFeeTransferFailed"),
              );
              if pipeline_failed {
                break;
              }
              continue;
            }
          }
        }
        // Evaluate conditions
        let condition_result = Self::evaluate_conditions(&step.conditions, &actor);
        match condition_result {
          Ok(true) => {}
          Ok(false) => {
            if is_user && !matches!(step.task, TaskKind::Noop) {
              // Condition skip means dispatch will not happen, so release reserved execution fee
              let skip_exec_fee =
                T::WeightToFee::weight_to_fee(&Self::weight_upper_bound(&step.task));
              reserved_fee_remaining = reserved_fee_remaining.saturating_sub(skip_exec_fee);
            }
            Self::deposit_event(Event::StepSkipped {
              aaa_id,
              cycle_nonce,
              step: step_num,
            });
            continue;
          }
          Err(e) => {
            if is_user && !matches!(step.task, TaskKind::Noop) {
              // Failed condition evaluation also skips dispatch, so reserved execution fee is released
              let skip_exec_fee =
                T::WeightToFee::weight_to_fee(&Self::weight_upper_bound(&step.task));
              reserved_fee_remaining = reserved_fee_remaining.saturating_sub(skip_exec_fee);
            }
            Self::deposit_event(Event::StepFailed {
              aaa_id,
              cycle_nonce,
              step: step_num,
              error: e,
            });
            pipeline_failed =
              Self::apply_error_policy(aaa_id, cycle_nonce, step_num, step.on_error, e);
            if pipeline_failed {
              break;
            }
            continue;
          }
        }
        // §3.4 — execution fee (User only, Noop exempt)
        let exec_fee = T::WeightToFee::weight_to_fee(&Self::weight_upper_bound(&step.task));
        if is_user && !matches!(step.task, TaskKind::Noop) {
          if !exec_fee.is_zero() {
            reserved_fee_remaining = reserved_fee_remaining.saturating_sub(exec_fee);
            let native = T::NativeAssetId::get();
            let balance = T::AssetOps::balance(&actor, native);
            let fee_sink = T::FeeSink::get();
            if balance < exec_fee {
              let err = DispatchError::from(Error::<T>::InsufficientExecutionFee);
              Self::deposit_event(Event::StepFailed {
                aaa_id,
                cycle_nonce,
                step: step_num,
                error: err,
              });
              pipeline_failed =
                Self::apply_error_policy(aaa_id, cycle_nonce, step_num, step.on_error, err);
              if pipeline_failed {
                break;
              }
              continue;
            }
            if T::AssetOps::transfer(&actor, &fee_sink, native, exec_fee).is_err() {
              Self::deposit_event(Event::StepFailed {
                aaa_id,
                cycle_nonce,
                step: step_num,
                error: DispatchError::Other("ExecutionFeeTransferFailed"),
              });
              pipeline_failed = Self::apply_error_policy(
                aaa_id,
                cycle_nonce,
                step_num,
                step.on_error,
                DispatchError::Other("ExecutionFeeTransferFailed"),
              );
              if pipeline_failed {
                break;
              }
              continue;
            }
          }
        }
        match Self::execute_task(
          &step.task,
          aaa_id,
          &actor,
          instance.aaa_type,
          reserved_fee_remaining,
        ) {
          Ok(()) => {
            steps_executed += 1;
          }
          Err(e) => {
            Self::deposit_event(Event::StepFailed {
              aaa_id,
              cycle_nonce,
              step: step_num,
              error: e,
            });
            pipeline_failed =
              Self::apply_error_policy(aaa_id, cycle_nonce, step_num, step.on_error, e);
            if pipeline_failed {
              break;
            }
          }
        }
      }
      if !pipeline_failed {
        Self::deposit_event(Event::PipelineExecuted {
          aaa_id,
          cycle_nonce,
          steps_executed,
        });
        AaaInstances::<T>::mutate(aaa_id, |maybe| {
          if let Some(inst) = maybe.as_mut() {
            inst.consecutive_failures = 0;
          }
        });
      } else {
        AaaInstances::<T>::mutate(aaa_id, |maybe| {
          if let Some(inst) = maybe.as_mut() {
            inst.consecutive_failures = inst.consecutive_failures.saturating_add(1);
          }
        });
        if let Some(inst) = AaaInstances::<T>::get(aaa_id) {
          if !inst.is_paused
            && T::MaxConsecutiveFailures::get() > 0
            && inst.consecutive_failures > T::MaxConsecutiveFailures::get()
          {
            let _ = Self::do_terminal_refund(aaa_id, &inst, RefundReason::ConsecutiveFailures);
          }
        }
      }
      base_weight.saturating_add(Weight::from_parts(
        5_000_000u64.saturating_mul(steps_executed as u64 + 1),
        1000u64.saturating_mul(steps_executed as u64 + 1),
      ))
    }

    fn compute_eval_fee(num_conditions: u32) -> BalanceOf<T> {
      let base = T::StepBaseFee::get();
      let per_cond = T::ConditionReadFee::get();
      base.saturating_add(per_cond.saturating_mul(num_conditions.into()))
    }

    fn apply_error_policy(
      aaa_id: AaaId,
      cycle_nonce: u64,
      step: u32,
      policy: PipelineErrorPolicy,
      error: DispatchError,
    ) -> bool {
      match policy {
        PipelineErrorPolicy::AbortCycle => {
          Self::deposit_event(Event::PipelineFailed {
            aaa_id,
            cycle_nonce,
            failed_step: step,
            error,
          });
          true
        }
        PipelineErrorPolicy::ContinueNextStep => false,
      }
    }

    fn execute_task(
      task: &TaskKind<T::AssetId, T::Balance, T::AccountId>,
      aaa_id: AaaId,
      actor: &T::AccountId,
      aaa_type: AaaType,
      reserved: T::Balance,
    ) -> DispatchResult {
      match task {
        TaskKind::Transfer { to, asset, amount } => {
          let resolved = Self::resolve_amount(amount, *asset, actor, reserved)?;
          T::AssetOps::transfer(actor, to, *asset, resolved)?;
          Self::deposit_event(Event::TransferExecuted {
            aaa_id,
            to: to.clone(),
            asset: *asset,
            amount: resolved,
          });
        }
        TaskKind::SplitTransfer {
          asset,
          amount,
          total_shares,
          legs,
          remainder_to,
        } => {
          Self::validate_split_transfer_legs(*total_shares, legs)?;
          let total = Self::resolve_amount(amount, *asset, actor, reserved)?;
          let total_u128: u128 = total.saturated_into();
          let mut distributed = T::Balance::zero();
          let mut leg_amounts: alloc::vec::Vec<T::Balance> =
            alloc::vec::Vec::with_capacity(legs.len());
          for leg in legs.iter() {
            let leg_amount_u128 = polkadot_sdk::sp_core::U256::from(total_u128)
              .saturating_mul(polkadot_sdk::sp_core::U256::from(leg.share))
              .checked_div(polkadot_sdk::sp_core::U256::from(*total_shares))
              .unwrap_or(polkadot_sdk::sp_core::U256::zero())
              .try_into()
              .unwrap_or(u128::MAX);
            let leg_amount: T::Balance = leg_amount_u128.saturated_into();
            leg_amounts.push(leg_amount);
            distributed = distributed.saturating_add(leg_amount);
          }
          let remainder = total.saturating_sub(distributed);
          let actor_balance = T::AssetOps::balance(actor, *asset);
          ensure!(actor_balance >= total, Error::<T>::InsufficientBalance);
          for (i, leg) in legs.iter().enumerate() {
            T::AssetOps::transfer(actor, &leg.to, *asset, leg_amounts[i])?;
          }
          if !remainder.is_zero() {
            let fallback = legs.first().map(|l| l.to.clone());
            let receiver = remainder_to
              .clone()
              .or(fallback)
              .ok_or(Error::<T>::SplitTransferInvalid)?;
            T::AssetOps::transfer(actor, &receiver, *asset, remainder)?;
          }
          Self::deposit_event(Event::SplitTransferExecuted {
            aaa_id,
            asset: *asset,
            total,
            legs_count: legs.len() as u32,
          });
        }
        TaskKind::Burn { asset, amount } => {
          let resolved = Self::resolve_amount(amount, *asset, actor, reserved)?;
          T::AssetOps::burn(actor, *asset, resolved)?;
          Self::deposit_event(Event::BurnExecuted {
            aaa_id,
            asset: *asset,
            amount: resolved,
          });
        }
        TaskKind::Mint { asset, amount } => {
          ensure!(
            aaa_type == AaaType::System,
            Error::<T>::MintNotAllowedForUserAaa
          );
          let resolved = Self::resolve_amount(amount, *asset, actor, reserved)?;
          T::AssetOps::mint(actor, *asset, resolved)?;
          Self::deposit_event(Event::MintExecuted {
            aaa_id,
            asset: *asset,
            amount: resolved,
          });
        }
        TaskKind::SwapExactIn {
          asset_in,
          asset_out,
          amount_in,
          min_out,
        } => {
          let resolved_in = Self::resolve_amount(amount_in, *asset_in, actor, reserved)?;
          let amount_out =
            T::DexOps::swap_exact_in(actor, *asset_in, *asset_out, resolved_in, *min_out)?;
          Self::deposit_event(Event::SwapExecuted {
            aaa_id,
            asset_in: *asset_in,
            asset_out: *asset_out,
            amount_in: resolved_in,
            amount_out,
          });
        }
        TaskKind::SwapExactOut {
          asset_in,
          asset_out,
          amount_out,
          max_in,
        } => {
          let consumed_in =
            T::DexOps::swap_exact_out(actor, *asset_in, *asset_out, *amount_out, *max_in)?;
          Self::deposit_event(Event::SwapExecuted {
            aaa_id,
            asset_in: *asset_in,
            asset_out: *asset_out,
            amount_in: consumed_in,
            amount_out: *amount_out,
          });
        }
        TaskKind::AddLiquidity {
          asset_a,
          asset_b,
          amount_a,
          amount_b,
        } => {
          let resolved_a = Self::resolve_amount(amount_a, *asset_a, actor, reserved)?;
          let resolved_b = Self::resolve_amount(amount_b, *asset_b, actor, reserved)?;
          let (used_a, used_b, lp_minted) =
            T::DexOps::add_liquidity(actor, *asset_a, *asset_b, resolved_a, resolved_b)?;
          Self::deposit_event(Event::LiquidityAdded {
            aaa_id,
            asset_a: *asset_a,
            asset_b: *asset_b,
            amount_a: used_a,
            amount_b: used_b,
            lp_minted,
          });
        }
        TaskKind::RemoveLiquidity { lp_asset, amount } => {
          let resolved = Self::resolve_amount(amount, *lp_asset, actor, reserved)?;
          let (out_a, out_b) = T::DexOps::remove_liquidity(actor, *lp_asset, resolved)?;
          Self::deposit_event(Event::LiquidityRemoved {
            aaa_id,
            lp_asset: *lp_asset,
            amount: resolved,
            amount_a_out: out_a,
            amount_b_out: out_b,
          });
        }
        TaskKind::Noop => {}
      }
      Ok(())
    }

    fn execute_zombie_sweep() -> Weight {
      let max_check = T::MaxSweepPerBlock::get();
      let max_id = NextAaaId::<T>::get();
      if max_id == 0 {
        return Weight::zero();
      }
      let mut cursor = SweepCursor::<T>::get();
      let mut checked = 0u32;
      let mut sweep_weight = Weight::zero();
      while checked < max_check {
        cursor = (cursor + 1) % max_id;
        SweepCursor::<T>::put(cursor);
        if let Some(mut instance) = AaaInstances::<T>::get(cursor) {
          if !instance.is_paused {
            if let Some(ref window) = instance.schedule_window {
              let now = frame_system::Pallet::<T>::block_number();
              if now > window.end {
                let _ = Self::do_terminal_refund(cursor, &instance, RefundReason::WindowExpired);
                checked = checked.saturating_add(1);
                sweep_weight = sweep_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
                continue;
              }
            }
            let (paid, debt) = Self::charge_rent(&mut instance);
            if !debt.is_zero() {
              AaaInstances::<T>::insert(cursor, &instance);
              let _ = Self::do_terminal_refund(cursor, &instance, RefundReason::RentInsolvent);
              checked = checked.saturating_add(1);
              sweep_weight = sweep_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
              continue;
            }
            if instance.aaa_type == AaaType::User {
              let native = T::NativeAssetId::get();
              let balance = T::AssetOps::balance(&instance.sovereign_account, native);
              if balance < T::MinUserBalance::get() {
                if !paid.is_zero() {
                  AaaInstances::<T>::insert(cursor, &instance);
                }
                let _ = Self::do_terminal_refund(cursor, &instance, RefundReason::BalanceExhausted);
                checked = checked.saturating_add(1);
                sweep_weight = sweep_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
                continue;
              }
            }
            if !paid.is_zero() {
              AaaInstances::<T>::insert(cursor, &instance);
            }
          }
        }
        checked = checked.saturating_add(1);
        sweep_weight = sweep_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
      }
      sweep_weight
    }

    /// §8.7 — evaluate liveness for a single actor
    fn evaluate_actor_liveness(aaa_id: AaaId) -> DispatchResult {
      let mut instance = AaaInstances::<T>::get(aaa_id).ok_or(Error::<T>::AaaNotFound)?;
      if !instance.is_paused {
        if let Some(ref window) = instance.schedule_window {
          let now = frame_system::Pallet::<T>::block_number();
          if now > window.end {
            return Self::do_terminal_refund(aaa_id, &instance, RefundReason::WindowExpired);
          }
        }
      }
      if !instance.is_paused && instance.aaa_type == AaaType::User {
        let (_paid, debt) = Self::charge_rent(&mut instance);
        if !debt.is_zero() {
          AaaInstances::<T>::insert(aaa_id, &instance);
          return Self::do_terminal_refund(aaa_id, &instance, RefundReason::RentInsolvent);
        }
        if !_paid.is_zero() {
          AaaInstances::<T>::insert(aaa_id, &instance);
        }
        let native = T::NativeAssetId::get();
        let balance = T::AssetOps::balance(&instance.sovereign_account, native);
        if balance < T::MinUserBalance::get() {
          return Self::do_terminal_refund(aaa_id, &instance, RefundReason::BalanceExhausted);
        }
      }
      Ok(())
    }

    fn evaluate_conditions(
      conditions: &BoundedVec<Condition<T::AssetId, T::Balance>, T::MaxConditionsPerStep>,
      who: &T::AccountId,
    ) -> Result<bool, DispatchError> {
      for cond in conditions.iter() {
        let pass = match cond {
          Condition::BalanceAbove { asset, threshold } => {
            T::AssetOps::balance(who, *asset) > *threshold
          }
          Condition::BalanceBelow { asset, threshold } => {
            T::AssetOps::balance(who, *asset) < *threshold
          }
          Condition::BalanceEquals { asset, threshold } => {
            T::AssetOps::balance(who, *asset) == *threshold
          }
          Condition::BalanceNotEquals { asset, threshold } => {
            T::AssetOps::balance(who, *asset) != *threshold
          }
        };
        if !pass {
          return Ok(false);
        }
      }
      Ok(true)
    }

    fn resolve_amount(
      spec: &AmountSpec<T::Balance>,
      asset: T::AssetId,
      who: &T::AccountId,
      reserved: T::Balance,
    ) -> Result<T::Balance, DispatchError> {
      let balance = T::AssetOps::balance(who, asset);
      // Native spend is bounded by remaining fee reserve to keep tail-step fee solvency deterministic
      let spendable = if asset == T::NativeAssetId::get() {
        balance.saturating_sub(reserved)
      } else {
        balance
      };
      let resolved = match spec {
        AmountSpec::Fixed(amount) => *amount,
        AmountSpec::AllBalance => spendable,
        AmountSpec::Percentage(pct) => pct.mul_floor(spendable),
      };
      ensure!(!resolved.is_zero(), Error::<T>::AmountZero);
      Ok(resolved)
    }
  }
}
