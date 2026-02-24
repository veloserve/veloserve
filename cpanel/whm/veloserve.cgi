#!/usr/bin/perl
# VeloServe WHM Plugin - Full Management Interface
use strict;
use warnings;
use CGI;
use JSON;
use POSIX qw(strftime);

my $VELOSERVE_BIN    = '/usr/local/bin/veloserve';
my $VELOSERVE_CONFIG = '/etc/veloserve/veloserve.toml';
my $SWAP_SCRIPT      = '/usr/local/veloserve/cpanel/import-apache-and-swap.sh';
my $HOOKS_LOG        = '/var/log/veloserve/hooks.log';
my $ERROR_LOG        = '/var/log/veloserve/error.log';
my $CHKSERVD_CONF    = '/etc/chkserv.d/chkservd.conf';

my $cgi = CGI->new;
my $action = $cgi->param('action') || 'dashboard';

if ($action =~ /^api_/) {
    print $cgi->header('application/json');
} else {
    print $cgi->header('text/html');
}

my %routes = (
    'dashboard'    => \&page_dashboard,
    'switch'       => \&page_switch,
    'vhosts'       => \&page_vhosts,
    'php'          => \&page_php,
    'cache'        => \&page_cache,
    'ssl'          => \&page_ssl,
    'config'       => \&page_config,
    'logs'         => \&page_logs,
    'about'        => \&page_about,
    'api_status'          => \&api_status,
    'api_start'           => \&api_start,
    'api_stop'            => \&api_stop,
    'api_restart'         => \&api_restart,
    'api_reload'          => \&api_reload,
    'api_switch_veloserve'=> \&api_switch_veloserve,
    'api_switch_apache'   => \&api_switch_apache,
    'api_vhost_delete'    => \&api_vhost_delete,
    'api_vhost_import'    => \&api_vhost_import,
    'api_cache_purge'     => \&api_cache_purge,
    'api_config_save'     => \&api_config_save,
    'api_php_switch'      => \&api_php_switch,
    'api_logs'            => \&api_logs,
);

if (exists $routes{$action}) {
    $routes{$action}->();
} else {
    page_dashboard();
}
exit 0;


###############################################################################
# HELPER FUNCTIONS
###############################################################################

sub cmd { my $c = shift; my $out = `$c 2>&1`; chomp $out; return $out; }

sub get_service_status {
    my ($svc) = @_;
    my $active = cmd("systemctl is-active $svc 2>/dev/null") eq 'active' ? 1 : 0;
    my $enabled = cmd("systemctl is-enabled $svc 2>/dev/null") eq 'enabled' ? 1 : 0;
    my $pid = 0;
    my $uptime = '';
    if ($active) {
        $pid = cmd("systemctl show $svc --property=MainPID --value 2>/dev/null") || 0;
        my $ts = cmd("systemctl show $svc --property=ActiveEnterTimestamp --value 2>/dev/null");
        $uptime = $ts ? time_ago($ts) : 'unknown';
    }
    return { active => $active, enabled => $enabled, pid => $pid, uptime => $uptime };
}

sub time_ago {
    my ($ts) = @_;
    return 'unknown' unless $ts;
    my $epoch = `date -d "$ts" +%s 2>/dev/null`; chomp $epoch;
    return 'unknown' unless $epoch && $epoch =~ /^\d+$/;
    my $diff = time() - $epoch;
    if ($diff < 60) { return "${diff}s"; }
    if ($diff < 3600) { return int($diff/60) . "m"; }
    if ($diff < 86400) { return int($diff/3600) . "h " . int(($diff%3600)/60) . "m"; }
    return int($diff/86400) . "d " . int(($diff%86400)/3600) . "h";
}

sub get_veloserve_version {
    return cmd("$VELOSERVE_BIN --version 2>/dev/null") || 'unknown';
}

sub get_active_webserver {
    my $vs = get_service_status('veloserve');
    my $ap = get_service_status('httpd');
    my $port80 = cmd("ss -tlnp sport = :80 2>/dev/null");
    if ($port80 =~ /veloserve/) { return 'veloserve'; }
    if ($port80 =~ /httpd|apache/) { return 'apache'; }
    return $vs->{active} ? 'veloserve' : ($ap->{active} ? 'apache' : 'none');
}

sub get_chkservd_status {
    my ($svc) = @_;
    return 0 unless -f $CHKSERVD_CONF;
    open my $fh, '<', $CHKSERVD_CONF or return 0;
    while (<$fh>) { if (/^$svc:(\d)/) { close $fh; return $1; } }
    close $fh;
    return 0;
}

sub read_config {
    open my $fh, '<', $VELOSERVE_CONFIG or return '';
    local $/; my $c = <$fh>; close $fh; return $c;
}

sub parse_vhosts {
    my @vhosts;
    my $config = read_config();
    my @blocks = split /\[\[virtualhost\]\]/, $config;
    shift @blocks;
    for my $block (@blocks) {
        my %vh;
        $vh{domain} = $1 if $block =~ /domain\s*=\s*"([^"]+)"/;
        $vh{root}   = $1 if $block =~ /root\s*=\s*"([^"]+)"/;
        $vh{platform} = $1 if $block =~ /platform\s*=\s*"([^"]+)"/;
        $vh{ssl_cert} = $1 if $block =~ /ssl_certificate\s*=\s*"([^"]+)"/;
        $vh{ssl_key}  = $1 if $block =~ /ssl_certificate_key\s*=\s*"([^"]+)"/;
        $vh{platform} ||= 'generic';
        if ($vh{root} && $vh{root} =~ m{/home/([^/]+)/}) {
            $vh{owner} = $1;
        } else {
            $vh{owner} = 'root';
        }
        $vh{has_ssl} = ($vh{ssl_cert} && -f $vh{ssl_cert}) ? 1 : 0;
        push @vhosts, \%vh if $vh{domain};
    }
    return @vhosts;
}

sub parse_php_config {
    my $config = read_config();
    my %php;
    if ($config =~ /\[php\](.*?)(?=\n\[|\z)/s) {
        my $block = $1;
        $php{enable} = ($block =~ /enable\s*=\s*true/) ? 1 : 0;
        $php{mode} = $1 if $block =~ /mode\s*=\s*"([^"]+)"/;
        $php{version} = $1 if $block =~ /version\s*=\s*"([^"]+)"/;
        $php{binary} = $1 if $block =~ /binary_path\s*=\s*"([^"]+)"/;
        $php{workers} = $1 if $block =~ /workers\s*=\s*(\d+)/;
        $php{memory} = $1 if $block =~ /memory_limit\s*=\s*"([^"]+)"/;
        $php{max_exec} = $1 if $block =~ /max_execution_time\s*=\s*(\d+)/;
    }
    return \%php;
}

sub parse_cache_config {
    my $config = read_config();
    my %cache;
    if ($config =~ /\[cache\](.*?)(?=\n\[|\z)/s) {
        my $block = $1;
        $cache{enable} = ($block =~ /enable\s*=\s*true/) ? 1 : 0;
        $cache{storage} = $1 if $block =~ /storage\s*=\s*"([^"]+)"/;
        $cache{memory_limit} = $1 if $block =~ /memory_limit\s*=\s*"([^"]+)"/;
        $cache{ttl} = $1 if $block =~ /default_ttl\s*=\s*(\d+)/;
        $cache{disk_path} = $1 if $block =~ /disk_path\s*=\s*"([^"]+)"/;
    }
    return \%cache;
}

sub parse_ssl_config {
    my $config = read_config();
    my %ssl;
    if ($config =~ /\[ssl\](.*?)(?=\n\[|\z)/s) {
        my $block = $1;
        $ssl{cert} = $1 if $block =~ /cert\s*=\s*"([^"]+)"/;
        $ssl{key}  = $1 if $block =~ /key\s*=\s*"([^"]+)"/;
    }
    return \%ssl;
}

sub get_cert_info {
    my ($path) = @_;
    return {} unless $path && -f $path;
    my $subj = cmd("openssl x509 -in '$path' -noout -subject 2>/dev/null");
    my $issuer = cmd("openssl x509 -in '$path' -noout -issuer 2>/dev/null");
    my $expiry = cmd("openssl x509 -in '$path' -noout -enddate 2>/dev/null");
    $subj =~ s/^subject=\s*//;
    $issuer =~ s/^issuer=\s*//;
    $expiry =~ s/^notAfter=//;
    return { subject => $subj, issuer => $issuer, expiry => $expiry };
}

sub find_ea_php_versions {
    my @versions;
    for my $dir (glob("/opt/cpanel/ea-php*/")) {
        if ($dir =~ /ea-php(\d+)/) {
            my $ver = $1;
            my $cgi_bin = "$dir/root/usr/bin/php-cgi";
            my $label = substr($ver, 0, 1) . '.' . substr($ver, 1);
            push @versions, {
                version => $ver,
                label   => $label,
                binary  => $cgi_bin,
                installed => -x $cgi_bin ? 1 : 0,
            };
        }
    }
    return sort { $b->{version} cmp $a->{version} } @versions;
}

sub html_escape {
    my ($s) = @_;
    $s =~ s/&/&amp;/g;
    $s =~ s/</&lt;/g;
    $s =~ s/>/&gt;/g;
    $s =~ s/"/&quot;/g;
    return $s;
}


###############################################################################
# HTML LAYOUT
###############################################################################

sub html_header {
    my ($title, $current_page) = @_;
    $current_page ||= 'dashboard';
    my $version = get_veloserve_version();
    my @nav = (
        ['dashboard', 'Dashboard'],
        ['switch',    'Switch'],
        ['vhosts',    'Virtual Hosts'],
        ['php',       'PHP'],
        ['cache',     'Cache'],
        ['ssl',       'SSL/TLS'],
        ['config',    'Config'],
        ['logs',      'Logs'],
        ['about',     'About'],
    );
    my $nav_html = '';
    for my $n (@nav) {
        my $cls = $n->[0] eq $current_page ? ' class="active"' : '';
        $nav_html .= qq{<a href="?action=$n->[0]"$cls>$n->[1]</a>};
    }

    print qq{<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>$title - VeloServe</title>
<link rel="stylesheet" href="/cgi/veloserve/assets/veloserve.css">
</head>
<body>
<div class="vs-topbar">
  <div class="logo">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>
    VeloServe
  </div>
  <nav>$nav_html</nav>
  <div class="right">$version</div>
</div>
<div class="vs-page">
};
}

sub html_footer {
    print qq{
</div>
<script src="/cgi/veloserve/assets/veloserve.js"></script>
</body>
</html>
};
}


###############################################################################
# PAGE: DASHBOARD
###############################################################################

sub page_dashboard {
    my $vs = get_service_status('veloserve');
    my $ap = get_service_status('httpd');
    my $active_ws = get_active_webserver();
    my @vhosts = parse_vhosts();
    my $php = parse_php_config();
    my $cache = parse_cache_config();
    my $vhost_count = scalar @vhosts;
    my $ssl_count = scalar grep { $_->{has_ssl} } @vhosts;

    html_header('Dashboard', 'dashboard');

    my $banner_class = $active_ws eq 'veloserve' ? 'success' : ($active_ws eq 'apache' ? 'warning' : 'danger');
    my $banner_text  = $active_ws eq 'veloserve' ? 'VeloServe is serving on ports 80/443'
                     : $active_ws eq 'apache'    ? 'Apache is currently serving (VeloServe inactive)'
                     : 'No web server is active on port 80';

    print qq{
<div class="vs-banner $banner_class">
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>
  $banner_text
</div>

<div class="vs-grid vs-grid-4 mt-6">
  <div class="vs-card"><div class="vs-stat">
    <div class="value">$vhost_count</div><div class="label">Virtual Hosts</div>
  </div></div>
  <div class="vs-card"><div class="vs-stat">
    <div class="value">$ssl_count</div><div class="label">SSL Certificates</div>
  </div></div>
  <div class="vs-card"><div class="vs-stat">
    <div class="value">} . ($php->{version} || 'N/A') . qq{</div><div class="label">PHP Version</div>
  </div></div>
  <div class="vs-card"><div class="vs-stat">
    <div class="value">} . ($cache->{enable} ? 'On' : 'Off') . qq{</div><div class="label">Cache</div>
  </div></div>
</div>

<div class="vs-grid vs-grid-2 mt-6">
  <div class="vs-card">
    <div class="vs-card-header">VeloServe Service</div>
    <div class="vs-card-body">
      <div class="vs-info-row"><div class="key">Status</div><div class="val"><span class="vs-badge } . ($vs->{active} ? 'running' : 'stopped') . qq{">} . ($vs->{active} ? 'Running' : 'Stopped') . qq{</span></div></div>
      <div class="vs-info-row"><div class="key">PID</div><div class="val">} . ($vs->{pid} || '-') . qq{</div></div>
      <div class="vs-info-row"><div class="key">Uptime</div><div class="val">} . ($vs->{uptime} || '-') . qq{</div></div>
      <div class="vs-info-row"><div class="key">Boot Enabled</div><div class="val">} . ($vs->{enabled} ? 'Yes' : 'No') . qq{</div></div>
      <div class="vs-info-row"><div class="key">chkservd Monitored</div><div class="val">} . (get_chkservd_status('veloserve') ? 'Yes' : 'No') . qq{</div></div>
      <div class="vs-btn-group mt-4">
        <button class="vs-btn vs-btn-success vs-btn-sm" onclick="VS.controlServer('start')">Start</button>
        <button class="vs-btn vs-btn-danger vs-btn-sm" onclick="VS.controlServer('stop')">Stop</button>
        <button class="vs-btn vs-btn-warning vs-btn-sm" onclick="VS.controlServer('restart')">Restart</button>
        <button class="vs-btn vs-btn-outline vs-btn-sm" onclick="VS.controlServer('reload')">Reload</button>
      </div>
    </div>
  </div>

  <div class="vs-card">
    <div class="vs-card-header">Apache (httpd)</div>
    <div class="vs-card-body">
      <div class="vs-info-row"><div class="key">Status</div><div class="val"><span class="vs-badge } . ($ap->{active} ? 'running' : 'stopped') . qq{">} . ($ap->{active} ? 'Running' : 'Stopped') . qq{</span></div></div>
      <div class="vs-info-row"><div class="key">PID</div><div class="val">} . ($ap->{pid} || '-') . qq{</div></div>
      <div class="vs-info-row"><div class="key">Uptime</div><div class="val">} . ($ap->{uptime} || '-') . qq{</div></div>
      <div class="vs-info-row"><div class="key">Boot Enabled</div><div class="val">} . ($ap->{enabled} ? 'Yes' : 'No') . qq{</div></div>
      <div class="vs-info-row"><div class="key">chkservd Monitored</div><div class="val">} . (get_chkservd_status('httpd') ? 'Yes' : 'No') . qq{</div></div>
      <div class="mt-4">
        <a href="?action=switch" class="vs-btn vs-btn-primary vs-btn-sm">Switch Web Server &rarr;</a>
      </div>
    </div>
  </div>
</div>

<div class="vs-card mt-6">
  <div class="vs-card-header">Quick Actions</div>
  <div class="vs-card-body vs-btn-group">
    <a href="?action=vhosts" class="vs-btn vs-btn-outline">Manage Virtual Hosts</a>
    <a href="?action=php" class="vs-btn vs-btn-outline">PHP Settings</a>
    <a href="?action=cache" class="vs-btn vs-btn-outline">Cache Management</a>
    <a href="?action=ssl" class="vs-btn vs-btn-outline">SSL/TLS Status</a>
    <a href="?action=config" class="vs-btn vs-btn-outline">Edit Configuration</a>
    <a href="?action=logs" class="vs-btn vs-btn-outline">View Logs</a>
  </div>
</div>
};

    html_footer();
}


###############################################################################
# PAGE: WEB SERVER SWITCH
###############################################################################

sub page_switch {
    my $active_ws = get_active_webserver();
    my $vs_chk = get_chkservd_status('veloserve');
    my $ap_chk = get_chkservd_status('httpd');

    html_header('Switch Web Server', 'switch');

    print qq{
<h1>Switch Web Server</h1>
<p class="text-muted mb-4">Switch between VeloServe and Apache. This updates chkservd monitoring, systemd services, and port bindings.</p>

<div class="vs-switch-box">
  <div class="vs-switch-option } . ($active_ws eq 'veloserve' ? 'current' : '') . qq{">
    <h3>VeloServe</h3>
    <p>High-performance Rust web server with PHP CGI, caching, and SNI TLS.</p>
    <div class="vs-info-row"><div class="key">chkservd</div><div class="val">} . ($vs_chk ? '<span class="vs-badge active">Monitored</span>' : '<span class="vs-badge inactive">Not monitored</span>') . qq{</div></div>
    } . ($active_ws eq 'veloserve'
        ? '<div class="vs-badge success mt-4">Currently Active</div>'
        : '<button class="vs-btn vs-btn-primary vs-btn-lg mt-4" onclick="VS.switchWebServer(\'veloserve\')">Switch to VeloServe</button>')
    . qq{
  </div>
  <div class="vs-switch-option } . ($active_ws eq 'apache' ? 'current' : '') . qq{">
    <h3>Apache (httpd)</h3>
    <p>cPanel default web server with mod_security, EasyApache, and full compatibility.</p>
    <div class="vs-info-row"><div class="key">chkservd</div><div class="val">} . ($ap_chk ? '<span class="vs-badge active">Monitored</span>' : '<span class="vs-badge inactive">Not monitored</span>') . qq{</div></div>
    } . ($active_ws eq 'apache'
        ? '<div class="vs-badge success mt-4">Currently Active</div>'
        : '<button class="vs-btn vs-btn-warning vs-btn-lg mt-4" onclick="VS.switchWebServer(\'apache\')">Switch to Apache</button>')
    . qq{
  </div>
</div>

<div class="vs-card mt-6">
  <div class="vs-card-header">What happens during a switch</div>
  <div class="vs-card-body">
    <table class="vs-table">
      <thead><tr><th>Step</th><th>Switch to VeloServe</th><th>Switch to Apache</th></tr></thead>
      <tbody>
        <tr><td>1</td><td>Import Apache vhosts + SSL certs</td><td>Stop VeloServe</td></tr>
        <tr><td>2</td><td>Set httpd:0 in chkservd</td><td>Set veloserve:0 in chkservd</td></tr>
        <tr><td>3</td><td>Stop Apache</td><td>Set httpd:1 in chkservd</td></tr>
        <tr><td>4</td><td>Start VeloServe on 80/443</td><td>Start Apache on 80/443</td></tr>
        <tr><td>5</td><td>Set veloserve:1 in chkservd</td><td>Restart tailwatchd</td></tr>
        <tr><td>6</td><td>Restart tailwatchd</td><td>-</td></tr>
      </tbody>
    </table>
  </div>
</div>
};

    html_footer();
}


###############################################################################
# PAGE: VIRTUAL HOSTS
###############################################################################

sub page_vhosts {
    my @vhosts = parse_vhosts();

    html_header('Virtual Hosts', 'vhosts');

    print qq{
<h1>Virtual Hosts</h1>
<div class="vs-btn-group mb-4">
  <button class="vs-btn vs-btn-primary" onclick="VS.importApache()">Import from Apache</button>
  <span class="text-muted text-sm" style="align-self:center;margin-left:8px;">} . scalar(@vhosts) . qq{ virtual host(s) configured</span>
</div>

<div class="vs-card">
  <div class="vs-card-body" style="padding:0;overflow-x:auto;">
    <table class="vs-table">
      <thead>
        <tr>
          <th>Domain</th>
          <th>Document Root</th>
          <th>Owner</th>
          <th>Platform</th>
          <th>SSL</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
};

    if (@vhosts) {
        for my $v (@vhosts) {
            my $ssl_badge = $v->{has_ssl}
                ? '<span class="vs-badge success">Active</span>'
                : ($v->{ssl_cert} ? '<span class="vs-badge warning">Cert missing</span>' : '<span class="vs-badge inactive">None</span>');
            my $dom = html_escape($v->{domain});
            my $root = html_escape($v->{root} || '');
            my $owner = html_escape($v->{owner} || '');
            my $plat = html_escape($v->{platform} || '');
            print qq{
        <tr>
          <td><strong>$dom</strong></td>
          <td class="mono text-sm">$root</td>
          <td>$owner</td>
          <td>$plat</td>
          <td>$ssl_badge</td>
          <td><button class="vs-btn vs-btn-danger vs-btn-sm" onclick="VS.deleteVhost('$dom')">Remove</button></td>
        </tr>};
        }
    } else {
        print qq{<tr><td colspan="6" style="text-align:center;padding:24px;color:var(--vs-text-muted);">No virtual hosts configured. Import from Apache or create accounts in cPanel.</td></tr>};
    }

    print qq{
      </tbody>
    </table>
  </div>
</div>
};

    html_footer();
}


###############################################################################
# PAGE: PHP CONFIGURATION
###############################################################################

sub page_php {
    my $php = parse_php_config();
    my @ea_versions = find_ea_php_versions();

    html_header('PHP Configuration', 'php');

    print qq{
<h1>PHP Configuration</h1>

<div class="vs-grid vs-grid-2">
  <div class="vs-card">
    <div class="vs-card-header">Current PHP Settings</div>
    <div class="vs-card-body">
      <div class="vs-info-row"><div class="key">Enabled</div><div class="val">} . ($php->{enable} ? '<span class="vs-badge success">Yes</span>' : '<span class="vs-badge danger">No</span>') . qq{</div></div>
      <div class="vs-info-row"><div class="key">Mode</div><div class="val">} . html_escape($php->{mode} || 'cgi') . qq{</div></div>
      <div class="vs-info-row"><div class="key">Version</div><div class="val">} . html_escape($php->{version} || '-') . qq{</div></div>
      <div class="vs-info-row"><div class="key">Binary Path</div><div class="val mono text-sm">} . html_escape($php->{binary} || 'auto-detect') . qq{</div></div>
      <div class="vs-info-row"><div class="key">Workers</div><div class="val">} . ($php->{workers} || '-') . qq{</div></div>
      <div class="vs-info-row"><div class="key">Memory Limit</div><div class="val">} . html_escape($php->{memory} || '-') . qq{</div></div>
      <div class="vs-info-row"><div class="key">Max Execution Time</div><div class="val">} . ($php->{max_exec} || '-') . qq{s</div></div>
    </div>
  </div>

  <div class="vs-card">
    <div class="vs-card-header">Installed EA-PHP Versions</div>
    <div class="vs-card-body">
      <p class="text-muted text-sm mb-4">Select a PHP version for VeloServe to use. This updates binary_path in veloserve.toml and reloads.</p>
      <table class="vs-table">
        <thead><tr><th>Version</th><th>Binary</th><th>Status</th><th>Action</th></tr></thead>
        <tbody>
};

    for my $v (@ea_versions) {
        my $is_current = ($php->{binary} && $php->{binary} eq $v->{binary}) ? 1 : 0;
        my $status = $v->{installed}
            ? ($is_current ? '<span class="vs-badge success">Active</span>' : '<span class="vs-badge info">Available</span>')
            : '<span class="vs-badge inactive">Not installed</span>';
        my $action = ($v->{installed} && !$is_current)
            ? qq{<button class="vs-btn vs-btn-primary vs-btn-sm" onclick="VS.changePhp('$v->{version}')">Use</button>}
            : ($is_current ? '<span class="text-muted text-sm">In use</span>' : '-');
        print qq{
          <tr>
            <td><strong>EA-PHP $v->{label}</strong></td>
            <td class="mono text-sm">$v->{binary}</td>
            <td>$status</td>
            <td>$action</td>
          </tr>};
    }

    print qq{
        </tbody>
      </table>
    </div>
  </div>
</div>
};

    html_footer();
}


###############################################################################
# PAGE: CACHE MANAGEMENT
###############################################################################

sub page_cache {
    my $cache = parse_cache_config();
    my $disk_usage = '-';
    if ($cache->{disk_path} && -d $cache->{disk_path}) {
        $disk_usage = cmd("du -sh '$cache->{disk_path}' 2>/dev/null | cut -f1") || '0';
    }

    html_header('Cache Management', 'cache');

    print qq{
<h1>Cache Management</h1>

<div class="vs-grid vs-grid-3">
  <div class="vs-card"><div class="vs-stat">
    <div class="value">} . ($cache->{enable} ? 'Enabled' : 'Disabled') . qq{</div>
    <div class="label">Cache Status</div>
  </div></div>
  <div class="vs-card"><div class="vs-stat">
    <div class="value">} . html_escape($cache->{storage} || '-') . qq{</div>
    <div class="label">Storage Type</div>
  </div></div>
  <div class="vs-card"><div class="vs-stat">
    <div class="value">} . html_escape($cache->{memory_limit} || '-') . qq{</div>
    <div class="label">Memory Limit</div>
  </div></div>
</div>

<div class="vs-card mt-6">
  <div class="vs-card-header">Cache Settings</div>
  <div class="vs-card-body">
    <div class="vs-info-row"><div class="key">Storage</div><div class="val">} . html_escape($cache->{storage} || 'memory') . qq{</div></div>
    <div class="vs-info-row"><div class="key">Memory Limit</div><div class="val">} . html_escape($cache->{memory_limit} || '-') . qq{</div></div>
    <div class="vs-info-row"><div class="key">Default TTL</div><div class="val">} . ($cache->{ttl} || '-') . qq{s</div></div>
    <div class="vs-info-row"><div class="key">Disk Path</div><div class="val mono text-sm">} . html_escape($cache->{disk_path} || '-') . qq{</div></div>
    <div class="vs-info-row"><div class="key">Disk Usage</div><div class="val">$disk_usage</div></div>
    <div class="mt-4">
      <button class="vs-btn vs-btn-danger" onclick="VS.purgeCache()">Purge All Cache</button>
    </div>
  </div>
</div>
};

    html_footer();
}


###############################################################################
# PAGE: SSL/TLS STATUS
###############################################################################

sub page_ssl {
    my %global_ssl = parse_ssl_config();
    my @vhosts = parse_vhosts();
    my $hooks_registered = (-f '/usr/local/veloserve/hooks/veloserve-hook.sh') ? 1 : 0;

    html_header('SSL/TLS Status', 'ssl');

    print qq{<h1>SSL / TLS Status</h1>};

    # Global SSL
    print qq{
<div class="vs-card mb-4">
  <div class="vs-card-header">Global SSL Certificate</div>
  <div class="vs-card-body">
};
    if ($global_ssl{cert}) {
        my $info = get_cert_info($global_ssl{cert});
        print qq{
    <div class="vs-info-row"><div class="key">Certificate</div><div class="val mono text-sm">} . html_escape($global_ssl{cert}) . qq{</div></div>
    <div class="vs-info-row"><div class="key">Key</div><div class="val mono text-sm">} . html_escape($global_ssl{key} || '-') . qq{</div></div>
    <div class="vs-info-row"><div class="key">Subject</div><div class="val">} . html_escape($info->{subject} || '-') . qq{</div></div>
    <div class="vs-info-row"><div class="key">Issuer</div><div class="val">} . html_escape($info->{issuer} || '-') . qq{</div></div>
    <div class="vs-info-row"><div class="key">Expires</div><div class="val">} . html_escape($info->{expiry} || '-') . qq{</div></div>
};
    } else {
        print qq{<p class="text-muted">No global [ssl] section configured. Per-vhost SSL is used via SNI.</p>};
    }
    print qq{</div></div>};

    # AutoSSL hooks
    print qq{
<div class="vs-card mb-4">
  <div class="vs-card-header">AutoSSL Integration</div>
  <div class="vs-card-body">
    <div class="vs-info-row"><div class="key">Hook Script</div><div class="val">} . ($hooks_registered ? '<span class="vs-badge success">Installed</span>' : '<span class="vs-badge danger">Not installed</span>') . qq{</div></div>
    <div class="vs-info-row"><div class="key">SSLStorage::add_ssl</div><div class="val">} . ($hooks_registered ? 'Auto-updates cert paths on SSL install' : 'Not configured') . qq{</div></div>
    <p class="text-muted text-sm mt-4">When AutoSSL provisions a Let's Encrypt certificate, the hook automatically updates the matching virtualhost's ssl_certificate/ssl_certificate_key and reloads VeloServe.</p>
  </div>
</div>};

    # Per-vhost SSL table
    print qq{
<div class="vs-card">
  <div class="vs-card-header">Per-Domain SSL Certificates (SNI)</div>
  <div class="vs-card-body" style="padding:0;overflow-x:auto;">
    <table class="vs-table">
      <thead><tr><th>Domain</th><th>Certificate Path</th><th>Issuer</th><th>Expires</th><th>Status</th></tr></thead>
      <tbody>
};

    my $has_any = 0;
    for my $v (@vhosts) {
        next unless $v->{ssl_cert};
        $has_any = 1;
        my $info = get_cert_info($v->{ssl_cert});
        my $status = (-f $v->{ssl_cert})
            ? '<span class="vs-badge success">Valid</span>'
            : '<span class="vs-badge danger">File missing</span>';
        print qq{
      <tr>
        <td><strong>} . html_escape($v->{domain}) . qq{</strong></td>
        <td class="mono text-sm">} . html_escape($v->{ssl_cert}) . qq{</td>
        <td class="text-sm">} . html_escape($info->{issuer} || '-') . qq{</td>
        <td class="text-sm">} . html_escape($info->{expiry} || '-') . qq{</td>
        <td>$status</td>
      </tr>};
    }
    unless ($has_any) {
        print qq{<tr><td colspan="5" style="text-align:center;padding:24px;color:var(--vs-text-muted);">No per-domain SSL certificates configured. Use AutoSSL or import from Apache.</td></tr>};
    }

    print qq{
      </tbody>
    </table>
  </div>
</div>
};

    html_footer();
}


###############################################################################
# PAGE: CONFIGURATION EDITOR
###############################################################################

sub page_config {
    my $config = read_config();

    html_header('Configuration', 'config');

    print qq{
<h1>Configuration Editor</h1>
<p class="text-muted mb-4">Edit <code>$VELOSERVE_CONFIG</code>. Changes are validated and a backup is created before saving.</p>

<div class="vs-card">
  <div class="vs-card-header">
    veloserve.toml
    <div class="vs-btn-group">
      <button class="vs-btn vs-btn-primary vs-btn-sm" onclick="VS.saveConfig()">Save &amp; Reload</button>
      <button class="vs-btn vs-btn-outline vs-btn-sm" onclick="location.reload()">Discard</button>
    </div>
  </div>
  <div class="vs-card-body" style="padding:0;">
    <textarea id="config-editor" class="vs-textarea" style="border:none;border-radius:0;min-height:500px;">} . html_escape($config) . qq{</textarea>
  </div>
</div>
};

    html_footer();
}


###############################################################################
# PAGE: LOGS VIEWER
###############################################################################

sub page_logs {
    html_header('Logs', 'logs');

    my $default_log = '';
    if (-f $ERROR_LOG) {
        $default_log = cmd("tail -100 '$ERROR_LOG' 2>/dev/null") || 'No logs available';
    } else {
        $default_log = 'Log file not found at ' . $ERROR_LOG;
    }

    print qq{
<h1>Log Viewer</h1>

<div class="vs-card">
  <div class="vs-card-header">
    <div style="display:flex;gap:12px;align-items:center;">
      <select id="log-source" class="vs-select" style="width:200px;">
        <option value="error">Error Log</option>
        <option value="hooks">Hook Activity</option>
        <option value="journal">systemd Journal</option>
      </select>
      <select id="log-lines" class="vs-select" style="width:120px;">
        <option value="50">50 lines</option>
        <option value="100" selected>100 lines</option>
        <option value="500">500 lines</option>
      </select>
      <button class="vs-btn vs-btn-primary vs-btn-sm" onclick="VS.refreshLogs()">Refresh</button>
      <button class="vs-btn vs-btn-outline vs-btn-sm" onclick="this.classList.toggle('vs-btn-primary');VS.startLogRefresh()">Auto-refresh</button>
    </div>
  </div>
  <div class="vs-card-body" style="padding:0;">
    <pre id="log-output" class="vs-log">} . html_escape($default_log) . qq{</pre>
  </div>
</div>
};

    html_footer();
}


###############################################################################
# PAGE: ABOUT
###############################################################################

sub page_about {
    my $version = get_veloserve_version();
    my $os = cmd("cat /etc/redhat-release 2>/dev/null || cat /etc/os-release 2>/dev/null | head -1") || 'unknown';
    my $kernel = cmd("uname -r 2>/dev/null") || 'unknown';
    my $cpanel_ver = cmd("cat /usr/local/cpanel/version 2>/dev/null") || 'unknown';

    html_header('About', 'about');

    print qq{
<h1>About VeloServe</h1>

<div class="vs-grid vs-grid-2">
  <div class="vs-card">
    <div class="vs-card-header">VeloServe</div>
    <div class="vs-card-body">
      <div class="vs-info-row"><div class="key">Version</div><div class="val">$version</div></div>
      <div class="vs-info-row"><div class="key">Binary</div><div class="val mono text-sm">$VELOSERVE_BIN</div></div>
      <div class="vs-info-row"><div class="key">Config</div><div class="val mono text-sm">$VELOSERVE_CONFIG</div></div>
      <div class="vs-info-row"><div class="key">GitHub</div><div class="val"><a href="https://github.com/veloserve/veloserve" target="_blank">veloserve/veloserve</a></div></div>
      <div class="vs-info-row"><div class="key">License</div><div class="val">Open Source</div></div>
    </div>
  </div>

  <div class="vs-card">
    <div class="vs-card-header">System</div>
    <div class="vs-card-body">
      <div class="vs-info-row"><div class="key">OS</div><div class="val">} . html_escape($os) . qq{</div></div>
      <div class="vs-info-row"><div class="key">Kernel</div><div class="val">} . html_escape($kernel) . qq{</div></div>
      <div class="vs-info-row"><div class="key">cPanel</div><div class="val">} . html_escape($cpanel_ver) . qq{</div></div>
      <div class="vs-info-row"><div class="key">Hostname</div><div class="val">} . html_escape(cmd("hostname 2>/dev/null") || '-') . qq{</div></div>
    </div>
  </div>
</div>
};

    html_footer();
}


###############################################################################
# API ENDPOINTS
###############################################################################

sub api_status {
    my $vs = get_service_status('veloserve');
    $vs->{version} = get_veloserve_version();
    $vs->{active_webserver} = get_active_webserver();
    print encode_json($vs);
}

sub api_start {
    my $r = system("systemctl start veloserve 2>/dev/null");
    print encode_json({ success => ($r == 0), message => $r == 0 ? 'VeloServe started' : 'Failed to start VeloServe' });
}

sub api_stop {
    my $r = system("systemctl stop veloserve 2>/dev/null");
    print encode_json({ success => ($r == 0), message => $r == 0 ? 'VeloServe stopped' : 'Failed to stop VeloServe' });
}

sub api_restart {
    my $r = system("systemctl restart veloserve 2>/dev/null");
    print encode_json({ success => ($r == 0), message => $r == 0 ? 'VeloServe restarted' : 'Failed to restart VeloServe' });
}

sub api_reload {
    my $r = system("systemctl reload veloserve 2>/dev/null || systemctl restart veloserve 2>/dev/null");
    print encode_json({ success => 1, message => 'Configuration reloaded' });
}

sub api_switch_veloserve {
    my $out = cmd("bash '$SWAP_SCRIPT' --swap 2>&1");
    my $ok = ($out =~ /VeloServe is now serving/) ? 1 : 0;
    print encode_json({ success => $ok, message => $ok ? 'Switched to VeloServe on ports 80/443' : "Switch failed: $out" });
}

sub api_switch_apache {
    my $out = cmd("bash '$SWAP_SCRIPT' --revert 2>&1");
    my $ok = ($out =~ /Reverted to Apache/) ? 1 : 0;
    print encode_json({ success => $ok, message => $ok ? 'Switched back to Apache' : "Revert failed: $out" });
}

sub api_vhost_delete {
    my $domain = $cgi->param('domain') || '';
    if (!$domain) {
        print encode_json({ success => 0, message => 'No domain specified' });
        return;
    }
    # Use python to safely remove the block
    my $r = system("python3 - '$VELOSERVE_CONFIG' '$domain' <<'PYEOF'\n" .
        "import sys, re\n" .
        "cfg, target = sys.argv[1], sys.argv[2]\n" .
        "with open(cfg) as f: c = f.read()\n" .
        "blocks = re.split(r'(?=\\[\\[virtualhost\\]\\])', c)\n" .
        "out = [b for b in blocks if 'domain = \"' + target + '\"' not in b]\n" .
        "with open(cfg, 'w') as f: f.write(''.join(out))\n" .
        "PYEOF\n");
    system("systemctl reload veloserve 2>/dev/null || systemctl restart veloserve 2>/dev/null");
    print encode_json({ success => 1, message => "Removed virtual host: $domain" });
}

sub api_vhost_import {
    my $out = cmd("bash '$SWAP_SCRIPT' --config-only 2>&1");
    my $count = 0;
    $count = $1 if $out =~ /Imported (\d+) virtual/;
    print encode_json({ success => 1, message => "Imported $count virtual host(s) from Apache", imported => $count });
}

sub api_cache_purge {
    if (-d '/var/cache/veloserve') {
        system("rm -rf /var/cache/veloserve/* 2>/dev/null");
    }
    system("systemctl reload veloserve 2>/dev/null || systemctl restart veloserve 2>/dev/null");
    print encode_json({ success => 1, message => 'Cache purged successfully' });
}

sub api_config_save {
    my $content = $cgi->param('content') || '';
    if (!$content) {
        print encode_json({ success => 0, message => 'No content provided' });
        return;
    }
    # Backup first
    my $ts = strftime('%Y%m%d%H%M%S', localtime);
    system("cp -a '$VELOSERVE_CONFIG' '${VELOSERVE_CONFIG}.bak.${ts}' 2>/dev/null");

    open my $fh, '>', $VELOSERVE_CONFIG or do {
        print encode_json({ success => 0, message => "Cannot write config: $!" });
        return;
    };
    print $fh $content;
    close $fh;

    system("systemctl reload veloserve 2>/dev/null || systemctl restart veloserve 2>/dev/null");
    print encode_json({ success => 1, message => "Configuration saved and reloaded (backup: .bak.$ts)" });
}

sub api_php_switch {
    my $version = $cgi->param('version') || '';
    if (!$version || $version !~ /^\d+$/) {
        print encode_json({ success => 0, message => 'Invalid PHP version' });
        return;
    }
    my $binary = "/opt/cpanel/ea-php${version}/root/usr/bin/php-cgi";
    unless (-x $binary) {
        print encode_json({ success => 0, message => "EA-PHP $version not installed at $binary" });
        return;
    }
    my $label = substr($version, 0, 1) . '.' . substr($version, 1);

    # Update binary_path in config
    my $config = read_config();
    if ($config =~ /binary_path\s*=\s*"[^"]*"/) {
        $config =~ s/binary_path\s*=\s*"[^"]*"/binary_path = "$binary"/;
    } else {
        $config =~ s/(\[php\])/$1\nbinary_path = "$binary"/;
    }
    if ($config =~ /version\s*=\s*"[^"]*"/) {
        $config =~ s/version\s*=\s*"[^"]*"/version = "$label"/;
    }

    my $ts = strftime('%Y%m%d%H%M%S', localtime);
    system("cp -a '$VELOSERVE_CONFIG' '${VELOSERVE_CONFIG}.bak.${ts}' 2>/dev/null");
    open my $fh, '>', $VELOSERVE_CONFIG or do {
        print encode_json({ success => 0, message => "Cannot write config: $!" });
        return;
    };
    print $fh $config;
    close $fh;

    system("systemctl reload veloserve 2>/dev/null || systemctl restart veloserve 2>/dev/null");
    print encode_json({ success => 1, message => "Switched to EA-PHP $label ($binary)" });
}

sub api_logs {
    my $source = $cgi->param('source') || 'error';
    my $lines = int($cgi->param('lines') || 100);
    $lines = 500 if $lines > 500;

    my $content = '';
    if ($source eq 'hooks') {
        $content = cmd("tail -$lines '$HOOKS_LOG' 2>/dev/null") || 'No hook logs found';
    } elsif ($source eq 'journal') {
        $content = cmd("journalctl -u veloserve -n $lines --no-pager 2>/dev/null") || 'No journal entries';
    } else {
        $content = cmd("tail -$lines '$ERROR_LOG' 2>/dev/null") || 'No error logs found';
    }

    print encode_json({ content => $content });
}

1;
