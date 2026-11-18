<template>
  <Teleport to="body">
    <div class="emd-modal-bg" @click.self="close" @keydown.esc.stop="close">
      <section class="emd-modal share-dialog" role="dialog" aria-modal="true" aria-labelledby="share-title" tabindex="-1" ref="dialogEl" @keydown.tab="trapFocus">
        <h2 id="share-title">分享知识库</h2>
        <p class="share-help">分享已保存的笔记和附件（不含画布），退出 EasyMD 后停止服务。</p>
        <form @submit.prevent="start">
          <div class="share-fields">
          <label>端口<input v-model.number="port" type="number" min="1" max="65535" required :disabled="busy || status.running" /></label>
          <label>访问密码<input v-model="password" type="password" autocomplete="new-password" placeholder="留空则无需密码" :disabled="busy || status.running" /></label>
          </div>
          <p class="share-help">访客只需输入分享密码。此服务使用 HTTP，适合可信局域网。</p>
          <div class="share-status" aria-live="polite">
            <strong>{{ status.running ? `正在分享：${status.vaultName}` : '服务未启动' }}</strong>
            <div v-for="url in status.urls" :key="url" class="share-url">
              <input :value="url" readonly aria-label="分享地址" @focus="($event.target as HTMLInputElement).select()" />
              <button class="emd-btn" type="button" @click="copy(url)">复制地址</button>
              <button class="emd-btn" type="button" @click="open(url)">打开</button>
            </div>
            <div v-if="status.running && lanUrls.length" class="share-qr">
              <label v-if="lanUrls.length > 1">扫码地址<select v-model="qrUrl"><option v-for="url in lanUrls" :key="url" :value="url">{{ url }}</option></select></label>
              <img v-if="qrImage" :src="qrImage" width="128" height="128" alt="局域网分享地址二维码" />
              <p class="share-help">手机连接同一网络后扫码打开<br />{{ qrUrl }}</p>
            </div>
            <p v-else-if="status.running" class="share-help">未检测到局域网地址，连接 Wi-Fi 或有线网络后重新启动分享。</p>
            <p v-if="status.running" class="share-help">目录覆盖启动时的知识库。切换知识库后，请先停止，再重新启动。</p>
            <p v-if="message">{{ message }}</p>
          </div>
          <p v-if="error" class="share-error" role="alert">{{ error }}</p>
          <div class="share-actions">
            <button type="button" class="emd-btn" :disabled="busy || loading" @click="exportStatic">导出 ZIP 部署包</button>
            <button type="button" class="emd-btn" :disabled="busy" @click="close">关闭窗口</button>
            <button v-if="status.running" type="button" class="emd-btn" :disabled="busy" @click="stop">{{ busy ? '正在停止…' : '停止分享' }}</button>
            <button v-else class="emd-btn emd-btn-accent" :disabled="busy || loading" type="submit">{{ busy ? '正在启动…' : '启动分享' }}</button>
          </div>
        </form>
      </section>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import QRCode from 'qrcode';
import { save } from '@tauri-apps/plugin-dialog';
import { api, type ShareStatus } from '../../ipc/tauri';
import { useEditorStore } from '../../stores/editor';
const emit = defineEmits<{ close: [] }>();
const editor = useEditorStore();
const status = ref<ShareStatus>({ running: false, port: null, vaultName: null, urls: [] });
const lanUrls = computed(() => status.value.urls.filter(url => {
  try { const host = new URL(url).hostname; return host !== 'localhost' && host !== '127.0.0.1' && host !== '[::1]'; }
  catch { return false; }
}));
const qrUrl = ref('');
const qrImage = ref('');
watch(lanUrls, urls => { if (!urls.includes(qrUrl.value)) qrUrl.value = urls[0] ?? ''; });
watch(qrUrl, async url => {
  qrImage.value = '';
  if (!url) return;
  try {
    const image = await QRCode.toDataURL(url, { width: 224, margin: 4, errorCorrectionLevel: 'M' });
    if (qrUrl.value === url) qrImage.value = image;
  } catch (e) { error.value = `二维码生成失败：${e}`; }
});
const port = ref(80);
const password = ref('');
const busy = ref(false);
const loading = ref(true);
const error = ref('');
const message = ref('');
const dialogEl = ref<HTMLElement>();
const previousFocus = document.activeElement as HTMLElement | null;
function close() { if (!busy.value) emit('close'); }
function trapFocus(event: KeyboardEvent) {
  const controls = Array.from(dialogEl.value?.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled)') ?? []);
  const first = controls[0], last = controls[controls.length - 1];
  if (!first) { event.preventDefault(); return; }
  if (event.shiftKey && (document.activeElement === first || document.activeElement === dialogEl.value)) {
    event.preventDefault(); last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault(); first.focus();
  }
}
onMounted(async () => {
  dialogEl.value?.focus();
  try { status.value = await api.shareStatus(); port.value = status.value.port ?? 80; }
  catch (e) { error.value = String(e); }
  finally { loading.value = false; }
});
onUnmounted(() => previousFocus?.focus());
async function start() {
  if (!Number.isInteger(port.value) || port.value < 1 || port.value > 65535) { error.value = '端口必须是 1 到 65535 之间的整数'; return; }
  busy.value = true; error.value = ''; message.value = '';
  try {
    if (editor.isDirty) await editor.save();
    status.value = await api.shareStart(port.value, password.value);
    password.value = '';
  } catch (e) { error.value = `${e}。端口被占用或没有权限时，可改用 8080 等端口。`; }
  finally { busy.value = false; }
}
async function stop() {
  busy.value = true; error.value = ''; message.value = '';
  try { status.value = await api.shareStop(); }
  catch (e) { error.value = String(e); }
  finally { busy.value = false; }
}
async function open(url: string) {
  error.value = '';
  try { await api.openExternalUrl(url); }
  catch (e) { error.value = `无法打开浏览器：${e}`; }
}
async function exportStatic() {
  busy.value = true; error.value = ''; message.value = '';
  try {
    const destination = await save({
      defaultPath: 'EasyMD-Share.zip',
      filters: [{ name: 'ZIP 部署包', extensions: ['zip'] }],
    });
    if (!destination) return;
    await api.shareExportStatic(destination);
    message.value = '部署包已导出，可交给 NGINX 部署';
  } catch (e) { error.value = `导出失败：${e}`; }
  finally { busy.value = false; }
}
async function copy(url: string) {
  try { await navigator.clipboard.writeText(url); message.value = '地址已复制'; }
  catch { error.value = '复制失败，请选中地址手动复制'; }
}
</script>

<style scoped>
.share-dialog { width: min(520px, calc(100vw - 40px)); max-height: calc(100vh - 60px); overflow: auto; padding: 18px; }
h2 { font-size: 18px; margin: 0 0 8px; }
.share-help { color: var(--text-muted); font-size: 12px; line-height: 1.5; margin: 8px 0; }
label { display: grid; gap: 4px; margin: 8px 0; }
.share-fields { display: grid; grid-template-columns: 100px minmax(0, 1fr); gap: 12px; }
input { width: 100%; box-sizing: border-box; }
.share-status { border-top: 1px solid var(--background-modifier-border); padding-top: 10px; }
.share-url { display: flex; gap: 8px; margin-top: 8px; }
.share-url input { flex: 1; min-width: 0; }
.share-qr { margin-top: 10px; display: grid; grid-template-columns: 128px minmax(0, 1fr); align-items: center; gap: 10px; }
.share-qr label { grid-column: 1 / -1; }
.share-qr img { display: block; margin: 0 auto; }
.share-qr select { max-width: 100%; }
.share-qr .share-help { overflow-wrap: anywhere; }
.share-error { color: var(--text-error, var(--text-normal)); overflow-wrap: anywhere; }
.share-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px; flex-wrap: wrap; }
</style>
