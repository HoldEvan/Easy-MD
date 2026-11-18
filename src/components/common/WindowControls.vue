<template>
  <!--
    Windows/Linux 自绘窗口按钮（最小化 / 最大化还原 / 关闭）。
    macOS 使用系统原生红黄绿按钮（窗口配置 titleBarStyle: Overlay 提供），
    此处整块隐藏，避免出现两套按钮。
  -->
  <div v-if="!isMac" class="wc-controls">
    <button class="wc-btn" :title="t('tb.min')" @click="minimize">
      <Icon name="minus" :size="14" />
    </button>
    <button class="wc-btn" :title="t('tb.max')" @click="toggleMax">
      <Icon name="square" :size="12" />
    </button>
    <button class="wc-btn wc-close" :title="t('c.close')" @click="closeWin">
      <Icon name="x" :size="14" />
    </button>
  </div>
</template>

<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import Icon from "./Icon.vue";
import { t } from "../../i18n";
import { isMac } from "../../lib/platform";

const win = getCurrentWindow();
const minimize = () => win.minimize();
const toggleMax = () => win.toggleMaximize();
const closeWin = () => win.close();
</script>

<style scoped>
.wc-controls {
  display: flex;
  flex-shrink: 0;
}
.wc-btn {
  width: 46px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  transition: background var(--anim-fast);
}
.wc-btn:hover {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.wc-close:hover {
  background: #e81123;
  color: #fff;
}
</style>
