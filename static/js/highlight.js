// Code highlighting functionality
document.addEventListener('DOMContentLoaded', function() {
    // Load highlight.js from CDN
    const highlightScript = document.createElement('script');
    highlightScript.src = 'https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js';
    highlightScript.onload = function() {
        // Initialize highlight.js
        hljs.highlightAll();
        
        // Add line numbers to code blocks
        document.querySelectorAll('pre code').forEach((block) => {
            const lines = block.innerHTML.split('\n');
            const numberedLines = lines.map((line, index) => {
                return `<span class="line-number">${index + 1}</span>${line}`;
            }).join('\n');
            block.innerHTML = numberedLines;
            block.classList.add('line-numbers');
        });
    };
    document.head.appendChild(highlightScript);
    
    // Load highlight.js CSS
    const highlightCSS = document.createElement('link');
    highlightCSS.rel = 'stylesheet';
    highlightCSS.href = 'https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css';
    highlightCSS.media = '(prefers-color-scheme: dark)';
    document.head.appendChild(highlightCSS);
    
    const highlightCSSLight = document.createElement('link');
    highlightCSSLight.rel = 'stylesheet';
    highlightCSSLight.href = 'https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github.min.css';
    highlightCSSLight.media = '(prefers-color-scheme: light)';
    document.head.appendChild(highlightCSSLight);
    
    // Update CSS based on dark mode
    function updateHighlightCSS() {
        const isDark = document.documentElement.classList.contains('dark');
        highlightCSS.media = isDark ? 'all' : 'not all';
        highlightCSSLight.media = isDark ? 'not all' : 'all';
    }
    
    // Watch for dark mode changes
    const observer = new MutationObserver(function(mutations) {
        mutations.forEach(function(mutation) {
            if (mutation.attributeName === 'class') {
                updateHighlightCSS();
            }
        });
    });
    
    observer.observe(document.documentElement, { attributes: true });
    updateHighlightCSS();
    
    // Copy code button functionality
    document.addEventListener('click', function(e) {
        if (e.target.classList.contains('copy-code-btn')) {
            const codeBlock = e.target.parentElement.querySelector('code');
            const codeText = Array.from(codeBlock.childNodes)
                .filter(node => node.nodeType === Node.TEXT_NODE || node.tagName !== 'SPAN')
                .map(node => node.textContent)
                .join('');
            
            navigator.clipboard.writeText(codeText).then(function() {
                e.target.textContent = 'コピーしました！';
                setTimeout(function() {
                    e.target.textContent = 'コピー';
                }, 2000);
            });
        }
    });
});