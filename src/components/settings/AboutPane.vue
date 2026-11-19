<template>
  <div class="ab-head">
    <div class="ab-name">EasyMD</div>
    <p class="ab-tagline">{{ t("st.about.tagline") }}</p>
  </div>
  <div class="st-row">
    <label>{{ t("st.about.version") }}</label>
    <span class="ab-value">v{{ version }}</span>
  </div>
  <h3 class="ab-section">{{ t("st.about.forkSection") }}</h3>
  <div class="st-row">
    <label>{{ t("st.about.maintainer") }}</label>
    <span class="ab-value ab-select">{{ MAINTAINER }}</span>
  </div>
  <div class="st-row">
    <label>{{ t("st.about.email") }}</label>
    <span class="ab-value ab-select">{{ EMAIL }}</span>
    <button class="emd-btn" @click="copyEmail">
      <Icon :name="copied ? 'check' : 'copy'" :size="13" />
      {{ copied ? t("st.about.copied") : t("st.about.copy") }}
    </button>
  </div>
  <div class="st-row">
    <label>{{ t("st.about.repo") }}</label>
    <a class="ab-link" :href="REPO" @click.prevent="openRepo(REPO)">
      <Icon name="external-link" :size="13" /> github.com/HoldEvan/Easy-MD
    </a>
  </div>
  <p class="st-tip">{{ t("st.about.tip") }}</p>
  <h3 class="ab-section">{{ t("st.about.upstreamSection") }}</h3>
  <div class="st-row">
    <label>{{ t("st.about.originalAuthor") }}</label>
    <span class="ab-value ab-select">{{ ORIGINAL_AUTHOR }}</span>
  </div>
  <div class="st-row">
    <label>{{ t("st.about.originalEmail") }}</label>
    <span class="ab-value ab-select">{{ ORIGINAL_EMAIL }}</span>
  </div>
  <div class="st-row">
    <label>{{ t("st.about.upstreamRepo") }}</label>
    <a class="ab-link" :href="UPSTREAM_REPO" @click.prevent="openRepo(UPSTREAM_REPO)">
      <Icon name="external-link" :size="13" /> github.com/FortC/Easy-MD
    </a>
  </div>
  <div class="st-row">
    <label>{{ t("st.about.license") }}</label>
    <span class="ab-value">MIT</span>
  </div>
  <p class="st-tip">{{ t("st.about.forkNotice") }}</p>
  <details class="ab-license">
    <summary>{{ t("st.about.licenseText") }}</summary>
    <pre>{{ licenseText }}</pre>
  </details>
</template>

<script setup lang="ts">
import { onUnmounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import Icon from "../common/Icon.vue";
import { t } from "../../i18n";
import { api } from "../../ipc/tauri";
import pkg from "../../../package.json";
import licenseText from "../../../LICENSE?raw";

const MAINTAINER = "HoldEvan";
const EMAIL = "java_lyf@163.com";
const REPO = "https://github.com/HoldEvan/Easy-MD";
const ORIGINAL_AUTHOR = "clb (FortC)";
const ORIGINAL_EMAIL = "lamthebest@foxmail.com";
const UPSTREAM_REPO = "https://github.com/FortC/Easy-MD";

async function openRepo(url: string) {
  try {
    await api.openExternalUrl(url);
  } catch (e) {
    console.error("打开源码仓库失败：", e);
  }
}

// 版本运行时从 Tauri 读取，与安装包保持一致；读取失败退回清单版本
const version = ref(pkg.version);
getVersion()
  .then((v) => (version.value = v))
  .catch(() => {});

const copied = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined;
onUnmounted(() => clearTimeout(timer));

async function copyEmail() {
  try {
    await navigator.clipboard.writeText(EMAIL);
    copied.value = true;
    clearTimeout(timer);
    timer = setTimeout(() => (copied.value = false), 2000);
  } catch {
    // 剪贴板不可用时不提示，邮箱文本本身可选中手动复制
  }
}
</script>

<style scoped>
.ab-head {
  padding: 10px 0 8px;
}
.ab-name {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-normal);
}
.ab-tagline {
  margin-top: 4px;
  color: var(--text-muted);
}
.ab-section {
  margin: 18px 0 4px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-normal);
}
.st-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
  border-bottom: 1px solid var(--background-modifier-border);
}
.st-row label:first-child {
  width: 190px;
  flex-shrink: 0;
  color: var(--text-muted);
}
.ab-value {
  color: var(--text-normal);
  overflow-wrap: anywhere;
}
.ab-select {
  user-select: text;
}
.st-tip {
  color: var(--text-muted);
  padding: 6px 0;
}
.ab-link {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
  overflow-wrap: anywhere;
  color: var(--link-color);
}
.ab-link:hover {
  color: var(--link-color-hover);
}
.ab-license {
  color: var(--text-muted);
  padding: 6px 0;
}
.ab-license summary {
  cursor: pointer;
  color: var(--text-normal);
}
.ab-license pre {
  margin: 8px 0 0;
  padding: 12px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  user-select: text;
  font-family: var(--font-mono);
  font-size: 12px;
  background: var(--background-primary);
  border: 1px solid var(--background-modifier-border);
  border-radius: 4px;
}
</style>
