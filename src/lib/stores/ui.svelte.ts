export const ui = (() => {
  let sidebarOpen = $state(true);

  return {
    get sidebarOpen() {
      return sidebarOpen;
    },
    set sidebarOpen(value: boolean) {
      sidebarOpen = value;
    },
    toggleSidebar() {
      sidebarOpen = !sidebarOpen;
    },
  };
})();
