# Каталог сценариев использования AAA (RU)

> **Source of Truth** для реализации и тестирования `pallet-aaa`.
> Документ содержит **только сценарии**: что запускаем, зачем, как конфигурируем, что обязаны проверить.

## 0. Правила использования каталога

1. Каждый новый продуктовый/технический use-case AAA должен быть добавлен сюда с новым `SC-*` или `MESH-*` ID.
2. Любой PR, меняющий поведение AAA, обязан указать затронутые сценарии из этого каталога.
3. Для каждого сценария должны существовать тесты соответствующего уровня (`unit` / `integration` / `benchmark`).
4. Маркер источника:
   - `[SPEC]` — сценарий прямо следует из `docs/aaa-specification.md`.
   - `[EXT]` — расширенный сценарий с практическим обоснованием (без противоречия спецификации).

---

## 1. Матрица покрытия примитивов (чтобы не было «дыр»)

| Примитив                                                               | Покрывающие сценарии                                                    |
| ---------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `AaaType::User`                                                        | SC-001, SC-002, SC-003, SC-004, SC-006, SC-007, SC-016..SC-024          |
| `AaaType::System`                                                      | SC-005, SC-008, SC-009, SC-010, SC-011, SC-015, SC-023, SC-024, MESH-\* |
| `Mutability::Mutable`                                                  | SC-001, SC-002, SC-004, SC-005, SC-006, SC-007, SC-010, SC-011          |
| `Mutability::Immutable`                                                | SC-003, SC-015                                                          |
| Trigger `Manual`                                                       | SC-001, SC-005, SC-006, SC-007, SC-008, SC-011                          |
| Trigger `ProbabilisticTimer`                                           | SC-002, SC-004, SC-009                                                  |
| Trigger `OnAddressEvent`                                               | SC-010, SC-012, SC-013, SC-014, MESH-001..MESH-004                      |
| `InboxDrainMode::Single`                                               | SC-012                                                                  |
| `InboxDrainMode::Batch(n)`                                             | SC-013                                                                  |
| `InboxDrainMode::Drain`                                                | SC-010, SC-014                                                          |
| `AssetFilter::IncludeOnly`                                             | SC-012                                                                  |
| `AssetFilter::Exclude`                                                 | SC-013                                                                  |
| `SourceFilter::Any`                                                    | SC-010, SC-013                                                          |
| `SourceFilter::OwnerOnly`                                              | SC-012                                                                  |
| `SourceFilter::RefundAddressOnly`                                      | SC-014                                                                  |
| `SourceFilter::Whitelist`                                              | SC-014                                                                  |
| `AmountSpec::Fixed`                                                    | SC-001, SC-006, SC-007, SC-008                                          |
| `AmountSpec::AllBalance`                                               | SC-004, SC-005, SC-009, SC-010                                          |
| `AmountSpec::Percentage`                                               | SC-002, SC-011                                                          |
| `TaskKind::Transfer`                                                   | SC-001, SC-003, SC-004, SC-011                                          |
| `TaskKind::SplitTransfer`                                              | SC-005, SC-011                                                          |
| `TaskKind::SwapExactIn`                                                | SC-002, SC-006                                                          |
| `TaskKind::SwapExactOut`                                               | SC-007                                                                  |
| `TaskKind::AddLiquidity`                                               | SC-008, SC-009                                                          |
| `TaskKind::RemoveLiquidity`                                            | SC-009                                                                  |
| `TaskKind::Burn`                                                       | SC-010                                                                  |
| `TaskKind::Mint`                                                       | SC-011                                                                  |
| `TaskKind::Noop`                                                       | SC-015                                                                  |
| `PipelineErrorPolicy::AbortCycle`                                      | SC-016                                                                  |
| `PipelineErrorPolicy::ContinueNextStep`                                | SC-017                                                                  |
| `DeferReason::InsufficientBudget`                                      | SC-018                                                                  |
| `DeferReason::QueueOverflow`                                           | SC-019                                                                  |
| `RefundReason::OwnerInitiated`                                         | SC-020                                                                  |
| `RefundReason::RentInsolvent`                                          | SC-021                                                                  |
| `RefundReason::BalanceExhausted`                                       | SC-022                                                                  |
| `RefundReason::ConsecutiveFailures`                                    | SC-023                                                                  |
| `RefundReason::WindowExpired`                                          | SC-003, SC-024                                                          |
| `RefundReason::CycleNonceExhausted`                                    | SC-024                                                                  |
| Детерминизм адаптера (канонический порядок + фиксированное округление) | SC-027                                                                  |
| Rent списывается один раз при double-touch в одном блоке               | SC-028                                                                  |

---

## 2. Одиночные сценарии AAA (SC-\*)

### SC-001 — Базовый ручной перевод User AAA `[SPEC]`

**Зачем:** минимальный happy-path для `Transfer`.

```yaml
aaa_type: User
mutability: Mutable
schedule: { trigger: Manual, cooldown_blocks: 0 }
schedule_window: null
policy: { default_error_policy: AbortCycle }
pipeline:
  - conditions: []
    task:
      { Transfer: { to: BOB, asset: Native, amount: { Fixed: 1_000_000_000 } } }
refund_to: ALICE
```

**Проверки:** `CycleStarted`, `PipelineExecuted`, баланс BOB вырос.

---

### SC-002 — User DCA (таймер + процент) `[SPEC]`

**Зачем:** периодический DCA-паттерн из спецификации.

```yaml
aaa_type: User
mutability: Mutable
schedule:
  trigger:
    { ProbabilisticTimer: { every_blocks: 10, probability_ppm: 1_000_000 } }
  cooldown_blocks: 10
pipeline:
  - conditions: []
    task:
      SwapExactIn:
        asset_in: Stable
        asset_out: Target
        amount_in: { Percentage: "20%" }
        min_out: 1
```

**Проверки:** не чаще cooldown; повторяемый swap при готовности.

---

### SC-003 — Timelock Transfer (Immutable + Window) `[SPEC]`

**Зачем:** гарантированное окно исполнения и авто-терминал после `end`.

```yaml
aaa_type: User
mutability: Immutable
schedule: { trigger: Manual, cooldown_blocks: 0 }
schedule_window: { start: 10_000, end: 11_000 }
pipeline:
  - conditions: []
    task:
      {
        Transfer:
          { to: Beneficiary, asset: Native, amount: { Fixed: 5_000_000_000 } },
      }
```

**Проверки:** до `start` не исполняется; после `end` → `WindowExpired` + destroy.

---

### SC-004 — Revocable Payroll `[SPEC]`

**Зачем:** повторяющиеся выплаты, управляемые владельцем.

```yaml
aaa_type: User
mutability: Mutable
schedule:
  trigger:
    { ProbabilisticTimer: { every_blocks: 600, probability_ppm: 1_000_000 } }
  cooldown_blocks: 600
pipeline:
  - conditions: []
    task:
      {
        Transfer: { to: Employee, asset: Native, amount: { AllBalance: true } },
      }
```

**Проверки:** pause/resume/update работают; immutable-ограничения не применяются.

---

### SC-005 — SplitTransfer казначейства `[SPEC]`

**Зачем:** fan-out распределение с детерминированным remainder.

```yaml
aaa_type: System
mutability: Mutable
schedule: { trigger: Manual, cooldown_blocks: 0 }
pipeline:
  - task:
      SplitTransfer:
        asset: Native
        amount: { AllBalance: true }
        total_shares: 10
        legs:
          - { to: BurnPot, share: 5 }
          - { to: RewardsPot, share: 3 }
          - { to: OpsPot, share: 2 }
        remainder_to: BurnPot
```

**Проверки:** `sum(legs)==total_shares`; duplicate/zero-share запрещены.

---

### SC-006 — SwapExactIn ребаланс `[SPEC]`

**Зачем:** контролируемый swap по input.

```yaml
aaa_type: User
schedule: { trigger: Manual, cooldown_blocks: 0 }
pipeline:
  - task:
      SwapExactIn:
        asset_in: Native
        asset_out: ForeignUSDC
        amount_in: { Fixed: 1_000_000_000 }
        min_out: 1
```

**Проверки:** route через адаптер детерминирован; `min_out` соблюдается.

---

### SC-007 — SwapExactOut точный выход `[SPEC]`

**Зачем:** фиксированный target output с ограничением `max_in`.

```yaml
aaa_type: User
schedule: { trigger: Manual, cooldown_blocks: 0 }
pipeline:
  - task:
      SwapExactOut:
        asset_in: Native
        asset_out: ForeignUSDC
        amount_out: 500_000_000
        max_in: 1_000_000_000
```

**Проверки:** превышение `max_in` даёт step failure по policy.

---

### SC-008 — AddLiquidity (двусторонний) `[SPEC]`

**Зачем:** LP-провиженинг в одну операцию.

```yaml
aaa_type: System
schedule: { trigger: Manual, cooldown_blocks: 0 }
pipeline:
  - task:
      AddLiquidity:
        asset_a: Native
        asset_b: ForeignUSDC
        amount_a: { Fixed: 10_000_000_000 }
        amount_b: { Fixed: 10_000_000_000 }
```

**Проверки:** LP появляется; bounded behavior адаптера.

---

### SC-009 — RemoveLiquidity + повторный вход `[EXT]`

**Зачем:** controlled unwind LP и повторное размещение.

```yaml
aaa_type: System
schedule:
  trigger:
    { ProbabilisticTimer: { every_blocks: 1200, probability_ppm: 1_000_000 } }
  cooldown_blocks: 1200
pipeline:
  - task:
      {
        RemoveLiquidity:
          { lp_asset: LP_NATIVE_USDC, amount: { AllBalance: true } },
      }
  - conditions:
      - { BalanceAbove: { asset: Native, threshold: 1_000 } }
      - { BalanceAbove: { asset: ForeignUSDC, threshold: 1_000 } }
    task:
      {
        AddLiquidity:
          {
            asset_a: Native,
            asset_b: ForeignUSDC,
            amount_a: { AllBalance: true },
            amount_b: { AllBalance: true },
          },
      }
```

**Проверки:** path bounded по `MaxK`; fee/weight upper-bound корректен.

---

### SC-010 — Burn Actor (event-driven drain) `[SPEC]`

**Зачем:** пассивное сжигание входящих средств.

```yaml
aaa_type: System
schedule:
  trigger:
    OnAddressEvent:
      asset_filter: { IncludeOnly: [Native] }
      source_filter: Any
      drain_mode: Drain
  cooldown_blocks: 0
pipeline:
  - task: { Burn: { asset: Native, amount: { AllBalance: true } } }
```

**Проверки:** inbox saturation не ломает обработку; при pending>0 actor ready.

---

### SC-011 — Mint (только System) `[SPEC]`

**Зачем:** сервисная эмиссия для системных процессов.

```yaml
aaa_type: System
schedule: { trigger: Manual, cooldown_blocks: 0 }
pipeline:
  - task: { Mint: { asset: LocalReward, amount: { Percentage: "5%" } } }
  - task:
      {
        Transfer:
          { to: RewardsPot, asset: LocalReward, amount: { AllBalance: true } },
      }
```

**Проверки:** User AAA с `Mint` отклоняется при create; System — исполняется.

---

### SC-012 — OnAddressEvent + IncludeOnly + OwnerOnly `[SPEC]`

**Зачем:** реакция только на нужный asset и источник.

```yaml
trigger:
  OnAddressEvent:
    asset_filter: { IncludeOnly: [Native] }
    source_filter: OwnerOnly
    drain_mode: Single
```

**Проверки:** события от не-owner игнорируются; Single уменьшает `pending_count` на 1.

---

### SC-013 — OnAddressEvent + Exclude + Batch `[SPEC]`

**Зачем:** пакетная обработка всех активов, кроме исключений.

```yaml
trigger:
  OnAddressEvent:
    asset_filter: { Exclude: [LP_NATIVE_USDC] }
    source_filter: Any
    drain_mode: { Batch: 5 }
```

**Проверки:** `Batch(max)` валиден только при `0 < max <= MaxAddressEventInboxCount`.

---

### SC-014 — RefundAddressOnly / Whitelist source filters `[SPEC]`

**Зачем:** ограничение trust-domain для address events.

```yaml
trigger:
  OnAddressEvent:
    asset_filter: { IncludeOnly: [Native, ForeignUSDC] }
    source_filter: { Whitelist: [Treasury, Router] }
    drain_mode: Drain
```

**Проверки:** только whitelist источники приводят к готовности.

---

### SC-015 — Observation Actor (Noop-only) `[SPEC]`

**Зачем:** дешёвый мониторинг условий без side-effects.

```yaml
aaa_type: System
mutability: Immutable
schedule: { trigger: Manual, cooldown_blocks: 0 }
pipeline:
  - conditions:
      - { BalanceBelow: { asset: Native, threshold: 1_000_000 } }
    task: { Noop: {} }
```

**Проверки:** только события цикла/skip; балансы не меняются.

---

### SC-016 — Error policy `AbortCycle` `[SPEC]`

**Зачем:** fail-fast пайплайн.

```yaml
policy: { default_error_policy: AbortCycle }
pipeline:
  - task: { Transfer: { to: BOB, asset: LocalA, amount: { Fixed: huge } } } # ожидаемо fail
  - task: { Transfer: { to: BOB, asset: Native, amount: { Fixed: 1_000 } } }
```

**Проверки:** шаг 2 не выполняется; `PipelineFailed` emitted.

---

### SC-017 — Error policy `ContinueNextStep` `[SPEC]`

**Зачем:** частичный успех в multi-step цепочке.

```yaml
pipeline:
  - on_error: ContinueNextStep
    task: { Transfer: { to: BOB, asset: LocalA, amount: { Fixed: huge } } } # fail
  - task: { Transfer: { to: BOB, asset: Native, amount: { Fixed: 1_000 } } } # success
```

**Проверки:** шаг 2 выполняется несмотря на fail шага 1.

---

### SC-018 — Deferral по `InsufficientBudget` `[SPEC]`

**Зачем:** admission не должен стартовать цикл при нехватке budget/fee.

```yaml
schedule: { trigger: Manual, cooldown_blocks: 0 }
# runtime call: on_idle(remaining_weight = very_small)
```

**Проверки:** `cycle_nonce` не растёт, `manual_trigger_pending` сохраняется.

---

### SC-019 — Deferral по `QueueOverflow` `[SPEC]`

**Зачем:** bounded queues без unbounded push.

```yaml
# setup: создать > MaxReadyRingLength акторов
# ожидание: overflow уходит в DeferredRing c reason QueueOverflow
```

**Проверки:** ready/deferred не превышают Max\* bounds.

---

### SC-020 — OwnerInitiated close `[SPEC]`

**Зачем:** штатный ручной shutdown с refund.

```yaml
call: refund_and_close(owner, aaa_id)
```

**Проверки:** `AAARefunded` + `AAADestroyed`, индексы и owner-slot освобождены.

---

### SC-021 — `RentInsolvent` terminal `[SPEC]`

**Зачем:** не держать «зомби»-акторы без rent покрытия.

```yaml
# setup: actor native_balance < rent_due
# touch path: on_idle / permissionless_sweep
```

**Проверки:** немедленный terminal refund/destroy.

---

### SC-022 — `BalanceExhausted` terminal `[SPEC]`

**Зачем:** enforce `MinUserBalance` pre-cycle.

```yaml
# setup: native_balance < MinUserBalance
```

**Проверки:** destroy до старта шага 0.

---

### SC-023 — `ConsecutiveFailures` terminal (важно: `limit+1`) `[SPEC]`

**Зачем:** защита от бесконечно failing actor.

```yaml
MaxConsecutiveFailures: 10
# actor умирает на 11-й подряд cycle failure
```

**Проверки:** при `==10` ещё жив; при `==11` terminal.

---

### SC-024 — `WindowExpired` + `CycleNonceExhausted` lifecycle `[SPEC]`

**Зачем:** полный контроль terminal/pause веток.

```yaml
# A) schedule_window.end < now -> WindowExpired destroy
# B) cycle_nonce == u64::MAX:
#    User -> destroy (CycleNonceExhausted)
#    System -> pause (CycleNonceExhausted)
```

**Проверки:** user/system ветки различаются строго по спецификации.

---

### SC-025 — Global circuit breaker `[SPEC]`

**Зачем:** аварийная остановка execution plane.

```yaml
set_global_circuit_breaker(true)
```

**Проверки:**

- create + execution блокируются,
- `fund_aaa`, `refund_and_close`, `permissionless_sweep` остаются живы.

---

### SC-026 — Immutable guardrails `[SPEC]`

**Зачем:** owner не может «подвинуть» immutable actor после создания.

```yaml
mutability: Immutable
```

**Проверки:** запрет update/pause/resume/schedule-change, при этом fund/refund разрешены.

---

### SC-027 — Детерминизм DEX-адаптера `[SPEC+EXT]`

**Зачем:** защита от консенсусных расхождений при storage-итерациях и округлениях.

```yaml
# Поведенческий контракт сценария:
# - любые O(K) итерации по storage в адаптере идут в каноническом порядке
# - округление в числовых преобразованиях фиксировано (например, floor)
# - повторный запуск на одинаковом state -> бит-в-бит одинаковый результат
```

**Проверки:** одинаковые входы/состояние дают одинаковый выход и одинаковые side-effects.

---

### SC-028 — Rent single-charge в одном блоке (double-touch) `[SPEC+EXT]`

**Зачем:** не допустить двойного списания rent, если actor трогают в `on_initialize` и `on_idle` одного блока.

```yaml
# setup:
# block = N
# touch #1: readiness path (on_initialize / equivalent touch)
# touch #2: execute/sweep path в этом же block N
# ожидание: второй touch не списывает дополнительный rent
```

**Проверки:** `last_rent_block` не допускает второго списания при `blocks_elapsed = 0`.

---

## 3. Сценарии связанности (MESH-\*) — пайплайны из нескольких AAA

### MESH-001 — Protocol Fee Mesh `[SPEC+EXT]`

**Идея:** разделение комиссии на сжигание и награды через отдельные AAA.

```text
Upstream fees -> AAA-FeeSink (OnAddressEvent/Drain, SplitTransfer 50/50)
  -> AAA-Burner sovereign (OnAddressEvent/Drain, Burn AllBalance)
  -> AAA-Rewards sovereign (OnAddressEvent/Batch, Transfer -> StakingPot)
```

**Мини-конфиг:**

- `AAA-FeeSink`: System, Drain, SplitTransfer(AllBalance)
- `AAA-Burner`: System, Drain, Burn(AllBalance)
- `AAA-Rewards`: System, Batch(10), Transfer(Fixed/AllBalance)

**Проверки:** нет петли, нет потери суммы, каждый актор bounded.

---

### MESH-002 — Liquidity Flywheel Mesh `[SPEC+EXT]`

**Идея:** автоматический цикл «сбор -> swap/add-liquidity -> LP-distribution -> burn dust».

```text
AAA-Collector (OnAddressEvent) -> Transfer -> AAA-Zap sovereign
AAA-Zap (Manual/Timer) -> AddLiquidity / SplitTransfer(LP)
AAA-DustBurn (OnAddressEvent/Drain) -> Burn(AllBalance Native)
```

**Проверки:** ветвление условий детерминировано, MaxK bounded, queue bounds соблюдены.

---

### MESH-003 — Payroll + Tax + Treasury Mesh `[EXT]`

**Идея:** payroll разделяется на зарплату и налог/treasury поток.

```text
AAA-Payroll(User Timer) -> SplitTransfer(Native)
  -> Employee
  -> AAA-Tax sovereign (OnAddressEvent/Batch)
AAA-Tax(System) -> Transfer -> TreasuryPot
```

**Проверки:** доли + remainder стабильны, налоговый актор не обрабатывает чужие source.

---

### MESH-004 — Event-driven Liquidation Mesh `[EXT]`

**Идея:** входящие проблемные активы автоматически конвертируются и сжигаются.

```text
AAA-RiskCollector (OnAddressEvent IncludeOnly[RiskAsset])
  -> SwapExactIn(RiskAsset -> Native)
  -> Transfer -> AAA-Burner sovereign
AAA-Burner -> Burn(AllBalance Native)
```

**Проверки:** deterministic route/rounding в адаптере, backpressure bounded.

---

### MESH-005 — Emergency Safe-Mode Mesh `[EXT]`

**Идея:** при breaker остаются только cleanup-ветки.

```text
Breaker ON:
  Execution AAAs: halted
  Cleanup AAAs (manual refund/sweep lanes): active
```

**Проверки:** governance может безопасно «осушить» и закрыть акторы без запуска обычных циклов.

---

## 4. Минимум, который обязателен к реализации и тестированию

Для релизного статуса AAA обязателен зелёный контур по:

1. **Все SC-001..SC-028** (unit+integration в зависимости от природы сценария).
2. **MESH-001 и MESH-002** минимум как integration/e2e (остальные MESH — минимум design+tests roadmap).
3. Bench/weights для bounded-path сценариев (`QueueOverflow`, deferred retry, adapter MaxK).
4. Документ и тесты синхронны: если сценарий добавлен/изменён здесь, покрытие обновляется в том же PR.

---

## 5. Политика эволюции каталога

- Любой новый Task/Trigger/Policy или новая lifecycle-ветка **обязана** получить:
  1. новый `SC-*` сценарий,
  2. пример конфига,
  3. требуемый уровень теста,
  4. при необходимости `MESH-*` пример связанности.
- Если поведение меняется, старый сценарий не удаляется молча: он либо мигрирует, либо помечается как deprecated с указанием версии спецификации.

---

## 6. Привязка сценариев к тестовым целям (обязательные target-файлы)

| Сценарии                                             | Уровень                       | Целевые файлы                                                                              |
| ---------------------------------------------------- | ----------------------------- | ------------------------------------------------------------------------------------------ |
| SC-001..SC-017                                       | unit + integration            | `template/pallets/aaa/src/tests.rs`, `template/runtime/src/tests/aaa_integration_tests.rs` |
| SC-018..SC-028                                       | unit + integration            | `template/pallets/aaa/src/tests.rs`, `template/runtime/src/tests/aaa_integration_tests.rs` |
| bounded-path сценарии (`SC-018`, `SC-019`, `SC-009`) | benchmark + runtime-benchmark | `template/pallets/aaa/src/benchmarking.rs`, runtime weights review                         |
| MESH-001..MESH-002                                   | integration/e2e               | `template/runtime/src/tests/aaa_integration_tests.rs` (+ cross-pallet integration suite)   |
| MESH-003..MESH-005                                   | integration roadmap           | runtime integration test backlog с обязательной фиксацией в test-matrix                    |

> Любой сценарий без теста в целевом файле считается незавершённой реализацией сценария.
