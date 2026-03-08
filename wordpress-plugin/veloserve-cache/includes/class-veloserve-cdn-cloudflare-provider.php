<?php

if (!defined('ABSPATH')) {
    exit;
}

class VeloServe_CDN_Cloudflare_Provider implements VeloServe_CDN_Provider
{
    public function provider_key()
    {
        return 'cloudflare';
    }

    public function test_connection(array $settings)
    {
        $zone_id = $this->zone_id($settings);
        if ($zone_id === '') {
            return new WP_Error('veloserve_cdn_missing_zone', 'Cloudflare Zone ID is required.');
        }

        $response = $this->request_json('GET', '/zones/' . rawurlencode($zone_id), [], $settings);
        if (is_wp_error($response)) {
            return $response;
        }

        $zone_name = isset($response['result']['name']) ? sanitize_text_field((string) $response['result']['name']) : '';
        return [
            'provider' => 'cloudflare',
            'zone_id' => $zone_id,
            'zone_name' => $zone_name,
        ];
    }

    public function purge(array $settings, array $params = [])
    {
        $zone_id = $this->zone_id($settings);
        if ($zone_id === '') {
            return new WP_Error('veloserve_cdn_missing_zone', 'Cloudflare Zone ID is required.');
        }

        $payload = $this->build_purge_payload($params);
        return $this->request_json(
            'POST',
            '/zones/' . rawurlencode($zone_id) . '/purge_cache',
            $payload,
            $settings
        );
    }

    private function build_purge_payload(array $params)
    {
        if (!empty($params['url'])) {
            $url = esc_url_raw((string) $params['url']);
            if ($url !== '') {
                return ['files' => [$url]];
            }
        }

        $domain = isset($params['domain']) ? sanitize_text_field((string) $params['domain']) : '';
        $path = isset($params['path']) ? sanitize_text_field((string) $params['path']) : '';
        $tag = isset($params['tag']) ? sanitize_text_field((string) $params['tag']) : '';

        if ($domain !== '' && $path !== '') {
            if (strpos($path, '/') !== 0) {
                $path = '/' . $path;
            }

            return ['files' => ['https://' . $domain . $path]];
        }

        if ($domain !== '') {
            return ['hosts' => [$domain]];
        }

        if ($tag !== '') {
            return ['tags' => [$tag]];
        }

        return ['purge_everything' => true];
    }

    private function request_json($method, $path, array $body, array $settings)
    {
        $headers = $this->auth_headers($settings);
        if (is_wp_error($headers)) {
            return $headers;
        }

        $headers['Accept'] = 'application/json';
        $headers['Content-Type'] = 'application/json';

        $args = [
            'timeout' => 8,
            'headers' => $headers,
        ];

        $url = 'https://api.cloudflare.com/client/v4' . $path;
        if ($method === 'GET') {
            $response = wp_remote_get($url, $args);
        } else {
            $args['body'] = wp_json_encode($body);
            $response = wp_remote_post($url, $args);
        }

        if (is_wp_error($response)) {
            return $response;
        }

        $status_code = wp_remote_retrieve_response_code($response);
        $payload = json_decode((string) wp_remote_retrieve_body($response), true);

        if ($status_code < 200 || $status_code >= 300) {
            return new WP_Error(
                'veloserve_cdn_request_failed',
                sprintf('Cloudflare API request failed with status %d.', $status_code)
            );
        }

        if (!is_array($payload)) {
            return [];
        }

        if (isset($payload['success']) && !$payload['success']) {
            $message = 'Cloudflare API returned an error.';
            if (!empty($payload['errors']) && is_array($payload['errors'])) {
                $first = reset($payload['errors']);
                if (is_array($first) && !empty($first['message'])) {
                    $message = sanitize_text_field((string) $first['message']);
                }
            }

            return new WP_Error('veloserve_cdn_api_error', $message);
        }

        return $payload;
    }

    private function auth_headers(array $settings)
    {
        $token = isset($settings['cloudflare_api_token']) ? sanitize_text_field((string) $settings['cloudflare_api_token']) : '';
        if ($token !== '') {
            return ['Authorization' => 'Bearer ' . $token];
        }

        $email = isset($settings['cloudflare_email']) ? sanitize_text_field((string) $settings['cloudflare_email']) : '';
        $api_key = isset($settings['cloudflare_api_key']) ? sanitize_text_field((string) $settings['cloudflare_api_key']) : '';
        if ($email !== '' && $api_key !== '') {
            return [
                'X-Auth-Email' => $email,
                'X-Auth-Key' => $api_key,
            ];
        }

        return new WP_Error(
            'veloserve_cdn_missing_auth',
            'Cloudflare auth is required. Set API Token or Email + API Key.'
        );
    }

    private function zone_id(array $settings)
    {
        return isset($settings['cloudflare_zone_id']) ? sanitize_text_field((string) $settings['cloudflare_zone_id']) : '';
    }
}
