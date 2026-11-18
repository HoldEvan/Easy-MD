import { invoke } from "@tauri-apps/api/core";

export type ImageUploadProvider = "local" | "s3" | "picgo";

export interface ImageUploadConfig {
  provider: ImageUploadProvider;
  s3: {
    endpoint: string;
    region: string;
    bucket: string;
    access_key: string;
    public_base: string;
    prefix: string;
    path_style: boolean;
  };
  picgo: { url: string };
  has_secret: boolean;
}

export interface ImageUploadResult {
  url: string;
}

export interface ImageUploadTestResult {
  url: string | null;
  upload_ok: boolean;
  accessible: boolean | null;
  cleanup_ok: boolean | null;
  message: string;
}

export function getImageUploadConfig(): Promise<ImageUploadConfig> {
  return invoke<ImageUploadConfig>("image_upload_get_config");
}

/** Omit secret to preserve it; an explicit empty string removes it. */
export function saveImageUploadConfig(
  config: ImageUploadConfig,
  secret?: string,
): Promise<ImageUploadConfig> {
  return invoke<ImageUploadConfig>("image_upload_save_config", {
    config,
    ...(secret === undefined ? {} : { secret }),
  });
}

export function uploadImage(bytes: number[], ext: string): Promise<ImageUploadResult> {
  return invoke<ImageUploadResult>("image_upload", { bytes, ext });
}

export function testImageUpload(): Promise<ImageUploadTestResult> {
  return invoke<ImageUploadTestResult>("image_upload_test");
}
