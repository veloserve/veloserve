<?php

if (!defined('ABSPATH')) {
    exit;
}

class VeloServe_Client
{
    public function register_site(array $settings)
    {
        if (empty($settings['endpoint_url']) || empty($settings['api_token'])) {
            return new WP_Error('veloserve_missing_config', 'Endpoint URL and API token are required.');
        }

        $body = [
            'site_url' => home_url('/'),
            'site_name' => get_bloginfo('name'),
            'plugin_version' => VELOSERVE_PLUGIN_VERSION,
            'wp_version' => get_bloginfo('version'),
            'php_version' => PHP_VERSION,
            'optimization' => [
                'minify_css' => !empty($settings['opt_minify_css']),
                'combine_css' => !empty($settings['opt_combine_css']),
                'critical_css' => !empty($settings['opt_critical_css']),
                'minify_js' => !empty($settings['opt_minify_js']),
                'combine_js' => !empty($settings['opt_combine_js']),
                'defer_js' => !empty($settings['opt_defer_js']),
                'minify_html' => !empty($settings['opt_minify_html']),
                'prefetch_hints' => !empty($settings['opt_prefetch_hints']),
                'prefetch_urls' => $this->parse_prefetch_urls($settings),
                'images' => [
                    'lazyload' => !empty($settings['opt_lazyload_images']),
                    'webp' => !empty($settings['opt_image_webp']),
                    'avif' => !empty($settings['opt_image_avif']),
                    'quality' => isset($settings['opt_image_quality']) ? (int) $settings['opt_image_quality'] : 82,
                    'queue_enabled' => !empty($settings['opt_image_queue']),
                ],
            ],
        ];

        $response = wp_remote_post(
            esc_url_raw(untrailingslashit($settings['endpoint_url']) . '/api/v1/wordpress/register'),
            [
                'timeout' => 10,
                'headers' => [
                    'Authorization' => 'Bearer ' . $settings['api_token'],
                    'Content-Type' => 'application/json',
                ],
                'body' => wp_json_encode($body),
            ]
        );

        if (is_wp_error($response)) {
            return $response;
        }

        $code = wp_remote_retrieve_response_code($response);
        $payload = json_decode((string) wp_remote_retrieve_body($response), true);

        if ($code < 200 || $code >= 300) {
            return new WP_Error('veloserve_register_failed', sprintf('Registration failed with status %d.', $code));
        }

        return [
            'node_id' => isset($payload['node_id']) ? sanitize_text_field($payload['node_id']) : '',
            'registered_at' => current_time('mysql', true),
        ];
    }

    private function parse_prefetch_urls(array $settings)
    {
        $raw = isset($settings['opt_prefetch_urls']) ? (string) $settings['opt_prefetch_urls'] : '';
        if ($raw === '') {
            return [];
        }

        $urls = preg_split('/\r\n|\r|\n/', $raw);
        if (!is_array($urls)) {
            return [];
        }

        $normalized = [];
        foreach ($urls as $url) {
            $url = trim((string) $url);
            if ($url !== '') {
                $normalized[] = esc_url_raw($url);
            }
        }

        return $normalized;
    }
}
