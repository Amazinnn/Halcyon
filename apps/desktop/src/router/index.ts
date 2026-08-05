import { createRouter, createWebHashHistory } from "vue-router";

// Panel window uses two internal routes (Chat / Statistics). Other windows
// render their own root component from App.vue and never mount <router-view>.
export default createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/panel/chat" },
    { path: "/panel/chat", component: () => import("../views/panel/ChatPanel.vue") },
    { path: "/panel/statistics", component: () => import("../views/panel/StatisticsPanel.vue") },
  ],
});