export type PanelId = 'swap' | 'chart' | 'info' | 'buckets' | 'log';

export const ALL_PANELS: PanelId[] = ['swap', 'chart', 'info', 'buckets', 'log'];

export const PANEL_LABELS: Record<PanelId, string> = {
	swap: 'Swap',
	chart: 'Chart',
	info: 'Info',
	buckets: 'Buckets',
	log: 'Log'
};

export type TileLeaf = {
	type: 'leaf';
	id: string;
	tabs: PanelId[];
	activeTab: PanelId;
};

export type TileSplit = {
	type: 'split';
	id: string;
	direction: 'horizontal' | 'vertical';
	ratio: number;
	children: [TileNode, TileNode];
};

export type TileNode = TileLeaf | TileSplit;

export type DropEdge = 'right' | 'bottom' | 'left';
