import type { Adapter } from "$lib/adapters/types";
import type {
  SystemConfig,
  SystemSnapshot,
  PricePoint,
  Quote,
  SwapResult,
  LogEntry,
  LogType,
} from "$lib/shared/types";

export type {
  Adapter,
  SystemConfig,
  SystemSnapshot,
  PricePoint,
  Quote,
  SwapResult,
  LogEntry,
  LogType,
};

export type ConfigValues = {
  priceInitial: number;
  slope: number;
  routerFeePer1000: number;
  userSharePpm: number;
  initialForeignBalance: number;
};
