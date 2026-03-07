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
}
