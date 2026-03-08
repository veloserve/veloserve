<?php

if (!defined('ABSPATH')) {
    exit;
}

class VeloServe_Server
{
    private $default_api_bases;

    public function __construct(?array $default_api_bases = null)
    {
        $this->default_api_bases = $default_api_bases ?: [
            'http://127.0.0.1:8080',
            'http://localhost:8080',
        ];
    }

    public function detect_server(array $settings = [])
    {
        $candidates = $this->build_api_candidates($settings);

        foreach ($candidates as $api_base) {
            $result = $this->request_json('GET', $this->build_url($api_base, '/api/v1/status'), $settings);
            if (is_wp_error($result)) {
                continue;
            }

            return [
                'detected' => true,
                'api_base' => $api_base,
                'status' => isset($result['status']) ? sanitize_text_field((string) $result['status']) : '',
                'server' => isset($result['server']) ? sanitize_text_field((string) $result['server']) : '',
                'version' => isset($result['version']) ? sanitize_text_field((string) $result['version']) : '',
                'php_available' => !empty($result['php_available']),
                'cache_enabled' => !empty($result['cache_enabled']),
            ];
        }

        return new WP_Error(
            'veloserve_server_not_detected',
            'Could not detect a reachable VeloServe API endpoint.'
        );
    }

    public function get_cache_stats(array $settings = [])
    {
        $api_base = $this->resolve_api_base($settings);
        if (is_wp_error($api_base)) {
            return $api_base;
        }

        return $this->request_json('GET', $this->build_url($api_base, '/api/v1/cache/stats'), $settings);
    }

    public function get_cache_config(array $settings = [])
    {
        $api_base = $this->resolve_api_base($settings);
        if (is_wp_error($api_base)) {
            return $api_base;
        }

        return $this->request_json('GET', $this->build_url($api_base, '/api/v1/cache/config'), $settings);
    }

    public function purge_cache(array $settings = [], array $params = [])
    {
        $api_base = $this->resolve_api_base($settings);
        if (is_wp_error($api_base)) {
            return $api_base;
        }

        $query = $this->build_purge_query($params);
        $path = '/api/v1/cache/purge' . ($query ? '?' . http_build_query($query) : '');

        return $this->request_json('POST', $this->build_url($api_base, $path), $settings);
    }

    private function build_purge_query(array $params)
    {
        $query = [];

        if (!empty($params['key'])) {
            $query['key'] = sanitize_text_field((string) $params['key']);
            return $query;
        }

        if (!empty($params['tag'])) {
            $query['tag'] = sanitize_text_field((string) $params['tag']);
            return $query;
        }

        $domain = !empty($params['domain']) ? sanitize_text_field((string) $params['domain']) : '';
        $path = !empty($params['path']) ? (string) $params['path'] : '';

        if (!empty($params['url'])) {
            $parts = wp_parse_url((string) $params['url']);
            if (!empty($parts['host'])) {
                $domain = (string) $parts['host'];
                if (!empty($parts['port'])) {
                    $domain .= ':' . (int) $parts['port'];
                }
            }
            if (!empty($parts['path'])) {
                $path = (string) $parts['path'];
            }
        }

        if ($domain !== '' && $path !== '') {
            $query['domain'] = $domain;
            $query['path'] = $path;
            return $query;
        }

        if ($domain !== '') {
            $query['domain'] = $domain;
        }

        return $query;
    }

    private function request_json($method, $url, array $settings)
    {
        $headers = [
            'Accept' => 'application/json',
        ];

        if (!empty($settings['api_token'])) {
            $headers['Authorization'] = 'Bearer ' . $settings['api_token'];
        }

        $args = [
            'timeout' => 8,
            'headers' => $headers,
        ];

        if ($method === 'GET') {
            $response = wp_remote_get($url, $args);
        } else {
            $response = wp_remote_post($url, $args);
        }

        if (is_wp_error($response)) {
            return $response;
        }

        $status_code = wp_remote_retrieve_response_code($response);
        $payload = json_decode((string) wp_remote_retrieve_body($response), true);

        if ($status_code < 200 || $status_code >= 300) {
            return new WP_Error(
                'veloserve_api_request_failed',
                sprintf('VeloServe API request failed with status %d.', $status_code)
            );
        }

        if (!is_array($payload)) {
            return [];
        }

        return $payload;
    }

    private function resolve_api_base(array $settings)
    {
        $detected = $this->detect_server($settings);
        if (is_wp_error($detected)) {
            return $detected;
        }

        return $detected['api_base'];
    }

    private function build_api_candidates(array $settings)
    {
        if (!empty($settings['endpoint_url'])) {
            return [untrailingslashit((string) $settings['endpoint_url'])];
        }

        return $this->default_api_bases;
    }

    private function build_url($api_base, $path)
    {
        return esc_url_raw(untrailingslashit((string) $api_base) . $path);
    }
}
