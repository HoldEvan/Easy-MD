// Authentication also protects share.js, so this page must initialize itself.
(() => {
  const messages = {
    'zh-CN': { language: '语言', title: '访问知识库', prompt: '请输入分享密码', password: '分享密码', submit: '登录', error: '密码错误，请重试' },
    en: { language: 'Language', title: 'Access vault', prompt: 'Enter the share password', password: 'Share password', submit: 'Log in', error: 'Incorrect password. Please try again.' }
  };
  let saved = null;
  try {
    saved = localStorage.getItem('share-language');
    document.documentElement.dataset.theme = localStorage.getItem('share-theme') === 'light' ? 'light' : 'dark';
  } catch { /* Storage may be disabled. */ }
  let language = saved === 'zh-CN' || saved === 'en' ? saved : null;
  if (!language) {
    for (const candidate of navigator.languages?.length ? navigator.languages : [navigator.language]) {
      if (/^zh(?:-|$)/i.test(candidate)) { language = 'zh-CN'; break; }
      if (/^en(?:-|$)/i.test(candidate)) { language = 'en'; break; }
    }
  }
  language = language || 'en';
  const picker = document.getElementById('login-language');
  const error = document.getElementById('login-error');
  const knownError = error.textContent.trim() === messages['zh-CN'].error;
  function apply() {
    const text = messages[language];
    document.documentElement.lang = language;
    document.title = text.title;
    document.querySelectorAll('[data-login-i18n]').forEach(node => { node.textContent = text[node.dataset.loginI18n]; });
    const password = document.getElementById('share-password');
    password.placeholder = text.password;
    password.setAttribute('aria-label', text.password);
    picker.setAttribute('aria-label', text.language);
    picker.value = language;
    if (knownError) error.textContent = text.error;
  }
  picker.addEventListener('change', () => {
    language = picker.value === 'en' ? 'en' : 'zh-CN';
    try { localStorage.setItem('share-language', language); } catch { /* Continue without persistence. */ }
    apply();
  });
  apply();
})();
