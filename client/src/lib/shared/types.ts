export type {
	MintShareConfig,
	TmcConfig,
	XykConfig,
	RouterConfig,
	TolBucketConfig,
	TolConfig,
	SystemConfig,
	BucketBalance,
	TolResult,
	SwapResult,
	Quote,
	SystemSnapshot
} from '$simulator/types';

// Runtime constants (not in .d.ts)
export const DECIMALS = 12n;
export const PRECISION = 10n ** DECIMALS;
export const PPM = 1_000_000n;

// ============ Chart/Log Types (UI-specific) ============

export type PricePoint = {
	step: number;
	priceEffTMC: number;
	priceXYK: number;
	priceRouter: number | null;
	supply: number;
};

export type LogType = 'info' | 'buy' | 'sell' | 'error';

export type LogEntry = {
	id: number;
	step: number;
	message: string;
	type: LogType;
};
