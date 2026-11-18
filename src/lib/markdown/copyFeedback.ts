const checkIcon = '<svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"><path d="m20 6-11 11-5-5"/></svg>';

type FeedbackState = {
  icon: string;
  title: string;
  label: string | null;
  request: number;
  timer?: ReturnType<typeof setTimeout>;
};
const states = new WeakMap<HTMLButtonElement, FeedbackState>();

/** Keep feedback local to each button and ignore superseded clipboard results. */
export async function copyWithFeedback(button: HTMLButtonElement, copy: () => Promise<boolean>): Promise<void> {
  let state = states.get(button);
  if (!state) {
    state = { icon: button.innerHTML, title: button.title, label: button.getAttribute('aria-label'), request: 0 };
    states.set(button, state);
  }
  clearTimeout(state.timer);
  const request = ++state.request;
  let copied = false;
  try { copied = await copy(); } catch { /* Report a failed clipboard write below. */ }
  if (request !== state.request) return;
  button.innerHTML = copied ? checkIcon : state.icon;
  button.title = copied ? '已复制' : '复制失败';
  button.setAttribute('aria-label', button.title);
  state.timer = setTimeout(() => {
    button.innerHTML = state.icon;
    button.title = state.title;
    if (state.label === null) button.removeAttribute('aria-label');
    else button.setAttribute('aria-label', state.label);
    states.delete(button);
  }, 2000);
}
