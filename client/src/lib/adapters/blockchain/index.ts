import type { Adapter } from "$lib/adapters/types";
import type {
  SystemConfig,
  SystemSnapshot,
  SwapResult,
  Quote,
} from "$lib/shared/types";

export class BlockchainAdapter implements Adapter {
  init(_overrides: Partial<SystemConfig>, _initialForeign: number): void {
    throw new Error("BlockchainAdapter not implemented yet");
  }

  getSnapshot(): SystemSnapshot {
    throw new Error("BlockchainAdapter not implemented yet");
  }

  getUserBalance(): { native: bigint; foreign: bigint } {
    throw new Error("BlockchainAdapter not implemented yet");
  }

  buyNative(_foreignAmount: bigint): SwapResult {
    throw new Error("BlockchainAdapter not implemented yet");
  }

  sellNative(_nativeAmount: bigint): SwapResult {
    throw new Error("BlockchainAdapter not implemented yet");
  }

  depositForeign(_amount: bigint): void {
    throw new Error("BlockchainAdapter not implemented yet");
  }

  getQuoteBuy(_foreignAmount: bigint): Quote | null {
    throw new Error("BlockchainAdapter not implemented yet");
  }

  getQuoteSell(_nativeAmount: bigint): Quote | null {
    throw new Error("BlockchainAdapter not implemented yet");
  }

  getEffectiveMintPrice(_probeAmount: bigint): number {
    throw new Error("BlockchainAdapter not implemented yet");
  }
}
