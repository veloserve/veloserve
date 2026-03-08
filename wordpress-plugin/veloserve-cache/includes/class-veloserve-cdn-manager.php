<?php

if (!defined('ABSPATH')) {
    exit;
}

class VeloServe_CDN_Manager
{
    private $providers;

    public function __construct(?array $providers = null)
    {
        $this->providers = $providers ?: [
            new VeloServe_CDN_Cloudflare_Provider(),
        ];
    }

    public function test_connection(array $settings)
    {
        $provider = $this->resolve_provider($settings);
        if (is_wp_error($provider)) {
            return $provider;
        }

        return $provider->test_connection($settings);
    }

    public function purge(array $settings, array $params = [])
    {
        $provider = $this->resolve_provider($settings);
        if (is_wp_error($provider)) {
            return $provider;
        }

        return $provider->purge($settings, $params);
    }

    public function should_purge(array $settings)
    {
        if (empty($settings['cdn_enabled'])) {
            return false;
        }

        return !is_wp_error($this->resolve_provider($settings));
    }

    private function resolve_provider(array $settings)
    {
        $key = isset($settings['cdn_provider']) ? sanitize_text_field((string) $settings['cdn_provider']) : 'none';
        if ($key === '' || $key === 'none') {
            return new WP_Error('veloserve_cdn_not_configured', 'CDN provider is not configured.');
        }

        foreach ($this->providers as $provider) {
            if (!($provider instanceof VeloServe_CDN_Provider)) {
                continue;
            }

            if ($provider->provider_key() === $key) {
                return $provider;
            }
        }

        return new WP_Error('veloserve_cdn_provider_invalid', 'Unsupported CDN provider selected.');
    }
}
