<script lang="ts">
	import { SidebarFooter, SidebarMenu, SidebarMenuItem, SidebarMenuButton } from '$lib/components/ui/sidebar';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import { goto } from '$app/navigation';
	import { i18n } from '$lib/stores/i18n.svelte';
</script>

<SidebarFooter>
	<SidebarMenu>
		<SidebarMenuItem>
			<SidebarMenuButton onclick={() => goto('/settings')} class="cursor-pointer">
				<SettingsIcon class="size-4" />
				<span>{i18n.t.settings}</span>
			</SidebarMenuButton>
		</SidebarMenuItem>
	</SidebarMenu>
</SidebarFooter>
