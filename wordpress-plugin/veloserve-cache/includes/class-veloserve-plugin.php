<?php

if (!defined('ABSPATH')) {
    exit;
}

class VeloServe_Plugin
{
    private static $instance;

    private $client;
    private $admin;

    public static function instance()
    {
        if (!self::$instance) {
            self::$instance = new self();
        }

        return self::$instance;
    }

    public static function activate()
    {
        if (!get_option(VELOSERVE_OPTION_KEY)) {
            add_option(VELOSERVE_OPTION_KEY, self::default_settings());
        }

        if (!get_option(VELOSERVE_STATUS_KEY)) {
            add_option(VELOSERVE_STATUS_KEY, self::default_status());
        }
    }

    public static function deactivate()
    {
        $status = get_option(VELOSERVE_STATUS_KEY, self::default_status());
        $status['connected'] = false;
        update_option(VELOSERVE_STATUS_KEY, $status);
    }

    public static function default_settings()
    {
        return [
            'endpoint_url' => '',
            'api_token' => '',
            'auto_purge' => 1,
        ];
    }

    public static function default_status()
    {
        return [
            'connected' => false,
            'node_id' => '',
            'registered_at' => null,
            'last_error' => '',
        ];
    }

    public function bootstrap()
    {
        $this->client = new VeloServe_Client();
        $this->admin = new VeloServe_Admin();
        $this->admin->hooks();

        add_action('save_post', [$this, 'purge_cache_on_content_change'], 10, 2);
        add_action('deleted_post', [$this, 'purge_cache_on_delete'], 10, 1);
        add_action('switch_theme', [$this, 'purge_cache_on_switch_theme']);
        add_action('customize_save_after', [$this, 'purge_cache_on_switch_theme']);
    }

    public function register_with_endpoint()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, self::default_settings());
        $result = $this->client->register_site($settings);

        if (is_wp_error($result)) {
            return $result;
        }

        $this->set_status([
            'connected' => true,
            'node_id' => $result['node_id'],
            'registered_at' => $result['registered_at'],
            'last_error' => '',
        ]);

        return $result;
    }

    public function set_status(array $status)
    {
        $current = get_option(VELOSERVE_STATUS_KEY, self::default_status());
        update_option(VELOSERVE_STATUS_KEY, array_merge($current, $status));
    }

    public function purge_cache_on_content_change($post_id, $post)
    {
        if (wp_is_post_revision($post_id)) {
            return;
        }

        if (!($post instanceof WP_Post) || $post->post_status !== 'publish') {
            return;
        }

        $settings = get_option(VELOSERVE_OPTION_KEY, self::default_settings());
        if (empty($settings['auto_purge'])) {
            return;
        }

        $this->send_purge_for_url(get_permalink($post_id));
    }

    public function purge_cache_on_delete($post_id)
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, self::default_settings());
        if (empty($settings['auto_purge'])) {
            return;
        }

        $this->send_purge_for_url(home_url('/'));
    }

    public function purge_cache_on_switch_theme()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, self::default_settings());
        if (empty($settings['auto_purge'])) {
            return;
        }

        $this->send_purge_for_url(home_url('/'));
    }

    private function send_purge_for_url($url)
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, self::default_settings());

        if (empty($settings['endpoint_url']) || empty($settings['api_token'])) {
            return;
        }

        wp_remote_post(
            esc_url_raw(untrailingslashit($settings['endpoint_url']) . '/api/v1/cache/purge'),
            [
                'timeout' => 8,
                'headers' => [
                    'Authorization' => 'Bearer ' . $settings['api_token'],
                    'Content-Type' => 'application/json',
                ],
                'body' => wp_json_encode([
                    'url' => $url,
                    'source' => 'wordpress-plugin',
                ]),
            ]
        );
    }
}
