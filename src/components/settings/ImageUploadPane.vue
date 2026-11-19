<template>
  <div class="upload-pane">
    <p class="upload-tip">{{ t('upload.scopeTip') }}</p>
    <p v-if="loading" role="status" class="upload-tip">{{ t('upload.loading') }}</p>
    <button v-if="!loading && !config" class="emd-btn" @click="load">{{ t('upload.reload') }}</button>
    <form v-if="config" @submit.prevent="save(false)">
      <fieldset :disabled="busy">
        <div class="upload-row">
          <label for="image-upload-provider">{{ t('upload.provider') }}</label>
          <select id="image-upload-provider" v-model="config.provider" @change="clearFeedback">
            <option value="local">{{ t('upload.local') }}</option>
            <option value="s3">{{ t('upload.s3') }}</option>
            <option value="picgo">PicGo</option>
          </select>
        </div>
        <p v-if="config.provider === 'local'" class="upload-tip">
          {{ t('upload.localTip') }}
        </p>
        <template v-else-if="config.provider === 's3'">
          <div class="upload-row">
            <label for="image-upload-endpoint">{{ t('upload.endpoint') }}</label>
            <input id="image-upload-endpoint" v-model="config.s3.endpoint" type="url" placeholder="https://s3.example.com" @input="clearFeedback" />
          </div>
          <p class="upload-tip">{{ t('upload.endpointTip') }}</p>
          <div class="upload-row">
            <label for="image-upload-region">{{ t('upload.region') }}</label>
            <input id="image-upload-region" v-model="config.s3.region" placeholder="us-east-1" @input="clearFeedback" />
          </div>
          <div class="upload-row">
            <label for="image-upload-bucket">{{ t('upload.bucket') }}</label>
            <input id="image-upload-bucket" v-model="config.s3.bucket" placeholder="images" @input="clearFeedback" />
          </div>
          <div class="upload-row">
            <label for="image-upload-access-key">{{ t('upload.accessKey') }}</label>
            <input id="image-upload-access-key" v-model="config.s3.access_key" autocomplete="off" spellcheck="false" @input="clearFeedback" />
          </div>
          <div class="upload-row">
            <label for="image-upload-secret">{{ t('upload.secretKey') }}</label>
            <input id="image-upload-secret" v-model="secret" type="password" autocomplete="new-password" :disabled="removeSecret" :placeholder="t(config.has_secret ? 'upload.secretSaved' : 'upload.secretPlaceholder')" @input="clearFeedback" />
            <button v-if="config.has_secret || removeSecret" type="button" class="emd-btn" @click="toggleRemoveSecret">
              {{ t(removeSecret ? 'upload.cancelRemove' : 'upload.removeSecret') }}
            </button>
          </div>
          <p class="upload-tip" :class="{ 'upload-error': removeSecret }">
            {{ t(removeSecret ? 'upload.removeSecretTip' : 'upload.secretTip') }}
          </p>
          <div class="upload-row">
            <label for="image-upload-public-base">{{ t('upload.publicBase') }}</label>
            <input id="image-upload-public-base" v-model="config.s3.public_base" type="url" placeholder="https://images.example.com" @input="clearFeedback" />
          </div>
          <p class="upload-tip">{{ t('upload.publicBaseTip') }}</p>
          <div class="upload-row">
            <label for="image-upload-prefix">{{ t('upload.prefix') }}</label>
            <input id="image-upload-prefix" v-model="config.s3.prefix" placeholder="images/" @input="clearFeedback" />
          </div>
          <details class="upload-advanced">
            <summary>{{ t('upload.advanced') }}</summary>
            <label class="upload-checkbox">
              <input v-model="config.s3.path_style" type="checkbox" @change="clearFeedback" />
              {{ t('upload.pathStyle') }}
            </label>
            <p class="upload-tip">{{ t('upload.pathStyleTip') }}</p>
          </details>
        </template>
        <template v-else>
          <div class="upload-row">
            <label for="image-upload-picgo">{{ t('upload.picgoUrl') }}</label>
            <input id="image-upload-picgo" v-model="config.picgo.url" type="url" placeholder="http://127.0.0.1:36677/upload" @input="clearFeedback" />
          </div>
          <p class="upload-tip">{{ t('upload.picgoTip') }}</p>
          <p class="upload-tip">{{ t('upload.giteeTip') }}</p>
        </template>
        <p v-if="config.provider !== 'local'" class="upload-tip">{{ t('upload.formatsTip') }}</p>
        <div class="upload-actions">
          <button class="emd-btn" type="submit">{{ t(operation === 'save' ? 'upload.saving' : 'upload.save') }}</button>
          <button class="emd-btn" type="button" :disabled="config.provider === 'local'" @click="save(true)">
            {{ t(operation === 'test' ? 'upload.testing' : 'upload.saveTest') }}
          </button>
        </div>
        <p v-if="config.provider !== 'local'" class="upload-tip">{{ t('upload.testTip') }}</p>
      </fieldset>
    </form>
    <p v-if="status" role="status" class="upload-status">{{ t(status) }}</p>
    <p v-if="error" role="alert" class="upload-error">{{ tf(error.key, { error: error.detail }) }}</p>
    <div v-if="testResult" class="upload-result" aria-live="polite">
      <p>{{ testMessage }}</p>
      <dl>
        <div><dt>{{ t('upload.upload') }}</dt><dd>{{ t(testResult.upload_ok ? 'upload.success' : 'upload.failed') }}</dd></div>
        <div><dt>{{ t('upload.publicAccess') }}</dt><dd>{{ resultLabel(testResult.accessible, 'upload.accessible', 'upload.inaccessible') }}</dd></div>
        <div><dt>{{ t('upload.cleanup') }}</dt><dd>{{ resultLabel(testResult.cleanup_ok, 'upload.cleaned', 'upload.notCleaned') }}</dd></div>
      </dl>
      <template v-if="testUrl">
        <a :href="testUrl" @click.prevent="openTestUrl">{{ testUrl }}</a>
        <p v-if="testResult.cleanup_ok" class="upload-tip">{{ t('upload.cleanedTip') }}</p>
        <template v-else>
          <img v-if="!previewFailed" :src="testUrl" :alt="t('upload.testImageAlt')" referrerpolicy="no-referrer" @error="previewFailed = true" />
          <p v-else class="upload-tip">{{ t('upload.previewFailed') }}</p>
        </template>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { t, tf } from "../../i18n";
import { api } from "../../ipc/tauri";
import {
  getImageUploadConfig,
  saveImageUploadConfig,
  testImageUpload,
  type ImageUploadConfig,
  type ImageUploadTestResult,
} from "../../lib/imageUploadApi";

const config = ref<ImageUploadConfig | null>(null);
const loading = ref(true);
const operation = ref<"save" | "test" | null>(null);
const busy = computed(() => loading.value || operation.value !== null);
const secret = ref("");
const removeSecret = ref(false);
const status = ref("");
const error = ref<{ key: string; detail: string } | null>(null);
const testResult = ref<ImageUploadTestResult | null>(null);
const previewFailed = ref(false);
// Translate only known fixed backend messages; preserve unexpected diagnostic details.
const testMessage = computed(() => {
  const result = testResult.value;
  if (!result) return "";
  const fixedMessages: Record<string, string> = {
    "无法生成测试图片": "upload.testGeneratedFailed",
    "测试图片已上传并删除；URL 仅供检查，已不再保留": "upload.testCleaned",
    "测试图片上传成功，但删除失败；请在图床手动删除该 URL 对应对象": "upload.testCleanupFailed",
    "测试图片上传成功；PicGo 不提供通用删除接口，请在图床手动删除该 URL 对应图片": "upload.testManualCleanup",
  };
  const key = fixedMessages[result.message];
  return key ? t(key) : result.message;
});
const testUrl = computed(() => {
  const value = testResult.value?.url;
  if (!value) return null;
  try {
    const url = new URL(value);
    return url.protocol === "https:" || url.protocol === "http:" ? url.href : null;
  } catch {
    return null;
  }
});

function clearFeedback() {
  status.value = "";
  error.value = null;
  testResult.value = null;
  previewFailed.value = false;
}

async function openTestUrl() {
  if (!testUrl.value) return;
  error.value = null;
  try {
    await api.openExternalUrl(testUrl.value);
  } catch (e) {
    error.value = { key: "upload.openTestFailed", detail: String(e) };
  }
}

function toggleRemoveSecret() {
  removeSecret.value = !removeSecret.value;
  secret.value = "";
  clearFeedback();
}

function resultLabel(value: boolean | null, yes: string, no: string): string {
  return t(value === null ? "upload.notChecked" : value ? yes : no);
}

async function load() {
  loading.value = true;
  clearFeedback();
  try {
    config.value = await getImageUploadConfig();
  } catch (e) {
    error.value = { key: "upload.loadFailed", detail: String(e) };
  } finally {
    loading.value = false;
  }
}

async function save(test: boolean) {
  if (!config.value || busy.value) return;
  operation.value = test ? "test" : "save";
  clearFeedback();
  let saved = false;
  try {
    config.value = await saveImageUploadConfig(
      config.value,
      removeSecret.value ? "" : secret.value || undefined,
    );
    secret.value = "";
    removeSecret.value = false;
    saved = true;
    status.value = test ? "upload.savedTesting" : "upload.saved";
    if (test) {
      testResult.value = await testImageUpload();
      status.value = "upload.testComplete";
    }
  } catch (e) {
    status.value = "";
    error.value = { key: saved ? "upload.testFailed" : "upload.saveFailed", detail: String(e) };
  } finally {
    operation.value = null;
  }
}

onMounted(load);
</script>

<style scoped>
.upload-pane { display: flex; flex-direction: column; gap: 10px; }
fieldset { border: 0; min-width: 0; padding: 0; margin: 0; }
.upload-row { display: flex; align-items: center; gap: 10px; padding: 10px 0; border-bottom: 1px solid var(--background-modifier-border); }
.upload-row > label { width: 104px; flex-shrink: 0; color: var(--text-muted); }
.upload-row input, .upload-row select { min-width: 0; flex: 1; }
.upload-row input { height: var(--input-height); padding: 0 8px; background: var(--background-primary); border: 1px solid var(--background-modifier-border); border-radius: var(--radius-s); }
.upload-row select { font: inherit; color: var(--text-normal); background: var(--background-primary); border: 1px solid var(--background-modifier-border); border-radius: var(--radius-s); height: var(--input-height); padding: 0 8px; }
.upload-tip { color: var(--text-muted); font-size: var(--font-ui-smaller); line-height: 1.6; padding-top: 6px; overflow-wrap: anywhere; }
.upload-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 16px; }
.upload-advanced { padding-top: 14px; }
.upload-advanced summary { cursor: pointer; color: var(--text-muted); }
.upload-checkbox { display: flex; align-items: center; gap: 8px; margin-top: 12px; }
.upload-checkbox input { accent-color: var(--interactive-accent); }
.upload-status { color: var(--text-success); line-height: 1.6; }
.upload-error { color: var(--text-error); line-height: 1.6; overflow-wrap: anywhere; }
.upload-result { background: var(--background-primary); border: 1px solid var(--background-modifier-border); border-radius: var(--radius-m); padding: 12px; line-height: 1.6; overflow-wrap: anywhere; user-select: text; }
.upload-result dl { margin: 8px 0; }
.upload-result dl > div { display: flex; gap: 12px; }
.upload-result dt { flex: 0 0 104px; color: var(--text-muted); }
.upload-result dd { margin: 0; }
.upload-result a { color: var(--text-accent); }
.upload-result img { display: block; max-width: 100%; max-height: 160px; margin-top: 10px; object-fit: contain; }
button:disabled, fieldset:disabled button { opacity: 0.5; cursor: default; }
input:focus-visible, select:focus-visible, button:focus-visible, summary:focus-visible { outline: 2px solid var(--interactive-accent); outline-offset: 2px; }
</style>
