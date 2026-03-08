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

        add_submenu_page('veloserve', 'Overview', 'Overview', 'manage_options', 'veloserve', [$this, 'render_page']);
        add_submenu_page('veloserve', 'Connection', 'Connection', 'manage_options', 'veloserve&tab=connection', [$this, 'render_page']);
        add_submenu_page('veloserve', 'Cache', 'Cache', 'manage_options', 'veloserve&tab=cache', [$this, 'render_page']);
        add_submenu_page('veloserve', 'Tools', 'Tools', 'manage_options', 'veloserve&tab=tools', [$this, 'render_page']);
    }

    public function register_settings()
    {
        register_setting('veloserve_settings_group', VELOSERVE_OPTION_KEY, [$this, 'sanitize']);

        add_settings_section('veloserve_main', 'Connection', '__return_false', 'veloserve');

        add_settings_field('endpoint_url', 'Endpoint URL', [$this, 'render_endpoint_field'], 'veloserve', 'veloserve_main');
        add_settings_field('api_token', 'API Token', [$this, 'render_token_field'], 'veloserve', 'veloserve_main');
        add_settings_field('auto_purge', 'Auto Purge', [$this, 'render_auto_purge_field'], 'veloserve', 'veloserve_main');
    }

    public function sanitize($input)
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());

        $settings['endpoint_url'] = isset($input['endpoint_url']) ? esc_url_raw(trim($input['endpoint_url'])) : '';
        $settings['api_token'] = isset($input['api_token']) ? sanitize_text_field(trim($input['api_token'])) : '';
        $settings['auto_purge'] = !empty($input['auto_purge']) ? 1 : 0;

        return $settings;
    }

    public function render_notices()
    {
        $screen = get_current_screen();
        if (!$screen || $screen->id !== 'toplevel_page_veloserve') {
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

    public function handle_admin_bar_register()
    {
        if (!current_user_can('manage_options')) {
            wp_die('Insufficient permissions');
        }

        check_admin_referer('veloserve_register_action');
        $this->perform_register();
    }

    public function render_page()
    {
        if (!current_user_can('manage_options')) {
            return;
        }

        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        $status = get_option(VELOSERVE_STATUS_KEY, []);
        $tab = $this->active_tab();
        ?>
        <div class="wrap veloserve-shell">
            <?php $this->render_brand_header($status); ?>
            <?php $this->render_tabs($tab); ?>

            <div class="veloserve-panel">
                <?php if ($tab === 'connection'): ?>
                    <?php $this->render_connection_tab(); ?>
                <?php elseif ($tab === 'cache'): ?>
                    <?php $this->render_cache_tab($settings); ?>
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

        $response = wp_remote_post(
            esc_url_raw(untrailingslashit($settings['endpoint_url']) . '/api/v1/cache/purge'),
            [
                'timeout' => 10,
                'headers' => [
                    'Authorization' => 'Bearer ' . $settings['api_token'],
                    'Content-Type' => 'application/json',
                ],
                'body' => wp_json_encode([
                    'url' => home_url('/'),
                    'purge_all' => true,
                    'source' => 'wordpress-plugin',
                ]),
            ]
        );

        if (is_wp_error($response)) {
            wp_safe_redirect(add_query_arg('veloserve_purge_error', rawurlencode($response->get_error_message()), $this->admin_page_url('cache')));
            exit;
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
        ?>
        <h2>Overview</h2>
        <table class="widefat striped" style="max-width: 720px;">
            <tbody>
                <tr>
                    <th>Connection</th>
                    <td><?php echo !empty($status['connected']) ? 'Connected' : 'Not connected'; ?></td>
                </tr>
                <tr>
                    <th>Node ID</th>
                    <td><?php echo !empty($status['node_id']) ? esc_html($status['node_id']) : 'N/A'; ?></td>
                </tr>
                <tr>
                    <th>Last registration</th>
                    <td><?php echo !empty($status['registered_at']) ? esc_html($status['registered_at']) : 'Never'; ?></td>
                </tr>
                <?php if (!empty($status['last_error'])): ?>
                <tr>
                    <th>Last error</th>
                    <td style="color: #d63638;"><?php echo esc_html($status['last_error']); ?></td>
                </tr>
                <?php endif; ?>
            </tbody>
        </table>
        <div class="veloserve-actions">
            <a href="<?php echo esc_url($this->admin_page_url('connection')); ?>" class="button">Open Connection</a>
            <a href="<?php echo esc_url($this->admin_page_url('cache')); ?>" class="button">Open Cache Controls</a>
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
            do_settings_sections('veloserve');
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

    private function render_cache_tab($settings)
    {
        ?>
        <h2>Cache Controls</h2>
        <p>Run immediate full-site purges and verify automatic purge behavior.</p>
        <p><strong>Auto Purge:</strong> <?php echo !empty($settings['auto_purge']) ? 'Enabled' : 'Disabled'; ?></p>

        <form method="post" action="<?php echo esc_url(admin_url('admin-post.php')); ?>" style="display:inline-block; margin-top: 8px;">
            <?php wp_nonce_field('veloserve_purge_all_action', 'veloserve_purge_all_nonce'); ?>
            <input type="hidden" name="action" value="veloserve_purge_all" />
            <?php submit_button('Purge All Cache', 'secondary', 'submit', false); ?>
        </form>
        <?php
    }

    private function render_tools_tab()
    {
        ?>
        <h2>Tools</h2>
        <p>Quick access to operational actions and docs for support workflows.</p>
        <ul>
            <li><a href="<?php echo esc_url($this->admin_page_url('dashboard')); ?>">View system overview</a></li>
            <li><a href="<?php echo esc_url($this->admin_page_url('connection')); ?>">Run site registration</a></li>
            <li><a href="<?php echo esc_url($this->admin_page_url('cache')); ?>">Run cache purge</a></li>
        </ul>
        <?php
    }

    private function tabs()
    {
        return [
            'dashboard' => 'Overview',
            'connection' => 'Connection',
            'cache' => 'Cache',
            'tools' => 'Tools',
        ];
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
}
