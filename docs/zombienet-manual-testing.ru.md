# Ручное тестирование TMCTOL в Zombienet

> **Среда**: локальный Zombienet с rococo-local relay chain + парачейн ID 2000
> **Инструмент**: Polkadot.js Apps (`https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9988`)
> **Условие**: все операции выполняются после полной инициализации сети (блоки финализируются)

---

## 0. Подготовка: регистрация Foreign-ассета

В production Foreign-ассеты (например, DOT) поступают через XCM reserve transfer с AssetHub (Parachain 1000). В локальном зомбинете AssetHub не запущен, поэтому мы создаём синтетический Foreign через `asset-registry`.

### 0.1 Регистрация wDOT

**Extrinsic**: `sudo → assetRegistry → registerForeignAsset`

| Параметр | Значение | Пояснение |
|----------|----------|-----------|
| `location` | `{"parents": 1, "interior": {"X3": [{"Parachain": 1000}, {"PalletInstance": 50}, {"GeneralIndex": 0}]}}` | XCM Location DOT на AssetHub |
| `metadata` | `{ name: "Wrapped DOT", symbol: "wDOT", decimals: 10 }` | Метаданные ассета |
| `min_balance` | `1000000000` | ED для wDOT (0.1 DOT при 10 decimals) |
| `is_sufficient` | `true` | Аккаунт может существовать только с этим ассетом |

**Результат**: в событиях появится `assetRegistry.ForeignAssetRegistered` с `asset_id`. Запишите этот ID — он понадобится далее.

> **Почему именно этот Location?**
> - `parents: 1` — поднимаемся на relay chain
> - `Parachain(1000)` — AssetHub
> - `PalletInstance(50)` — `pallet-assets` на AssetHub (стандартный индекс)
> - `GeneralIndex(0)` — DOT как актив с индексом 0
>
> Этот Location идентичен тому, что будет использоваться в production. `asset-registry` хэширует его через Blake2 и генерирует детерминированный `AssetId` в Foreign-пространстве bitmask. Сам AssetHub при этом не нужен — Location это идентификатор, а не маршрут доставки.

### 0.2 Минтинг wDOT на тестовые аккаунты

**Extrinsic**: `sudo → assets → mint`

| Параметр | Значение |
|----------|----------|
| `id` | `<asset_id из шага 0.1>` |
| `beneficiary` | Alice / Bob / Dave |
| `amount` | `100000000000000` (10000 DOT) |

> **Безопасность**: в production mint невозможен — owner ассета это системный аккаунт `asset-registry` (PalletId `assetreg`), у которого нет приватного ключа. Поступление Foreign возможно только через XCM reserve transfer.

### 0.3 Верификация

- `Developer → Chain State → assets → account`: проверить баланс wDOT у Alice/Bob/Dave
- `Developer → Chain State → assetRegistry → foreignAssetMapping`: проверить что Location → AssetId маппинг записан

---

## 1. Полный экономический цикл: TMC → Zap → TOL

Этот сценарий проверяет основной pipeline: покупка Native через TMC → 66.6% уходит Zap Manager → Zap добавляет ликвидность → LP токены в аккаунт паллеты TOL.

### 1.1 Создание Bonding Curve

**Extrinsic**: `sudo → tokenMintingCurve → createCurve`

| Параметр | Значение | Пояснение |
|----------|----------|-----------|
| `tokenAsset` | `Native` | Какой токен минтит кривая |
| `foreignAsset` | `Local(<asset_id wDOT>)` или `Foreign(<asset_id wDOT>)` | Коллатеральный ассет |
| `initialPrice` | `1000000000000` | P₀ = 1.0 (в PRECISION) |
| `slope` | `100000000` | 0.0001 за токен |

### 1.2 Создание пула Native/wDOT

**Extrinsic**: `assetConversion → createPool`

| Параметр | Значение |
|----------|----------|
| `asset1` | `Native` |
| `asset2` | `Foreign(<asset_id>)` |

Затем добавить начальную ликвидность:

**Extrinsic**: `assetConversion → addLiquidity` (от Alice)

| Параметр | Значение |
|----------|----------|
| `asset1` | `Native` |
| `asset2` | `Foreign(<asset_id>)` |
| `amount1Desired` | `1000000000000000` (1000 UNIT) |
| `amount2Desired` | `1000000000000000` (1000 wDOT) |
| `amount1Min` | `0` |
| `amount2Min` | `0` |

### 1.3 Настройка Zap Manager

**Extrinsic**: `sudo → zapManager → enableAsset`

| Параметр | Значение |
|----------|----------|
| `asset` | `Foreign(<asset_id>)` |

### 1.4 Создание TOL

**Extrinsic**: `sudo → treasuryOwnedLiquidity → createTol`

| Параметр | Значение |
|----------|----------|
| `tokenAsset` | `Native` |
| `foreignAsset` | `Foreign(<asset_id>)` |
| `totalAllocation` | `1000000` (100% в PPM) |

### 1.5 Выполнение mint-side пути через Router

**Extrinsic**: `axialRouter → swap` (от Bob)

| Параметр | Значение |
|----------|----------|
| `from` | `Foreign(<asset_id>)` |
| `to` | `Native` |
| `amountIn` | `100000000000000` (100 wDOT) |
| `amountOutMin` | `0` |
| `recipient` | Bob |
| `deadline` | `текущий_блок + запас` |

`Примечание`: Router выбирает оптимальный механизм. Если условия благоприятны для curve-маршрута, будет использован TMC mint path.

### 1.6 Проверка результатов

**Ожидаемые события**:
1. `axialRouter.SwapExecuted` — swap обработан Router
2. `tokenMintingCurve.ZapAllocationDistributed` — появился mint-side split (если выбран TMC путь)
3. `axialRouter.FeeCollected` — router fee отправлен в Burning Manager (для обычного user swap)

**Через 1-2 блока** (on_initialize → on_idle):
4. `zapManager.ZapCompleted` — ликвидность добавлена
5. `zapManager.LPTokensDistributed` — LP → аккаунт паллеты TOL
6. `zapManager.NativeHeld` — остаток Native удержан (если был)

**Верификация state**:
- `treasuryOwnedLiquidity → bucketA(Native)`: `lp_tokens > 0`
- `assetConversion → pools`: резервы пула увеличились
- Баланс Zap Manager: Native может остаться (Patriotic Accumulation)

Если `tokenMintingCurve.ZapAllocationDistributed` не появился, Router выбрал не TMC, а XYK-маршрут. Для принудительной проверки TMC-пути скорректируйте ликвидность/цену пула или параметры curve.

---

## 2. Axial Router: маршрутизация и выбор оптимального пути

### 2.1 Swap через Router (авто-выбор XYK/TMC)

**Extrinsic**: `axialRouter → swap` (от Dave)

| Параметр | Значение |
|----------|----------|
| `from` | `Foreign(<asset_id>)` |
| `to` | `Native` |
| `amountIn` | `10000000000000` (10 wDOT) |
| `amountOutMin` | `0` |
| `recipient` | Dave |
| `deadline` | `999999999` |

**Ожидание**: событие `axialRouter.SwapExecuted`. Поле `amount_out` покажет сколько Native получил Dave.

### 2.2 Проверка fee-потока

Fee (0.5%) от swap уходит на Burning Manager:
- `Developer → Chain State → system → account(<burning_manager_account>)`: баланс Native > 0

---

## 3. Burning Manager: цикл сжигания

### 3.1 Добавить Foreign ассет в BM

**Extrinsic**: `sudo → burningManager → addBurnableAsset`

| Параметр | Значение |
|----------|----------|
| `asset` | `Foreign(<asset_id>)` |

### 3.2 Накопить fee

Выполнить несколько swap через Router (повторить сценарий 2.1 несколько раз). Каждый swap отправляет 0.5% fee на BM.

### 3.3 Наблюдение за сжиганием

BM работает в `on_idle`. Подождите 2-5 блоков.

**Ожидаемые события**:
- `burningManager.NativeBurned` — Native fee сожжены
- `burningManager.ForeignSwapped` — Foreign fee свапнуты в Native → сожжены

**Верификация**:
- `burningManager → totalBurned`: значение > 0
- Total supply Native должен уменьшиться

---

## 4. Governance: изменение Bucket Allocation

### 4.1 Изменить распределение

**Extrinsic**: `sudo → treasuryOwnedLiquidity → updateBucketAllocation`

Вызвать 3 раза для бакетов A, B, C (D получает остаток):

| bucket_index | target_allocation_ppm | Доля |
|---|---|---|
| 0 (A) | `400000` | 40% |
| 1 (B) | `300000` | 30% |
| 2 (C) | `200000` | 20% |
| — (D) | автоматически | 10% |

### 4.2 Проверить эффект

Выполнить ещё один минт через TMC (сценарий 1.5). Дождаться LP распределения.

**Верификация**: соотношение `lp_tokens` в бакетах A:B:C:D ≈ 4:3:2:1 (а не дефолтное 50:16.67:16.67:16.66).

---

## 5. Edge Cases

### 5.1 Только Native на Zap Manager (без Foreign)

**Действие**: отправить Native на аккаунт Zap Manager напрямую (`balances → transfer`).

**Ожидание**: **ничего не происходит**. Zap Manager реагирует только на Foreign-балансы. Native лежит в "Patriotic Accumulation". Событий `ZapCompleted` не будет.

**Проверка**: когда позже поступит Foreign (через Router swap в сторону Native или прямой перевод) — Zap использует накопленный Native для создания ликвидности.

### 5.2 Collateral mismatch (Security Fix #1)

**Действие**: выполнить `axialRouter → swap` из `Foreign(<asset_id_X>)` в `Native`, где collateral `asset_id_X` не совпадает с collateral, указанным в `createCurve`.

**Ожидание**:
- TMC route не будет выбран
- при отсутствии XYK-маршрута — ошибка `NoRouteFound`
- при наличии XYK-пула swap пройдет по XYK, без TMC-события `ZapAllocationDistributed`

### 5.3 Slippage rejection (Security Fix #2)

**Действие**: сделать очень большой swap (>10% резервов пула) через Router.

**Ожидание**: если price impact > 2% slippage tolerance, BM откажется свапать Foreign fee. `burningManager → totalBurned` не увеличится для этого Foreign.

### 5.4 Withdraw с Preserve (Security Fix #5)

**Действие**: `sudo → treasuryOwnedLiquidity → withdrawBuffer` — попробовать вывести **весь** баланс определённого ассета с аккаунта паллеты TOL.

**Ожидание**: ошибка (Preserve не позволит обнулить баланс ниже ED). Вывод `balance - ED` должен пройти успешно.

### 5.5 Zap Manager RetryCooldown

**Действие**: исказить пул (большой swap → отклонение от oracle) → отправить Foreign на Zap.

**Ожидание**: Zap fail → cooldown 10 блоков → в `NextZapAttempt` появится запись. Через 10 блоков Zap retry.

---

## 6. Системные аккаунты (справочник)

| Актор | PalletId | Назначение |
|-------|----------|------------|
| Axial Router | `axialrt0` | Маршрутизация swap, сбор fee |
| Token Minting Curve | `tmcurve0` | Эмиссия через bonding curve |
| Burning Manager | `burnmgr0` | Сжигание fee |
| Zap Manager | `zapmgr00` | Провижининг ликвидности |
| TOL Pallet Account | `tolpalle` | Хранение Protocol-Owned Liquidity |
| Asset Registry | `assetreg` | Owner всех Foreign-ассетов |
| Bucket A-D | `bucket-{a,b,c,d}` | LP-распределение по стратегии |

> Все системные аккаунты создаются при genesis через `inc_providers` — они существуют с нулевым балансом и не могут быть reaped.

---

## 7. FAQ

### Откуда событие `balances.BurnedDebt` при первом переводе на системный аккаунт?

Это внутренняя бухгалтерия `pallet_balances` в Substrate SDK. При первом зачислении Native на аккаунт, который существовал через `inc_providers` (без баланса), Substrate генерирует `Endowed` + `Transfer` + `BurnedDebt`. `BurnedDebt` — это компенсация внутреннего issuance-трекинга, **не реальное сжигание токенов**. Общий supply не меняется.

### Можно ли добавить AssetHub в зомбинет?

Да. Добавьте второй парачейн (ID 1000) в `zombienet.toml`:

```toml
[[parachains]]
id = 1000
chain = "asset-hub-rococo-local"
onboard_as_parachain = true

[parachains.collator]
name = "dave"
rpc_port = 9977
command = "polkadot-parachain"
```

Это потребует бинарник `polkadot-parachain` от Parity и настройку HRMP-каналов между парачейнами 1000 и 2000. Для тестирования экономики TMCTOL это избыточно — синтетический Foreign через `register_foreign_asset` достаточен.

### Можно ли автоматически регистрировать wDOT при genesis?

Да, и это **не bad practice** для testnet. Для этого нужно добавить `GenesisConfig` в `pallet-asset-registry` и вызвать `register_foreign_asset` из `genesis_build`. Многие production-парачейны (Astar, Moonbeam) предрегистрируют known foreign assets в genesis.

Однако для production-deployment рекомендуется регистрация через governance proposal — это прозрачнее и поддаётся аудиту.
