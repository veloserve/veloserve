var VS = {
    api: function(action, params, cb) {
        var url = '?action=' + action;
        if (params) {
            for (var k in params) url += '&' + k + '=' + encodeURIComponent(params[k]);
        }
        fetch(url).then(function(r){ return r.json(); }).then(function(d){
            if (cb) cb(d);
        }).catch(function(e){
            VS.toast('Error: ' + e, 'danger');
        });
    },

    post: function(action, body, cb) {
        fetch('?action=' + action, { method: 'POST', headers: {'Content-Type':'application/x-www-form-urlencoded'}, body: body || '' })
            .then(function(r) { return r.text(); })
            .then(function(text) {
                var d;
                text = (text || '').trim();
                try {
                    d = JSON.parse(text);
                } catch (e) {
                    var m = text.match(/\{[\s\S]*\}/);
                    try { d = m ? JSON.parse(m[0]) : null; } catch (e2) { d = null; }
                }
                if (cb) cb(d || { success: 0, message: 'Invalid response from server' }, text);
            })
            .catch(function(e){ VS.toast('Error: ' + e, 'danger'); });
    },

    confirm: function(msg, cb) {
        if (window.confirm(msg)) cb();
    },

    toast: function(msg, type) {
        var el = document.createElement('div');
        el.style.cssText = 'position:fixed;top:70px;right:24px;padding:12px 20px;border-radius:8px;font-size:14px;font-weight:500;z-index:9999;box-shadow:0 4px 12px rgba(0,0,0,.15);transition:opacity .3s;';
        if (type === 'success') el.style.background = '#dcfce7', el.style.color = '#166534';
        else if (type === 'danger') el.style.background = '#fee2e2', el.style.color = '#991b1b';
        else el.style.background = '#dbeafe', el.style.color = '#1e40af';
        el.textContent = msg;
        document.body.appendChild(el);
        setTimeout(function(){ el.style.opacity = '0'; setTimeout(function(){ el.remove(); }, 300); }, 3000);
    },

    controlServer: function(action) {
        VS.confirm('Are you sure you want to ' + action + ' VeloServe?', function() {
            VS.api('api_' + action, null, function(d) {
                VS.toast(d.message, d.success ? 'success' : 'danger');
                setTimeout(function(){ location.reload(); }, 1500);
            });
        });
    },

    switchWebServer: function(target) {
        var msg = target === 'veloserve'
            ? 'Switch to VeloServe? This will stop Apache and start VeloServe on ports 80/443.'
            : 'Switch to Apache? This will stop VeloServe and start Apache.';
        VS.confirm(msg, function() {
            var btn = event.target;
            btn.disabled = true;
            btn.innerHTML = '<span class="vs-spinner"></span> Switching...';
            VS.api('api_switch_' + target, null, function(d) {
                VS.toast(d.message, d.success ? 'success' : 'danger');
                setTimeout(function(){ location.reload(); }, 2000);
            });
        });
    },

    deleteVhost: function(domain) {
        VS.confirm('Remove virtual host: ' + domain + '?', function() {
            VS.api('api_vhost_delete', {domain: domain}, function(d) {
                VS.toast(d.message, d.success ? 'success' : 'danger');
                setTimeout(function(){ location.reload(); }, 1000);
            });
        });
    },

    importApache: function() {
        VS.confirm('Import virtual hosts from Apache httpd.conf?', function() {
            VS.api('api_vhost_import', null, function(d) {
                VS.toast(d.message, d.success ? 'success' : 'danger');
                setTimeout(function(){ location.reload(); }, 1000);
            });
        });
    },

    purgeCache: function() {
        VS.confirm('Purge all cached content?', function() {
            VS.api('api_cache_purge', null, function(d) {
                VS.toast(d.message, d.success ? 'success' : 'danger');
            });
        });
    },

    saveConfig: function() {
        var content = document.getElementById('config-editor').value;
        VS.post('api_config_save', 'content=' + encodeURIComponent(content), function(d) {
            VS.toast(d.message, d.success ? 'success' : 'danger');
        });
    },

    changePhp: function(version) {
        VS.confirm('Switch VeloServe PHP to EA-PHP ' + version + '?', function() {
            VS.api('api_php_switch', {version: version}, function(d) {
                VS.toast(d.message, d.success ? 'success' : 'danger');
                setTimeout(function(){ location.reload(); }, 1500);
            });
        });
    },

    refreshLogs: function() {
        var src = document.getElementById('log-source').value;
        var lines = document.getElementById('log-lines').value;
        VS.api('api_logs', {source: src, lines: lines}, function(d) {
            document.getElementById('log-output').textContent = d.content || 'No logs found';
            var el = document.getElementById('log-output');
            el.scrollTop = el.scrollHeight;
        });
    },

    startLogRefresh: function() {
        if (VS._logTimer) { clearInterval(VS._logTimer); VS._logTimer = null; return; }
        VS._logTimer = setInterval(VS.refreshLogs, 5000);
    },

    createApiToken: function(btn) {
        if (!btn) btn = document.getElementById('vs-create-token-btn');
        var block = document.getElementById('vs-new-token-block');
        if (btn) { btn.disabled = true; btn.textContent = 'Creating…'; }
        VS.post('api_token_create', 'action=api_token_create', function(d, rawText) {
            if (btn) { btn.disabled = false; btn.textContent = 'Create API token'; }
            if (d.success && d.token && block) {
                block.innerHTML = '<div class="vs-alert vs-alert-success mt-4">' +
                    '<strong>New token (copy now - it will not be shown again):</strong>' +
                    '<div class="vs-token-display mt-2"><code id="vs-new-token">' + VS.escapeHtml(d.token) + '</code> ' +
                    '<button type="button" class="vs-btn vs-btn-sm" onclick="VS.copyTokenEl(\'vs-new-token\')">Copy</button></div></div>';
                VS.toast('Token created. Copy it now.', 'success');
            } else if (block && rawText !== undefined) {
                block.innerHTML = '<div class="vs-alert vs-alert-danger mt-4">' +
                    '<strong>Server returned non-JSON response (first 600 chars):</strong>' +
                    '<pre class="vs-debug-pre">' + VS.escapeHtml(rawText.substring(0, 600)) + '</pre></div>';
                VS.toast(d.message || 'Failed to create token', 'danger');
            } else {
                VS.toast(d.message || 'Failed to create token', 'danger');
            }
        });
    },

    escapeHtml: function(s) {
        if (!s) return '';
        var d = document.createElement('div');
        d.textContent = s;
        return d.innerHTML;
    },

    copyTokenFromRow: function(btn) {
        var row = btn.closest('tr');
        var code = row ? row.querySelector('code[data-token]') : null;
        var token = code ? code.getAttribute('data-token') : null;
        if (token) VS.copyToClipboard(token, btn);
    },

    copyTokenEl: function(id) {
        var el = document.getElementById(id);
        if (el) VS.copyToClipboard(el.textContent.trim(), el.nextElementSibling);
    },

    copyToClipboard: function(text, feedbackEl) {
        if (navigator.clipboard && navigator.clipboard.writeText) {
            navigator.clipboard.writeText(text).then(function() {
                VS.toast('Copied to clipboard', 'success');
                if (feedbackEl) { feedbackEl.textContent = 'Copied!'; setTimeout(function(){ feedbackEl.textContent = 'Copy'; }, 2000); }
            }).catch(function() { VS.fallbackCopy(text, feedbackEl); });
        } else {
            VS.fallbackCopy(text, feedbackEl);
        }
    },

    fallbackCopy: function(text, feedbackEl) {
        var ta = document.createElement('textarea');
        ta.value = text;
        ta.style.position = 'fixed'; ta.style.left = '-9999px';
        document.body.appendChild(ta);
        ta.select();
        try {
            document.execCommand('copy');
            VS.toast('Copied to clipboard', 'success');
            if (feedbackEl) { feedbackEl.textContent = 'Copied!'; setTimeout(function(){ feedbackEl.textContent = 'Copy'; }, 2000); }
        } catch (e) { VS.toast('Copy failed', 'danger'); }
        document.body.removeChild(ta);
    }
};
