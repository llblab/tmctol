# Руководство по тестированию на Paseo

> Практическое руководство по ручному тестированию паллет TMCTOL на тестнете Paseo.

`Требования`:

- Скомпилированный runtime Polkadot SDK 2512.1.0
- Бинарник `polkadot-omni-node`
- Polkadot.js Apps или Substrate Connect
- Тестовые аккаунты с токенами PAS

---

## 1. Получение токенов PAS (Faucet)

Paseo использует `PAS` как нативный токен (эквивалент DOT на Polkadot).

### 1.1 Через Faucet Bot

1. Зайдите в [Paseo Element Channel](https://matrix.to/#/#paseo-faucet:matrix.org)
2. Отправьте сообщение: `!drip <ваш_адрес>`
3. Получите ~100 PAS для тестирования

### 1.2 Через Polkadot.js Apps

1. Перейдите на [Polkadot.js Apps](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fpaseo.rpc.amforc.com#/accounts)
2. Подключитесь к Paseo: `wss://paseo.rpc.amforc.com` или `wss://rpc.ibp.network/paseo`
3. Используйте dev-аккаунты (Alice, Bob) при локальном relay

---

## 2. Регистрация парачейна на Paseo

### 2.1 Резервирование Para ID

```
Polkadot.js → Developer → Extrinsics
  → registrar → reserve()
  → Подписать с профинансированного аккаунта
```

Запишите зарезервированный `ParaId` из событий (например, `2000`).

### 2.2 Генерация Chain Spec

```bash
# Сборка runtime
./scripts/03-build-runtime.sh

# Генерация chain spec с вашим Para ID
./scripts/04-generate-chain-spec.sh
# Отредактируйте output для установки правильного paraId
```

### 2.3 Регистрация Parathread

```
registrar → register(id, genesis_head, validation_code)
  → id: ваш зарезервированный ParaId
  → genesis_head: из chain spec (поле genesis.raw, SCALE encoded)
  → validation_code: runtime WASM blob
```

### 2.4 Покупка Coretime (On-Demand или Bulk)

`On-Demand (для тестирования)`:

```
onDemandAssignmentProvider → placeOrderAllowDeath(max_amount, para_id)
  → max_amount: 10_000_000_000_000 (10 PAS)
  → para_id: ваш ParaId
```

`Bulk Coretime (для продолжительного тестирования)`:

```
broker → purchase(region_id, max_price)
```

---

## 3. Открытие HRMP канала с AssetHub

AssetHub Paseo Para ID: `1000`

### 3.1 Запрос канала (Ваш Para → AssetHub)

С sovereign аккаунта вашего парачейна на relay:

```
hrmp → hrmpInitOpenChannel(recipient, proposed_max_capacity, proposed_max_message_size)
  → recipient: 1000 (AssetHub)
  → proposed_max_capacity: 1000
  → proposed_max_message_size: 102400
```

`Требуемый депозит`: ~10 PAS на каждое направление канала.

### 3.2 Принятие канала (AssetHub → Ваш Para)

Требует governance AssetHub или sudo Paseo. Для тестнета запросите через:

- [Paseo Matrix Channel](https://matrix.to/#/#paseo-support:matrix.org)
- Или используйте sudo Paseo если доступно

### 3.3 Запрос обратного канала (AssetHub → Ваш Para)

Тот же процесс, но инициируется со стороны AssetHub (требует governance/sudo).

### 3.4 Проверка каналов

```
Polkadot.js → Developer → Chain State
  → hrmp → hrmpChannels(sender, recipient)
```

Должно показать `HrmpChannel { max_capacity: 1000, ... }` для обоих направлений.

---

## 4. Регистрация Foreign Asset (PAS на вашем парачейне)

### 4.1 Определение Location для PAS

PAS с relay chain:

```json
{
  "parents": 1,
  "interior": "Here"
}
```

### 4.2 Регистрация через Governance/Sudo

```
assetRegistry → registerForeignAsset(location, metadata)
  → location: { "parents": 1, "interior": "Here" }
  → metadata: { "name": "Paseo", "symbol": "PAS", "decimals": 10 }
```

Это генерирует детерминированный AssetId через `Blake2(Location)` с маской 0xF... для foreign.

### 4.3 Проверка регистрации

```
Chain State → assetRegistry → foreignAssetMapping(location)
  → Возвращает: AssetId (например, 0xF0001234...)
```

---

## 5. Получение PAS через XCM Reserve Transfer

### 5.1 С Relay на ваш парачейн

На Paseo relay:

```
xcmPallet → limitedReserveTransferAssets(
  dest, beneficiary, assets, fee_asset_item, weight_limit
)
  → dest: { "parents": 0, "interior": { "X1": { "Parachain": YOUR_PARA_ID } } }
  → beneficiary: { "parents": 0, "interior": { "X1": { "AccountId32": { "id": "0x..." } } } }
  → assets: { "V4": [{ "id": { "parents": 0, "interior": "Here" }, "fun": { "Fungible": 10_000_000_000 } }] }
  → fee_asset_item: 0
  → weight_limit: "Unlimited"
```

### 5.2 Проверка получения

На вашем парачейне:

```
Chain State → assets → account(asset_id, account)
  → Показывает баланс PAS
```

---

## 6. Минтинг Native токенов через TMC

### 6.1 Проверка наличия TMC кривой

```
Chain State → tokenMintingCurve → tokenCurves(token_asset)
  → Должно показать: { initial_price, slope, initial_issuance, foreign_asset, native_asset }
```

Если не инициализирована, создайте через governance:

```
tokenMintingCurve → createCurve(token_asset, foreign_asset, initial_price, slope)
```

### 6.1.1 Привязка токена к TOL-домену (Phase coupling)

Актуальное поведение runtime:

- при регистрации foreign token через Asset Registry выполняется idempotent bootstrap TOL-домена
- при `tokenMintingCurve.createCurve(...)` выполняется runtime glue: подтверждение TOL-domain binding + авто-включение токена в Zap whitelist
- дефолтная конвенция: `tol_id = token_asset_id` для non-LP активов

Проверка:

```
Chain State → treasuryOwnedLiquidity → tokenTolBinding(token_asset)
  → Должно вернуть соответствующий tol_id (обычно asset_id)
```

Ручной override (опционально, governance):

```
treasuryOwnedLiquidity → bindTokenToTol(token_asset, tol_id)
```

### 6.2 Минтинг Native токенов

```
axialRouter → swap(asset_in, asset_out, amount_in, min_amount_out)
  → asset_in: Foreign(PAS_ASSET_ID)  // Зарегистрированный PAS
  → asset_out: Native
  → amount_in: 1_000_000_000_000  // 1 PAS
  → min_amount_out: 0  // Для тестирования; в продакшене используйте slippage
```

`Ожидаемый поток`:

1. Router забирает PAS
2. Маршрутизирует к TMC (если оптимально) или XYK пулу
3. Минтит Native токены: 33.3% пользователю, 66.6% в ZapManager
4. ZapManager добавляет ликвидность в XYK пул
5. LP токены переводятся в token-resolved TOL ingress account (`AssetId -> TolId`)
6. Распределение ingress LP по бакетам выполняется через путь `receiveLpTokens` (integration/automation path)

### 6.3 Проверка минтинга

```
# Проверьте ваш Native баланс
Chain State → system → account(your_account)

# Проверьте состояние TMC
Chain State → tokenMintingCurve → tokenCurves(token_asset)
  → initial_issuance фиксирован, а текущий effective supply проверяется через рост total issuance native

# Проверьте LP transfer в TOL ingress
Events → zapManager → LPTokensDistributed { token_asset, lp_amount, destination }
  → destination должен соответствовать домену tol_id (если токен привязан)

# Проверьте состояние бакетов TOL по домену
Chain State → treasuryOwnedLiquidity → bucketA(tol_id)
Chain State → treasuryOwnedLiquidity → bucketB(tol_id)
Chain State → treasuryOwnedLiquidity → bucketC(tol_id)
Chain State → treasuryOwnedLiquidity → bucketD(tol_id)
```

---

## 7. Проверка ликвидности в XYK пуле

### 7.1 Запрос резервов пула

```
Chain State → assetConversion → pools((Native, Foreign(PAS_ID)))
  → Возвращает: (reserve_native, reserve_foreign)
```

### 7.2 Расчёт Spot Price

```
Spot Price = reserve_foreign / reserve_native
```

### 7.3 Запрос баланса LP токенов

```
Chain State → poolAssets → account(pool_lp_id, account)
```

---

## 8. Свопы через Axial Router

### 8.1 Native → Foreign (Продажа)

```
axialRouter → swap(Native, Foreign(PAS_ID), amount, min_out)
```

Router выполнит:

1. Проверит цену XYK пула
2. Выполнит через XYK (TMC не поддерживает обратное направление)
3. Соберёт 0.5% комиссию → BurningManager

### 8.2 Foreign → Native (Покупка)

```
axialRouter → swap(Foreign(PAS_ID), Native, amount, min_out)
```

Router выполнит:

1. Сравнит цену TMC vs XYK
2. Маршрутизирует к лучшему источнику (efficiency score)
3. Соберёт комиссию

### 8.3 Проверка решения Router

В текущем runtime нет отдельного события `RouteSelected`.

Проверяйте комбинацию событий:

```
Events → axialRouter → SwapExecuted { from, to, amount_in, amount_out }
Events → tokenMintingCurve → ZapAllocationDistributed { ... }  // признак TMC route
```

Если `ZapAllocationDistributed` отсутствует при свопе, путь был через XYK.

---

## 9. Проверка Burning Manager

### 9.1 Проверка состояния Burning Manager

```
Chain State → burningManager → burnableAssets
Chain State → burningManager → minBurnNative
Chain State → burningManager → dustThreshold
```

### 9.2 Запуск обработки (Автоматический)

Обработка выполняется в `on_idle`:

1. один non-native актив (LP unwind приоритетно, затем foreign swap)
2. затем burn native на аккаунте BM

Подождите следующий блок и проверьте:

```
Events → burningManager → LpUnwound { ... }
Events → burningManager → ForeignTokensSwapped { ... }
Events → burningManager → NativeTokensBurned { amount, new_total }
```

### 9.3 Проверка общего сожжённого

```
Chain State → burningManager → totalBurned
```

---

## 10. Мониторинг экономических метрик

### 10.1 Состояние TMC

```
tokenMintingCurve → tokenCurves(token_asset)
  → { initial_price, slope, initial_issuance, foreign_asset, native_asset }
```

`Проверка инварианта`:

- `effective_supply = total_issuance(native) - initial_issuance`
- `current_price = initial_price + slope × effective_supply / PRECISION`

### 10.2 Состояние TOL

```
treasuryOwnedLiquidity → tokenTolBinding(token_asset)  // TolId routing binding
treasuryOwnedLiquidity → bucketA(tol_id)  // Anchor (50%)
treasuryOwnedLiquidity → bucketB(tol_id)  // Building (16.7%)
treasuryOwnedLiquidity → bucketC(tol_id)  // Capital (16.7%)
treasuryOwnedLiquidity → bucketD(tol_id)  // Dormant (16.6%)
```

### 10.3 Ценовой коридор

- `Потолок`: текущая цена TMC (цена минтинга)
- `Пол`: подразумеваемая цена XYK пула (можно продать по ней)

```
Пол < Рыночная цена < Потолок
```

По мере накопления TOL пол растёт. По мере сжигания потолок снижается. Коридор сжимается вверх.

---

## 11. Устранение неполадок

### XCM Transfer не проходит

| Ошибка           | Причина                          | Решение                                |
| ---------------- | -------------------------------- | -------------------------------------- |
| `NotHoldingFees` | Недостаточно fee asset           | Убедитесь в достаточном количестве PAS |
| `Barrier`        | Location не доверенный           | Проверьте фильтр `ReserveAssetsFrom`   |
| `AssetNotFound`  | Foreign asset не зарегистрирован | Выполните `registerForeignAsset`       |
| `Transport`      | Нет HRMP канала                  | Откройте каналы (раздел 3)             |

### TMC Minting не проходит

| Ошибка               | Причина                            | Решение                                           |
| -------------------- | ---------------------------------- | ------------------------------------------------- |
| `NoCurveExists`      | TMC не инициализирована            | Создайте кривую через governance                  |
| `InvalidForeignAsset`| В своп передан collateral не из curve | Используйте asset_in, совпадающий с foreign_asset |
| `ZeroAmount`         | amount_in равен нулю               | Укажите ненулевой amount_in                       |

### Router возвращает NoRouteFound

| Причина                         | Решение                                               |
| ------------------------------- | ----------------------------------------------------- |
| Нет XYK пула                    | Создайте пул через `assetConversion.createPool`       |
| Нет TMC кривой                  | Создайте кривую через `tokenMintingCurve.createCurve` |
| Слишком высокое отклонение цены | Дождитесь стабилизации оракула                        |

---

## 12. Полезные RPC эндпоинты

### Paseo Relay Chain

- `wss://paseo.rpc.amforc.com`
- `wss://rpc.ibp.network/paseo`
- `wss://paseo-rpc.dwellir.com`

### AssetHub Paseo

- `wss://asset-hub-paseo-rpc.dwellir.com`
- `wss://sys.ibp.network/asset-hub-paseo`

### Ваш парачейн

- Локально: `ws://127.0.0.1:9944`
- Развёрнутый: ваш RPC эндпоинт

---

## 13. Быстрая последовательность тестов

Полный флоу для проверки работы всех паллет:

```bash
# 1. Получите PAS из faucet
!drip 5GrwvaEF...

# 2. Переведите PAS на ваш парачейн через XCM
xcmPallet.limitedReserveTransferAssets(...)

# 3. Создайте/проверьте TMC кривую для токена
#    - createCurve выполняет runtime glue (TOL-domain bootstrap + Zap enable)
tokenMintingCurve.createCurve(Foreign(PAS), Native, 1_000_000_000_000, 1_000_000_000_000)

# 4. (Опционально) ручной override TOL binding
treasuryOwnedLiquidity.bindTokenToTol(Foreign(PAS), CUSTOM_TOL_ID)

# 5. Минтните Native через Router
axialRouter.swap(Foreign(PAS), Native, 1_000_000_000_000, 0)

# 6. Проверьте распределение
#    - 33.3% Native на вашем аккаунте
#    - 66.6% ушло в ZapManager
#    - ZapManager перевёл LP в resolved TOL ingress account (смотрите событие LPTokensDistributed)

# 7. Проверьте состояние TOL
treasuryOwnedLiquidity.tokenTolBinding(Foreign(PAS))
treasuryOwnedLiquidity.bucketA(PAS_ASSET_ID)

# 8. Свопните обратно (продажа Native)
axialRouter.swap(Native, Foreign(PAS), 100_000_000_000, 0)

# 9. Проверьте сожжённую комиссию
burningManager.totalBurned
```

`Критерии успеха`:

- ✅ PAS получен на парачейне
- ✅ Native заминчен с правильным распределением 33/66
- ✅ XYK пул имеет ликвидность
- ✅ LP переведены в корректный TOL ingress account (по token-domain binding, обычно `tol_id = asset_id`)
- ✅ Burns накапливаются
- ✅ Ценовой коридор сжимается вверх со временем
