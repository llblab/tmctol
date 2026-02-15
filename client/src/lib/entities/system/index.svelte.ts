import type {
  SystemConfig,
  SystemSnapshot,
  PricePoint,
  Quote,
  SwapResult,
  LogEntry,
  LogType,
} from "./types";
import type { Adapter } from "$lib/adapters/types";
import { PRECISION, PPM } from "$lib/shared/types";
import { SimulatorAdapter } from "$lib/adapters/simulator";
import { toFloat, toBigInt } from "$lib/shared/format";

const PROBE_AMOUNT = 100n * PRECISION;

class SystemStore {
  adapter: Adapter = $state(new SimulatorAdapter());
  snapshot: SystemSnapshot = $state(null!);
  userBalance: { native: bigint; foreign: bigint } = $state({
    native: 0n,
    foreign: 0n,
  });
  direction: "buy" | "sell" = $state("buy");
  history: PricePoint[] = $state([]);
  log: LogEntry[] = $state([]);
  private logCounter = 0;

  configValues = $state({
    priceInitial: 0.001,
    slope: 0.001,
    routerFeePer1000: 5,
    userSharePpm: 333333,
    initialForeignBalance: 100000,
  });

  init(overrides: Partial<SystemConfig> = {}, initialForeign?: number) {
    const foreign = initialForeign ?? this.configValues.initialForeignBalance;
    this.adapter.init(overrides, foreign);
    this.direction = "buy";
    this.refresh();
  }

  refresh() {
    this.snapshot = this.adapter.getSnapshot();
    this.userBalance = this.adapter.getUserBalance();
    this.pushPricePoint();
  }

  private pushPricePoint() {
    const snap = this.snapshot;
    const step = this.history.length;

    const pEffTMC = this.adapter.getEffectiveMintPrice(PROBE_AMOUNT);
    const pXYK = snap.priceXyk ? toFloat(snap.priceXyk) : 0;

    // Router price: cost of buying 1 NATIVE with FOREIGN through the router
    let priceRouter: number | null = null;
    const probeRouter = PROBE_AMOUNT;
    const quoteBuy = this.adapter.getQuoteBuy(probeRouter);
    if (quoteBuy && quoteBuy.out > 0n) {
      priceRouter = toFloat(probeRouter) / toFloat(quoteBuy.out);
    }

    const supply = toFloat(snap.supply);

    this.history = [
      ...this.history,
      { step, priceEffTMC: pEffTMC, priceXYK: pXYK, priceRouter, supply },
    ];
  }

  buyNative(foreignAmount: bigint): SwapResult {
    const result = this.adapter.buyNative(foreignAmount);
    this.refresh();
    return result;
  }

  sellNative(nativeAmount: bigint): SwapResult {
    const result = this.adapter.sellNative(nativeAmount);
    this.refresh();
    return result;
  }

  depositForeign(amount: bigint) {
    this.adapter.depositForeign(amount);
    this.userBalance = this.adapter.getUserBalance();
  }

  getQuoteBuy(foreignAmount: bigint): Quote | null {
    return this.adapter.getQuoteBuy(foreignAmount);
  }

  getQuoteSell(nativeAmount: bigint): Quote | null {
    return this.adapter.getQuoteSell(nativeAmount);
  }

  flipDirection() {
    this.direction = this.direction === "buy" ? "sell" : "buy";
  }

  addLog(message: string, type: LogType = "info") {
    const entry: LogEntry = {
      id: this.logCounter++,
      step: this.history.length - 1,
      message,
      type,
    };
    this.log = [entry, ...this.log.slice(0, 199)];
  }

  clearLog() {
    this.log = [];
  }

  buildConfig(): Partial<SystemConfig> {
    const c = this.configValues;
    return {
      tmc: {
        price_initial: toBigInt(c.priceInitial),
        slope: toBigInt(c.slope),
        mint_shares: {
          user_ppm: BigInt(c.userSharePpm),
          tol_ppm: BigInt(1_000_000 - c.userSharePpm),
        },
      },
      router: {
        min_initial_foreign: 100n * PRECISION,
        min_swap_foreign: PRECISION / 100n,
        fee_router_ppm: (BigInt(c.routerFeePer1000) * PPM) / 1000n,
      },
    };
  }

  resetSimulation() {
    this.history = [];
    this.log = [];
    this.logCounter = 0;
    this.init(this.buildConfig());
    this.addLog("Simulation reset", "info");
  }
}

export const systemStore = new SystemStore();
