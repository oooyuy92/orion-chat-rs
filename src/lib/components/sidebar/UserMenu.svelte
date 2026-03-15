<script lang="ts">
	import { SidebarFooter, SidebarMenu, SidebarMenuItem, SidebarMenuButton } from '$lib/components/ui/sidebar';
	import { useSidebar } from '$lib/components/ui/sidebar/context.svelte.js';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import PanelLeftCloseIcon from '@lucide/svelte/icons/panel-left-close';
	import { goto } from '$app/navigation';
	import { i18n } from '$lib/stores/i18n.svelte';

	const sidebar = useSidebar();
</script>

<SidebarFooter>
	<SidebarMenu>
		<SidebarMenuItem>
			<div class="flex items-center gap-0.5">
				<SidebarMenuButton onclick={() => goto('/settings')} class="cursor-pointer flex-1">
					<SettingsIcon class="size-4" />
					<span>{i18n.t.settings}</span>
				</SidebarMenuButton>
				<button
					class="collapse-btn"
					onclick={() => sidebar.toggle()}
					title={sidebar.open ? i18n.t.collapseSidebar : i18n.t.expandSidebar}
				>
					<PanelLeftCloseIcon class="size-4" />
				</button>
			</div>
		</SidebarMenuItem>
	</SidebarMenu>
</SidebarFooter>

<style>
	.collapse-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 1.75rem;
		height: 1.75rem;
		border-radius: 0.375rem;
		color: var(--sidebar-foreground);
		opacity: 0.5;
		cursor: pointer;
		flex-shrink: 0;
		transition: opacity 0.15s, background-color 0.15s;
	}
	.collapse-btn:hover {
		opacity: 1;
		background-color: var(--sidebar-accent);
	}
</style>
