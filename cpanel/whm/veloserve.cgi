#!/usr/bin/perl
# VeloServe WHM Plugin - Main Controller
# 
# Provides WHM interface for managing VeloServe web server
# as Apache replacement on cPanel servers.

use strict;
use warnings;
use CGI;
use JSON;
use Cpanel::Logger;

# VeloServe configuration
my $VELOSERVE_BASE = '/usr/local/veloserve';
my $VELOSERVE_CONFIG = '/etc/veloserve';
my $VELOSERVE_BIN = '/usr/local/bin/veloserve';
my $VELOSERVE_PHP_BIN = '/usr/local/bin/veloserve-php';

# CGI object
my $cgi = CGI->new;

# Route requests
my $action = $cgi->param('action') || 'index';

print $cgi->header('application/json') if $action =~ /^(api_|ajax_)/;
print $cgi->header('text/html') unless $action =~ /^(api_|ajax_)/;

# Dispatch
my %routes = (
    'index' => \&show_dashboard,
    'status' => \&show_status,
    'config' => \&show_config,
    'vhosts' => \&show_vhosts,
    'php_pools' => \&show_php_pools,
    'cache' => \&show_cache,
    'logs' => \&show_logs,
    'api_status' => \&api_status,
    'api_start' => \&api_start,
    'api_stop' => \&api_stop,
    'api_restart' => \&api_restart,
    'api_reload' => \&api_reload,
    'api_php_pool_start' => \&api_php_pool_start,
    'api_php_pool_stop' => \&api_php_pool_stop,
    'api_cache_purge' => \&api_cache_purge,
    'api_vhost_add' => \&api_vhost_add,
    'api_vhost_delete' => \&api_vhost_delete,
);

if (exists $routes{$action}) {
    $routes{$action}->();
} else {
    show_dashboard();
}

#------------------------------------------------------------------------------
# Dashboard
#------------------------------------------------------------------------------
sub show_dashboard {
    my $status = get_veloserve_status();
    my $stats = get_veloserve_stats();
    
    print_html_header('VeloServe Dashboard');
    
    print qq{
        <div class="container">
            <h1>VeloServe Web Server</h1>
            <p class="lead">High-performance Apache replacement for cPanel</p>
            
            <!-- Status Card -->
            <div class="row">
                <div class="col-md-6">
                    <div class="panel panel-default">
                        <div class="panel-heading">
                            <h3 class="panel-title">Server Status</h3>
                        </div>
                        <div class="panel-body">
                            <div id="server-status">
                                <span class="status-badge status-$status->{state}">$status->{state}</span>
                                <p>PID: $status->{pid}</p>
                                <p>Uptime: $status->{uptime}</p>
                                <p>Version: $status->{version}</p>
                            </div>
                            <div class="btn-group">
                                <button class="btn btn-success" onclick="controlServer('start')">Start</button>
                                <button class="btn btn-danger" onclick="controlServer('stop')">Stop</button>
                                <button class="btn btn-warning" onclick="controlServer('restart')">Restart</button>
                                <button class="btn btn-info" onclick="controlServer('reload')">Reload</button>
                            </div>
                        </div>
                    </div>
                </div>
                
                <div class="col-md-6">
                    <div class="panel panel-default">
                        <div class="panel-heading">
                            <h3 class="panel-title">Statistics</h3>
                        </div>
                        <div class="panel-body">
                            <div class="stat-item">
                                <span class="stat-label">Active Connections:</span>
                                <span class="stat-value">$stats->{connections}</span>
                            </div>
                            <div class="stat-item">
                                <span class="stat-label">Requests/sec:</span>
                                <span class="stat-value">$stats->{rps}</span>
                            </div>
                            <div class="stat-item">
                                <span class="stat-label">Cache Hit Rate:</span>
                                <span class="stat-value">$stats->{cache_hit_rate}%</span>
                            </div>
                            <div class="stat-item">
                                <span class="stat-label">Virtual Hosts:</span>
                                <span class="stat-value">$stats->{vhosts}</span>
                            </div>
                            <div class="stat-item">
                                <span class="stat-label">PHP Workers:</span>
                                <span class="stat-value">$stats->{php_workers}</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            
            <!-- Quick Actions -->
            <div class="row">
                <div class="col-md-12">
                    <div class="panel panel-default">
                        <div class="panel-heading">
                            <h3 class="panel-title">Quick Actions</h3>
                        </div>
                        <div class="panel-body">
                            <a href="?action=vhosts" class="btn btn-primary">Manage Virtual Hosts</a>
                            <a href="?action=php_pools" class="btn btn-primary">PHP Worker Pools</a>
                            <a href="?action=cache" class="btn btn-primary">Cache Management</a>
                            <a href="?action=config" class="btn btn-primary">Configuration</a>
                            <button class="btn btn-warning" onclick="purgeCache()">Purge All Cache</button>
                        </div>
                    </div>
                </div>
            </div>
            
            <!-- Recent Logs -->
            <div class="row">
                <div class="col-md-12">
                    <div class="panel panel-default">
                        <div class="panel-heading">
                            <h3 class="panel-title">Recent Logs</h3>
                        </div>
                        <div class="panel-body">
                            <pre id="recent-logs">} . get_recent_logs(20) . qq{</pre>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <script>
        function controlServer(action) {
            if (!confirm('Are you sure you want to ' + action + ' VeloServe?')) return;
            
            fetch('?action=api_' + action)
                .then(r => r.json())
                .then(data => {
                    alert(data.message);
                    location.reload();
                })
                .catch(e => alert('Error: ' + e));
        }
        
        function purgeCache() {
            if (!confirm('Purge all cache?')) return;
            
            fetch('?action=api_cache_purge')
                .then(r => r.json())
                .then(data => {
                    alert(data.message);
                });
        }
        </script>
    };
    
    print_html_footer();
}

#------------------------------------------------------------------------------
# Virtual Hosts Management
#------------------------------------------------------------------------------
sub show_vhosts {
    my $vhosts = get_vhosts_list();
    
    print_html_header('Virtual Hosts');
    
    print qq{
        <div class="container">
            <h1>Virtual Hosts</h1>
            
            <div class="panel panel-default">
                <div class="panel-heading">
                    <h3 class="panel-title">Configured Virtual Hosts</h3>
                </div>
                <div class="panel-body">
                    <table class="table table-striped">
                        <thead>
                            <tr>
                                <th>Domain</th>
                                <th>Document Root</th>
                                <th>User</th>
                                <th>Status</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
    };
    
    for my $vhost (@$vhosts) {
        print qq{
                            <tr>
                                <td>$vhost->{domain}</td>
                                <td>$vhost->{document_root}</td>
                                <td>$vhost->{user}</td>
                                <td><span class="badge badge-$vhost->{status}">$vhost->{status}</span></td>
                                <td>
                                    <button class="btn btn-sm btn-danger" onclick="deleteVhost('$vhost->{domain}')">Delete</button>
                                </td>
                            </tr>
        };
    }
    
    print qq{
                        </tbody>
                    </table>
                </div>
            </div>
            
            <div class="panel panel-default">
                <div class="panel-heading">
                    <h3 class="panel-title">Import from Apache</h3>
                </div>
                <div class="panel-body">
                    <p>Import existing Apache virtual hosts into VeloServe.</p>
                    <button class="btn btn-primary" onclick="importApacheVhosts()">Import Apache Configs</button>
                </div>
            </div>
        </div>
        
        <script>
        function deleteVhost(domain) {
            if (!confirm('Delete virtual host: ' + domain + '?')) return;
            
            fetch('?action=api_vhost_delete&domain=' + encodeURIComponent(domain))
                .then(r => r.json())
                .then(data => {
                    alert(data.message);
                    location.reload();
                });
        }
        
        function importApacheVhosts() {
            fetch('?action=api_vhost_import')
                .then(r => r.json())
                .then(data => {
                    alert('Imported ' + data.imported + ' virtual hosts');
                    location.reload();
                });
        }
        </script>
    };
    
    print_html_footer();
}

#------------------------------------------------------------------------------
# PHP Pools Management
#------------------------------------------------------------------------------
sub show_php_pools {
    my $pools = get_php_pools();
    
    print_html_header('PHP Worker Pools');
    
    print qq{
        <div class="container">
            <h1>PHP Worker Pools</h1>
            
            <div class="row">
                <div class="col-md-12">
                    <div class="panel panel-default">
                        <div class="panel-heading">
                            <h3 class="panel-title">Active PHP Pools</h3>
                        </div>
                        <div class="panel-body">
                            <table class="table table-striped">
                                <thead>
                                    <tr>
                                        <th>Socket</th>
                                        <th>User</th>
                                        <th>Workers</th>
                                        <th>Busy</th>
                                        <th>Memory Limit</th>
                                        <th>Status</th>
                                        <th>Actions</th>
                                    </tr>
                                </thead>
                                <tbody>
    };
    
    for my $pool (@$pools) {
        print qq{
                                    <tr>
                                        <td>$pool->{socket}</td>
                                        <td>$pool->{user}</td>
                                        <td>$pool->{total_workers}</td>
                                        <td>$pool->{busy_workers}</td>
                                        <td>$pool->{memory_limit}</td>
                                        <td><span class="badge badge-$pool->{status}">$pool->{status}</span></td>
                                        <td>
                                            <button class="btn btn-sm btn-success" onclick="controlPool('$pool->{socket}', 'start')">Start</button>
                                            <button class="btn btn-sm btn-danger" onclick="controlPool('$pool->{socket}', 'stop')">Stop</button>
                                            <button class="btn btn-sm btn-warning" onclick="controlPool('$pool->{socket}', 'restart')">Restart</button>
                                        </td>
                                    </tr>
        };
    }
    
    print qq{
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>
            </div>
            
            <div class="panel panel-default">
                <div class="panel-heading">
                    <h3 class="panel-title">Create New Pool</h3>
                </div>
                <div class="panel-body">
                    <form id="create-pool-form" class="form-horizontal">
                        <div class="form-group">
                            <label class="col-sm-2 control-label">cPanel User:</label>
                            <div class="col-sm-4">
                                <input type="text" name="user" class="form-control" placeholder="username">
                            </div>
                        </div>
                        <div class="form-group">
                            <label class="col-sm-2 control-label">Workers:</label>
                            <div class="col-sm-4">
                                <input type="number" name="workers" class="form-control" value="8" min="1" max="64">
                            </div>
                        </div>
                        <div class="form-group">
                            <label class="col-sm-2 control-label">Memory Limit:</label>
                            <div class="col-sm-4">
                                <input type="text" name="memory" class="form-control" value="256M">
                            </div>
                        </div>
                        <div class="form-group">
                            <div class="col-sm-offset-2 col-sm-4">
                                <button type="button" class="btn btn-primary" onclick="createPool()">Create Pool</button>
                            </div>
                        </div>
                    </form>
                </div>
            </div>
        </div>
        
        <script>
        function controlPool(socket, action) {
            fetch('?action=api_php_pool_' + action + '&socket=' + encodeURIComponent(socket))
                .then(r => r.json())
                .then(data => {
                    alert(data.message);
                    location.reload();
                });
        }
        
        function createPool() {
            const form = document.getElementById('create-pool-form');
            const formData = new FormData(form);
            
            fetch('?action=api_php_pool_create', {
                method: 'POST',
                body: formData
            })
            .then(r => r.json())
            .then(data => {
                alert(data.message);
                location.reload();
            });
        }
        </script>
    };
    
    print_html_footer();
}

#------------------------------------------------------------------------------
# API Endpoints
#------------------------------------------------------------------------------
sub api_status {
    print encode_json(get_veloserve_status());
}

sub api_start {
    my $result = system("$VELOSERVE_BIN start");
    print encode_json({ success => $result == 0, message => $result == 0 ? 'VeloServe started' : 'Failed to start' });
}

sub api_stop {
    my $result = system("$VELOSERVE_BIN stop");
    print encode_json({ success => $result == 0, message => $result == 0 ? 'VeloServe stopped' : 'Failed to stop' });
}

sub api_restart {
    my $result = system("$VELOSERVE_BIN restart");
    print encode_json({ success => $result == 0, message => $result == 0 ? 'VeloServe restarted' : 'Failed to restart' });
}

sub api_reload {
    my $result = system("$VELOSERVE_BIN reload");
    print encode_json({ success => $result == 0, message => $result == 0 ? 'Configuration reloaded' : 'Failed to reload' });
}

sub api_cache_purge {
    my $result = system("$VELOSERVE_BIN cache purge --all");
    print encode_json({ success => $result == 0, message => $result == 0 ? 'Cache purged' : 'Failed to purge cache' });
}

#------------------------------------------------------------------------------
# Helper Functions
#------------------------------------------------------------------------------
sub get_veloserve_status {
    # Read from PID file or status socket
    my $pid_file = '/var/run/veloserve.pid';
    my $pid = -f $pid_file ? `cat $pid_file` : 0;
    chomp $pid;
    
    my $running = $pid && kill(0, $pid);
    
    return {
        state => $running ? 'running' : 'stopped',
        pid => $pid || 0,
        uptime => $running ? '2h 15m' : '0m',
        version => '1.0.0',
    };
}

sub get_veloserve_stats {
    # Mock data for now - would query VeloServe API
    return {
        connections => 42,
        rps => 156,
        cache_hit_rate => 87,
        vhosts => 15,
        php_workers => 24,
    };
}

sub get_vhosts_list {
    # Read from VeloServe config
    my @vhosts = (
        { domain => 'example.com', document_root => '/home/user1/public_html', user => 'user1', status => 'active' },
        { domain => 'test.com', document_root => '/home/user2/public_html', user => 'user2', status => 'active' },
    );
    return \@vhosts;
}

sub get_php_pools {
    # Query veloserve-php instances
    my @pools = (
        { socket => '/run/veloserve/php.sock', user => 'nobody', total_workers => 8, busy_workers => 2, memory_limit => '256M', status => 'active' },
        { socket => '/run/veloserve/user1.sock', user => 'user1', total_workers => 4, busy_workers => 0, memory_limit => '128M', status => 'active' },
    );
    return \@pools;
}

sub get_recent_logs {
    my ($lines) = @_;
    my $log_file = '/var/log/veloserve/error.log';
    
    if (-f $log_file) {
        return `tail -$lines $log_file 2>/dev/null` || 'No logs available';
    }
    return 'Log file not found';
}

sub print_html_header {
    my ($title) = @_;
    
    print qq{<!DOCTYPE html>
<html>
<head>
    <title>$title - VeloServe</title>
    <link rel="stylesheet" href="https://maxcdn.bootstrapcdn.com/bootstrap/3.3.7/css/bootstrap.min.css">
    <style>
        .status-badge { padding: 5px 10px; border-radius: 4px; font-weight: bold; }
        .status-running { background: #5cb85c; color: white; }
        .status-stopped { background: #d9534f; color: white; }
        .stat-item { margin: 10px 0; }
        .stat-label { font-weight: bold; display: inline-block; width: 200px; }
        .stat-value { font-size: 1.2em; }
        #recent-logs { max-height: 300px; overflow-y: auto; background: #f5f5f5; padding: 10px; }
    </style>
</head>
<body>
    <nav class="navbar navbar-default">
        <div class="container">
            <div class="navbar-header">
                <a class="navbar-brand" href="?action=index">VeloServe</a>
            </div>
            <ul class="nav navbar-nav">
                <li><a href="?action=index">Dashboard</a></li>
                <li><a href="?action=vhosts">Virtual Hosts</a></li>
                <li><a href="?action=php_pools">PHP Pools</a></li>
                <li><a href="?action=cache">Cache</a></li>
                <li><a href="?action=config">Config</a></li>
            </ul>
        </div>
    </nav>
};
}

sub print_html_footer {
    print qq{
    <script src="https://ajax.googleapis.com/ajax/libs/jquery/1.12.4/jquery.min.js"></script>
    <script src="https://maxcdn.bootstrapcdn.com/bootstrap/3.3.7/js/bootstrap.min.js"></script>
</body>
</html>
};
}

1;
