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

    public function render_page()
    {
        if (!current_user_can('manage_options')) {
            return;
        }

        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        $status = get_option(VELOSERVE_STATUS_KEY, []);
        ?>
        <div class="wrap">
            <h1>VeloServe Integration</h1>
            <p>Configure connectivity between this WordPress site and your VeloServe control plane.</p>

            <h2>Status</h2>
            <table class="widefat striped" style="max-width: 680px;">
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
                </tbody>
            </table>

            <form method="post" action="options.php">
                <?php
                settings_fields('veloserve_settings_group');
                do_settings_sections('veloserve');
                submit_button('Save Settings');
                ?>
            </form>

            <form method="post" action="<?php echo esc_url(admin_url('admin-post.php')); ?>">
                <?php wp_nonce_field('veloserve_register_action', 'veloserve_register_nonce'); ?>
                <input type="hidden" name="action" value="veloserve_register" />
                <?php submit_button('Register Site with VeloServe', 'primary'); ?>
            </form>
        </div>
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

        $result = VeloServe_Plugin::instance()->register_with_endpoint();

        if (is_wp_error($result)) {
            VeloServe_Plugin::instance()->set_status([
                'connected' => false,
                'last_error' => $result->get_error_message(),
            ]);
            wp_safe_redirect(add_query_arg('veloserve_error', rawurlencode($result->get_error_message()), wp_get_referer()));
            exit;
        }

        wp_safe_redirect(add_query_arg('veloserve_registered', '1', wp_get_referer()));
        exit;
    }
}
