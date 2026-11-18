<template>
  <div class="upload-pane">
    <p class="upload-tip">图片上传设置在整个应用中生效，所有知识库共用。保存后，新插入的图片使用所选方式。</p>
    <p v-if="loading" role="status" class="upload-tip">正在读取设置…</p>
    <button v-if="!loading && !config" class="emd-btn" @click="load">重新读取</button>
    <form v-if="config" @submit.prevent="save(false)">
      <fieldset :disabled="busy">
        <div class="upload-row">
          <label for="image-upload-provider">上传方式</label>
          <select id="image-upload-provider" v-model="config.provider" @change="clearFeedback">
            <option value="local">本地附件</option>
            <option value="s3">S3 兼容存储</option>
            <option value="picgo">PicGo</option>
          </select>
        </div>
        <p v-if="config.provider === 'local'" class="upload-tip">
          图片保存在当前知识库的附件目录。附件目录仍在「通用」设置中配置。
        </p>
        <template v-else-if="config.provider === 's3'">
          <div class="upload-row">
            <label for="image-upload-endpoint">API 端点</label>
            <input id="image-upload-endpoint" v-model="config.s3.endpoint" type="url" placeholder="https://s3.example.com" @input="clearFeedback" />
          </div>
          <p class="upload-tip">填写存储服务用于上传的 S3 API 地址，不是图片访问域名。</p>
          <div class="upload-row">
            <label for="image-upload-region">区域</label>
            <input id="image-upload-region" v-model="config.s3.region" placeholder="us-east-1" @input="clearFeedback" />
          </div>
          <div class="upload-row">
            <label for="image-upload-bucket">存储桶</label>
            <input id="image-upload-bucket" v-model="config.s3.bucket" placeholder="images" @input="clearFeedback" />
          </div>
          <div class="upload-row">
            <label for="image-upload-access-key">Access Key</label>
            <input id="image-upload-access-key" v-model="config.s3.access_key" autocomplete="off" spellcheck="false" @input="clearFeedback" />
          </div>
          <div class="upload-row">
            <label for="image-upload-secret">Secret Key</label>
            <input id="image-upload-secret" v-model="secret" type="password" autocomplete="new-password" :disabled="removeSecret" :placeholder="config.has_secret ? '已保存，留空保留' : '输入 Secret Key'" @input="clearFeedback" />
            <button v-if="config.has_secret || removeSecret" type="button" class="emd-btn" @click="toggleRemoveSecret">
              {{ removeSecret ? '取消移除' : '移除密钥' }}
            </button>
          </div>
          <p class="upload-tip" :class="{ 'upload-error': removeSecret }">
            {{ removeSecret ? '保存时将移除已保存的密钥。' : '留空会保留已保存的密钥；输入新值会替换。密钥不会回显。' }}
          </p>
          <div class="upload-row">
            <label for="image-upload-public-base">公开访问地址</label>
            <input id="image-upload-public-base" v-model="config.s3.public_base" type="url" placeholder="https://images.example.com" @input="clearFeedback" />
          </div>
          <p class="upload-tip">图片链接的基础地址，可填写公开桶地址或 CDN 域名；需允许公开读取。它与上传用的 API 端点可以不同。</p>
          <div class="upload-row">
            <label for="image-upload-prefix">对象路径前缀</label>
            <input id="image-upload-prefix" v-model="config.s3.prefix" placeholder="images/" @input="clearFeedback" />
          </div>
          <details class="upload-advanced">
            <summary>高级设置</summary>
            <label class="upload-checkbox">
              <input v-model="config.s3.path_style" type="checkbox" @change="clearFeedback" />
              使用路径风格（Path Style）
            </label>
            <p class="upload-tip">将桶名放在请求路径中。部分 S3 兼容服务需要开启，请按存储服务说明设置。</p>
          </details>
        </template>
        <template v-else>
          <div class="upload-row">
            <label for="image-upload-picgo">PicGo 服务地址</label>
            <input id="image-upload-picgo" v-model="config.picgo.url" type="url" placeholder="http://127.0.0.1:36677/upload" @input="clearFeedback" />
          </div>
          <p class="upload-tip">先启动外部 PicGo 并开启 HTTP 服务，默认地址为 http://127.0.0.1:36677/upload。</p>
          <p class="upload-tip">使用 Gitee 时，请在 PicGo 中安装并配置对应图床插件，再将其设为当前图床。EasyMD 通过 PicGo 上传，不内置 Gitee 图床。</p>
        </template>
        <p v-if="config.provider !== 'local'" class="upload-tip">云端上传支持 PNG、JPEG、GIF、WebP、BMP、ICO，单张图片不超过 20 MiB。</p>
        <div class="upload-actions">
          <button class="emd-btn" type="submit">{{ operation === 'save' ? '正在保存…' : '保存设置' }}</button>
          <button class="emd-btn" type="button" :disabled="config.provider === 'local'" @click="save(true)">
            {{ operation === 'test' ? '正在测试…' : '保存并测试上传' }}
          </button>
        </div>
        <p v-if="config.provider !== 'local'" class="upload-tip">测试会先保存当前设置，再上传一张测试图片并检查访问和清理结果。</p>
      </fieldset>
    </form>
    <p v-if="status" role="status" class="upload-status">{{ status }}</p>
    <p v-if="error" role="alert" class="upload-error">{{ error }}</p>
    <div v-if="testResult" class="upload-result" aria-live="polite">
      <p>{{ testResult.message }}</p>
      <dl>
        <div><dt>上传</dt><dd>{{ testResult.upload_ok ? '成功' : '失败' }}</dd></div>
        <div><dt>公开访问</dt><dd>{{ resultLabel(testResult.accessible, '可访问', '无法访问') }}</dd></div>
        <div><dt>测试文件清理</dt><dd>{{ resultLabel(testResult.cleanup_ok, '已清理', '未清理，请手动删除测试文件') }}</dd></div>
      </dl>
      <template v-if="testUrl">
        <a :href="testUrl" @click.prevent="openTestUrl">{{ testUrl }}</a>
        <p v-if="testResult.cleanup_ok" class="upload-tip">测试文件已清理，此链接可能已无法访问。</p>
        <template v-else>
          <img v-if="!previewFailed" :src="testUrl" alt="上传测试图片" referrerpolicy="no-referrer" @error="previewFailed = true" />
          <p v-else class="upload-tip">预览未能加载，可通过上方链接检查图片。</p>
        </template>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
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
const error = ref("");
const testResult = ref<ImageUploadTestResult | null>(null);
const previewFailed = ref(false);
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
  error.value = "";
  testResult.value = null;
  previewFailed.value = false;
}

async function openTestUrl() {
  if (!testUrl.value) return;
  error.value = "";
  try {
    await api.openExternalUrl(testUrl.value);
  } catch (e) {
    error.value = `打开测试图片链接失败：${String(e)}`;
  }
}

function toggleRemoveSecret() {
  removeSecret.value = !removeSecret.value;
  secret.value = "";
  clearFeedback();
}

function resultLabel(value: boolean | null, yes: string, no: string): string {
  return value === null ? "未检查或不支持" : value ? yes : no;
}

async function load() {
  loading.value = true;
  clearFeedback();
  try {
    config.value = await getImageUploadConfig();
  } catch (e) {
    error.value = `读取图片上传设置失败：${String(e)}`;
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
    status.value = test ? "设置已保存，正在测试上传…" : "图片上传设置已保存，对所有知识库生效。";
    if (test) {
      testResult.value = await testImageUpload();
      status.value = "设置已保存，测试完成。";
    }
  } catch (e) {
    status.value = "";
    error.value = `${saved ? '设置已保存，但测试失败' : '保存图片上传设置失败'}：${String(e)}`;
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
