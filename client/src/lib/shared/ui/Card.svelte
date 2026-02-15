<script lang="ts">
	import type { Snippet } from 'svelte';

	type Props = {
		level?: 0 | 1 | 2 | 3;
		class?: string;
		children: Snippet;
	};

	let { level = 1, children, class: cls = '' }: Props = $props();

	const levels = {
		0: 'bg-(--mono-bg)',
		1: 'bg-white',
		2: 'bg-(--mono-bg)',
		3: 'bg-[#eaece3]'
	};
</script>

<div class={[levels[level], "rounded-2xl border border-(--mono-border) shadow-[0_2px_10px_rgba(44,50,30,0.06)]", cls]}>
	{@render children()}
</div>
