document.addEventListener('DOMContentLoaded', () => {
    // 네비게이션 링크 호버 효과
    const navLinks = document.querySelectorAll('.nav-link');
    navLinks.forEach(link => {
        link.addEventListener('mouseenter', function() {
            const icon = this.querySelector('.nav-icon');
            if (icon) {
                icon.style.transform = 'scale(1.2) rotate(5deg)';
            }
        });

        link.addEventListener('mouseleave', function() {
            const icon = this.querySelector('.nav-icon');
            if (icon) {
                icon.style.transform = 'scale(1) rotate(0deg)';
            }
        });
    });
});

// 탭 전환 함수
window.showTab = function(tabId, clickedElement) {
    const problemTab = document.getElementById('problem-tab');
    const submitTab = document.getElementById('submit-tab');
    const targetTab = document.getElementById(tabId);

    if (!problemTab || !submitTab || !targetTab) return;

    problemTab.style.display = 'none';
    submitTab.style.display = 'none';
    targetTab.style.display = 'block';

    document.querySelectorAll('.nav-tabs .nav-link').forEach(link => {
        link.classList.remove('active');
    });

    if (clickedElement) {
        clickedElement.classList.add('active');
    }
};
