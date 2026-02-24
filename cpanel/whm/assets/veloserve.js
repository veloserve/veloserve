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
        fetch('?action=' + action, { method: 'POST', headers: {'Content-Type':'application/x-www-form-urlencoded'}, body: body })
            .then(function(r){ return r.json(); })
            .then(function(d){ if (cb) cb(d); })
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
    }
};
