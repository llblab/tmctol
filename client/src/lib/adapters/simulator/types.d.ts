declare module '$simulator/model.js' {
	export const DECIMALS: bigint;
	export const PRECISION: bigint;
	export const PPM: bigint;
	export const DEFAULT_CONFIG: any;

	export class BigMath {
		static mul_div(a: bigint, b: bigint, c: bigint): bigint;
		static div_ceil(a: bigint, b: bigint): bigint;
		static isqrt(n: bigint): bigint;
		static min(a: bigint, b: bigint): bigint;
		static max(a: bigint, b: bigint): bigint;
		static abs(a: bigint): bigint;
	}

	export class User {
		balance_native: bigint;
		balance_foreign: bigint;
		router: Router | null;
		constructor(initial_native?: bigint, initial_foreign?: bigint);
		set_router(router: Router): void;
		get_balance(): { native: bigint; foreign: bigint };
		buy_native(foreign_amount: bigint): any;
		sell_native(native_amount: bigint): any;
		deposit_foreign(amount: bigint): void;
		deposit_native(amount: bigint): void;
	}

	export class LiquidityBucket {
		id: string;
		lp_tokens: bigint;
		contributed_native: bigint;
		contributed_foreign: bigint;
		constructor(id: string);
		get_balance(): any;
		receive_lp_tokens(lp: bigint, native_used: bigint, foreign_used: bigint): void;
	}

	export class Tol {
		xyk: Xyk;
		bucket_config: Record<string, bigint>;
		bucket_keys: string[];
		buckets: Map<string, LiquidityBucket>;
		buffer_native: bigint;
		buffer_foreign: bigint;
		constructor(xyk: Xyk, config: any);
		get_balance(): any;
		receive_mint_allocation(total_native: bigint, total_foreign: bigint): any;
	}

	export class Xyk {
		fee_ppm: bigint;
		reserve_native: bigint;
		reserve_foreign: bigint;
		supply_lp: bigint;
		constructor(config: any);
		get_price(): bigint;
		has_liquidity(): boolean;
		get_out_native(foreign: bigint): bigint;
		get_out_foreign(native: bigint): bigint;
		add_liquidity(native: bigint, foreign: bigint): any;
		swap_native_to_foreign(native_in: bigint, min_foreign_out?: bigint): any;
		swap_foreign_to_native(foreign_in: bigint, min_native_out?: bigint): any;
	}

	export class Tmc {
		price_initial: bigint;
		slope: bigint;
		user_ppm: bigint;
		tol_ppm: bigint;
		tol: Tol;
		supply: bigint;
		constructor(tol: Tol, config: any);
		get_price(): bigint;
		calculate_mint(foreign: bigint): bigint;
		mint_native(foreign_in: bigint): any;
		get_mint_quote(foreign: bigint): { minted: bigint; user: bigint; tol: bigint } | null;
		burn_native(amount: bigint): any;
	}

	export class FeeManager {
		xyk: Xyk;
		tmc: Tmc;
		min_swap_foreign: bigint;
		buffer_native: bigint;
		buffer_foreign: bigint;
		total_native_burned: bigint;
		total_foreign_swapped: bigint;
		constructor(xyk: Xyk, tmc: Tmc, config: any);
		receive_fee_native(native: bigint): void;
		receive_fee_foreign(foreign: bigint): void;
	}

	export class RouteSelector {
		constructor(xyk: Xyk, tmc: Tmc);
		select_route_for_foreign_to_native(foreign_net: bigint, min_native_out: bigint): any;
	}

	export class SwapExecutor {
		constructor(xyk: Xyk, tmc: Tmc, fee_manager: FeeManager);
		execute_tmc_route(foreign_net: bigint, foreign_in: bigint, foreign_fee: bigint): any;
		execute_xyk_route(foreign_net: bigint, foreign_in: bigint, foreign_fee: bigint, min_native_out: bigint): any;
		execute_xyk_sell_route(native_in: bigint, native_fee: bigint, min_foreign_out: bigint): any;
	}

	export class Router {
		xyk: Xyk;
		tmc: Tmc;
		fee_manager: FeeManager;
		route_selector: RouteSelector;
		swap_executor: SwapExecutor;
		fee_router_ppm: bigint;
		min_swap_foreign: bigint;
		min_initial_foreign: bigint;
		constructor(xyk: Xyk, tmc: Tmc, fee_manager: FeeManager, config: any);
		swap_foreign_to_native(foreign_in: bigint, min_native_out?: bigint): any;
		swap_native_to_foreign(native_in: bigint, min_foreign_out?: bigint): any;
	}

	export function create_system(config_override?: any): {
		xyk: Xyk;
		tol: Tol;
		tmc: Tmc;
		fee_manager: FeeManager;
		router: Router;
	};
}
