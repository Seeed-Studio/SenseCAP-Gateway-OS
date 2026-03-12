'use strict';
'require view';
'require form';
'require fs';
'require ui';
'require uci';
'require network';

return view.extend({
    load: function() {
        return Promise.all([
            network.getDevices(),
            uci.load('network'),
            uci.load('hardware')
        ]);
    },

    render: function(data) {
        var m, s, o;

        m = new form.Map('network', _('LTE Configuration'),
            _('Configure 4G/LTE modem settings'));

        s = m.section(form.NamedSection, 'LTE', 'interface', _('LTE Settings'));
        s.addremove = false;

        // Enable 4G/LTE
        o = s.option(form.Flag, 'enable', _('Enable 4G/LTE'),
            _('Enable or disable 4G/LTE connection'));
        o.default = '1';
        o.rmempty = false;

        // Restart Button (only shown when 4G/LTE is enabled)
        o = s.option(form.Button, '_restart_network', _('Restart'));
        o.inputtitle = _('Restart Connection');
        o.inputstyle = 'action';
        o.depends('enable', '1');
        o.onclick = L.bind(function(ev) {
            var btn = ev.target;
            btn.disabled = true;
            btn.innerText = _('Restarting...');

            return fs.exec('/sbin/ifdown', ['LTE'])
                .then(function() {
                    return new Promise(function(resolve) {
                        setTimeout(resolve, 2000);
                    });
                })
                .then(function() {
                    return fs.exec('/sbin/ifup', ['LTE']);
                })
                .then(function() {
                    ui.addNotification(null, E('p', _('Connection restarted successfully')), 'info');
                    btn.disabled = false;
                    btn.innerText = _('Restart Connection');
                })
                .catch(function(err) {
                    ui.addNotification(null, E('p', _('Failed to restart connection: ') + (err.message || err)), 'error');
                    btn.disabled = false;
                    btn.innerText = _('Restart Connection');
                });
        }, this);

        // Protocol
        o = s.option(form.Value, 'proto', _('Protocol'));
        o.default = 'qmi';
        o.readonly = true;
        o.cfgvalue = function(section_id) {
            return 'QMI Cellular';
        };

        // Device
        o = s.option(form.ListValue, 'device', _('Device'));
        o.value('/dev/cdc-wdm0', '/dev/cdc-wdm0');
        o.default = '/dev/cdc-wdm0';

        // APN Settings Header
        o = s.option(form.DummyValue, '_apn_header');
        o.rawhtml = true;
        o.cfgvalue = function() {
            return '<h3 style="margin-top:20px;padding-top:10px;">APN Settings</h3>';
        };

        // APN
        o = s.option(form.Value, 'apn', _('APN'),
            _('Access Point Name for your carrier'));

        // Username
        o = s.option(form.Value, 'username', _('User'),
            _('Username for APN authentication'));
        o.placeholder = '';

        // Authentication Type
        o = s.option(form.ListValue, 'auth', _('Auth'),
            _('Authentication protocol'));
        o.value('none', _('None'));
        o.value('pap', 'PAP');
        o.value('chap', 'CHAP');
        o.value('both', 'PAP/CHAP');
        o.default = 'none';

        // Password
        o = s.option(form.Value, 'password', _('Password'),
            _('Password for APN authentication'));
        o.password = true;
        o.placeholder = '';

        // PIN Code
        o = s.option(form.Value, 'pincode', _('Pincode'),
            _('SIM card PIN code (leave empty if not required)'));
        o.placeholder = '';

        // PDP Type
        o = s.option(form.ListValue, 'pdptype', _('PDP Type'));
        o.value('ipv4v6', 'IPv4/IPv6');
        o.value('ipv4', 'IPv4');
        o.value('ipv6', 'IPv6');
        o.default = 'ipv4';

        // Modem Control Buttons
        o = s.option(form.DummyValue, '_modem_control');
        o.rawhtml = true;
        o.cfgvalue = function() {
            return '<h3 style="margin-top:20px;padding-top:10px;">Modem Control</h3>';
        };

        // Reset Modem Button
        o = s.option(form.Button, '_reset_modem', _('Reset Modem'));
        o.inputtitle = _('Reset and Reconnect');
        o.inputstyle = 'apply';
        o.onclick = L.bind(function(ev) {
            var btn = ev.target;
            btn.disabled = true;
            btn.innerText = _('Resetting module...');

            return fs.exec('/sbin/ifdown', ['LTE'])
                .then(function() {
                    btn.innerText = _('Waiting for modem ready...');
                    // Send AT+CFUN=0 to disable RF
                    return fs.exec('/bin/sh', ['-c', 'echo -e \"AT+CFUN=0\\r\" > ' + (uci.get('hardware', 'hardware', 'lte_usb_port') || '/dev/ttyUSB2')]);
                })
                .then(function() {
                    // Wait 5 seconds
                    return new Promise(function(resolve) {
                        setTimeout(resolve, 5000);
                    });
                })
                .then(function() {
                    // Send AT+CFUN=1 to enable RF
                    return fs.exec('/bin/sh', ['-c', 'echo -e \"AT+CFUN=1\\r\" > ' + (uci.get('hardware', 'hardware', 'lte_usb_port') || '/dev/ttyUSB2')]);
                })
                .then(function() {
                    // Wait 10 seconds for modem to be ready
                    return new Promise(function(resolve) {
                        setTimeout(resolve, 10000);
                    });
                })
                .then(function() {
                    btn.innerText = _('Reconnecting network...');
                    // Wait 2 more seconds before ifup
                    return new Promise(function(resolve) {
                        setTimeout(resolve, 2000);
                    });
                })
                .then(function() {
                    return fs.exec('/sbin/ifup', ['LTE']);
                })
                .then(function() {
                    ui.addNotification(null, E('p', _('Modem reset successfully. Network reconnected.')), 'info');
                    btn.disabled = false;
                    btn.innerText = _('Reset and Reconnect');
                })
                .catch(function(err) {
                    ui.addNotification(null, E('p', _('Failed to reset modem: ') + (err.message || err)), 'error');
                    btn.disabled = false;
                    btn.innerText = _('Reset and Reconnect');
                });
        }, this);

        return m.render();
    },

    // Override handleSaveApply to set disabled=1 when config changes
    handleSaveApply: function(ev, mode) {
        var self = this;

        // Set disabled=1 before saving (will be set to 0 by lte-serve after initialization)
        uci.set('network', 'LTE', 'disabled', '1');

        // Call parent's handleSaveApply
        return this.super('handleSaveApply', [ev, mode])
            .then(function() {
                // Restart lte-serve service
                return fs.exec('/etc/init.d/lte-serve', ['restart']);
            })
            .then(function() {
                ui.addNotification(null, E('p', _('LTE service restarted')), 'info');
            })
            .catch(function(err) {
                console.error('Failed to restart lte-serve:', err);
            });
    }
});
