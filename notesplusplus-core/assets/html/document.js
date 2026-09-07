document.addEventListener('DOMContentLoaded', function() {
    document.querySelectorAll('.document-content pre').forEach(function(pre) {
        pre.style.position = 'relative';
        var copyBtn = document.createElement('button');
        copyBtn.className = 'copy-code-btn';
        copyBtn.innerText = '📋 Copy';
        copyBtn.title = 'Copy code to clipboard';
        copyBtn.onclick = function(e) {
            e.stopPropagation();
            var codeEl = pre.querySelector('code') || pre;
            var text = codeEl.innerText || codeEl.textContent;
            navigator.clipboard.writeText(text).then(function() {
                copyBtn.innerText = '✓ Copied!';
                setTimeout(function() { copyBtn.innerText = '📋 Copy'; }, 2000);
            });
        };
        pre.appendChild(copyBtn);
    });
});
