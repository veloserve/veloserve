<?php

if (!defined('ABSPATH')) {
    exit;
}

class VeloServe_Plugin
{
    private static $instance;

    private $client;
    private $server;
    private $cdn_manager;
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
            'auto_detect_server' => 1,
            'guest_mode' => 0,
            'server_ip_override' => '',
            'notifications_enabled' => 1,
            'cache_ttl' => 3600,
            'purge_policy' => 'all',
            'purge_domain' => '',
            'purge_path' => '/',
            'purge_tag' => '',
            'opt_minify_css' => 1,
            'opt_combine_css' => 0,
            'opt_critical_css' => 0,
            'opt_minify_js' => 1,
            'opt_combine_js' => 0,
            'opt_defer_js' => 1,
            'opt_minify_html' => 1,
            'opt_prefetch_hints' => 0,
            'opt_prefetch_urls' => '',
            'cdn_enabled' => 0,
            'cdn_provider' => 'none',
            'cloudflare_zone_id' => '',
            'cloudflare_api_token' => '',
            'cloudflare_email' => '',
            'cloudflare_api_key' => '',
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
        $this->server = new VeloServe_Server();
        $this->cdn_manager = new VeloServe_CDN_Manager();
        $this->admin = new VeloServe_Admin();
        $this->admin->hooks();

        add_action('save_post', [$this, 'purge_cache_on_content_change'], 10, 2);
        add_action('deleted_post', [$this, 'purge_cache_on_delete'], 10, 1);
        add_action('trashed_post', [$this, 'purge_cache_on_delete'], 10, 1);
        add_action('untrashed_post', [$this, 'purge_cache_on_untrash'], 10, 1);
        add_action('switch_theme', [$this, 'purge_cache_on_switch_theme']);
        add_action('customize_save_after', [$this, 'purge_cache_on_switch_theme']);
        add_action('activated_plugin', [$this, 'purge_cache_on_plugin_change'], 10, 2);
        add_action('deactivated_plugin', [$this, 'purge_cache_on_plugin_change'], 10, 2);
        add_action('upgrader_process_complete', [$this, 'purge_cache_on_upgrader_event'], 10, 2);
        add_action('woocommerce_order_status_changed', [$this, 'purge_cache_on_order_status_change'], 10, 4);
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

        $this->purge_targets($this->build_post_targets($post_id, $post));
    }

    public function purge_cache_on_delete($post_id)
    {
        if (!$this->auto_purge_enabled()) {
            return;
        }

        $targets = [['url' => home_url('/')]];
        if (function_exists('get_post_type') && function_exists('get_post_type_archive_link')) {
            $post_type = get_post_type($post_id);
            if ($post_type && $this->is_public_post_type($post_type)) {
                $archive = get_post_type_archive_link($post_type);
                if (is_string($archive) && $archive !== '') {
                    $targets[] = ['url' => $archive];
                }
            }
        }

        $this->purge_targets($targets);
    }

    public function purge_cache_on_untrash($post_id)
    {
        if (!$this->auto_purge_enabled()) {
            return;
        }

        $post = function_exists('get_post') ? get_post($post_id) : null;
        if (!($post instanceof WP_Post)) {
            return;
        }

        $this->purge_targets($this->build_post_targets($post_id, $post));
    }

    public function purge_cache_on_switch_theme()
    {
        if (!$this->auto_purge_enabled()) {
            return;
        }

        $this->purge_targets([
            ['url' => home_url('/')],
            ['path' => '/'],
            ['path' => '/wp-json/'],
        ]);
    }

    public function purge_cache_on_plugin_change()
    {
        if (!$this->auto_purge_enabled()) {
            return;
        }

        $this->purge_targets([
            ['url' => home_url('/')],
            ['path' => '/wp-json/'],
        ]);
    }

    public function purge_cache_on_upgrader_event($upgrader, $hook_extra)
    {
        if (!$this->auto_purge_enabled()) {
            return;
        }

        if (!is_array($hook_extra)) {
            return;
        }

        $type = isset($hook_extra['type']) ? (string) $hook_extra['type'] : '';
        $action = isset($hook_extra['action']) ? (string) $hook_extra['action'] : '';
        if (!in_array($type, ['plugin', 'theme'], true) || !in_array($action, ['update', 'install', 'delete'], true)) {
            return;
        }

        $this->purge_targets([
            ['url' => home_url('/')],
            ['path' => '/wp-json/'],
        ]);
    }

    public function purge_cache_on_order_status_change($order_id, $old_status, $new_status, $order)
    {
        if (!$this->auto_purge_enabled()) {
            return;
        }

        $targets = [
            ['url' => home_url('/')],
            ['path' => '/shop/'],
            ['path' => '/cart/'],
            ['path' => '/checkout/'],
            ['path' => '/my-account/'],
        ];

        if (function_exists('wc_get_page_permalink')) {
            $shop_url = wc_get_page_permalink('shop');
            if (is_string($shop_url) && $shop_url !== '') {
                $targets[] = ['url' => $shop_url];
            }
        }

        $this->purge_targets($targets);
    }

    private function auto_purge_enabled()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, self::default_settings());
        return !empty($settings['auto_purge']);
    }

    private function build_post_targets($post_id, $post)
    {
        $targets = [
            ['url' => home_url('/')],
            ['url' => get_permalink($post_id)],
        ];

        if (function_exists('get_post_type') && function_exists('get_post_type_archive_link')) {
            $post_type = get_post_type($post_id);
            if ($post_type && $this->is_public_post_type($post_type)) {
                $archive = get_post_type_archive_link($post_type);
                if (is_string($archive) && $archive !== '') {
                    $targets[] = ['url' => $archive];
                }
            }
        }

        $taxonomies = function_exists('get_object_taxonomies') ? get_object_taxonomies($post->post_type, 'names') : [];
        if (is_array($taxonomies) && !empty($taxonomies) && function_exists('wp_get_post_terms') && function_exists('get_term_link')) {
            foreach ($taxonomies as $taxonomy) {
                $terms = wp_get_post_terms($post_id, $taxonomy);
                if (!is_array($terms)) {
                    continue;
                }
                foreach ($terms as $term) {
                    if (!is_object($term) || !isset($term->term_id)) {
                        continue;
                    }
                    $term_link = get_term_link((int) $term->term_id, $taxonomy);
                    if (is_string($term_link) && $term_link !== '') {
                        $targets[] = ['url' => $term_link];
                    }
                }
            }
        }

        return $targets;
    }

    private function send_purge_for_url($url)
    {
        $this->purge_targets([['url' => $url]]);
    }

    private function purge_targets(array $targets)
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, self::default_settings());
        $settings = array_merge(self::default_settings(), is_array($settings) ? $settings : []);
        if (!$this->server) {
            $this->server = new VeloServe_Server();
        }
        if (!$this->cdn_manager) {
            $this->cdn_manager = new VeloServe_CDN_Manager();
        }

        $seen = [];
        foreach ($targets as $target) {
            if (!is_array($target)) {
                continue;
            }

            $params = [];
            if (!empty($target['url'])) {
                $params['url'] = (string) $target['url'];
            } elseif (!empty($target['domain']) || !empty($target['path']) || !empty($target['tag'])) {
                if (!empty($target['domain'])) {
                    $params['domain'] = (string) $target['domain'];
                }
                if (!empty($target['path'])) {
                    $params['path'] = (string) $target['path'];
                }
                if (!empty($target['tag'])) {
                    $params['tag'] = (string) $target['tag'];
                }
            }

            if (empty($params)) {
                continue;
            }

            $key = md5(wp_json_encode($params));
            if (isset($seen[$key])) {
                continue;
            }
            $seen[$key] = true;

            $this->server->purge_cache($settings, $params);
            if ($this->cdn_manager->should_purge($settings)) {
                $this->cdn_manager->purge($settings, $params);
            }
        }
    }

    private function is_public_post_type($post_type)
    {
        if (!is_string($post_type) || $post_type === '') {
            return false;
        }

        if (function_exists('get_post_type_object')) {
            $obj = get_post_type_object($post_type);
            if (is_object($obj) && isset($obj->public)) {
                return !empty($obj->public);
            }
        }

        if (function_exists('post_type_exists')) {
            return post_type_exists($post_type);
        }

        return false;
    }
}
