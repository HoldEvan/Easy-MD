<template>
  <div v-if="jobs.length || notice" class="image-upload-queue" aria-live="polite" :aria-busy="busy">
    <div v-if="notice" class="image-upload-notice">
      <span>{{ notice }}</span><button type="button" :title="t('upload.dismissTitle')" @click="$emit('dismiss')">{{ t('upload.close') }}</button>
    </div>
    <div v-for="job in jobs" :key="job.id" class="image-upload-job">
      <Icon :name="job.state === 'busy' || job.state === 'queued' ? 'upload' : 'image'" :size="14" />
      <div class="image-upload-detail"><strong>{{ job.name || t('upload.pastedImage') }}</strong><span>{{ job.messageKey ? tf(job.messageKey, job.messageVars || {}) : job.message }}</span><small>{{ tf('upload.target', { path: job.target.path }) }}</small></div>
      <template v-if="job.state === 'failed' || job.state === 'pending'">
        <button type="button" @click="$emit('retry', job.id)">{{ t(job.url ? 'upload.retryInsert' : 'upload.retry') }}</button>
        <button v-if="!job.url" type="button" @click="$emit('local', job.id)">{{ t('upload.saveLocal') }}</button>
      </template>
      <button type="button" :title="t('upload.cancelTitle')" @click="$emit('cancel', job.id)">{{ t('upload.cancel') }}</button>
    </div>
  </div>
</template>
<script setup lang="ts">
import { t, tf } from "../../i18n";
import Icon from "../common/Icon.vue";
import type { ImageInsertionJob } from "../../composables/useImageInsertion";
defineProps<{ jobs: ImageInsertionJob[]; notice: string; busy: boolean }>();
defineEmits<{ retry: [id: number]; local: [id: number]; cancel: [id: number]; dismiss: [] }>();
</script>
<style scoped>
.image-upload-queue { flex: 0 0 auto; max-height: 180px; overflow-y: auto; border-bottom: 1px solid var(--background-modifier-border); background: var(--background-secondary); padding: 6px 12px; font-size: 13px; }
.image-upload-notice, .image-upload-job { display: flex; align-items: center; gap: 10px; padding: 4px 0; color: var(--text-muted); }
.image-upload-notice > span, .image-upload-detail { flex: 1; min-width: 0; }
.image-upload-detail { display: flex; flex-direction: column; gap: 2px; overflow-wrap: anywhere; }
.image-upload-detail strong { font-weight: 500; color: var(--text-normal); }
.image-upload-detail small { color: var(--text-faint); }
button { flex: 0 0 auto; padding: 4px 7px; border: 1px solid var(--background-modifier-border); border-radius: 4px; color: var(--text-normal); background: var(--background-primary); cursor: pointer; }
button:hover { background: var(--background-modifier-hover); }
button:focus-visible { outline: 2px solid var(--interactive-accent); outline-offset: 2px; }
</style>
