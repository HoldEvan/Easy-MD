import { localize, type MessageKey } from './i18n';

const checkIcon = '<svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"><path d="m20 6-11 11-5-5"/></svg>';
type FeedbackState = { icon: string; request: number; timer?: ReturnType<typeof setTimeout> };
const states = new WeakMap<HTMLButtonElement, FeedbackState>();
function label(button: HTMLButtonElement, key: MessageKey): void {
  localize(button, key, {}, 'title');
  localize(button, key, {}, 'aria-label');
}

/** Keep translated feedback local to each button and ignore superseded clipboard results. */
export async function copyWithFeedback(button: HTMLButtonElement, copy: () => Promise<boolean>): Promise<void> {
  let state = states.get(button);
  if (!state) { state = { icon: button.innerHTML, request: 0 }; states.set(button, state); }
  clearTimeout(state.timer);
  const request = ++state.request;
  let copied = false;
  try { copied = await copy(); } catch { /* Report the failed clipboard write below. */ }
  if (request !== state.request) return;
  button.innerHTML = copied ? checkIcon : state.icon;
  label(button, copied ? 'copied' : 'copyFailed');
  state.timer = setTimeout(() => {
    button.innerHTML = state.icon;
    label(button, 'copyCode');
    states.delete(button);
  }, 2000);
}
