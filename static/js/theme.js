(function() {
    // preload 클래스 제거 (초기 로드 후 transition 활성화)
    document.body.classList.add('preload');
    window.addEventListener('load', function() {
        setTimeout(function() {
            document.body.classList.remove('preload');
            // transition 활성화
            document.body.style.transition = 'background-color var(--transition-base), color var(--transition-base)';
        }, 100);
    });

    const themeToggle = document.getElementById('theme-toggle');
    const html = document.documentElement;
    const savedTheme = localStorage.getItem('theme') || 'dark';

    updateThemeButton(savedTheme);

    themeToggle.addEventListener('click', function() {
        const currentTheme = html.getAttribute('data-bs-theme');
        const newTheme = currentTheme === 'dark' ? 'light' : 'dark';

        html.setAttribute('data-bs-theme', newTheme);
        localStorage.setItem('theme', newTheme);
        updateThemeButton(newTheme);
    });

    function updateThemeButton(theme) {
        const icon = themeToggle.querySelector('.theme-icon');
        const text = themeToggle.querySelector('.theme-text');

        if (theme === 'dark') {
            icon.textContent = '🌙';
            text.textContent = '다크 모드';
        } else {
            icon.textContent = '☀️';
            text.textContent = '라이트 모드';
        }
    }
})();