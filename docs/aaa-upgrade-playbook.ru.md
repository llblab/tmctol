# AAA Upgrade Playbook

> Нерелизный операционный playbook для инкрементальной эволюции AAA

## Принципы

- Инкрементальные изменения малыми шагами
- Каждый шаг закрывается тестами и benchmark-обновлением
- Никаких скрытых semantic shifts без version note

## Шаги апгрейда

### Step 1 — Spec-first
- Обновить спецификацию и changelog
- Обновить матрицу `docs/aaa-spec-test-matrix.ru.md`

### Step 2 — Compat layer
- Добавить новые поля/enum/конфиг в совместимом режиме
- Ввести feature flags для staged activation

### Step 3 — Runtime migration
- Реализовать storage migration (если требуется)
- Добавить pre/post migration checks

### Step 3A — Owner-slot sovereign rollout (если активируется)
- Добавить `owner_slot` и `OwnerSlots`/`SovereignIndex`
- Включить bounded first-free scan (`slot=0..MaxOwnerSlots-1`)
- Проверить формулу derivation: `hash(owner + b"aaa" + slot)`

### Step 4 — Execution switch
- Включить новую семантику через governance-controlled параметр
- Наблюдать метрики в ограниченном окне

### Step 5 — Finalization
- Удалить legacy fallback paths
- Обновить benchmarks/weights
- Закрыть release-gate в CI

## Обязательные проверки перед активацией

- `cargo check --workspace`
- `cargo test`
- `cargo test --features runtime-benchmarks`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Governance checklist

- Параметры конфигурации заданы явно
- Rollback-план зафиксирован
- Временное окно активации согласовано
- Пост-активационный мониторинг включен
