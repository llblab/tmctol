# AAA Next Hardening Plan (v0.43.0 track)

> Новый рабочий план после закрытия базовой реализации AAA.
> Фокус: довести паллету от «уже хорошо работает» до «устойчиво и предсказуемо под production-нагрузкой».

## 1) Для тех, кто не в теме (кратко)

`pallet-aaa` — это автоматический исполнитель сценариев (pipeline) от имени sovereign-аккаунтов.

Просто говоря:

- мы задаём «что делать» (перевести, свапнуть, сжечь и т.д.),
- паллета сама решает «когда делать» по триггерам,
- и делает это строго детерминированно, безопасно для runtime.

Сейчас фундамент закрыт. Этот план — про финальную инженерную полировку в 3 направлениях:

1. Строже связать budget scheduler-а с реальными upper-bound weight.
2. Добавить настоящие generative/property проверки инвариантов.
3. Провести внешний adversarial/audit цикл.

Плюс: включить точечные правки спецификации `0.43.0`, чтобы не было разночтений между ожиданиями и фактическим контрактом.

---

## 2) Контекст и границы этого плана

### Уже реализовано

- `ScheduleWindow`, dynamic `WeightToFee`, pre-flight reserve,
- breaker semantics,
- weighted fairness + deferred retry,
- bounded queues/cursors,
- sweep lifecycle completeness,
- matrix alignment и hardening тестов.

### Что НЕ делаем в этом плане

- Никаких runtime migration/backward-compat слоёв (pre-release policy сохраняется).
- Никакого расширения функционала task-модели.
- Никаких изменений экономической философии TMCTOL.

---

## 3) Workstream A — Strict budget accounting (без эвристик)

### A.1 Для тех, кто не в теме

Сейчас scheduler решает «успеем ли выполнить ещё один actor в этом блоке» по внутренней оценке.
Нужно, чтобы это решение опиралось на тот же строгий upper-bound контракт, что и fee/weight модель runtime.

### A.2 Почему это важно

- Убирает риск drift между admission-логикой scheduler-а и runtime weight reality.
- Делает поведение предсказуемым для benchmarking и governance-тюнинга.
- Снижает шанс «проходит admission, но фактически съедает больше бюджета, чем ожидали».

### A.3 Как именно делаем

1. Ввести явный расчет `cycle_weight_upper_bound` для actor cycle:
   - `Σ(weight_upper_bound(task_i))`
   - - фиксированный bounded overhead цикла (events, checks, bookkeeping),
   - - bounded overhead condition evaluation.
2. В `execute_cycle` заменить эвристический `estimated_cost` на этот upper-bound admission.
3. Синхронизировать runtime benchmark assumptions с новой admission-моделью.
4. Добавить тесты границы:
   - «ровно хватает budget» → стартует,
   - «на 1 unit меньше» → deferral,
   - не инкрементит nonce при deferral.

### A.4 Артефакты

- `template/pallets/aaa/src/lib.rs`
- `template/pallets/aaa/src/weights.rs`
- `template/pallets/aaa/src/tests.rs`
- `template/runtime/src/tests/aaa_integration_tests.rs`
- `template/pallets/aaa/src/benchmarking.rs`

### A.5 DoD (готово)

- Admission основывается только на верхних bounded weight контрактах.
- Тесты границ budget зелёные на pallet + runtime уровне.
- `runtime-benchmarks` и clippy без регрессий.

---

## 4) Workstream B — Generative property tests (инварианты под множеством сценариев)

### B.1 Для тех, кто не в теме

Обычные тесты проверяют несколько заранее придуманных сценариев.
Generative/property подход генерирует много разных комбинаций и проверяет, что инвариант всегда держится.

### B.2 Почему это важно

- Ловит edge-cases, которые сложно вручную придумать.
- Даёт более сильную уверенность в scheduler/fairness/budget/reserve.
- Снижает риск «прошло 100 unit-тестов, но упало на необычной комбинации состояния».

### B.3 Как именно делаем

1. Добавить property-suite для AAA (детерминированно, фиксированные seeds).
2. Обязательные инварианты:
   - budget cap не нарушается,
   - deferred retry прогрессирует и не starve-ит,
   - fairness остаётся детерминированной при фиксированном состоянии,
   - pre-flight reserve предотвращает fee-starvation,
   - saturating arithmetic не приводит к потере/созданию баланса.
3. Для каждого property иметь:
   - минимальный shrinking-friendly input model,
   - воспроизводимость по seed,
   - явные failure diagnostics.
4. Включить suite в регулярный CI test path (не только локально).

### B.4 Артефакты

- `template/pallets/aaa/src/tests.rs` (или выделенный property-модуль рядом)
- `template/runtime/src/tests/aaa_integration_tests.rs`
- при необходимости: `template/pallets/aaa/Cargo.toml` (dev-deps)

### B.5 DoD (готово)

- Property-suite стабильно воспроизводима.
- Закрывает P0/P1 инварианты из test-matrix.
- Нет flaky-поведения в CI.

---

## 5) Workstream C — External adversarial audit loop

### C.1 Для тех, кто не в теме

Даже хорошая команда «замыливает глаз». Внешний аудит нужен, чтобы найти уязвимости и логические дыры, которые внутренняя команда может пропустить.

### C.2 Почему это важно

- AAA — это исполнитель экономически значимых действий.
- Ошибка в scheduler/fees/lifecycle может быть дорогой.
- Внешний adversarial взгляд — последний фильтр перед production.

### C.3 Как именно делаем

1. Подготовить pre-audit пакет:
   - frozen спецификация,
   - threat model,
   - инварианты и ожидаемые guarantees,
   - покрытие тестов/бенчей.
2. Определить audit scope:
   - scheduler arbitration,
   - fee reserve + budget admission,
   - terminal lifecycle,
   - adapter boundedness/determinism,
   - sovereign slot model.
3. Провести triage findings:
   - Critical/High/Medium/Low,
   - SLA на исправления,
   - обязательные regression tests на каждый finding.
4. Выпустить post-audit report:
   - что найдено,
   - что исправлено,
   - что принято как residual risk и почему.

### C.4 Артефакты

- `docs/aaa-specification.md` (frozen audit target)
- `docs/aaa-spec-test-matrix.ru.md`
- `docs/aaa-plan.md` (этот план, статусы)
- `CHANGELOG.md` (findings/remediations)

### C.5 DoD (готово)

- Нет открытых Critical/High.
- На все Medium/Low есть решение или формально принятый risk acceptance.
- Внесены regression tests для исправленных находок.

---

## 6) Spec patch track — точечные уточнения для `aaa-specification.md` (v0.43.0)

> Это не новый функционал, а устранение двусмысленностей.

### S.1 Consecutive failures threshold (`>` vs `>=`)

### Для тех, кто не в теме

Если лимит = 10, важно явно сказать: actor умирает на 10-й ошибке или на 11-й.

### Почему

Без явного текста разные разработчики и тесты трактуют по-разному.

### Как именно

- Зафиксировать в спецификации текущую семантику: terminal при `consecutive_failures > MaxConsecutiveFailures`.
- Добавить явную фразу: «умирает на `(limit + 1)`-й ошибке».
- Добавить boundary тесты:
  - при `== limit` ещё жив,
  - при `== limit + 1` terminal refund/destroy.

### S.2 Determinism contract for DexOps internals

### Для тех, кто не в теме

Если внутри адаптера есть цикл по storage, порядок должен быть одинаковый на всех нодах.

### Почему

Иначе одинаковый блок может дать разный результат на разных нодах (консенсусный риск).

### Как именно

- В `DexOps` разделе добавить норму:
  - итерации по storage только в каноническом порядке,
  - правила округления фиксированы и неизменны (например, `floor`).
- Добавить/обновить тесты детерминизма адаптера на повторяемость результата при одинаковом state.

### S.3 Rent “once per cycle/touch” no double-charge in same block

### Для тех, кто не в теме

Если actor проверили дважды в одном блоке, rent не должен списаться два раза.

### Почему

Иначе можно случайно удвоить rent из-за порядка вызовов хук/экстинзиков.

### Как именно

- В спецификации явно зафиксировать:
  - повторный touch в том же блоке не даёт второго rent charge.
- Явно привязать это к механике `last_rent_block` и `blocks_elapsed = 0` при повторном touch внутри блока.
- Добавить regression тест на double-touch same-block path.

---

## 7) Порядок выполнения (рекомендуемый)

1. `S` (spec patch) — сначала снимаем двусмысленности контракта.
2. `A` (strict budget accounting) — затем фиксируем вычислительную модель admission.
3. `B` (generative properties) — усиливаем доказательность инвариантов.
4. `C` (external audit) — финальный внешний adversarial check.

---

## 8) Единые проверки на каждом milestone

- `deno ./simulator/tests.js`
- `cd template && cargo check --workspace`
- `cd template && cargo test --workspace`
- `cd template && cargo test --features runtime-benchmarks`
- `cd template && cargo clippy --workspace --all-targets -- -D warnings`

---

## 9) Финальный критерий завершения плана

План считается закрытым, когда:

- Спецификация `0.43.0` не содержит двусмысленностей по пунктам S.1/S.2/S.3,
- scheduler budget admission строго согласован с bounded weight contract,
- property-suite устойчиво подтверждает ключевые инварианты,
- внешний audit не оставляет открытых Critical/High находок.
