import type { PanelId, TileNode, TileLeaf, TileSplit, DropEdge } from './types';
import { ALL_PANELS } from './types';

let nextId = 0;
function genId(): string { return `t${nextId++}`; }

function createDefaultLayout(): TileSplit {
	const swapLeaf: TileLeaf = { type: 'leaf', id: genId(), tabs: ['swap', 'info'], activeTab: 'swap' };
	const chartLeaf: TileLeaf = { type: 'leaf', id: genId(), tabs: ['chart'], activeTab: 'chart' };
	const bucketsLeaf: TileLeaf = { type: 'leaf', id: genId(), tabs: ['buckets'], activeTab: 'buckets' };
	const logLeaf: TileLeaf = { type: 'leaf', id: genId(), tabs: ['log'], activeTab: 'log' };
	const bottomRight: TileSplit = {
		type: 'split', id: genId(), direction: 'horizontal', ratio: 0.5,
		children: [bucketsLeaf, logLeaf]
	};
	const rightPane: TileSplit = {
		type: 'split', id: genId(), direction: 'vertical', ratio: 0.6,
		children: [chartLeaf, bottomRight]
	};
	return {
		type: 'split', id: genId(), direction: 'horizontal', ratio: 0.3,
		children: [swapLeaf, rightPane]
	};
}

function recalcNextId(node: TileNode) {
	const num = parseInt(node.id.replace('t', ''));
	if (!isNaN(num) && num >= nextId) nextId = num + 1;
	if (node.type === 'split') {
		recalcNextId(node.children[0]);
		recalcNextId(node.children[1]);
	}
}

function collectPanels(node: TileNode, out: Set<PanelId>) {
	if (node.type === 'leaf') {
		for (const t of node.tabs) out.add(t);
	} else {
		collectPanels(node.children[0], out);
		collectPanels(node.children[1], out);
	}
}

function isValidTree(node: TileNode): boolean {
	const panels = new Set<PanelId>();
	collectPanels(node, panels);
	return ALL_PANELS.every(p => panels.has(p)) && panels.size === ALL_PANELS.length;
}

function findLeaf(node: TileNode, id: string): TileLeaf | null {
	if (node.type === 'leaf') return node.id === id ? node : null;
	return findLeaf(node.children[0], id) || findLeaf(node.children[1], id);
}

function removeTabFromLeaf(node: TileNode, leafId: string, tabId: PanelId): TileNode {
	if (node.type === 'leaf') {
		if (node.id !== leafId) return node;
		const tabs = node.tabs.filter(t => t !== tabId);
		return { ...node, tabs, activeTab: node.activeTab === tabId ? tabs[0] : node.activeTab };
	}
	return {
		...node,
		children: [
			removeTabFromLeaf(node.children[0], leafId, tabId),
			removeTabFromLeaf(node.children[1], leafId, tabId)
		] as [TileNode, TileNode]
	};
}

function splitLeafWithTab(node: TileNode, leafId: string, tabId: PanelId, edge: DropEdge): TileNode {
	if (node.type === 'leaf') {
		if (node.id !== leafId) return node;
		const newLeaf: TileLeaf = { type: 'leaf', id: genId(), tabs: [tabId], activeTab: tabId };
		const direction = (edge === 'left' || edge === 'right') ? 'horizontal' : 'vertical';
		const first = edge === 'left' ? newLeaf : node;
		const second = edge === 'left' ? node : newLeaf;
		return { type: 'split', id: genId(), direction, ratio: 0.5, children: [first, second] };
	}
	return {
		...node,
		children: [
			splitLeafWithTab(node.children[0], leafId, tabId, edge),
			splitLeafWithTab(node.children[1], leafId, tabId, edge)
		] as [TileNode, TileNode]
	};
}

function addTabToLeaf(node: TileNode, leafId: string, tabId: PanelId, index?: number): TileNode {
	if (node.type === 'leaf') {
		if (node.id !== leafId) return node;
		const tabs = [...node.tabs];
		if (index !== undefined) {
			tabs.splice(index, 0, tabId);
		} else {
			tabs.push(tabId);
		}
		return { ...node, tabs, activeTab: tabId };
	}
	return {
		...node,
		children: [
			addTabToLeaf(node.children[0], leafId, tabId, index),
			addTabToLeaf(node.children[1], leafId, tabId, index)
		] as [TileNode, TileNode]
	};
}

function collapseEmpty(node: TileNode): TileNode {
	if (node.type === 'leaf') return node;
	const left = collapseEmpty(node.children[0]);
	const right = collapseEmpty(node.children[1]);
	if (left.type === 'leaf' && left.tabs.length === 0) return right;
	if (right.type === 'leaf' && right.tabs.length === 0) return left;
	return { ...node, children: [left, right] as [TileNode, TileNode] };
}

function updateSplitRatio(node: TileNode, splitId: string, ratio: number): TileNode {
	if (node.type === 'leaf') return node;
	if (node.id === splitId) {
		return { ...node, ratio: Math.max(0.15, Math.min(0.85, ratio)) };
	}
	return {
		...node,
		children: [
			updateSplitRatio(node.children[0], splitId, ratio),
			updateSplitRatio(node.children[1], splitId, ratio)
		] as [TileNode, TileNode]
	};
}

function setActiveInLeaf(node: TileNode, leafId: string, tabId: PanelId): TileNode {
	if (node.type === 'leaf') {
		if (node.id !== leafId || !node.tabs.includes(tabId)) return node;
		return { ...node, activeTab: tabId };
	}
	return {
		...node,
		children: [
			setActiveInLeaf(node.children[0], leafId, tabId),
			setActiveInLeaf(node.children[1], leafId, tabId)
		] as [TileNode, TileNode]
	};
}

function reorderTabInLeaf(node: TileNode, leafId: string, tabId: PanelId, newIndex: number): TileNode {
	if (node.type === 'leaf') {
		if (node.id !== leafId) return node;
		const filtered = node.tabs.filter(t => t !== tabId);
		filtered.splice(Math.min(newIndex, filtered.length), 0, tabId);
		return { ...node, tabs: filtered };
	}
	return {
		...node,
		children: [
			reorderTabInLeaf(node.children[0], leafId, tabId, newIndex),
			reorderTabInLeaf(node.children[1], leafId, tabId, newIndex)
		] as [TileNode, TileNode]
	};
}

class LayoutStore {
	root: TileNode = $state(createDefaultLayout());
	dragTab: { tabId: PanelId; sourceLeafId: string } | null = $state(null);

	constructor() {
		this.load();
	}

	private load() {
		try {
			const raw = localStorage.getItem('tmctol-tile-layout');
			if (raw) {
				const parsed = JSON.parse(raw);
				if (parsed && (parsed.type === 'leaf' || parsed.type === 'split') && isValidTree(parsed)) {
					recalcNextId(parsed);
					this.root = parsed;
					return;
				}
			}
		} catch {}
	}

	private persist() {
		try { localStorage.setItem('tmctol-tile-layout', JSON.stringify(this.root)); } catch {}
	}

	startDrag(tabId: PanelId, sourceLeafId: string) {
		this.dragTab = { tabId, sourceLeafId };
	}

	endDrag() {
		this.dragTab = null;
	}

	setActiveTab(leafId: string, tabId: PanelId) {
		this.root = setActiveInLeaf(this.root, leafId, tabId);
		this.persist();
	}

	dropOnEdge(targetLeafId: string, edge: DropEdge) {
		if (!this.dragTab) return;
		const { tabId, sourceLeafId } = this.dragTab;

		if (sourceLeafId === targetLeafId) {
			const leaf = findLeaf(this.root, sourceLeafId);
			if (leaf && leaf.tabs.length <= 1) { this.endDrag(); return; }
		}

		let result = removeTabFromLeaf(this.root, sourceLeafId, tabId);
		result = splitLeafWithTab(result, targetLeafId, tabId, edge);
		result = collapseEmpty(result);

		this.root = result;
		this.persist();
		this.endDrag();
	}

	dropOnTabBar(targetLeafId: string, insertIndex?: number) {
		if (!this.dragTab) return;
		const { tabId, sourceLeafId } = this.dragTab;

		if (sourceLeafId === targetLeafId) { this.endDrag(); return; }

		let result = removeTabFromLeaf(this.root, sourceLeafId, tabId);
		result = addTabToLeaf(result, targetLeafId, tabId, insertIndex);
		result = collapseEmpty(result);

		this.root = result;
		this.persist();
		this.endDrag();
	}

	reorderTab(leafId: string, tabId: PanelId, newIndex: number) {
		this.root = reorderTabInLeaf(this.root, leafId, tabId, newIndex);
		this.persist();
	}

	resizeSplit(splitId: string, ratio: number) {
		this.root = updateSplitRatio(this.root, splitId, ratio);
		this.persist();
	}

	resetLayout() {
		nextId = 0;
		this.root = createDefaultLayout();
		this.persist();
	}
}

export const layoutStore = new LayoutStore();
