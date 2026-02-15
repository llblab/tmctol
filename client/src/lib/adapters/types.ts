import type { SystemConfig, SystemSnapshot, SwapResult, Quote } from '$lib/shared/types';

export type Adapter = {
	init(overrides: Partial<SystemConfig>, initialForeign: number): void;
	getSnapshot(): SystemSnapshot;
	getUserBalance(): { native: bigint; foreign: bigint };
	buyNative(foreignAmount: bigint): SwapResult;
	sellNative(nativeAmount: bigint): SwapResult;
	depositForeign(amount: bigint): void;
	getQuoteBuy(foreignAmount: bigint): Quote | null;
	getQuoteSell(nativeAmount: bigint): Quote | null;
	getEffectiveMintPrice(probeAmount: bigint): number;
};
