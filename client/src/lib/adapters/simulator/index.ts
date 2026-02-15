import type { Adapter } from '$lib/adapters/types';
import type { SystemConfig, SystemSnapshot, SwapResult, Quote, BucketBalance } from '$lib/shared/types';
import { PPM } from '$lib/shared/types';
import { toFloat } from '$lib/shared/format';
import {
	create_system,
	User,
	type Xyk as ModelXyk,
	type Tmc as ModelTmc,
	type Tol as ModelTol,
	type FeeManager as ModelFeeManager,
	type Router as ModelRouter
} from '$simulator/model.js';

type System = {
	xyk: ModelXyk;
	tol: ModelTol;
	tmc: ModelTmc;
	fee_manager: ModelFeeManager;
	router: ModelRouter;
};

export class SimulatorAdapter implements Adapter {
	private sys!: System;
	private user!: InstanceType<typeof User>;

	init(overrides: Partial<SystemConfig> = {}, initialForeign: number = 100_000): void {
		this.sys = create_system(overrides) as System;
		this.user = new User(0n, BigInt(Math.round(initialForeign * 1e12)));
		this.user.set_router(this.sys.router);
	}

	getSnapshot(): SystemSnapshot {
		const sys = this.sys;
		const hasPool = sys.xyk.has_liquidity();
		const supply = sys.tmc.supply;
		const priceTmc = sys.tmc.get_price();
		let priceXyk: bigint | null = null;
		if (hasPool) {
			priceXyk = sys.xyk.get_price();
		}

		let gravityWellRatio = 0;
		if (hasPool) {
			const mcap = toFloat(supply) * toFloat(priceTmc);
			gravityWellRatio = mcap > 0 ? ((toFloat(sys.xyk.reserve_foreign) * 2) / mcap) * 100 : 0;
		}

		const buckets = new Map<string, BucketBalance>();
		for (const [key, bucket] of sys.tol.buckets) {
			buckets.set(key as string, {
				lp_tokens: (bucket as any).lp_tokens,
				contributed_native: (bucket as any).contributed_native,
				contributed_foreign: (bucket as any).contributed_foreign
			});
		}

		return {
			supply,
			priceTmc,
			priceXyk,
			reserveNative: sys.xyk.reserve_native,
			reserveForeign: sys.xyk.reserve_foreign,
			totalBurned: sys.fee_manager.total_native_burned,
			supplyLp: sys.xyk.supply_lp,
			hasPool,
			gravityWellRatio,
			buckets,
			bufferNative: sys.tol.buffer_native,
			bufferForeign: sys.tol.buffer_foreign
		};
	}

	getUserBalance(): { native: bigint; foreign: bigint } {
		const bal = this.user.get_balance();
		return { native: bal.native, foreign: bal.foreign };
	}

	buyNative(foreignAmount: bigint): SwapResult {
		return this.user.buy_native(foreignAmount) as SwapResult;
	}

	sellNative(nativeAmount: bigint): SwapResult {
		return this.user.sell_native(nativeAmount) as SwapResult;
	}

	depositForeign(amount: bigint): void {
		this.user.deposit_foreign(amount);
	}

	getQuoteBuy(foreignAmount: bigint): Quote | null {
		const sys = this.sys;
		const fee = (foreignAmount * sys.router.fee_router_ppm) / PPM;
		const net = foreignAmount - fee;
		const tmcQ = sys.tmc.get_mint_quote(net);
		const xykOut = sys.xyk.has_liquidity() ? sys.xyk.get_out_native(net) : 0n;
		const tmcOut = tmcQ?.user ?? 0n;
		const useTMC = tmcOut > 0n && tmcOut >= xykOut;
		const out = useTMC ? tmcOut : xykOut;

		if (out <= 0n) return null;

		const effPrice = toFloat(foreignAmount) / toFloat(out);
		return {
			out,
			route: useTMC ? 'TMC' : 'XYK',
			effectivePrice: effPrice,
			fee,
			tmcOut,
			xykOut,
			isSell: false
		};
	}

	getQuoteSell(nativeAmount: bigint): Quote | null {
		const sys = this.sys;
		if (!sys.xyk.has_liquidity()) return null;

		const fee = (nativeAmount * sys.router.fee_router_ppm) / PPM;
		const net = nativeAmount - fee;
		const out = sys.xyk.get_out_foreign(net);

		if (out <= 0n) return null;

		const effPrice = toFloat(out) / toFloat(nativeAmount);
		return {
			out,
			route: 'XYK',
			effectivePrice: effPrice,
			fee,
			tmcOut: 0n,
			xykOut: out,
			isSell: true
		};
	}

	getEffectiveMintPrice(probeAmount: bigint): number {
		const sys = this.sys;
		const routerFee = (probeAmount * sys.router.fee_router_ppm) / PPM;
		const probeNet = probeAmount - routerFee;
		const tmcQ = sys.tmc.get_mint_quote(probeNet);
		if (tmcQ && tmcQ.user > 0n) {
			return toFloat(probeAmount) / toFloat(tmcQ.user);
		}
		return toFloat(sys.tmc.get_price());
	}
}
