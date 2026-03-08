<?php

if (!defined('ABSPATH')) {
    exit;
}

class VeloServe_Admin
{
    public function hooks()
    {
        add_action('admin_menu', [$this, 'add_menu']);
        add_action('admin_init', [$this, 'register_settings']);
        add_action('admin_post_veloserve_register', [$this, 'handle_register']);
        add_action('admin_post_veloserve_admin_bar_register', [$this, 'handle_admin_bar_register']);
        add_action('admin_post_veloserve_purge_all', [$this, 'handle_purge_all']);
        add_action('admin_post_veloserve_admin_bar_purge_all', [$this, 'handle_admin_bar_purge_all']);
        add_action('admin_post_veloserve_test_cdn', [$this, 'handle_test_cdn']);
        add_action('admin_post_veloserve_tools_db_optimize', [$this, 'handle_tools_db_optimize']);
        add_action('admin_post_veloserve_tools_warm_sitemap', [$this, 'handle_tools_warm_sitemap']);
        add_action('admin_post_veloserve_tools_export_settings', [$this, 'handle_tools_export_settings']);
        add_action('admin_post_veloserve_tools_import_settings', [$this, 'handle_tools_import_settings']);
        add_action('admin_post_veloserve_tools_download_debug', [$this, 'handle_tools_download_debug']);
        add_action('admin_bar_menu', [$this, 'add_admin_bar_nodes'], 90);
        add_action('admin_notices', [$this, 'render_notices']);
    }

    public function add_menu()
    {
        add_menu_page(
            'VeloServe',
            'VeloServe',
            'manage_options',
            'veloserve',
            [$this, 'render_page'],
            'dashicons-performance',
            65
        );

        add_submenu_page('veloserve', 'Dashboard', 'Dashboard', 'manage_options', 'veloserve', [$this, 'render_page']);
        add_submenu_page('veloserve', 'Connection', 'Connection', 'manage_options', 'veloserve&tab=connection', [$this, 'render_page']);
        add_submenu_page('veloserve', 'General', 'General', 'manage_options', 'veloserve&tab=general', [$this, 'render_page']);
        add_submenu_page('veloserve', 'Cache', 'Cache', 'manage_options', 'veloserve&tab=cache', [$this, 'render_page']);
        add_submenu_page('veloserve', 'CDN', 'CDN', 'manage_options', 'veloserve&tab=cdn', [$this, 'render_page']);
        add_submenu_page('veloserve', 'Tools', 'Tools', 'manage_options', 'veloserve&tab=tools', [$this, 'render_page']);
    }

    public function register_settings()
    {
        register_setting('veloserve_settings_group', VELOSERVE_OPTION_KEY, [$this, 'sanitize']);

        add_settings_section('veloserve_connection', 'Connection', '__return_false', 'veloserve_connection');
        add_settings_section('veloserve_general', 'General', '__return_false', 'veloserve_general');

        add_settings_field('endpoint_url', 'Endpoint URL', [$this, 'render_endpoint_field'], 'veloserve_connection', 'veloserve_connection');
        add_settings_field('api_token', 'API Token', [$this, 'render_token_field'], 'veloserve_connection', 'veloserve_connection');
        add_settings_field('auto_detect_server', 'Auto-Detect Server', [$this, 'render_auto_detect_field'], 'veloserve_general', 'veloserve_general');
        add_settings_field('guest_mode', 'Guest Mode', [$this, 'render_guest_mode_field'], 'veloserve_general', 'veloserve_general');
        add_settings_field('server_ip_override', 'Server IP Override', [$this, 'render_server_ip_override_field'], 'veloserve_general', 'veloserve_general');
        add_settings_field('notifications_enabled', 'Notifications', [$this, 'render_notifications_field'], 'veloserve_general', 'veloserve_general');
        add_settings_field('auto_purge', 'Auto Purge', [$this, 'render_auto_purge_field'], 'veloserve_general', 'veloserve_general');
    }

    public function sanitize($input)
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        if (!is_array($settings)) {
            $settings = VeloServe_Plugin::default_settings();
        }

        $settings['endpoint_url'] = isset($input['endpoint_url']) ? esc_url_raw(trim($input['endpoint_url'])) : '';
        $settings['api_token'] = isset($input['api_token']) ? sanitize_text_field(trim($input['api_token'])) : '';
        $settings['auto_detect_server'] = !empty($input['auto_detect_server']) ? 1 : 0;
        $settings['guest_mode'] = !empty($input['guest_mode']) ? 1 : 0;

        $server_ip = isset($input['server_ip_override']) ? sanitize_text_field(trim($input['server_ip_override'])) : '';
        if ($server_ip !== '' && filter_var($server_ip, FILTER_VALIDATE_IP) === false) {
            $server_ip = '';
        }

        $settings['server_ip_override'] = $server_ip;
        $settings['notifications_enabled'] = !empty($input['notifications_enabled']) ? 1 : 0;
        $settings['auto_purge'] = !empty($input['auto_purge']) ? 1 : 0;

        $cache_ttl = isset($input['cache_ttl']) ? (int) $input['cache_ttl'] : (isset($settings['cache_ttl']) ? (int) $settings['cache_ttl'] : 3600);
        if ($cache_ttl < 30) {
            $cache_ttl = 30;
        }
        if ($cache_ttl > 604800) {
            $cache_ttl = 604800;
        }
        $settings['cache_ttl'] = $cache_ttl;

        $purge_policy = isset($input['purge_policy']) ? sanitize_text_field((string) $input['purge_policy']) : 'all';
        if (!in_array($purge_policy, ['all', 'domain', 'path', 'tag'], true)) {
            $purge_policy = 'all';
        }
        $settings['purge_policy'] = $purge_policy;
        $settings['purge_domain'] = isset($input['purge_domain']) ? sanitize_text_field(trim((string) $input['purge_domain'])) : '';
        $settings['purge_tag'] = isset($input['purge_tag']) ? sanitize_text_field(trim((string) $input['purge_tag'])) : '';

        $purge_path = isset($input['purge_path']) ? trim((string) $input['purge_path']) : '/';
        if ($purge_path === '') {
            $purge_path = '/';
        }
        if (strpos($purge_path, '/') !== 0) {
            $purge_path = '/' . $purge_path;
        }
        $settings['purge_path'] = sanitize_text_field($purge_path);

        $settings['opt_minify_css'] = !empty($input['opt_minify_css']) ? 1 : 0;
        $settings['opt_combine_css'] = !empty($input['opt_combine_css']) ? 1 : 0;
        $settings['opt_critical_css'] = !empty($input['opt_critical_css']) ? 1 : 0;
        $settings['opt_minify_js'] = !empty($input['opt_minify_js']) ? 1 : 0;
        $settings['opt_combine_js'] = !empty($input['opt_combine_js']) ? 1 : 0;
        $settings['opt_defer_js'] = !empty($input['opt_defer_js']) ? 1 : 0;
        $settings['opt_minify_html'] = !empty($input['opt_minify_html']) ? 1 : 0;
        $settings['opt_prefetch_hints'] = !empty($input['opt_prefetch_hints']) ? 1 : 0;
        $settings['opt_lazyload_images'] = !empty($input['opt_lazyload_images']) ? 1 : 0;
        $settings['opt_image_webp'] = !empty($input['opt_image_webp']) ? 1 : 0;
        $settings['opt_image_avif'] = !empty($input['opt_image_avif']) ? 1 : 0;
        $settings['opt_image_queue'] = !empty($input['opt_image_queue']) ? 1 : 0;

        $image_quality = isset($input['opt_image_quality']) ? (int) $input['opt_image_quality'] : (isset($settings['opt_image_quality']) ? (int) $settings['opt_image_quality'] : 82);
        if ($image_quality < 30) {
            $image_quality = 30;
        }
        if ($image_quality > 100) {
            $image_quality = 100;
        }
        $settings['opt_image_quality'] = $image_quality;

        $prefetch_urls = isset($input['opt_prefetch_urls']) ? trim((string) $input['opt_prefetch_urls']) : '';
        if ($prefetch_urls !== '') {
            $prefetch_urls = preg_replace('/\r\n|\r/', "\n", $prefetch_urls);
            $prefetch_urls = implode("\n", array_filter(array_map('trim', explode("\n", (string) $prefetch_urls))));
        }
        $settings['opt_prefetch_urls'] = sanitize_textarea_field($prefetch_urls);

        $cdn_provider = isset($input['cdn_provider']) ? sanitize_key((string) $input['cdn_provider']) : 'none';
        if (!in_array($cdn_provider, ['none', 'cloudflare'], true)) {
            $cdn_provider = 'none';
        }

        $settings['cdn_provider'] = $cdn_provider;
        $settings['cdn_enabled'] = !empty($input['cdn_enabled']) && $cdn_provider !== 'none' ? 1 : 0;
        $settings['cloudflare_zone_id'] = isset($input['cloudflare_zone_id']) ? sanitize_text_field(trim((string) $input['cloudflare_zone_id'])) : '';
        $settings['cloudflare_api_token'] = isset($input['cloudflare_api_token']) ? sanitize_text_field(trim((string) $input['cloudflare_api_token'])) : '';
        $settings['cloudflare_email'] = isset($input['cloudflare_email']) ? sanitize_email(trim((string) $input['cloudflare_email'])) : '';
        $settings['cloudflare_api_key'] = isset($input['cloudflare_api_key']) ? sanitize_text_field(trim((string) $input['cloudflare_api_key'])) : '';

        return $settings;
    }

    public function render_notices()
    {
        $screen = get_current_screen();
        if (!$screen || $screen->id !== 'toplevel_page_veloserve') {
            return;
        }

        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        if (empty($settings['notifications_enabled'])) {
            return;
        }

        if (!empty($_GET['veloserve_registered'])) {
            printf('<div class="notice notice-success is-dismissible"><p>%s</p></div>', 'Site registered with VeloServe successfully.');
        }

        if (!empty($_GET['veloserve_error'])) {
            printf('<div class="notice notice-error is-dismissible"><p>Registration error: %s</p></div>', esc_html(urldecode($_GET['veloserve_error'])));
        }

        if (!empty($_GET['veloserve_purged'])) {
            printf('<div class="notice notice-success is-dismissible"><p>%s</p></div>', 'Full cache purge request sent.');
        }

        if (!empty($_GET['veloserve_purge_error'])) {
            printf('<div class="notice notice-error is-dismissible"><p>Purge error: %s</p></div>', esc_html(urldecode($_GET['veloserve_purge_error'])));
        }

        if (!empty($_GET['veloserve_cdn_tested'])) {
            printf('<div class="notice notice-success is-dismissible"><p>%s</p></div>', 'CDN connectivity test passed.');
        }

        if (!empty($_GET['veloserve_cdn_error'])) {
            printf('<div class="notice notice-error is-dismissible"><p>CDN error: %s</p></div>', esc_html(urldecode($_GET['veloserve_cdn_error'])));
        }

        if (!empty($_GET['veloserve_tools_done'])) {
            printf('<div class="notice notice-success is-dismissible"><p>Tools: %s</p></div>', esc_html(urldecode($_GET['veloserve_tools_done'])));
        }

        if (!empty($_GET['veloserve_tools_error'])) {
            printf('<div class="notice notice-error is-dismissible"><p>Tools error: %s</p></div>', esc_html(urldecode($_GET['veloserve_tools_error'])));
        }
    }

    public function add_admin_bar_nodes($wp_admin_bar)
    {
        if (!is_admin_bar_showing() || !current_user_can('manage_options')) {
            return;
        }

        $wp_admin_bar->add_node([
            'id' => 'veloserve-root',
            'title' => 'VeloServe',
            'href' => $this->admin_page_url('dashboard'),
            'meta' => ['class' => 'veloserve-admin-bar-root'],
        ]);

        $wp_admin_bar->add_node([
            'id' => 'veloserve-register',
            'parent' => 'veloserve-root',
            'title' => 'Register Site',
            'href' => wp_nonce_url(
                admin_url('admin-post.php?action=veloserve_admin_bar_register'),
                'veloserve_register_action'
            ),
        ]);

        $wp_admin_bar->add_node([
            'id' => 'veloserve-purge',
            'parent' => 'veloserve-root',
            'title' => 'Purge All Cache',
            'href' => wp_nonce_url(
                admin_url('admin-post.php?action=veloserve_admin_bar_purge_all'),
                'veloserve_purge_all_action'
            ),
        ]);
    }

    public function handle_purge_all()
    {
        if (!current_user_can('manage_options')) {
            wp_die('Insufficient permissions');
        }

        check_admin_referer('veloserve_purge_all_action', 'veloserve_purge_all_nonce');
        $this->perform_purge_all();
    }

    public function handle_admin_bar_purge_all()
    {
        if (!current_user_can('manage_options')) {
            wp_die('Insufficient permissions');
        }

        check_admin_referer('veloserve_purge_all_action');
        $this->perform_purge_all();
    }

    public function handle_test_cdn()
    {
        if (!current_user_can('manage_options')) {
            wp_die('Insufficient permissions');
        }

        check_admin_referer('veloserve_test_cdn_action', 'veloserve_test_cdn_nonce');

        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        $settings = array_merge(VeloServe_Plugin::default_settings(), is_array($settings) ? $settings : []);
        $cdn_manager = new VeloServe_CDN_Manager();
        $result = $cdn_manager->test_connection($settings);

        if (is_wp_error($result)) {
            wp_safe_redirect(add_query_arg('veloserve_cdn_error', rawurlencode($result->get_error_message()), $this->admin_page_url('cdn')));
            exit;
        }

        wp_safe_redirect(add_query_arg('veloserve_cdn_tested', '1', $this->admin_page_url('cdn')));
        exit;
    }

    public function handle_admin_bar_register()
    {
        if (!current_user_can('manage_options')) {
            wp_die('Insufficient permissions');
        }

        check_admin_referer('veloserve_register_action');
        $this->perform_register();
    }

    public function handle_tools_db_optimize()
    {
        if (!current_user_can('manage_options')) {
            wp_die('Insufficient permissions');
        }

        check_admin_referer('veloserve_tools_db_optimize_action', 'veloserve_tools_db_optimize_nonce');

        $result = $this->perform_database_optimize();
        if (is_wp_error($result)) {
            wp_safe_redirect(add_query_arg('veloserve_tools_error', rawurlencode($result->get_error_message()), $this->admin_page_url('tools')));
            exit;
        }

        $message = sprintf(
            'Database optimize completed. Optimized %d/%d tables.',
            isset($result['optimized']) ? (int) $result['optimized'] : 0,
            isset($result['tables']) ? (int) $result['tables'] : 0
        );
        wp_safe_redirect(add_query_arg('veloserve_tools_done', rawurlencode($message), $this->admin_page_url('tools')));
        exit;
    }

    public function handle_tools_warm_sitemap()
    {
        if (!current_user_can('manage_options')) {
            wp_die('Insufficient permissions');
        }

        check_admin_referer('veloserve_tools_warm_sitemap_action', 'veloserve_tools_warm_sitemap_nonce');

        $result = $this->perform_sitemap_warm();
        if (is_wp_error($result)) {
            wp_safe_redirect(add_query_arg('veloserve_tools_error', rawurlencode($result->get_error_message()), $this->admin_page_url('tools')));
            exit;
        }

        $message = sprintf(
            'Sitemap warm submitted. URLs discovered: %d, queued: %d.',
            isset($result['discovered']) ? (int) $result['discovered'] : 0,
            isset($result['queued']) ? (int) $result['queued'] : 0
        );
        wp_safe_redirect(add_query_arg('veloserve_tools_done', rawurlencode($message), $this->admin_page_url('tools')));
        exit;
    }

    public function handle_tools_export_settings()
    {
        if (!current_user_can('manage_options')) {
            wp_die('Insufficient permissions');
        }

        check_admin_referer('veloserve_tools_export_settings_action');

        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        $payload = [
            'version' => defined('VELOSERVE_PLUGIN_VERSION') ? VELOSERVE_PLUGIN_VERSION : 'unknown',
            'generated_at' => gmdate('c'),
            'settings' => array_merge(VeloServe_Plugin::default_settings(), is_array($settings) ? $settings : []),
        ];

        $filename = 'veloserve-settings-' . gmdate('Ymd-His') . '.json';
        nocache_headers();
        header('Content-Type: application/json; charset=utf-8');
        header('Content-Disposition: attachment; filename="' . $filename . '"');
        echo wp_json_encode($payload, JSON_PRETTY_PRINT);
        exit;
    }

    public function handle_tools_import_settings()
    {
        if (!current_user_can('manage_options')) {
            wp_die('Insufficient permissions');
        }

        check_admin_referer('veloserve_tools_import_settings_action', 'veloserve_tools_import_settings_nonce');

        if (empty($_FILES['veloserve_settings_file']) || !is_array($_FILES['veloserve_settings_file'])) {
            wp_safe_redirect(add_query_arg('veloserve_tools_error', rawurlencode('No settings file uploaded.'), $this->admin_page_url('tools')));
            exit;
        }

        $file = $_FILES['veloserve_settings_file'];
        $tmp_name = isset($file['tmp_name']) ? (string) $file['tmp_name'] : '';
        if ($tmp_name === '' || !is_uploaded_file($tmp_name)) {
            wp_safe_redirect(add_query_arg('veloserve_tools_error', rawurlencode('Uploaded settings file is invalid.'), $this->admin_page_url('tools')));
            exit;
        }

        $raw = file_get_contents($tmp_name);
        if (!is_string($raw) || trim($raw) === '') {
            wp_safe_redirect(add_query_arg('veloserve_tools_error', rawurlencode('Settings file is empty.'), $this->admin_page_url('tools')));
            exit;
        }

        $decoded = json_decode($raw, true);
        if (!is_array($decoded)) {
            wp_safe_redirect(add_query_arg('veloserve_tools_error', rawurlencode('Settings file is not valid JSON.'), $this->admin_page_url('tools')));
            exit;
        }

        $import_data = isset($decoded['settings']) && is_array($decoded['settings']) ? $decoded['settings'] : $decoded;
        $sanitized = $this->sanitize($import_data);
        update_option(VELOSERVE_OPTION_KEY, $sanitized);

        wp_safe_redirect(add_query_arg('veloserve_tools_done', rawurlencode('Settings imported successfully.'), $this->admin_page_url('tools')));
        exit;
    }

    public function handle_tools_download_debug()
    {
        if (!current_user_can('manage_options')) {
            wp_die('Insufficient permissions');
        }

        check_admin_referer('veloserve_tools_download_debug_action');

        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        $settings = array_merge(VeloServe_Plugin::default_settings(), is_array($settings) ? $settings : []);
        $status = get_option(VELOSERVE_STATUS_KEY, VeloServe_Plugin::default_status());

        $debug = [
            'generated_at' => gmdate('c'),
            'plugin_version' => defined('VELOSERVE_PLUGIN_VERSION') ? VELOSERVE_PLUGIN_VERSION : 'unknown',
            'site' => [
                'url' => home_url('/'),
                'wp_version' => get_bloginfo('version'),
                'php_version' => PHP_VERSION,
            ],
            'status' => $status,
            'settings' => $settings,
        ];

        if (!empty($settings['endpoint_url']) && !empty($settings['api_token'])) {
            $server = new VeloServe_Server();
            $debug['server_detection'] = $server->detect_server($settings);
            $debug['cache_stats'] = $server->get_cache_stats($settings);
            $debug['cache_config'] = $server->get_cache_config($settings);
        }

        $filename = 'veloserve-debug-' . gmdate('Ymd-His') . '.json';
        nocache_headers();
        header('Content-Type: application/json; charset=utf-8');
        header('Content-Disposition: attachment; filename="' . $filename . '"');
        echo wp_json_encode($debug, JSON_PRETTY_PRINT);
        exit;
    }

    public function render_page()
    {
        if (!current_user_can('manage_options')) {
            return;
        }

        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        $settings = array_merge(VeloServe_Plugin::default_settings(), is_array($settings) ? $settings : []);
        $status = get_option(VELOSERVE_STATUS_KEY, []);
        $tab = $this->active_tab();
        ?>
        <div class="wrap veloserve-shell">
            <?php $this->render_brand_header($status); ?>
            <?php $this->render_tabs($tab); ?>

            <div class="veloserve-panel">
                <?php if ($tab === 'connection'): ?>
                    <?php $this->render_connection_tab(); ?>
                <?php elseif ($tab === 'general'): ?>
                    <?php $this->render_general_tab(); ?>
                <?php elseif ($tab === 'cache'): ?>
                    <?php $this->render_cache_tab($settings); ?>
                <?php elseif ($tab === 'cdn'): ?>
                    <?php $this->render_cdn_tab($settings); ?>
                <?php elseif ($tab === 'tools'): ?>
                    <?php $this->render_tools_tab(); ?>
                <?php else: ?>
                    <?php $this->render_dashboard_tab($status); ?>
                <?php endif; ?>
            </div>
        </div>

        <style>
            .veloserve-shell .veloserve-header {
                background: linear-gradient(135deg, #0f2e5e 0%, #1f4c8f 100%);
                color: #fff;
                border-radius: 8px;
                padding: 16px 20px;
                margin: 16px 0 12px;
            }
            .veloserve-shell .veloserve-brand {
                display: flex;
                align-items: center;
                gap: 10px;
                margin: 0;
            }
            .veloserve-shell .veloserve-mark {
                width: 28px;
                height: 28px;
                border-radius: 6px;
                background: #50c878;
                display: inline-flex;
                align-items: center;
                justify-content: center;
                color: #0c223e;
                font-weight: 700;
                font-size: 14px;
            }
            .veloserve-shell .veloserve-subtitle {
                margin: 8px 0 0;
                color: #d8e5ff;
            }
            .veloserve-shell .veloserve-meta {
                margin-top: 12px;
                color: #eaf2ff;
                display: flex;
                gap: 16px;
                flex-wrap: wrap;
            }
            .veloserve-shell .veloserve-panel {
                background: #fff;
                border: 1px solid #dcdcde;
                border-radius: 8px;
                padding: 16px;
            }
            .veloserve-shell .veloserve-actions {
                display: flex;
                gap: 8px;
                flex-wrap: wrap;
                margin-top: 12px;
            }
            .veloserve-shell .veloserve-grid {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
                gap: 12px;
                margin: 16px 0;
            }
            .veloserve-shell .veloserve-card {
                border: 1px solid #dcdcde;
                border-radius: 8px;
                background: #f8fbff;
                padding: 14px;
            }
            .veloserve-shell .veloserve-card h3 {
                margin: 0 0 6px;
                font-size: 13px;
                color: #1d2327;
            }
            .veloserve-shell .veloserve-card .veloserve-value {
                margin: 0;
                font-size: 22px;
                font-weight: 600;
                color: #0f2e5e;
                line-height: 1.2;
            }
            .veloserve-shell .veloserve-card .veloserve-note {
                margin-top: 5px;
                color: #50575e;
                font-size: 12px;
            }
            .veloserve-shell .veloserve-split {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
                gap: 16px;
                margin-top: 18px;
            }
            .veloserve-shell .veloserve-list {
                margin: 0;
            }
            .veloserve-shell .veloserve-list dt {
                font-weight: 600;
                margin-top: 8px;
            }
            .veloserve-shell .veloserve-list dd {
                margin: 0;
                color: #2c3338;
            }
        </style>
        <?php
    }

    public function render_endpoint_field()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        printf(
            '<input type="url" name="%1$s[endpoint_url]" value="%2$s" class="regular-text" placeholder="https://control.veloserve.local" required />',
            esc_attr(VELOSERVE_OPTION_KEY),
            esc_attr($settings['endpoint_url'])
        );
    }

    public function render_token_field()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        printf(
            '<input type="password" name="%1$s[api_token]" value="%2$s" class="regular-text" autocomplete="off" />',
            esc_attr(VELOSERVE_OPTION_KEY),
            esc_attr($settings['api_token'])
        );
    }

    public function render_auto_purge_field()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        printf(
            '<label><input type="checkbox" name="%1$s[auto_purge]" value="1" %2$s /> Purge cache on content updates</label>',
            esc_attr(VELOSERVE_OPTION_KEY),
            checked((int) $settings['auto_purge'], 1, false)
        );
    }

    public function render_auto_detect_field()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        printf(
            '<label><input type="checkbox" name="%1$s[auto_detect_server]" value="1" %2$s /> Automatically detect VeloServe runtime and API endpoints</label>',
            esc_attr(VELOSERVE_OPTION_KEY),
            checked((int) $settings['auto_detect_server'], 1, false)
        );
    }

    public function render_guest_mode_field()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        printf(
            '<label><input type="checkbox" name="%1$s[guest_mode]" value="1" %2$s /> Restrict to read-only dashboard controls for non-admin operator workflows</label>',
            esc_attr(VELOSERVE_OPTION_KEY),
            checked((int) $settings['guest_mode'], 1, false)
        );
    }

    public function render_server_ip_override_field()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        printf(
            '<input type="text" name="%1$s[server_ip_override]" value="%2$s" class="regular-text" placeholder="203.0.113.10 or 2001:db8::10" />',
            esc_attr(VELOSERVE_OPTION_KEY),
            esc_attr($settings['server_ip_override'])
        );
        echo '<p class="description">Optional. Force API calls to this IP instead of auto-discovery.</p>';
    }

    public function render_notifications_field()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        printf(
            '<label><input type="checkbox" name="%1$s[notifications_enabled]" value="1" %2$s /> Show operational notices for registration, purge, and connectivity events</label>',
            esc_attr(VELOSERVE_OPTION_KEY),
            checked((int) $settings['notifications_enabled'], 1, false)
        );
    }

    public function handle_register()
    {
        if (!current_user_can('manage_options')) {
            wp_die('Insufficient permissions');
        }

        check_admin_referer('veloserve_register_action', 'veloserve_register_nonce');
        $this->perform_register();
    }

    private function perform_purge_all()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());

        if (empty($settings['endpoint_url']) || empty($settings['api_token'])) {
            wp_safe_redirect(add_query_arg('veloserve_purge_error', rawurlencode('Endpoint URL and API token are required.'), $this->admin_page_url('cache')));
            exit;
        }

        $server = new VeloServe_Server();
        $params = $this->build_purge_params_from_settings($settings);
        $response = $server->purge_cache($settings, $params);

        if (is_wp_error($response)) {
            wp_safe_redirect(add_query_arg('veloserve_purge_error', rawurlencode($response->get_error_message()), $this->admin_page_url('cache')));
            exit;
        }

        $cdn_manager = new VeloServe_CDN_Manager();
        if ($cdn_manager->should_purge($settings)) {
            $cdn_response = $cdn_manager->purge($settings, $params);
            if (is_wp_error($cdn_response)) {
                wp_safe_redirect(add_query_arg('veloserve_purge_error', rawurlencode($cdn_response->get_error_message()), $this->admin_page_url('cache')));
                exit;
            }
        }

        wp_safe_redirect(add_query_arg('veloserve_purged', '1', $this->admin_page_url('cache')));
        exit;
    }

    private function perform_register()
    {
        $result = VeloServe_Plugin::instance()->register_with_endpoint();

        if (is_wp_error($result)) {
            VeloServe_Plugin::instance()->set_status([
                'connected' => false,
                'last_error' => $result->get_error_message(),
            ]);
            wp_safe_redirect(add_query_arg('veloserve_error', rawurlencode($result->get_error_message()), $this->admin_page_url('connection')));
            exit;
        }

        wp_safe_redirect(add_query_arg('veloserve_registered', '1', $this->admin_page_url('dashboard')));
        exit;
    }

    private function render_brand_header($status)
    {
        ?>
        <div class="veloserve-header">
            <h1 class="veloserve-brand">
                <span class="veloserve-mark">VS</span>
                <span>VeloServe Control</span>
            </h1>
            <p class="veloserve-subtitle">Centralized WordPress performance controls, registration, and cache operations.</p>
            <div class="veloserve-meta">
                <span><strong>Connection:</strong> <?php echo !empty($status['connected']) ? 'Connected' : 'Not connected'; ?></span>
                <span><strong>Node ID:</strong> <?php echo !empty($status['node_id']) ? esc_html($status['node_id']) : 'N/A'; ?></span>
            </div>
        </div>
        <?php
    }

    private function render_tabs($tab)
    {
        $tabs = $this->tabs();
        echo '<nav class="nav-tab-wrapper" aria-label="VeloServe Sections">';

        foreach ($tabs as $key => $label) {
            $class = $tab === $key ? 'nav-tab nav-tab-active' : 'nav-tab';
            printf(
                '<a class="%1$s" href="%2$s">%3$s</a>',
                esc_attr($class),
                esc_url($this->admin_page_url($key)),
                esc_html($label)
            );
        }

        echo '</nav>';
    }

    private function render_dashboard_tab($status)
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        $has_credentials = !empty($settings['endpoint_url']) && !empty($settings['api_token']);
        $detected = null;
        $cache_stats = null;
        $server_error = '';
        $stats_error = '';

        if ($has_credentials) {
            $server = new VeloServe_Server();
            $detected = $server->detect_server($settings);
            if (is_wp_error($detected)) {
                $server_error = $detected->get_error_message();
                $detected = null;
            } else {
                $cache_stats = $server->get_cache_stats($settings);
                if (is_wp_error($cache_stats)) {
                    $stats_error = $cache_stats->get_error_message();
                    $cache_stats = null;
                }
            }
        }

        $status_label = !empty($status['connected']) ? 'Connected' : 'Not connected';
        $server_runtime_status = $detected ? $this->safe_text($detected, 'status', 'unknown') : ($has_credentials ? 'unreachable' : 'not configured');
        $server_version = $detected ? $this->safe_text($detected, 'version', 'n/a') : 'n/a';
        $hit_rate_raw = $this->get_nested($cache_stats, ['cache', 'hit_rate'], null);
        $hit_rate = is_numeric($hit_rate_raw) ? sprintf('%.1f%%', ((float) $hit_rate_raw) * 100) : 'n/a';
        $cache_entries = $this->format_number($this->get_nested($cache_stats, ['cache', 'entries'], null));
        $queued_total = $this->format_number($this->get_nested($cache_stats, ['warming', 'queued_total'], null));
        $quick_endpoint = !empty($settings['endpoint_url']) ? esc_html($settings['endpoint_url']) : 'Not set';
        $registered_at = !empty($status['registered_at']) ? esc_html($status['registered_at']) : 'Never';
        $server_memory = defined('WP_MEMORY_LIMIT') ? WP_MEMORY_LIMIT : 'Not defined';
        $php_memory = ini_get('memory_limit') ? ini_get('memory_limit') : 'Not set';
        $cache_size = $this->format_bytes($this->get_nested($cache_stats, ['cache', 'size_bytes'], null));
        $hits = $this->format_number($this->get_nested($cache_stats, ['cache', 'hits'], null));
        $misses = $this->format_number($this->get_nested($cache_stats, ['cache', 'misses'], null));

        ?>
        <h2>Dashboard</h2>
        <p>Live server and cache visibility for this WordPress installation.</p>

        <div class="veloserve-grid">
            <div class="veloserve-card">
                <h3>Plugin Connection</h3>
                <p class="veloserve-value"><?php echo esc_html($status_label); ?></p>
                <p class="veloserve-note">Node: <?php echo !empty($status['node_id']) ? esc_html($status['node_id']) : 'N/A'; ?></p>
            </div>
            <div class="veloserve-card">
                <h3>Server Status</h3>
                <p class="veloserve-value"><?php echo esc_html($server_runtime_status); ?></p>
                <p class="veloserve-note">Version: <?php echo esc_html($server_version); ?></p>
            </div>
            <div class="veloserve-card">
                <h3>Cache Hit Rate</h3>
                <p class="veloserve-value"><?php echo esc_html($hit_rate); ?></p>
                <p class="veloserve-note">Hits: <?php echo esc_html($hits); ?> | Misses: <?php echo esc_html($misses); ?></p>
            </div>
            <div class="veloserve-card">
                <h3>Queued Warmups</h3>
                <p class="veloserve-value"><?php echo esc_html($queued_total); ?></p>
                <p class="veloserve-note">Entries: <?php echo esc_html($cache_entries); ?> | Cache size: <?php echo esc_html($cache_size); ?></p>
            </div>
        </div>

        <?php if (!$has_credentials): ?>
            <div class="notice notice-warning inline"><p>Endpoint URL and API token are required for live server and cache stats.</p></div>
        <?php endif; ?>
        <?php if ($server_error !== ''): ?>
            <div class="notice notice-error inline"><p>Server detection failed: <?php echo esc_html($server_error); ?></p></div>
        <?php endif; ?>
        <?php if ($stats_error !== ''): ?>
            <div class="notice notice-error inline"><p>Cache stats request failed: <?php echo esc_html($stats_error); ?></p></div>
        <?php endif; ?>

        <div class="veloserve-actions">
            <a href="<?php echo esc_url($this->admin_page_url('dashboard')); ?>" class="button">Refresh Dashboard</a>
            <a href="<?php echo esc_url($this->admin_page_url('connection')); ?>" class="button">Open Connection</a>
            <a href="<?php echo esc_url($this->admin_page_url('cache')); ?>" class="button">Open Cache Controls</a>
            <form method="post" action="<?php echo esc_url(admin_url('admin-post.php')); ?>" style="display:inline-block;">
                <?php wp_nonce_field('veloserve_register_action', 'veloserve_register_nonce'); ?>
                <input type="hidden" name="action" value="veloserve_register" />
                <?php submit_button('Register Site', 'secondary', 'submit', false); ?>
            </form>
            <form method="post" action="<?php echo esc_url(admin_url('admin-post.php')); ?>" style="display:inline-block;">
                <?php wp_nonce_field('veloserve_purge_all_action', 'veloserve_purge_all_nonce'); ?>
                <input type="hidden" name="action" value="veloserve_purge_all" />
                <?php submit_button('Purge All Cache', 'secondary', 'submit', false); ?>
            </form>
        </div>

        <div class="veloserve-split">
            <div class="veloserve-card">
                <h3>Environment</h3>
                <dl class="veloserve-list">
                    <dt>Site URL</dt>
                    <dd><?php echo esc_html(home_url('/')); ?></dd>
                    <dt>WordPress Version</dt>
                    <dd><?php echo esc_html(get_bloginfo('version')); ?></dd>
                    <dt>PHP Version</dt>
                    <dd><?php echo esc_html(PHP_VERSION); ?></dd>
                    <dt>WordPress Memory Limit</dt>
                    <dd><?php echo esc_html($server_memory); ?></dd>
                    <dt>PHP Memory Limit</dt>
                    <dd><?php echo esc_html($php_memory); ?></dd>
                    <dt>Plugin Version</dt>
                    <dd><?php echo defined('VELOSERVE_PLUGIN_VERSION') ? esc_html(VELOSERVE_PLUGIN_VERSION) : 'n/a'; ?></dd>
                </dl>
            </div>
            <div class="veloserve-card">
                <h3>Runtime Details</h3>
                <dl class="veloserve-list">
                    <dt>Endpoint URL</dt>
                    <dd><?php echo $quick_endpoint; ?></dd>
                    <dt>Last Registration</dt>
                    <dd><?php echo esc_html($registered_at); ?></dd>
                    <dt>PHP Worker Available</dt>
                    <dd><?php echo $detected ? (!empty($detected['php_available']) ? 'Yes' : 'No') : 'n/a'; ?></dd>
                    <dt>Server Cache Enabled</dt>
                    <dd><?php echo $detected ? (!empty($detected['cache_enabled']) ? 'Yes' : 'No') : 'n/a'; ?></dd>
                    <?php if (!empty($status['last_error'])): ?>
                        <dt>Last Error</dt>
                        <dd style="color: #d63638;"><?php echo esc_html($status['last_error']); ?></dd>
                    <?php endif; ?>
                </dl>
            </div>
        </div>
        <?php
    }

    private function render_connection_tab()
    {
        ?>
        <h2>Connection Settings</h2>
        <p>Configure connectivity between this WordPress site and your VeloServe control plane.</p>

        <form method="post" action="options.php">
            <?php
            settings_fields('veloserve_settings_group');
            do_settings_sections('veloserve_connection');
            submit_button('Save Settings');
            ?>
        </form>

        <form method="post" action="<?php echo esc_url(admin_url('admin-post.php')); ?>" style="display:inline-block;">
            <?php wp_nonce_field('veloserve_register_action', 'veloserve_register_nonce'); ?>
            <input type="hidden" name="action" value="veloserve_register" />
            <?php submit_button('Register Site with VeloServe', 'primary', 'submit', false); ?>
        </form>
        <?php
    }

    private function render_general_tab()
    {
        ?>
        <h2>General Settings</h2>
        <p>Configure runtime discovery behavior, operator mode, and notification preferences.</p>

        <form method="post" action="options.php">
            <?php
            settings_fields('veloserve_settings_group');
            do_settings_sections('veloserve_general');
            submit_button('Save Settings');
            ?>
        </form>
        <?php
    }

    private function render_cache_tab($settings)
    {
        $cache_view = $this->active_cache_view();
        $server_ttl = 'n/a';
        $server_cache_enabled = 'n/a';
        $server_error = '';

        if (!empty($settings['endpoint_url']) && !empty($settings['api_token'])) {
            $server = new VeloServe_Server();
            $cache_config = $server->get_cache_config($settings);
            if (is_wp_error($cache_config)) {
                $server_error = $cache_config->get_error_message();
            } else {
                $ttl = $this->get_nested($cache_config, ['cache', 'default_ttl'], null);
                $server_ttl = is_numeric($ttl) ? number_format_i18n((int) $ttl) . ' seconds' : 'n/a';
                $server_cache_enabled = !empty($this->get_nested($cache_config, ['cache', 'enabled'], false)) ? 'Enabled' : 'Disabled';
            }
        }

        ?>
        <h2>Cache Controls</h2>
        <p>Manage cache behavior, TTL defaults, and purge strategy for this WordPress site.</p>

        <nav class="nav-tab-wrapper" aria-label="Cache Sections" style="margin-bottom: 16px;">
            <a class="<?php echo $cache_view === 'cache' ? 'nav-tab nav-tab-active' : 'nav-tab'; ?>" href="<?php echo esc_url($this->cache_tab_url('cache')); ?>">Cache</a>
            <a class="<?php echo $cache_view === 'ttl' ? 'nav-tab nav-tab-active' : 'nav-tab'; ?>" href="<?php echo esc_url($this->cache_tab_url('ttl')); ?>">TTL</a>
            <a class="<?php echo $cache_view === 'optimization' ? 'nav-tab nav-tab-active' : 'nav-tab'; ?>" href="<?php echo esc_url($this->cache_tab_url('optimization')); ?>">Optimization</a>
            <a class="<?php echo $cache_view === 'purge' ? 'nav-tab nav-tab-active' : 'nav-tab'; ?>" href="<?php echo esc_url($this->cache_tab_url('purge')); ?>">Purge</a>
        </nav>

        <?php if ($cache_view === 'cache'): ?>
            <form method="post" action="options.php">
                <?php settings_fields('veloserve_settings_group'); ?>
                <table class="form-table" role="presentation">
                    <tr>
                        <th scope="row">Auto Purge</th>
                        <td>
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[auto_purge]" value="1" <?php checked((int) $settings['auto_purge'], 1); ?> /> Purge cache on content updates</label>
                        </td>
                    </tr>
                    <tr>
                        <th scope="row">Server Cache Status</th>
                        <td><?php echo esc_html($server_cache_enabled); ?></td>
                    </tr>
                </table>
                <?php submit_button('Save Cache Settings'); ?>
            </form>
        <?php elseif ($cache_view === 'ttl'): ?>
            <form method="post" action="options.php">
                <?php settings_fields('veloserve_settings_group'); ?>
                <table class="form-table" role="presentation">
                    <tr>
                        <th scope="row">Plugin TTL (seconds)</th>
                        <td>
                            <input type="number" min="30" max="604800" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[cache_ttl]" value="<?php echo esc_attr((int) $settings['cache_ttl']); ?>" class="small-text" />
                            <p class="description">Operational TTL preference used by plugin workflows and policy defaults.</p>
                        </td>
                    </tr>
                    <tr>
                        <th scope="row">Server Default TTL</th>
                        <td><?php echo esc_html($server_ttl); ?></td>
                    </tr>
                </table>
                <?php submit_button('Save TTL Settings'); ?>
            </form>
        <?php elseif ($cache_view === 'optimization'): ?>
            <form method="post" action="options.php">
                <?php settings_fields('veloserve_settings_group'); ?>
                <table class="form-table" role="presentation">
                    <tr>
                        <th scope="row">CSS Optimization</th>
                        <td>
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_minify_css]" value="1" <?php checked((int) $settings['opt_minify_css'], 1); ?> /> Minify CSS assets</label><br />
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_combine_css]" value="1" <?php checked((int) $settings['opt_combine_css'], 1); ?> /> Combine CSS files</label><br />
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_critical_css]" value="1" <?php checked((int) $settings['opt_critical_css'], 1); ?> /> Generate and inline critical CSS</label>
                        </td>
                    </tr>
                    <tr>
                        <th scope="row">JavaScript Optimization</th>
                        <td>
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_minify_js]" value="1" <?php checked((int) $settings['opt_minify_js'], 1); ?> /> Minify JavaScript assets</label><br />
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_combine_js]" value="1" <?php checked((int) $settings['opt_combine_js'], 1); ?> /> Combine JavaScript files</label><br />
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_defer_js]" value="1" <?php checked((int) $settings['opt_defer_js'], 1); ?> /> Defer non-critical JavaScript</label>
                        </td>
                    </tr>
                    <tr>
                        <th scope="row">HTML Optimization</th>
                        <td>
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_minify_html]" value="1" <?php checked((int) $settings['opt_minify_html'], 1); ?> /> Minify HTML output</label>
                        </td>
                    </tr>
                    <tr>
                        <th scope="row">Prefetch Hints</th>
                        <td>
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_prefetch_hints]" value="1" <?php checked((int) $settings['opt_prefetch_hints'], 1); ?> /> Enable prefetch hints</label>
                            <p class="description">Optional list of URLs/origins to prefetch (one per line).</p>
                            <textarea name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_prefetch_urls]" rows="4" class="large-text code" placeholder="https://fonts.gstatic.com&#10;https://cdn.example.com"><?php echo esc_textarea((string) $settings['opt_prefetch_urls']); ?></textarea>
                        </td>
                    </tr>
                    <tr>
                        <th scope="row">Image Optimization</th>
                        <td>
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_lazyload_images]" value="1" <?php checked((int) $settings['opt_lazyload_images'], 1); ?> /> Enable lazy loading defaults for attachment images</label><br />
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_image_webp]" value="1" <?php checked((int) $settings['opt_image_webp'], 1); ?> /> Generate WebP variants when image queue runs</label><br />
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_image_avif]" value="1" <?php checked((int) $settings['opt_image_avif'], 1); ?> /> Generate AVIF variants when supported by server image libraries</label><br />
                            <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_image_queue]" value="1" <?php checked((int) $settings['opt_image_queue'], 1); ?> /> Enable background image compression queue</label>
                            <p class="description">Queue processes new attachments, compresses source quality, generates modern formats, and submits warm targets to VeloServe cache warm API.</p>
                        </td>
                    </tr>
                    <tr>
                        <th scope="row">Image Quality</th>
                        <td>
                            <input type="number" min="30" max="100" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[opt_image_quality]" value="<?php echo esc_attr((int) $settings['opt_image_quality']); ?>" class="small-text" />
                            <p class="description">Compression quality for queued image processing. Range: 30-100.</p>
                        </td>
                    </tr>
                </table>
                <?php submit_button('Save Optimization Settings'); ?>
            </form>
        <?php else: ?>
            <form method="post" action="options.php">
                <?php settings_fields('veloserve_settings_group'); ?>
                <table class="form-table" role="presentation">
                    <tr>
                        <th scope="row">Purge Policy</th>
                        <td>
                            <select name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[purge_policy]">
                                <option value="all" <?php selected($settings['purge_policy'], 'all'); ?>>All cache</option>
                                <option value="domain" <?php selected($settings['purge_policy'], 'domain'); ?>>Domain</option>
                                <option value="path" <?php selected($settings['purge_policy'], 'path'); ?>>Domain + path</option>
                                <option value="tag" <?php selected($settings['purge_policy'], 'tag'); ?>>Tag</option>
                            </select>
                        </td>
                    </tr>
                    <tr>
                        <th scope="row">Domain</th>
                        <td><input type="text" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[purge_domain]" value="<?php echo esc_attr($settings['purge_domain']); ?>" class="regular-text" placeholder="example.com" /></td>
                    </tr>
                    <tr>
                        <th scope="row">Path</th>
                        <td><input type="text" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[purge_path]" value="<?php echo esc_attr($settings['purge_path']); ?>" class="regular-text" placeholder="/shop" /></td>
                    </tr>
                    <tr>
                        <th scope="row">Tag</th>
                        <td><input type="text" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[purge_tag]" value="<?php echo esc_attr($settings['purge_tag']); ?>" class="regular-text" placeholder="category:news" /></td>
                    </tr>
                </table>
                <?php submit_button('Save Purge Policy', 'secondary', 'submit', false); ?>
            </form>

            <form method="post" action="<?php echo esc_url(admin_url('admin-post.php')); ?>" style="display:inline-block; margin-top: 8px; margin-left: 8px;">
                <?php wp_nonce_field('veloserve_purge_all_action', 'veloserve_purge_all_nonce'); ?>
                <input type="hidden" name="action" value="veloserve_purge_all" />
                <?php submit_button('Run Purge Policy', 'secondary', 'submit', false); ?>
            </form>
        <?php endif; ?>

        <?php if ($server_error !== ''): ?>
            <div class="notice notice-warning inline"><p>Could not load server cache configuration: <?php echo esc_html($server_error); ?></p></div>
        <?php endif; ?>
        <?php
    }

    private function render_tools_tab()
    {
        ?>
        <h2>Tools</h2>
        <p>Operational toolbox for purge, database maintenance, sitemap warming, configuration transfer, and debug snapshots.</p>

        <div class="veloserve-split">
            <div class="veloserve-card">
                <h3>Maintenance</h3>
                <p>Run safe operational actions on demand.</p>
                <form method="post" action="<?php echo esc_url(admin_url('admin-post.php')); ?>" style="margin-bottom: 10px;">
                    <?php wp_nonce_field('veloserve_purge_all_action', 'veloserve_purge_all_nonce'); ?>
                    <input type="hidden" name="action" value="veloserve_purge_all" />
                    <?php submit_button('Purge Cache Now', 'secondary', 'submit', false); ?>
                </form>

                <form method="post" action="<?php echo esc_url(admin_url('admin-post.php')); ?>">
                    <?php wp_nonce_field('veloserve_tools_db_optimize_action', 'veloserve_tools_db_optimize_nonce'); ?>
                    <input type="hidden" name="action" value="veloserve_tools_db_optimize" />
                    <?php submit_button('Optimize Database Tables', 'secondary', 'submit', false); ?>
                </form>
            </div>

            <div class="veloserve-card">
                <h3>Sitemap Crawler Warming</h3>
                <p>Crawl public sitemaps and submit discovered URLs to the VeloServe warm queue.</p>
                <form method="post" action="<?php echo esc_url(admin_url('admin-post.php')); ?>">
                    <?php wp_nonce_field('veloserve_tools_warm_sitemap_action', 'veloserve_tools_warm_sitemap_nonce'); ?>
                    <input type="hidden" name="action" value="veloserve_tools_warm_sitemap" />
                    <?php submit_button('Warm from Sitemap', 'secondary', 'submit', false); ?>
                </form>
            </div>
        </div>

        <div class="veloserve-split">
            <div class="veloserve-card">
                <h3>Import / Export</h3>
                <p>Export settings for backup or import settings from another environment.</p>
                <p>
                    <a class="button button-secondary" href="<?php echo esc_url(wp_nonce_url(admin_url('admin-post.php?action=veloserve_tools_export_settings'), 'veloserve_tools_export_settings_action')); ?>">Export Settings</a>
                </p>
                <form method="post" action="<?php echo esc_url(admin_url('admin-post.php')); ?>" enctype="multipart/form-data">
                    <?php wp_nonce_field('veloserve_tools_import_settings_action', 'veloserve_tools_import_settings_nonce'); ?>
                    <input type="hidden" name="action" value="veloserve_tools_import_settings" />
                    <input type="file" name="veloserve_settings_file" accept=".json,application/json" required />
                    <?php submit_button('Import Settings', 'secondary', 'submit', false); ?>
                </form>
            </div>

            <div class="veloserve-card">
                <h3>Debug Bundle</h3>
                <p>Download a JSON snapshot with plugin config, status, environment, and server diagnostics.</p>
                <a class="button button-secondary" href="<?php echo esc_url(wp_nonce_url(admin_url('admin-post.php?action=veloserve_tools_download_debug'), 'veloserve_tools_download_debug_action')); ?>">Download Debug Snapshot</a>
            </div>
        </div>
        <?php
    }

    private function render_cdn_tab($settings)
    {
        $settings = array_merge(VeloServe_Plugin::default_settings(), is_array($settings) ? $settings : []);
        ?>
        <h2>CDN Controls</h2>
        <p>Configure edge cache purge cascading and verify provider connectivity.</p>

        <form method="post" action="options.php">
            <?php settings_fields('veloserve_settings_group'); ?>
            <table class="form-table" role="presentation">
                <tr>
                    <th scope="row">Enable CDN Purge Cascade</th>
                    <td>
                        <label><input type="checkbox" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[cdn_enabled]" value="1" <?php checked((int) $settings['cdn_enabled'], 1); ?> /> Send matching purge events to configured CDN provider</label>
                    </td>
                </tr>
                <tr>
                    <th scope="row">CDN Provider</th>
                    <td>
                        <select name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[cdn_provider]">
                            <option value="none" <?php selected($settings['cdn_provider'], 'none'); ?>>None</option>
                            <option value="cloudflare" <?php selected($settings['cdn_provider'], 'cloudflare'); ?>>Cloudflare</option>
                        </select>
                    </td>
                </tr>
                <tr>
                    <th scope="row">Cloudflare Zone ID</th>
                    <td><input type="text" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[cloudflare_zone_id]" value="<?php echo esc_attr($settings['cloudflare_zone_id']); ?>" class="regular-text" placeholder="023e105f4ecef8ad9ca31a8372d0c353" /></td>
                </tr>
                <tr>
                    <th scope="row">Cloudflare API Token</th>
                    <td>
                        <input type="password" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[cloudflare_api_token]" value="<?php echo esc_attr($settings['cloudflare_api_token']); ?>" class="regular-text" autocomplete="off" />
                        <p class="description">Preferred auth method. Token should include Zone:Read and Cache Purge permissions.</p>
                    </td>
                </tr>
                <tr>
                    <th scope="row">Cloudflare Email</th>
                    <td><input type="email" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[cloudflare_email]" value="<?php echo esc_attr($settings['cloudflare_email']); ?>" class="regular-text" autocomplete="off" /></td>
                </tr>
                <tr>
                    <th scope="row">Cloudflare API Key</th>
                    <td>
                        <input type="password" name="<?php echo esc_attr(VELOSERVE_OPTION_KEY); ?>[cloudflare_api_key]" value="<?php echo esc_attr($settings['cloudflare_api_key']); ?>" class="regular-text" autocomplete="off" />
                        <p class="description">Legacy fallback when token-based auth is unavailable.</p>
                    </td>
                </tr>
            </table>
            <?php submit_button('Save CDN Settings'); ?>
        </form>

        <form method="post" action="<?php echo esc_url(admin_url('admin-post.php')); ?>" style="display:inline-block;">
            <?php wp_nonce_field('veloserve_test_cdn_action', 'veloserve_test_cdn_nonce'); ?>
            <input type="hidden" name="action" value="veloserve_test_cdn" />
            <?php submit_button('Test CDN Connection', 'secondary', 'submit', false); ?>
        </form>
        <?php
    }

    private function tabs()
    {
        return [
            'dashboard' => 'Dashboard',
            'connection' => 'Connection',
            'general' => 'General',
            'cache' => 'Cache',
            'cdn' => 'CDN',
            'tools' => 'Tools',
        ];
    }

    private function safe_text($source, $key, $fallback)
    {
        if (!is_array($source) || !isset($source[$key])) {
            return $fallback;
        }

        return sanitize_text_field((string) $source[$key]);
    }

    private function get_nested($source, $keys, $fallback)
    {
        if (!is_array($source)) {
            return $fallback;
        }

        $cursor = $source;
        foreach ($keys as $key) {
            if (!is_array($cursor) || !array_key_exists($key, $cursor)) {
                return $fallback;
            }
            $cursor = $cursor[$key];
        }

        return $cursor;
    }

    private function format_number($value)
    {
        if (!is_numeric($value)) {
            return 'n/a';
        }

        return number_format_i18n((float) $value);
    }

    private function format_bytes($value)
    {
        if (!is_numeric($value)) {
            return 'n/a';
        }

        $bytes = (float) $value;
        if ($bytes < 1024) {
            return number_format_i18n($bytes, 0) . ' B';
        }

        $units = ['KB', 'MB', 'GB', 'TB'];
        $unit_index = -1;
        while ($bytes >= 1024 && $unit_index < (count($units) - 1)) {
            $bytes /= 1024;
            $unit_index++;
        }

        return number_format_i18n($bytes, 1) . ' ' . $units[$unit_index];
    }

    private function active_tab()
    {
        $tab = isset($_GET['tab']) ? sanitize_key(wp_unslash($_GET['tab'])) : 'dashboard';
        $tabs = $this->tabs();

        return isset($tabs[$tab]) ? $tab : 'dashboard';
    }

    private function admin_page_url($tab)
    {
        $tabs = $this->tabs();
        if (!isset($tabs[$tab])) {
            $tab = 'dashboard';
        }

        if ($tab === 'dashboard') {
            return admin_url('admin.php?page=veloserve');
        }

        return admin_url('admin.php?page=veloserve&tab=' . rawurlencode($tab));
    }

    private function active_cache_view()
    {
        $view = isset($_GET['cache_view']) ? sanitize_key(wp_unslash($_GET['cache_view'])) : 'cache';
        return in_array($view, ['cache', 'ttl', 'optimization', 'purge'], true) ? $view : 'cache';
    }

    private function cache_tab_url($view)
    {
        return add_query_arg(
            [
                'page' => 'veloserve',
                'tab' => 'cache',
                'cache_view' => $view,
            ],
            admin_url('admin.php')
        );
    }

    private function build_purge_params_from_settings($settings)
    {
        $policy = isset($settings['purge_policy']) ? (string) $settings['purge_policy'] : 'all';
        if ($policy === 'domain' && !empty($settings['purge_domain'])) {
            return ['domain' => (string) $settings['purge_domain']];
        }

        if ($policy === 'path' && !empty($settings['purge_domain']) && !empty($settings['purge_path'])) {
            return [
                'domain' => (string) $settings['purge_domain'],
                'path' => (string) $settings['purge_path'],
            ];
        }

        if ($policy === 'tag' && !empty($settings['purge_tag'])) {
            return ['tag' => (string) $settings['purge_tag']];
        }

        return [];
    }

    private function perform_database_optimize()
    {
        global $wpdb;
        if (!isset($wpdb) || !is_object($wpdb) || !isset($wpdb->prefix)) {
            return new WP_Error('veloserve_tools_no_db', 'Database layer is not available.');
        }

        $tables = [];
        if (method_exists($wpdb, 'get_col')) {
            $like = esc_sql($wpdb->prefix) . '%';
            $tables = $wpdb->get_col("SHOW TABLES LIKE '{$like}'");
        }

        if (!is_array($tables) || empty($tables)) {
            return [
                'tables' => 0,
                'optimized' => 0,
            ];
        }

        $optimized = 0;
        foreach ($tables as $table) {
            if (!is_string($table) || $table === '') {
                continue;
            }

            $safe_table = preg_replace('/[^A-Za-z0-9_]/', '', $table);
            if (!is_string($safe_table) || $safe_table === '') {
                continue;
            }

            $result = $wpdb->query("OPTIMIZE TABLE `{$safe_table}`");
            if ($result !== false) {
                $optimized++;
            }
        }

        return [
            'tables' => count($tables),
            'optimized' => $optimized,
        ];
    }

    private function perform_sitemap_warm()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        $settings = array_merge(VeloServe_Plugin::default_settings(), is_array($settings) ? $settings : []);

        if (empty($settings['endpoint_url']) || empty($settings['api_token'])) {
            return new WP_Error('veloserve_tools_missing_credentials', 'Endpoint URL and API token are required for sitemap warm.');
        }

        $seed_urls = [
            home_url('/wp-sitemap.xml'),
            home_url('/sitemap.xml'),
            home_url('/sitemap_index.xml'),
        ];

        $crawl = $this->collect_sitemap_urls($seed_urls, 8, 1000);
        if (is_wp_error($crawl)) {
            return $crawl;
        }

        $discovered = isset($crawl['urls']) && is_array($crawl['urls']) ? $crawl['urls'] : [];
        if (empty($discovered)) {
            return new WP_Error('veloserve_tools_no_sitemap_urls', 'No URLs were discovered from sitemap endpoints.');
        }

        $server = new VeloServe_Server();
        $warm = $server->warm_cache($settings, $discovered, 'wordpress-sitemap-crawler', 'sitemap');
        if (is_wp_error($warm)) {
            return $warm;
        }

        return [
            'discovered' => count($discovered),
            'queued' => isset($warm['queued']) ? (int) $warm['queued'] : 0,
        ];
    }

    private function collect_sitemap_urls(array $seed_urls, $max_sitemaps = 8, $max_urls = 1000)
    {
        $pending = array_values(array_unique(array_filter(array_map('trim', $seed_urls))));
        $visited_sitemaps = [];
        $urls = [];

        while (!empty($pending) && count($visited_sitemaps) < (int) $max_sitemaps && count($urls) < (int) $max_urls) {
            $sitemap_url = array_shift($pending);
            if (!is_string($sitemap_url) || $sitemap_url === '' || isset($visited_sitemaps[$sitemap_url])) {
                continue;
            }

            $visited_sitemaps[$sitemap_url] = true;
            $response = wp_remote_get($sitemap_url, ['timeout' => 8]);
            if (is_wp_error($response)) {
                continue;
            }

            $status = wp_remote_retrieve_response_code($response);
            if ($status < 200 || $status >= 300) {
                continue;
            }

            $body = wp_remote_retrieve_body($response);
            if (!is_string($body) || trim($body) === '') {
                continue;
            }

            $parsed = $this->parse_sitemap_xml($body);
            if (!empty($parsed['sitemap_urls']) && is_array($parsed['sitemap_urls'])) {
                foreach ($parsed['sitemap_urls'] as $child_sitemap) {
                    if (!isset($visited_sitemaps[$child_sitemap])) {
                        $pending[] = $child_sitemap;
                    }
                }
            }

            if (!empty($parsed['urls']) && is_array($parsed['urls'])) {
                $urls = array_merge($urls, $parsed['urls']);
                $urls = array_values(array_unique($urls));
            }
        }

        return [
            'urls' => array_slice($urls, 0, (int) $max_urls),
            'visited_sitemaps' => array_keys($visited_sitemaps),
        ];
    }

    private function parse_sitemap_xml($xml_body)
    {
        $xml_body = trim((string) $xml_body);
        if ($xml_body === '') {
            return ['urls' => [], 'sitemap_urls' => []];
        }

        $urls = [];
        $sitemap_urls = [];

        if (function_exists('simplexml_load_string')) {
            $xml = simplexml_load_string($xml_body);
            if ($xml instanceof SimpleXMLElement) {
                $root_name = strtolower($xml->getName());
                if ($root_name === 'urlset') {
                    foreach ($xml->url as $entry) {
                        $loc = isset($entry->loc) ? trim((string) $entry->loc) : '';
                        if ($loc !== '') {
                            $urls[] = esc_url_raw($loc);
                        }
                    }
                } elseif ($root_name === 'sitemapindex') {
                    foreach ($xml->sitemap as $entry) {
                        $loc = isset($entry->loc) ? trim((string) $entry->loc) : '';
                        if ($loc !== '') {
                            $sitemap_urls[] = esc_url_raw($loc);
                        }
                    }
                }
            }
        }

        if (empty($urls) && empty($sitemap_urls) && preg_match_all('/<loc>([^<]+)<\/loc>/i', $xml_body, $matches)) {
            foreach ($matches[1] as $loc) {
                $loc = esc_url_raw(trim((string) $loc));
                if ($loc === '') {
                    continue;
                }

                if (preg_match('/sitemap/i', $loc)) {
                    $sitemap_urls[] = $loc;
                } else {
                    $urls[] = $loc;
                }
            }
        }

        return [
            'urls' => array_values(array_unique($urls)),
            'sitemap_urls' => array_values(array_unique($sitemap_urls)),
        ];
    }
}
