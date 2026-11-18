<template>
  <div class="titlebar" :class="{ 'is-mac': isMac }">
    <div class="titlebar-left" data-tauri-drag-region>
      <span class="titlebar-name">EasyMD</span>
      <button
        v-if="vaultName"
        ref="vaultBtn"
        class="titlebar-vault-btn"
        :title="t('vs.switch')"
        @pointerdown.stop
        @click="vaultMenuOpen = !vaultMenuOpen"
      >
        <span class="titlebar-vault-prefix">{{ t("vs.prefix") }}</span>
        <span class="titlebar-vault-name">{{ vaultName }}</span>
        <Icon name="chevron-down" :size="10" class="titlebar-vault-caret" />
      </button>
    </div>

    <!-- 知识库切换下拉 -->
    <DropdownMenu
      :open="vaultMenuOpen"
      :anchor="vaultBtn"
      align="left"
      :title="t('vs.switch')"
      @close="vaultMenuOpen = false"
    >
      <button
        v-for="v in vaultList"
        :key="v.path"
        class="tb-vault-item"
        :class="{ 'is-current': v.path === vault.root }"
        @click="switchVault(v.path)"
      >
        <Icon :name="v.path === vault.root ? 'check' : 'book'" :size="13" />
        <span class="tb-vault-name">{{ v.name }}</span>
      </button>
      <div class="emd-dd-sep" />
      <button class="tb-vault-item" @click="vaultMenuOpen = false; ui.view = 'editor'; vault.close()">
        <Icon name="external-link" :size="13" />
        <span>{{ t("vs.back") }}</span>
      </button>
    </DropdownMenu>

    <!-- 全局搜索入口：点击唤起搜索面板 -->
    <div class="titlebar-search" data-tauri-drag-region>
      <div class="ts-box" :title="t('tb.searchTitle')" @pointerdown.stop @click="openSearch">
        <Icon name="search" :size="13" />
        <span class="ts-hint">{{ t("tb.searchHint") }}</span>
        <span class="ts-kbd">{{ keyMod }}+Shift+F</span>
      </div>
    </div>

    <div class="titlebar-drag" data-tauri-drag-region />
    <WindowControls />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import Icon from "./Icon.vue";
import WindowControls from "./WindowControls.vue";
import DropdownMenu from "./DropdownMenu.vue";
import { useVaultStore } from "../../stores/vault";
import { useUiStore } from "../../stores/ui";
import { t } from "../../i18n";
import { isMac, keyMod } from "../../lib/platform";
import type { VaultEntry } from "../../types";

const vault = useVaultStore();
const ui = useUiStore();
const vaultMenuOpen = ref(false);
const vaultBtn = ref<HTMLElement>();
const vaultList = ref<VaultEntry[]>([]);

const vaultName = computed(() => {
  if (!vault.opened || !vault.root) return "";
  const parts = vault.root.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || vault.root;
});

onMounted(async () => {
  await vault.loadVaultList();
  vaultList.value = vault.vaultList;
});

async function switchVault(path: string) {
  vaultMenuOpen.value = false;
  if (path === vault.root) return;
  try {
    if (vault.opened) await vault.close();
    await vault.open(path);
  } catch (e) {
    console.error("switch vault failed:", e);
  }
}

function openSearch() {
  ui.openSearch("content");
}
</script>

<style scoped>
.titlebar {
  position: relative;
  height: var(--titlebar-height);
  display: flex;
  align-items: stretch;
  background: var(--titlebar-background);
  border-bottom: 1px solid var(--background-modifier-border);
  user-select: none;
  flex-shrink: 0;
}
.titlebar-left {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  color: var(--text-muted);
  font-size: var(--font-ui-size);
  flex-shrink: 0;
}
/* macOS：原生红黄绿按钮浮在网页之上，左侧内容让开这段宽度，双击空白处仍可缩放窗口 */
.titlebar.is-mac .titlebar-left {
  padding-left: var(--traffic-light-inset);
}
.titlebar-name {
  color: var(--text-normal);
  font-weight: 600;
  letter-spacing: 0.02em;
}
.titlebar-vault-btn {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 3px 10px;
  border-radius: var(--radius-s);
  font-size: var(--font-ui-size);
  font-weight: 600;
  color: var(--text-normal);
  border: 1px solid var(--background-modifier-border);
  background: var(--background-primary);
  transition: background var(--anim-fast);
}
.titlebar-vault-btn:hover {
  background: var(--background-modifier-hover);
  border-color: var(--background-modifier-border-hover);
}
.titlebar-vault-prefix {
  font-weight: 400;
  font-size: var(--font-ui-smaller);
  color: var(--text-muted);
}
.titlebar-vault-caret {
  opacity: 0.6;
}
.titlebar-vault-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 220px;
}
.tb-vault-item {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  padding: 6px 10px;
  border-radius: var(--radius-s);
  color: var(--text-normal);
  font-size: var(--font-ui-size);
  text-align: left;
}
.tb-vault-item:hover {
  background: var(--background-modifier-hover);
}
.tb-vault-item.is-current {
  color: var(--text-accent);
}
.tb-vault-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.titlebar-search {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px 12px;
}
/* macOS：搜索框按窗口绝对居中（不因左侧让开红黄绿而右移），
   左侧内容因此需要自己收敛宽度，避免压到居中的搜索框 */
.titlebar.is-mac .titlebar-left {
  min-width: 0;
  max-width: calc(50% - 200px);
  overflow: hidden;
}
.titlebar.is-mac .titlebar-search {
  padding: 0;
}
.titlebar.is-mac .ts-box {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: min(360px, 40vw);
}
.ts-box {
  width: min(360px, 100%);
  height: 26px;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 10px;
  border-radius: var(--radius-m);
  border: 1px solid var(--background-modifier-border);
  background: var(--background-primary);
  color: var(--text-faint);
  cursor: pointer;
  transition: border-color var(--anim-fast), background var(--anim-fast);
}
.ts-box:hover {
  border-color: var(--background-modifier-border-hover);
  background: var(--background-primary-alt);
}
.ts-hint {
  flex: 1;
  font-size: var(--font-ui-smaller);
  text-align: left;
}
.ts-kbd {
  font-size: 10px;
  padding: 1px 5px;
  border-radius: var(--radius-s);
  border: 1px solid var(--background-modifier-border);
  color: var(--text-faint);
  white-space: nowrap;
}
.titlebar-drag {
  flex: 0 1 40px;
}
</style>
