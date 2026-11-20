<template>
  <section class="image-view" :aria-label="t('image.title')">
    <div class="image-canvas">
      <img v-if="path" :key="`${path}:${retry}`" :src="assetUrl(path)" :alt="fileName"
        :class="{ 'is-pending': loading || failed }" draggable="false" @load="onLoad" @error="onError" />
      <div v-if="loading" class="image-state" role="status">{{ t('image.loading') }}</div>
      <div v-else-if="failed" class="image-state" role="alert">
        <span>{{ t('image.failed') }}</span>
        <button @click="reload">{{ t('image.retry') }}</button>
      </div>
    </div>
    <footer class="image-info">
      <span class="image-name" :title="path">{{ fileName }}</span>
      <span v-if="width && height">{{ t('image.dimensions') }} {{ width }} × {{ height }} px</span>
      <span v-if="size !== null" :title="`${size} B`">{{ t('image.fileSize') }} {{ formatBytes(size) }}</span>
    </footer>
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { api, assetUrl } from "../../ipc/tauri";
import { t } from "../../i18n";

const props = defineProps<{ path: string }>();
const fileName = computed(() => props.path.split(/[\\/]/).pop() || props.path);
const loading = ref(false);
const failed = ref(false);
const size = ref<number | null>(null);
const width = ref(0);
const height = ref(0);
const retry = ref(0);
let request = 0;
onBeforeUnmount(() => { request++; });
watch(() => props.path, async (path) => {
  const id = ++request;
  loading.value = Boolean(path);
  failed.value = false;
  size.value = null;
  width.value = 0;
  height.value = 0;
  if (!path) return;
  try {
    const metadata = await api.fileMetadata(path);
    if (id === request) size.value = metadata.size;
  } catch {
    // 图片仍可正常展示；元信息不可用时不显示大小。
  }
}, { immediate: true });

function onLoad(event: Event) {
  const image = event.currentTarget as HTMLImageElement;
  if (image.getAttribute("src") !== assetUrl(props.path)) return;
  width.value = image.naturalWidth;
  height.value = image.naturalHeight;
  loading.value = false;
  failed.value = false;
}
function onError(event: Event) {
  const image = event.currentTarget as HTMLImageElement;
  if (image.getAttribute("src") !== assetUrl(props.path)) return;
  loading.value = false;
  failed.value = true;
}
function reload() {
  failed.value = false;
  loading.value = true;
  retry.value++;
}
function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let amount = value / 1024;
  let unit = 0;
  while (amount >= 1024 && unit < units.length - 1) { amount /= 1024; unit++; }
  return `${amount.toFixed(amount < 10 ? 1 : 0)} ${units[unit]}`;
}
</script>

<style scoped>
.image-view {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  background: var(--background-primary);
  color: var(--text-normal);
  overflow: hidden;
}
.image-canvas {
  position: relative;
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
  overflow: hidden;
}
.image-canvas img {
  display: block;
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}
.image-canvas img.is-pending { visibility: hidden; }
.image-state {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  gap: 12px;
  color: var(--text-muted);
}
.image-state button {
  padding: 5px 10px;
  border: 1px solid var(--background-modifier-border);
  border-radius: var(--radius-s);
  background: var(--interactive-normal);
  color: var(--text-normal);
  cursor: pointer;
}
.image-state button:hover { background: var(--interactive-hover); }
.image-info {
  flex-shrink: 0;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 16px;
  padding: 8px 12px;
  border-top: 1px solid var(--background-modifier-border);
  background: var(--background-secondary);
  color: var(--text-muted);
  font-size: var(--font-ui-size);
}
.image-info > span { min-width: 0; }
.image-name { margin-right: auto; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
